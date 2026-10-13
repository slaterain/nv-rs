//! `fallout/ai/combatcontroller.cpp` (Xbox PDB source unit), subsystem `fallout/ai`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit is translated in address order over several sessions. Done so
//! far: `0097d3a0` to `00980770` (the first 60 functions of the queue). The
//! next session continues at `00980790`.
//!
//! Notes for the next session:
//! - `CombatController` derives from `TESPackage`; the PC `TESForm` is 0x10
//!   bytes smaller than the Xbox PDB's, so every field sits 0x10 lower than
//!   the PDB says. This is confirmed by the code of this unit (the heights
//!   from `Actor::GetHeight`/`GetSneakHeight` at +0xcc/+0xd0, the weapon from
//!   `Actor::GetCurrentWeapon` at +0xf0, the procedures at +0x84/+0x88, the
//!   power-attack arrays at +0xfc/+0x110). PC size 0x188 (`operator new(0x188)`
//!   before the constructor at `00670c2f` and `009921c1`).
//! - The build is unoptimised (`this` in `[EBP-4]`, every field re-read from
//!   memory); the translations re-read fields where the code does.
//! - Functions with a C++ exception-unwinding frame (`FS:[0]`) carry a
//!   `Translation incomplete` marker: the unwinding is not translated. The
//!   stack probe `__alloca_probe` (`00ec8660`) of the functions that keep a
//!   `CombatPlanner` on the stack is left out; the planner is a
//!   [`Engine::with_stack`] block.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::{BSSimpleArray, NiPoint3};
use crate::units::fallout_ai::highprocess::CombatTimer;

layout! {
    /// `CombatController` (Xbox PDB), 0x188 bytes on PC. Field names from the
    /// Xbox PDB, offsets 0x10 lower than the PDB's (see the module notes).
    pub struct CombatController: 0x188 {
        /// `pCombatGroup` (Xbox PDB): `CombatGroup*`.
        0x80 pCombatGroup: Ptr,
        /// `pActionProcedure` (Xbox PDB): `CombatProcedure*`.
        0x84 pActionProcedure: Ptr,
        /// `pMovementProcedure` (Xbox PDB): `CombatProcedure*`.
        0x88 pMovementProcedure: Ptr,
        /// `BackgroundProcedures` (Xbox PDB):
        /// `BSSimpleArray<CombatProcedure *, 1024>`.
        0x8c BackgroundProcedures: Inline<BSSimpleArray>,
        /// `pCombatState` (Xbox PDB): `CombatState*`.
        0x9c pCombatState: Ptr,
        /// `pCombatPlan` (Xbox PDB): `CombatPlan*`.
        0xa0 pCombatPlan: Ptr,
        /// `pCombatPlannerData` (Xbox PDB): `CombatPlannerData*`.
        0xa4 pCombatPlannerData: Ptr,
        /// `pCurrentAction` (Xbox PDB): `CombatAction*`.
        0xa8 pCurrentAction: Ptr,
        /// `pCurrentGoal` (Xbox PDB): `CombatGoal*`.
        0xac pCurrentGoal: Ptr,
        /// `iCurrentActionValue` (Xbox PDB).
        0xb0 iCurrentActionValue: i32,
        /// `PlanTimer` (Xbox PDB): a `CombatTimer`.
        0xb4 PlanTimer: Inline<CombatTimer>,
        /// `pAttacker` (Xbox PDB): `Actor*`.
        0xbc pAttacker: Ptr,
        /// `pTarget` (Xbox PDB): `Actor*`.
        0xc0 pTarget: Ptr,
        /// `bForceCombat` (Xbox PDB).
        0xc4 bForceCombat: bool,
        /// `bUpdatedDetection` (Xbox PDB).
        0xc5 bUpdatedDetection: bool,
        /// `cUseIronSights` (Xbox PDB): a signed byte, -1 after construction.
        0xc6 cUseIronSights: i8,
        /// `bForceSneak` (Xbox PDB).
        0xc7 bForceSneak: bool,
        /// `iMoveMode` (Xbox PDB).
        0xc8 iMoveMode: u32,
        /// `fAttackerHeight` (Xbox PDB).
        0xcc fAttackerHeight: f32,
        /// `fAttackerSneakHeight` (Xbox PDB).
        0xd0 fAttackerSneakHeight: f32,
        /// `bAttackerCanSneak` (Xbox PDB): the byte `008eea60` returns.
        0xd4 bAttackerCanSneak: u8,
        /// `bAttackerCanMove` (Xbox PDB).
        0xd5 bAttackerCanMove: bool,
        /// `fWalkSpeedMult` (Xbox PDB).
        0xd8 fWalkSpeedMult: f32,
        /// `fRunSpeedMult` (Xbox PDB).
        0xdc fRunSpeedMult: f32,
        /// `fCombatPosition` (Xbox PDB).
        0xe0 fCombatPosition: f32,
        /// `bFirstUpdate` (Xbox PDB): set once the attacker information
        /// has been initialised (`0097d990`).
        0xe4 bFirstUpdate: bool,
        /// `spDebugGeometry` (Xbox PDB): `NiPointer<NiNode>`.
        0xe8 spDebugGeometry: Ptr,
        /// `spCellDebugGeometry` (Xbox PDB): `NiPointer<NiNode>`.
        0xec spCellDebugGeometry: Ptr,
        /// `pCurrentWeapon` (Xbox PDB): `TESObjectWEAP*`.
        0xf0 pCurrentWeapon: Ptr,
        /// `pCurrentCombatStyle` (Xbox PDB): `TESCombatStyle*`.
        0xf4 pCurrentCombatStyle: Ptr,
        /// `bDestructing` (Xbox PDB).
        0xf8 bDestructing: bool,
        /// `bDeterminedPowerAttackDeltas` (Xbox PDB).
        0xf9 bDeterminedPowerAttackDeltas: bool,
        /// `fPowerAttackRanges` (Xbox PDB): five floats.
        0xfc fPowerAttackRanges: f32,
        /// `fPowerAttackHitTimes` (Xbox PDB): five floats.
        0x110 fPowerAttackHitTimes: f32,
        /// `bHasForwardAttacks` (Xbox PDB).
        0x124 bHasForwardAttacks: bool,
        /// `bStopCombat` (Xbox PDB).
        0x125 bStopCombat: bool,
        /// `bStoppingCombat` (Xbox PDB).
        0x126 bStoppingCombat: bool,
        /// `LookAtTarget` (Xbox PDB): two `NiPoint3`.
        0x128 LookAtTarget: Inline<NiPoint3>,
        /// `cLookAtTargetPriority` (Xbox PDB): two bytes.
        0x140 cLookAtTargetPriority: u8,
        /// `cIgnoreLookAtTarget` (Xbox PDB).
        0x142 cIgnoreLookAtTarget: u8,
        /// `eLowState` (Xbox PDB).
        0x144 eLowState: u32,
        /// `fLowMovementDelta` (Xbox PDB).
        0x148 fLowMovementDelta: f32,
        /// `FailedCombatProcedures` (Xbox PDB): 0x3c bytes.
        0x14c FailedCombatProcedures: u32,
    }
}

/// `CombatController`'s vtable (RTTI `.?AVCombatController@@`).
pub(crate) const COMBAT_CONTROLLER_VTABLE: u32 = 0x0108_ce74;
/// `CombatPlanner`'s vtable (RTTI `.?AVCombatPlanner@@`).
pub(crate) const COMBAT_PLANNER_VTABLE: u32 = 0x0108_cfd8;
/// The room the stack-held `CombatPlanner` has in `0097da50` and `0097fb80`
/// (from its stack slot to the next local).
pub(crate) const COMBAT_PLANNER_STACK_SIZE: u32 = 0x21dc;

/// `operator new` (cdecl, size).
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `operator delete` (cdecl, block).
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// The `memset`-shaped helper `00403d30` (cdecl: destination, byte, count).
const FILL_BYTES: u32 = 0x0040_3d30;
/// `_ftol2` (`00ec62c0`): truncates ST0 (a leading `f64` argument).
const FTOL: u32 = 0x00ec_62c0;
/// `__RTDynamicCast` (cdecl: object, vfdelta, source type, target type,
/// is-reference).
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;

/// The getter `0097ae90` (in `combatactions.cpp`): returns the word at
/// +0xbc of its `this`, the controller's attacker.
const GET_ATTACKER: u32 = 0x0097_ae90;
/// The getter `004030b0`: returns the word at +0xc0 of its `this`, the
/// controller's target.
const GET_TARGET: u32 = 0x0040_30b0;
/// The getter `008d8520`: returns the word at +0x68 of its `this` (an
/// actor's process).
const GET_PROCESS: u32 = 0x008d_8520;
/// `BSSimpleArray` element count (`0044ddc0`, `this` only).
const ARRAY_SIZE: u32 = 0x0044_ddc0;
/// `BSSimpleArray` element address (`006a7ad0`, `this`, index).
const ARRAY_ELEMENT: u32 = 0x006a_7ad0;
/// `BSSimpleArray<..>::_MoveItems` (Xbox PDB name of the folded body
/// `009a4320`; `this`, index, count): removes elements.
const ARRAY_REMOVE: u32 = 0x009a_4320;
/// `BSSimpleArray<..>::AddUninitialized` (Xbox PDB name of the folded body
/// `007cb2e0`; `this`, pointer to the element).
const ARRAY_ADD: u32 = 0x007c_b2e0;
/// `Actor::GetBlocked` (Xbox PDB).
const ACTOR_GET_BLOCKED: u32 = 0x0089_4d60;
/// `Actor::SetBlock` (Xbox PDB, one word).
const ACTOR_SET_BLOCK: u32 = 0x0089_4cc0;
/// `Actor::GetAnimGroup` (Xbox PDB, four words, 16-bit result).
const ACTOR_GET_ANIM_GROUP: u32 = 0x0089_7910;
/// `Actor::SetIronSights` (Xbox PDB, three words).
const ACTOR_SET_IRON_SIGHTS: u32 = 0x008b_b650;
/// The map getter `00497280` (`this` only) applied to the result of the
/// actor's vtable slot 0x1e4.
const ANIM_MAP_OF: u32 = 0x0049_7280;
/// `NiTMapBase<..,unsigned short, AnimSequenceBase *>::GetAt` (Xbox PDB;
/// `this`, key, pointer to the value).
const ANIM_MAP_GET_AT: u32 = 0x0049_c390;
/// `TESAnimGroup::IsPowerAttackAction` (Xbox PDB, cdecl, one word).
const IS_POWER_ATTACK_ACTION: u32 = 0x005f_2670;
/// `Animation::ZeroGlobalTransform` (Xbox PDB name of `0048f7f0`).
const ANIM_GROUP_OF: u32 = 0x0048_f7f0;
/// `TESAnimGroup::GetTime` (Xbox PDB, one word, ST0 result).
const ANIM_GROUP_GET_TIME: u32 = 0x005f_3780;
/// `TESObjectREFR::GetScale` (Xbox PDB, ST0 result).
const REFR_GET_SCALE: u32 = 0x0056_7400;
/// The `CombatTimer` start `008d7f40` (`this`, float).
const COMBAT_TIMER_START: u32 = 0x008d_7f40;
/// `PlayerCharacter::ChangePerceivedActorHostileStatus` (Xbox PDB, two
/// words).
const CHANGE_PERCEIVED_HOSTILE_STATUS: u32 = 0x0096_7220;
/// The combat-group call `00986c60` (one word) whose result the code passes
/// to `00980830`.
const GROUP_CHOOSE_TARGET: u32 = 0x0098_6c60;
/// `00980830` in this unit (not translated yet; one word).
const SET_TARGET_00980830: u32 = 0x0098_0830;
/// `0097f200` (one `this`).
const AFTER_TARGET_CHANGE_0097F200: u32 = 0x0097_f200;
/// `0097f270` (`this`, procedure).
const RELEASE_PROCEDURE_0097F270: u32 = 0x0097_f270;
/// `CombatPlanner` planning call `00994bd0` (`this`, controller, flag).
const PLANNER_PLAN: u32 = 0x0099_4bd0;
/// `0084e3a0` on a `CombatPlan` (`this` only).
const PLAN_GOAL_0084E3A0: u32 = 0x0084_e3a0;
/// `00994050` on a `CombatPlan` (`this`, controller).
const PLAN_START_00994050: u32 = 0x0099_4050;

/// The `PlayerCharacter` pointer (the `this` of
/// `PlayerCharacter::ChangePerceivedActorHostileStatus` here).
const PLAYER_CHARACTER: u32 = 0x011d_ea3c;
/// The save/load game singleton pointer.
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
/// An object pointer the destructor asks `004226e0`.
const OBJECT_011C3F2C: u32 = 0x011c_3f2c;
/// An object pointer the destructor asks `0047c850`.
const OBJECT_011DE45C: u32 = 0x011d_e45c;
/// The combat manager pointer (`this` of
/// `CombatManager::SplitPlayerCombatGroup`).
const COMBAT_MANAGER: u32 = 0x011f_1958;
/// The global object `00483710` runs on around planning.
const OBJECT_011F15CC: u32 = 0x011f_15cc;
/// A running count the constructor increments and the destructor
/// decrements (never below 0).
const CONTROLLER_COUNT: u32 = 0x011f_1590;
/// The byte that gates the update (`0097da50` returns at once when it is 0;
/// 1 in the exe's data).
const UPDATE_ENABLED: u32 = 0x011a_4880;
/// A `NiPoint3` the power-attack queries start their result from (zero in
/// the exe's data).
const POINT_011F426C: u32 = 0x011f_426c;
/// `0.1` as a `double` (`0x3fb99999a0000000`, the float 0.1 widened).
const ONE_TENTH: u32 = 0x0101_ffa0;
/// `0.1` as a `float`.
const PLAN_TIMER_DELAY: u32 = 0x0101_e2bc;
/// `-1.0` as a `float`.
const MINUS_ONE: u32 = 0x0101_2054;
/// `128.0` as a `float`.
const DEFAULT_HEIGHT: u32 = 0x0101_e704;
/// `64.0` as a `float`.
const DEFAULT_SNEAK_HEIGHT: u32 = 0x0101_e6ec;
/// `0.0` as a `double`.
const ZERO: u32 = 0x0101_2060;
/// `1000.0` as a `double`.
const ELAPSED_LIMIT: u32 = 0x0101_7b70;

/// What `FLD dword` followed by `FSTP dword` leaves of a float: a
/// signalling NaN comes back quiet (bit 22 set), every other value
/// unchanged.
fn x87_quiet(value: f32) -> f32 {
    let bits = value.to_bits();
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        f32::from_bits(bits | 0x0040_0000)
    } else {
        value
    }
}

/// The attacker through `0097ae90`, as the code fetches it.
fn attacker_of(e: &mut Engine, this: Ptr<CombatController>) -> u32 {
    e.call(GET_ATTACKER, &args![this]).u32()
}

/// The background procedure array at +0x8c.
fn background(this: Ptr<CombatController>) -> u32 {
    this.addr().wrapping_add(0x8c)
}

/// The procedure in element `index` of the background array.
fn background_at(e: &mut Engine, this: Ptr<CombatController>, index: u32) -> u32 {
    let slot = e.call(ARRAY_ELEMENT, &args![background(this), index]).u32();
    e.mem.u32(slot)
}

/// Number of background procedures.
fn background_count(e: &mut Engine, this: Ptr<CombatController>) -> u32 {
    e.call(ARRAY_SIZE, &args![background(this)]).u32()
}

/// `delete procedure` as the code writes it: the vtable slot 0xc call, then
/// the scalar deleting destructor (slot 0, flag 1) when the pointer is not
/// null.
fn release_and_delete(e: &mut Engine, procedure: u32) {
    e.vcall(procedure, 0x0c, &[]);
    if procedure != 0 {
        e.vcall(procedure, 0x00, &args![1u32]);
    }
}

/// Calls vtable slot `slot` with `words` on the action procedure, when set.
fn action_vcall(e: &mut Engine, this: Ptr<CombatController>, slot: u32, words: &[u32]) {
    if !e.get(this, CombatController::pActionProcedure).is_null() {
        let procedure = e.get(this, CombatController::pActionProcedure);
        e.vcall(procedure.addr(), slot, words);
    }
}

/// Calls vtable slot `slot` with `words` on the movement procedure, when
/// set.
fn movement_vcall(e: &mut Engine, this: Ptr<CombatController>, slot: u32, words: &[u32]) {
    if !e.get(this, CombatController::pMovementProcedure).is_null() {
        let procedure = e.get(this, CombatController::pMovementProcedure);
        e.vcall(procedure.addr(), slot, words);
    }
}

// Translated from 0097d3a0 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 0097d3a0 - the C++ exception-unwinding frame (FS:[0], unwind table at f191cd) is not translated
/// `CombatController::CombatController` (Xbox PDB): builds the controller of
/// `attacker` in `combat_group`: the `TESPackage` base, the vtable, the
/// background array, the plan timer, the two debug-geometry pointers and the
/// two look-at points; a new `CombatState` (0x22c bytes, `00997600`) given
/// `(count % 10) * 0.1` from the running count at `011f1590` (which it then
/// increments); the package type 0x12 (`TESPackage::SetPackType`), the
/// procedure type 0xc (`00984f60`) and the field defaults. When there is an
/// attacker it runs `CombatController::UpdatePackageData` (`00980ae0`) with
/// `package`, or, when `package` is 0, with what the attacker's process
/// (`008d8520`) returns from its vtable slot 0x22c. Returns `this`.
pub fn combat_controller_combat_controller(
    e: &mut Engine,
    this: Ptr<CombatController>,
    attacker: Ptr,
    combat_group: Ptr,
    package: u32,
) -> Ptr<CombatController> {
    e.call(0x0067_07c0, &args![this]);
    e.mem.set_u32(this.addr(), COMBAT_CONTROLLER_VTABLE);
    e.call(0x0098_2430, &args![background(this)]);
    e.call(0x0090_6f60, &args![this.byte_add(0xb4)]);
    e.call(0x0063_3c90, &args![this.byte_add(0xe8), 0u32]);
    e.call(0x0063_3c90, &args![this.byte_add(0xec), 0u32]);
    e.call(
        0x0040_1050,
        &args![this.byte_add(0x128), 0x0cu32, 2u32, 0x0068_15c0u32],
    );

    let count: i32 = e.global(CONTROLLER_COUNT);
    let remainder = count.wrapping_rem(10);
    let one_tenth: f64 = e.global(ONE_TENTH);
    let stagger = (remainder as f64 * one_tenth) as f32;
    let count: i32 = e.global(CONTROLLER_COUNT);
    e.set_global(CONTROLLER_COUNT, count.wrapping_add(1));

    e.set(this, CombatController::pCombatGroup, combat_group);
    e.set(this, CombatController::pActionProcedure, Ptr::NULL);
    e.set(this, CombatController::pMovementProcedure, Ptr::NULL);

    let block = e.call(OPERATOR_NEW, &args![0x22cu32]).u32();
    let state = if block != 0 {
        e.call(0x0099_7600, &args![block, this, stagger]).u32()
    } else {
        0
    };
    e.set(this, CombatController::pCombatState, Ptr::new(state));
    e.set(this, CombatController::pCombatPlan, Ptr::NULL);
    e.set(this, CombatController::pCombatPlannerData, Ptr::NULL);
    e.set(this, CombatController::pCurrentAction, Ptr::NULL);
    e.set(this, CombatController::pCurrentGoal, Ptr::NULL);
    e.set(this, CombatController::iCurrentActionValue, 0);
    e.set(this, CombatController::pCurrentWeapon, Ptr::NULL);
    e.set(this, CombatController::pCurrentCombatStyle, Ptr::NULL);

    let delay = x87_quiet(e.global(PLAN_TIMER_DELAY));
    e.call(COMBAT_TIMER_START, &args![this.byte_add(0xb4), delay]);
    e.set(this, CombatController::fLowMovementDelta, 0.0);
    e.call(FILL_BYTES, &args![this.byte_add(0x14c), 0u32, 0x3cu32]);

    e.set(this, CombatController::pAttacker, attacker);
    e.set(this, CombatController::pTarget, Ptr::NULL);
    e.call(0x0067_0fc0, &args![this, 0x12u32]);

    let combat_position = x87_quiet(e.global(MINUS_ONE));
    e.set(this, CombatController::fCombatPosition, combat_position);
    e.set(this, CombatController::bForceCombat, false);
    e.set(this, CombatController::bFirstUpdate, false);
    e.set(this, CombatController::bForceSneak, false);
    e.set(this, CombatController::iMoveMode, 0);
    let height = x87_quiet(e.global(DEFAULT_HEIGHT));
    e.set(this, CombatController::fAttackerHeight, height);
    let sneak_height = x87_quiet(e.global(DEFAULT_SNEAK_HEIGHT));
    e.set(this, CombatController::fAttackerSneakHeight, sneak_height);
    e.set(this, CombatController::bAttackerCanSneak, 1);
    e.set(this, CombatController::bAttackerCanMove, true);
    e.set(this, CombatController::fWalkSpeedMult, 1.0);
    e.set(this, CombatController::fRunSpeedMult, 1.0);
    e.set(this, CombatController::fLowMovementDelta, 0.0);
    e.set(this, CombatController::bHasForwardAttacks, false);
    e.set(this, CombatController::cUseIronSights, -1);
    e.set(this, CombatController::bUpdatedDetection, false);
    e.set(this, CombatController::bDeterminedPowerAttackDeltas, false);
    e.call(0x0098_4f60, &args![this, 0x0cu32]);
    e.set(this, CombatController::bDestructing, false);
    e.set(this, CombatController::bStopCombat, false);
    e.set(this, CombatController::bStoppingCombat, false);
    e.set(this, CombatController::eLowState, 0);
    e.call(FILL_BYTES, &args![this.byte_add(0xfc), 0u32, 0x14u32]);
    e.call(FILL_BYTES, &args![this.byte_add(0x110), 0u32, 0x14u32]);
    e.call(FILL_BYTES, &args![this.byte_add(0x128), 0u32, 0x18u32]);
    e.call(FILL_BYTES, &args![this.byte_add(0x140), 0u32, 2u32]);
    e.set(this, CombatController::cIgnoreLookAtTarget, 0);

    if !e.get(this, CombatController::pAttacker).is_null() {
        let mut package = package;
        if package == 0 {
            let attacker = e.get(this, CombatController::pAttacker);
            let process = e.call(GET_PROCESS, &args![attacker]).u32();
            package = e.vcall(process, 0x22c, &[]).u32();
        }
        e.call(0x0098_0ae0, &args![this, package]);
    }
    this
}

// Translated from 0097d740 (decompiled, FalloutNV.exe 1.4.0.525)
/// The scalar deleting destructor of `CombatController`: runs the destructor
/// (`0097d770`) and frees the block when bit 0 of `flags` is set. Returns
/// `this`.
pub fn fn_0097d740(
    e: &mut Engine,
    this: Ptr<CombatController>,
    flags: u32,
) -> Ptr<CombatController> {
    fn_0097d770(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0097d770 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 0097d770 - the C++ exception-unwinding frame (FS:[0], unwind table at f19222) is not translated
/// The destructor of `CombatController` (vtable `0108ce74`): marks it
/// destructing; drops the combat group when the save/load game singleton
/// answers `0042ce10` true and `00444d20` false; unless `004226e0` on
/// `011c3f2c` answers true, clears the target (`00980830`), both procedures,
/// the attacker's look-at target and iron sights; deletes the background
/// procedures and the plan; takes the attacker out of the combat group
/// (`009869a0`); deletes the `CombatState` (`0097d960`); runs `004534f0(1)`;
/// unless `0047c850` on `011de45c` answers true, lowers the attacker's block;
/// decrements the running count (not below 0); then destroys the two
/// debug-geometry pointers, the background array and the `TESPackage` base.
pub fn fn_0097d770(e: &mut Engine, this: Ptr<CombatController>) {
    e.mem.set_u32(this.addr(), COMBAT_CONTROLLER_VTABLE);
    e.set(this, CombatController::bDestructing, true);

    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    if e.call(0x0042_ce10, &args![save_load]).bool() {
        let save_load = e.mem.u32(SAVE_LOAD_GAME);
        if !e.call(0x0044_4d20, &args![save_load]).bool() {
            e.set(this, CombatController::pCombatGroup, Ptr::NULL);
        }
    }

    let object = e.mem.u32(OBJECT_011C3F2C);
    if !e.call(0x0042_26e0, &args![object]).bool() {
        e.call(SET_TARGET_00980830, &args![this, 0u32]);
        combat_controller_set_movement_procedure(e, this, Ptr::NULL);
        let attacker = attacker_of(e, this);
        e.call(0x008b_3d30, &args![attacker]);
        let attacker = attacker_of(e, this);
        e.call(ACTOR_SET_IRON_SIGHTS, &args![attacker, 0u32, 0u32, 0u32]);
        combat_controller_set_action_procedure(e, this, Ptr::NULL);
    }

    fn_00980360(e, this);
    fn_00980550(e, this);

    if !e.get(this, CombatController::pCombatGroup).is_null() {
        let group = e.get(this, CombatController::pCombatGroup);
        let attacker = e.get(this, CombatController::pAttacker);
        e.call(0x0098_69a0, &args![group, attacker]);
    }
    let state = e.get(this, CombatController::pCombatState);
    if !state.is_null() {
        fn_0097d960(e, state, 1);
    }
    e.call(0x0045_34f0, &args![this, 1u32]);

    let object = e.mem.u32(OBJECT_011DE45C);
    if !e.call(0x0047_c850, &args![object]).bool() && attacker_of(e, this) != 0 {
        let attacker = attacker_of(e, this);
        if e.call(ACTOR_GET_BLOCKED, &args![attacker]).bool() {
            let attacker = attacker_of(e, this);
            e.call(ACTOR_SET_BLOCK, &args![attacker, 0u32]);
        }
    }

    let count = e.global::<i32>(CONTROLLER_COUNT).wrapping_sub(1);
    e.set_global(CONTROLLER_COUNT, count);
    if count < 0 {
        e.set_global(CONTROLLER_COUNT, 0i32);
    }

    e.call(0x0045_cec0, &args![this.byte_add(0xec)]);
    e.call(0x0045_cec0, &args![this.byte_add(0xe8)]);
    e.call(0x0098_2460, &args![background(this)]);
    e.call(0x0067_08b0, &args![this]);
}

// Translated from 0097d960 (decompiled, FalloutNV.exe 1.4.0.525)
/// A scalar deleting destructor: runs `00997aa0` (in `combatstate.cpp`) on
/// `this` and frees the block when bit 0 of `flags` is set. Returns `this`.
/// The controller's destructor uses it on its `CombatState`.
pub fn fn_0097d960(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x0099_7aa0, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0097d990 (decompiled, FalloutNV.exe 1.4.0.525)
/// First-update initialisation: picks the current weapon (`0097f390`), runs
/// `00997c30` on the combat state and, when the attacker's
/// `MobileObject::GetCurrentProcessType` is 0, recalculates the attacker
/// information.
pub fn fn_0097d990(e: &mut Engine, this: Ptr<CombatController>) {
    fn_0097f390(e, this);
    let state = e.get(this, CombatController::pCombatState);
    e.call(0x0099_7c30, &args![state]);
    let attacker = e.get(this, CombatController::pAttacker);
    if e.call(0x0093_1850, &args![attacker]).u32() == 0 {
        combat_controller_calculate_attacker_info(e, this);
    }
}

// Translated from 0097d9d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatController::CalculateAttackerInfo` (Xbox PDB): stores the
/// attacker's height and sneak height (`Actor::GetHeight`,
/// `Actor::GetSneakHeight(0)`), the byte `008eea60` returns for it, and
/// whether it can move (not `TESActorBase::IsImmobile` of what `004181e0`
/// returns for it).
pub fn combat_controller_calculate_attacker_info(e: &mut Engine, this: Ptr<CombatController>) {
    let attacker = e.get(this, CombatController::pAttacker);
    let height = e.call(0x0088_53a0, &args![attacker]).f64() as f32;
    e.set(this, CombatController::fAttackerHeight, height);
    let attacker = e.get(this, CombatController::pAttacker);
    let sneak_height = e.call(0x0088_54d0, &args![attacker, 0u32]).f64() as f32;
    e.set(this, CombatController::fAttackerSneakHeight, sneak_height);
    let attacker = e.get(this, CombatController::pAttacker);
    let can_sneak = e.call(0x008e_ea60, &args![attacker]).u8();
    e.set(this, CombatController::bAttackerCanSneak, can_sneak);
    let attacker = e.get(this, CombatController::pAttacker);
    let base = e.call(0x0041_81e0, &args![attacker]).u32();
    let immobile = e.call(0x005f_0c80, &args![base]).u8();
    e.set(this, CombatController::bAttackerCanMove, immobile == 0);
}

// Translated from 0097da50 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 0097da50 - the C++ exception-unwinding frame (FS:[0], unwind table at f1924b) is not translated
/// The controller's per-frame update. It does nothing unless the byte at
/// `011a4880` is set, the controller is not stopping combat, and the
/// attacker and its process (`008d8520`) exist. When `00981450` answers
/// true, or the attacker answers its vtable slot 0x22c(0) true, it calls the
/// attacker's slot 0x434 with 0 and stops; when the target stops it (slot
/// 0x22c(0) true and slot 0x2e8 false, or `00440da0`/`00440d80` true), it
/// calls slot 0x434 with the target. Other guards: the attacker's slots
/// 0x1e4 and 0x1d0 non-null, the process's slot 0x610 0, 1 or 2, the
/// attacker's slots 0x214, 0x2e8, 0x230 (twice), 0x234 false, the process
/// state (`[attacker+0x68]` slot 0x3e4) not 8 or 10, `bStopCombat` clear,
/// `009b73b0` on the combat state false, `00493830` false, and
/// `Actor::IsSurfacing` false.
///
/// It then replans when a procedure answers `0097e310`, when the plan timer
/// (`008d7f80`) says so, or when the procedures are done (`009804d0`) and
/// the plan (`0097e490`) is finished (the group then hears `009891d0`; an
/// unfinished plan is restarted with `00994050`). Replanning deletes the
/// plan and the background procedures, retargets, tells both procedures
/// (slot 0x2c), plans with a stack `CombatPlanner` (`00994bd0`), keeps the
/// plan's `0084e3a0` as the current goal, carries the planner data over
/// (`009805b0`), restarts the plan timer with 1.0, starts the plan or clears
/// the procedures, and tells both procedures again (slot 0x30). Then both
/// procedures update (slot 4) and, when `00493bb0` on the attacker allows,
/// every background procedure updates (slot 4) and is deleted when it is
/// done (`00666f00`) or answers `0097e310`; finally the look-at target is
/// updated, iron sights are set from `cUseIronSights == 1`, and `eLowState`
/// is cleared.
pub fn fn_0097da50(e: &mut Engine, this: Ptr<CombatController>) {
    if e.global::<u8>(UPDATE_ENABLED) == 0 {
        return;
    }
    if e.get(this, CombatController::bStoppingCombat) {
        return;
    }
    let attacker = attacker_of(e, this);
    if attacker == 0 || e.call(GET_PROCESS, &args![attacker]).u32() == 0 {
        return;
    }
    if e.call(0x0098_1450, &args![this]).bool() {
        e.vcall(attacker, 0x434, &args![0u32]);
        return;
    }
    let anim_data = e.vcall(attacker, 0x1e4, &[]).u32();
    let slot_1d0 = e.vcall(attacker, 0x1d0, &[]).u32();
    if slot_1d0 == 0 || anim_data == 0 {
        return;
    }
    let process = e.call(GET_PROCESS, &args![attacker]).u32();
    let process_kind = e.vcall(process, 0x610, &[]).u32();
    if process_kind != 0 && process_kind != 2 && process_kind != 1 {
        return;
    }
    if e.vcall(attacker, 0x214, &[]).u32() != 0 {
        return;
    }
    if !e.get(this, CombatController::bFirstUpdate) {
        fn_0097d990(e, this);
        e.set(this, CombatController::bFirstUpdate, true);
    }
    if e.vcall(attacker, 0x2e8, &[]).bool() {
        return;
    }
    if e.vcall(attacker, 0x22c, &args![0u32]).bool() {
        e.vcall(attacker, 0x434, &args![0u32]);
        return;
    }
    fn_0097f390(e, this);
    let target = e.call(GET_TARGET, &args![this]).u32();
    if target != 0 {
        let stops = if e.vcall(target, 0x22c, &args![0u32]).bool()
            && !e.vcall(target, 0x2e8, &[]).bool()
        {
            true
        } else {
            e.call(0x0044_0da0, &args![target]).bool() || e.call(0x0044_0d80, &args![target]).bool()
        };
        if stops {
            e.vcall(attacker, 0x434, &args![target]);
            return;
        }
    }
    let process = e.mem.u32(attacker.wrapping_add(0x68));
    let state_kind = e.vcall(process, 0x3e4, &[]).u16() as i16;
    if state_kind == 8 || state_kind == 10 {
        return;
    }
    if e.vcall(attacker, 0x230, &[]).bool() || e.vcall(attacker, 0x234, &[]).bool() {
        return;
    }
    let state = e.get(this, CombatController::pCombatState);
    e.call(0x0099_7cb0, &args![state]);
    if e.get(this, CombatController::bStopCombat) {
        return;
    }
    let state = e.get(this, CombatController::pCombatState);
    if e.call(0x009b_73b0, &args![state]).bool() {
        return;
    }
    if e.call(0x0049_3830, &args![anim_data]).bool() {
        return;
    }
    if e.vcall(attacker, 0x230, &[]).bool() {
        return;
    }
    if e.call(0x008a_7a40, &args![attacker]).bool() {
        return;
    }

    let mut replan = false;
    if fn_00980470(e, this) {
        replan = true;
        let action = e.get(this, CombatController::pActionProcedure);
        if !action.is_null() && fn_0097e310(e, action) {
            let action = e.get(this, CombatController::pActionProcedure);
            e.call(RELEASE_PROCEDURE_0097F270, &args![this, action]);
        }
        let movement = e.get(this, CombatController::pMovementProcedure);
        if !movement.is_null() && fn_0097e310(e, movement) {
            let movement = e.get(this, CombatController::pMovementProcedure);
            e.call(RELEASE_PROCEDURE_0097F270, &args![this, movement]);
        }
    }
    if e.call(0x008d_7f80, &args![this.byte_add(0xb4)]).bool() {
        replan = true;
    }
    if !replan && fn_009804d0(e, this) && !e.get(this, CombatController::pCombatPlan).is_null() {
        let plan = e.get(this, CombatController::pCombatPlan);
        if e.call(0x0097_e490, &args![plan]).bool() {
            let plan = e.get(this, CombatController::pCombatPlan);
            let goal = e.call(PLAN_GOAL_0084E3A0, &args![plan]).u32();
            let group = e.get(this, CombatController::pCombatGroup);
            e.call(0x0098_91d0, &args![group, this, goal]);
            replan = true;
        } else {
            let plan = e.get(this, CombatController::pCombatPlan);
            e.call(PLAN_START_00994050, &args![plan, this]);
        }
    }

    if replan {
        fn_00980550(e, this);
        fn_00980360(e, this);
        let group = e.get(this, CombatController::pCombatGroup);
        let attacker_field = e.get(this, CombatController::pAttacker);
        let chosen = e
            .call(GROUP_CHOOSE_TARGET, &args![group, attacker_field])
            .u32();
        e.call(SET_TARGET_00980830, &args![this, chosen]);
        movement_vcall(e, this, 0x2c, &[]);
        action_vcall(e, this, 0x2c, &[]);
        e.call(0x0048_3710, &args![OBJECT_011F15CC]);
        e.with_stack(COMBAT_PLANNER_STACK_SIZE, |e, planner| {
            fn_0097e330(e, planner);
            let plan = e.call(PLANNER_PLAN, &args![planner, this, 0u32]).u32();
            e.set(this, CombatController::pCombatPlan, Ptr::new(plan));
            let goal = if !e.get(this, CombatController::pCombatPlan).is_null() {
                let plan = e.get(this, CombatController::pCombatPlan);
                e.call(PLAN_GOAL_0084E3A0, &args![plan]).u32()
            } else {
                0
            };
            e.set(this, CombatController::pCurrentGoal, Ptr::new(goal));
            fn_009805b0(e, this);
            e.call(0x0099_5830, &args![planner]);
            e.set(this, CombatController::pCombatPlannerData, Ptr::NULL);
            e.call(0x0048_3710, &args![OBJECT_011F15CC]);
            e.call(COMBAT_TIMER_START, &args![this.byte_add(0xb4), 1.0f32]);
            if !e.get(this, CombatController::pCombatPlan).is_null() {
                let plan = e.get(this, CombatController::pCombatPlan);
                e.call(PLAN_START_00994050, &args![plan, this]);
            } else {
                fn_00980250(e, this);
                e.set(this, CombatController::pCurrentAction, Ptr::NULL);
            }
            movement_vcall(e, this, 0x30, &[]);
            action_vcall(e, this, 0x30, &[]);
            let state = e.get(this, CombatController::pCombatState);
            e.call(0x0099_a9a0, &args![state]);
            fn_0097f390(e, this);
            fn_0097e3e0(e, planner);
        });
    }

    movement_vcall(e, this, 0x04, &[]);
    action_vcall(e, this, 0x04, &[]);

    if !e.call(0x0049_3bb0, &args![attacker]).bool() {
        return;
    }
    let mut index: u32 = 0;
    while index < background_count(e, this) {
        let procedure = background_at(e, this, index);
        e.vcall(procedure, 0x04, &[]);
        let done = e.call(0x0066_6f00, &args![procedure]).bool();
        if done || fn_0097e310(e, Ptr::new(procedure)) {
            if fn_0097e310(e, Ptr::new(procedure)) {
                e.call(RELEASE_PROCEDURE_0097F270, &args![this, procedure]);
            }
            release_and_delete(e, procedure);
            e.call(ARRAY_REMOVE, &args![background(this), index, 1u32]);
            index = index.wrapping_sub(1);
        }
        index = index.wrapping_add(1);
    }
    e.call(0x0098_1740, &args![this]);
    let iron_sights = e.get(this, CombatController::cUseIronSights) == 1;
    e.call(
        ACTOR_SET_IRON_SIGHTS,
        &args![attacker, iron_sights as u32, 0u32, 0u32],
    );
    e.set(this, CombatController::eLowState, 0);
}

// Translated from 0097e310 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the word at +8 of `this` is 1 (the update asks it of procedures).
pub fn fn_0097e310(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr().wrapping_add(8)) == 1
}

// Translated from 0097e330 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 0097e330 - the C++ exception-unwinding frame (FS:[0], unwind table at f19294) is not translated
/// The `CombatPlanner` constructor (the vtable it stores, `0108cfd8`, is
/// `.?AVCombatPlanner@@`): the base (`00982480`), the vtable, a zero word at
/// +0x2058, the member at +0x205c (`009aa470`), a `ScrapHeap` at +0x20b8,
/// the member at +0x20c8 (`009958b0`) and its `BGSUnloadedFormBuffer`-named
/// setup with `this` (`00537e90`). Returns `this`.
pub fn fn_0097e330(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(0x0098_2480, &args![this]);
    e.mem.set_u32(this.addr(), COMBAT_PLANNER_VTABLE);
    e.mem.set_u32(this.addr().wrapping_add(0x2058), 0);
    e.call(0x009a_a470, &args![this.byte_add(0x205c)]);
    e.call(0x00aa_53f0, &args![this.byte_add(0x20b8)]);
    e.call(0x0099_58b0, &args![this.byte_add(0x20c8)]);
    e.call(0x0053_7e90, &args![this.byte_add(0x20c8), this]);
    this
}

// Translated from 0097e3e0 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 0097e3e0 - the C++ exception-unwinding frame (FS:[0], unwind table at f19294) is not translated
/// The `CombatPlanner` destructor: the vtable, `00995830` on `this`, the
/// members at +0x20c8 (`00995910`) and +0x20b8 (`00aa5460`), then the base
/// (`00982530`).
pub fn fn_0097e3e0(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), COMBAT_PLANNER_VTABLE);
    e.call(0x0099_5830, &args![this]);
    e.call(0x0099_5910, &args![this.byte_add(0x20c8)]);
    e.call(0x00aa_5460, &args![this.byte_add(0x20b8)]);
    e.call(0x0098_2530, &args![this]);
}

// Translated from 0097e460 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatPlanner::_scalar_deleting_destructor_` (Xbox PDB): the destructor
/// (`0097e3e0`), then frees the block when bit 0 of `flags` is set. Returns
/// `this`.
pub fn combat_planner_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_0097e3e0(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0097f310 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatController::CheckHasEquip` (Xbox PDB, cdecl): whether `actor` has
/// an animation sequence for the group `Actor::GetAnimGroup(0x18, item,
/// item == 0, 0)` names (looked up in the map of the actor's vtable slot
/// 0x1e4 result). False when the group is 0 or the slot returns null.
pub fn combat_controller_check_has_equip(e: &mut Engine, actor: Ptr, item: u32) -> bool {
    let group = e
        .call(
            ACTOR_GET_ANIM_GROUP,
            &args![actor, 0x18u32, item, (item == 0) as u32, 0u32],
        )
        .u16();
    if group == 0 {
        return false;
    }
    let owner = e.vcall(actor.addr(), 0x1e4, &[]).u32();
    if owner == 0 {
        return false;
    }
    e.with_stack(4, |e, sequence| {
        let map = e.call(ANIM_MAP_OF, &args![owner]).u32();
        e.call(ANIM_MAP_GET_AT, &args![map, group as u32, sequence])
            .bool()
    })
}

// Translated from 0097f390 (decompiled, FalloutNV.exe 1.4.0.525)
/// Picks the current weapon: from background procedure 5 (`0044edb0`) when
/// there is one, otherwise `Actor::GetCurrentWeapon` of the attacker.
pub fn fn_0097f390(e: &mut Engine, this: Ptr<CombatController>) {
    e.set(this, CombatController::pCurrentWeapon, Ptr::NULL);
    let procedure = combat_controller_get_background_procedure(e, this, 5);
    let weapon = if !procedure.is_null() {
        e.call(0x0044_edb0, &args![procedure]).u32()
    } else {
        let attacker = e.get(this, CombatController::pAttacker);
        e.call(0x008a_1710, &args![attacker]).u32()
    };
    e.set(this, CombatController::pCurrentWeapon, Ptr::new(weapon));
}

// Translated from 0097f3f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Refreshes the combat style from the attacker (`008a7f80`); when it
/// changed from a non-null one, sets byte +0x77 of the combat state
/// (`0097f450`) and runs `0097f200`.
pub fn fn_0097f3f0(e: &mut Engine, this: Ptr<CombatController>) {
    let previous = e.get(this, CombatController::pCurrentCombatStyle);
    let attacker = e.get(this, CombatController::pAttacker);
    let style = e.call(0x008a_7f80, &args![attacker]).u32();
    e.set(this, CombatController::pCurrentCombatStyle, Ptr::new(style));
    if !previous.is_null() && previous != e.get(this, CombatController::pCurrentCombatStyle) {
        let state = e.get(this, CombatController::pCombatState);
        fn_0097f450(e, state, 1);
        e.call(AFTER_TARGET_CHANGE_0097F200, &args![this]);
    }
}

// Translated from 0097f450 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the byte at +0x77 of `this` (a `CombatState` here).
pub fn fn_0097f450(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr().wrapping_add(0x77), value);
}

// Translated from 0097f470 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatController::CheckCurrentWeapon` (Xbox PDB): drops the current
/// weapon when `0097c150` on the combat state, given the weapon's
/// `TESObjectWEAP::GetCombatWeaponType`, does not return that weapon.
pub fn combat_controller_check_current_weapon(e: &mut Engine, this: Ptr<CombatController>) {
    if e.get(this, CombatController::pCurrentWeapon).is_null() {
        return;
    }
    let weapon = e.get(this, CombatController::pCurrentWeapon);
    let weapon_type = e.call(0x0052_2c80, &args![weapon]).u32();
    let state = e.get(this, CombatController::pCombatState);
    let found = e.call(0x0097_c150, &args![state, weapon_type]).u32();
    if found != e.get(this, CombatController::pCurrentWeapon).addr() {
        e.set(this, CombatController::pCurrentWeapon, Ptr::NULL);
    }
}

// Translated from 0097f4d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatController::StopCombat` (Xbox PDB): with a null `target`, returns
/// true; otherwise removes it from the targets and, when it was the current
/// target, retargets (`00986c60`, `00980830`) and, when no target is left,
/// clears both procedures, deletes the plan and runs `0097f200`; returns
/// false. Either way the attacker's block is lowered when it is up.
pub fn combat_controller_stop_combat(
    e: &mut Engine,
    this: Ptr<CombatController>,
    target: Ptr,
) -> bool {
    let mut stopped = false;
    if target.is_null() {
        stopped = true;
    } else {
        combat_controller_remove_target(e, this, target);
        if e.get(this, CombatController::pTarget) == target {
            let group = e.get(this, CombatController::pCombatGroup);
            let attacker = e.get(this, CombatController::pAttacker);
            let chosen = e.call(GROUP_CHOOSE_TARGET, &args![group, attacker]).u32();
            e.call(SET_TARGET_00980830, &args![this, chosen]);
            if e.get(this, CombatController::pTarget).is_null() {
                fn_00980250(e, this);
                fn_00980550(e, this);
                e.call(AFTER_TARGET_CHANGE_0097F200, &args![this]);
            }
        }
    }
    let attacker = attacker_of(e, this);
    if e.call(ACTOR_GET_BLOCKED, &args![attacker]).bool() {
        let attacker = attacker_of(e, this);
        e.call(ACTOR_SET_BLOCK, &args![attacker, 0u32]);
    }
    stopped
}

// Translated from 0097f580 (decompiled, FalloutNV.exe 1.4.0.525)
/// Considers `actor` as a new target: initialises on the first update, tells
/// the combat group (`00989230`), and, when `actor` is neither the target nor
/// the attacker, has a process, is not a target yet, and
/// `Actor::GetShouldAttackActor(actor, 1, &out, 0)` says yes, adds it with
/// the truncated value of the actor's process vtable slot 0x208 `(actor,
/// attacker)`. The second word is never read.
pub fn fn_0097f580(e: &mut Engine, this: Ptr<CombatController>, actor: Ptr, _unused_2: u32) {
    if !e.get(this, CombatController::bFirstUpdate) {
        fn_0097d990(e, this);
        e.set(this, CombatController::bFirstUpdate, true);
    }
    if !e.get(this, CombatController::pCombatGroup).is_null() {
        let group = e.get(this, CombatController::pCombatGroup);
        let attacker = e.get(this, CombatController::pAttacker);
        e.call(0x0098_9230, &args![group, attacker, actor]);
    }
    if actor.addr() == e.call(GET_TARGET, &args![this]).u32() {
        return;
    }
    if actor.addr() == attacker_of(e, this) {
        return;
    }
    if e.call(GET_PROCESS, &args![actor]).u32() == 0 {
        return;
    }
    let mut priority: i32 = 0;
    if combat_controller_is_actora_combat_target(e, this, actor) {
        return;
    }
    let should_attack = e.with_stack(4, |e, out| {
        let attacker = attacker_of(e, this);
        e.call(0x008b_06d0, &args![attacker, actor, 1u32, out, 0u32])
            .bool()
    });
    if should_attack {
        priority = 100;
    }
    if priority > 0 {
        let process = e.call(GET_PROCESS, &args![actor]).u32();
        let attacker = e.get(this, CombatController::pAttacker);
        let value = e.vcall(process, 0x208, &args![actor, attacker]).f64();
        let value = e.call(FTOL, &args![value]).u32();
        combat_controller_add_target(e, this, actor, value, 0, 0.0, 0.0);
    }
}

// Translated from 0097f6d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears `bDeterminedPowerAttackDeltas`, runs `0099a720` on the combat
/// state, and passes `value` to the action procedure's vtable slot 0x1c.
pub fn fn_0097f6d0(e: &mut Engine, this: Ptr<CombatController>, value: u32) {
    e.set(this, CombatController::bDeterminedPowerAttackDeltas, false);
    let state = e.get(this, CombatController::pCombatState);
    e.call(0x0099_a720, &args![state]);
    action_vcall(e, this, 0x1c, &args![value]);
}

// Translated from 0097f720 (decompiled, FalloutNV.exe 1.4.0.525)
/// Passes `value` to the combat state (`0099a5e0`) and to both procedures'
/// vtable slot 0x20.
pub fn fn_0097f720(e: &mut Engine, this: Ptr<CombatController>, value: u32) {
    let state = e.get(this, CombatController::pCombatState);
    e.call(0x0099_a5e0, &args![state, value]);
    action_vcall(e, this, 0x20, &args![value]);
    movement_vcall(e, this, 0x20, &args![value]);
}

// Translated from 0097f7a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards `(value, flag)` to `0099a650` on the combat state.
pub fn fn_0097f7a0(e: &mut Engine, this: Ptr<CombatController>, value: u32, flag: u8) {
    let state = e.get(this, CombatController::pCombatState);
    e.call(0x0099_a650, &args![state, value, flag]);
}

// Translated from 0097f7d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards `value` to `0099a680` on the combat state.
pub fn fn_0097f7d0(e: &mut Engine, this: Ptr<CombatController>, value: u32) {
    let state = e.get(this, CombatController::pCombatState);
    e.call(0x0099_a680, &args![state, value]);
}

// Translated from 0097f7f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards `(value, amount)` to `0099a6b0` on the combat state (the float
/// is copied through the x87 stack).
pub fn fn_0097f7f0(e: &mut Engine, this: Ptr<CombatController>, value: u32, amount: f32) {
    let state = e.get(this, CombatController::pCombatState);
    e.call(0x0099_a6b0, &args![state, value, x87_quiet(amount)]);
}

// Translated from 0097f820 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards four words (the last a byte) to `0099a7d0` on the combat state
/// and to both procedures' vtable slot 0x28.
pub fn fn_0097f820(
    e: &mut Engine,
    this: Ptr<CombatController>,
    first: u32,
    second: u32,
    third: u32,
    flag: u8,
) {
    let state = e.get(this, CombatController::pCombatState);
    e.call(0x0099_a7d0, &args![state, first, second, third, flag]);
    action_vcall(e, this, 0x28, &args![first, second, third, flag]);
    movement_vcall(e, this, 0x28, &args![first, second, third, flag]);
}

// Translated from 0097f8c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs `0099a8d0` on the combat state and both procedures' vtable slot
/// 0x24.
pub fn fn_0097f8c0(e: &mut Engine, this: Ptr<CombatController>) {
    let state = e.get(this, CombatController::pCombatState);
    e.call(0x0099_a8d0, &args![state]);
    action_vcall(e, this, 0x24, &[]);
    movement_vcall(e, this, 0x24, &[]);
}

// Translated from 0097f930 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatController::AddTarget` (Xbox PDB): adds `target` to the combat
/// group (`CombatGroup::AddTarget`). When it is the player, first marks the
/// attacker hostile to the player
/// (`PlayerCharacter::ChangePerceivedActorHostileStatus(attacker, 1)`) and,
/// when the controller's group is the player's (player vtable slot 0x3f8),
/// moves the attacker to its own group
/// (`CombatManager::SplitPlayerCombatGroup`). Null targets are ignored. The
/// four words after `target` are never read.
pub fn combat_controller_add_target(
    e: &mut Engine,
    this: Ptr<CombatController>,
    target: Ptr,
    _unused_2: u32,
    _unused_3: u32,
    _unused_4: f32,
    _unused_5: f32,
) {
    if target.is_null() {
        return;
    }
    if target.addr() == e.mem.u32(PLAYER_CHARACTER) {
        let player = e.mem.u32(PLAYER_CHARACTER);
        let attacker = e.get(this, CombatController::pAttacker);
        e.call(
            CHANGE_PERCEIVED_HOSTILE_STATUS,
            &args![player, attacker, 1u32],
        );
        let player = e.mem.u32(PLAYER_CHARACTER);
        let player_group = e.vcall(player, 0x3f8, &[]).u32();
        if e.get(this, CombatController::pCombatGroup).addr() == player_group {
            let manager = e.mem.u32(COMBAT_MANAGER);
            let attacker = e.get(this, CombatController::pAttacker);
            let group = e.call(0x0099_1f80, &args![manager, attacker]).u32();
            e.set(this, CombatController::pCombatGroup, Ptr::new(group));
        }
    }
    let group = e.get(this, CombatController::pCombatGroup);
    e.call(0x0098_6410, &args![group, target]);
}

// Translated from 0097f9c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatController::RemoveTarget` (Xbox PDB): when `target` is the player,
/// marks the attacker no longer hostile to the player; removes `target` from
/// the combat group when there is one (`CombatGroup::RemoveTarget`).
pub fn combat_controller_remove_target(e: &mut Engine, this: Ptr<CombatController>, target: Ptr) {
    if target.addr() == e.mem.u32(PLAYER_CHARACTER) {
        let player = e.mem.u32(PLAYER_CHARACTER);
        let attacker = e.get(this, CombatController::pAttacker);
        e.call(
            CHANGE_PERCEIVED_HOSTILE_STATUS,
            &args![player, attacker, 0u32],
        );
    }
    if !e.get(this, CombatController::pCombatGroup).is_null() {
        let group = e.get(this, CombatController::pCombatGroup);
        e.call(0x0098_6500, &args![group, target]);
    }
}

// Translated from 0097fa10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatController::IsActoraCombatTarget` (Xbox PDB): `CombatGroup::IsTarget`
/// of the combat group, false without a group.
pub fn combat_controller_is_actora_combat_target(
    e: &mut Engine,
    this: Ptr<CombatController>,
    actor: Ptr,
) -> bool {
    if e.get(this, CombatController::pCombatGroup).is_null() {
        return false;
    }
    let group = e.get(this, CombatController::pCombatGroup);
    e.call(0x0098_65b0, &args![group, actor]).bool()
}

// Translated from 0097fa50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the float at +4 of `this` is not 0.0 (true for a NaN; the x87
/// compare against the `double` at `01012060`). A `CombatTimer` keeps its
/// target time there; the callers' class is not confirmed.
pub fn fn_0097fa50(e: &mut Engine, this: Ptr) -> bool {
    let value = e.mem.f32(this.addr().wrapping_add(4));
    let zero: f64 = e.global(ZERO);
    // `FCOMP; TEST AH,0x44; JNP`: false only when equal.
    value as f64 != zero
}

// Translated from 0097fa90 (decompiled, FalloutNV.exe 1.4.0.525)
/// The time since the float at +0 of `this` by the clock `00435dd0`, capped
/// at 1000.0 (the `double` at `01017b70`) when it is greater (the clock is
/// read a second time below the cap).
pub fn fn_0097fa90(e: &mut Engine, this: Ptr) -> f32 {
    let now = e.call(0x0043_5dd0, &[]).f64();
    let elapsed = now - e.mem.f32(this.addr()) as f64;
    let limit: f64 = e.global(ELAPSED_LIMIT);
    // `TEST AH,0x41; JNZ`: below, equal or unordered reads the clock again.
    if elapsed > limit {
        limit as f32
    } else {
        let now = e.call(0x0043_5dd0, &[]).f64();
        (now - e.mem.f32(this.addr()) as f64) as f32
    }
}

// Translated from 0097fae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the word at +0x94 of `this` to `out` and returns `out`.
pub fn fn_0097fae0(e: &mut Engine, this: Ptr, out: Ptr) -> Ptr {
    let value = e.mem.u32(this.addr().wrapping_add(0x94));
    e.mem.set_u32(out.addr(), value);
    out
}

// Translated from 0097fb00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the word at +0x98 of `this` to `out` and returns `out`.
pub fn fn_0097fb00(e: &mut Engine, this: Ptr, out: Ptr) -> Ptr {
    let value = e.mem.u32(this.addr().wrapping_add(0x98));
    e.mem.set_u32(out.addr(), value);
    out
}

// Translated from 0097fb20 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at +0x71 of `this`.
pub fn fn_0097fb20(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr().wrapping_add(0x71))
}

// Translated from 0097fb40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the word at +0x9c of `this` to `out` and returns `out`.
pub fn fn_0097fb40(e: &mut Engine, this: Ptr, out: Ptr) -> Ptr {
    let value = e.mem.u32(this.addr().wrapping_add(0x9c));
    e.mem.set_u32(out.addr(), value);
    out
}

// Translated from 0097fb60 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at +0xe8 of `this`.
pub fn fn_0097fb60(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr().wrapping_add(0xe8))
}

// Translated from 0097fb80 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 0097fb80 - the C++ exception-unwinding frame (FS:[0], unwind table at f1933b) is not translated
/// Runs a stack `CombatPlanner` for the controller with flag 1
/// (`00994bd0`) and deletes the plan it returns (`0097fc40`).
pub fn fn_0097fb80(e: &mut Engine, this: Ptr<CombatController>) {
    e.with_stack(COMBAT_PLANNER_STACK_SIZE, |e, planner| {
        fn_0097e330(e, planner);
        let plan = e.call(PLANNER_PLAN, &args![planner, this, 1u32]).u32();
        if plan != 0 {
            fn_0097fc40(e, Ptr::new(plan), 1);
        }
        fn_0097e3e0(e, planner);
    });
}

// Translated from 0097fc40 (decompiled, FalloutNV.exe 1.4.0.525)
/// A scalar deleting destructor (of the plans the planner returns): runs
/// `00993f70` and frees the block when bit 0 of `flags` is set. Returns
/// `this`.
pub fn fn_0097fc40(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x0099_3f70, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0097fc70 (decompiled, FalloutNV.exe 1.4.0.525)
/// A skill value for the attacker: when its vtable slot 0x21c answers
/// false, `ActorValueOwner::GetClampedActorValue` (on the attacker + 0xa4)
/// of what `008d85e0` returns for the word `007031c0` reads from the
/// controller (+0xf0), or of actor value 0x2d when that word is null; when
/// the slot answers true, the truncated `008d46f0` of the attacker cast to
/// `Creature` (`__RTDynamicCast` from `Actor`), 0 when the cast fails.
pub fn fn_0097fc70(e: &mut Engine, this: Ptr<CombatController>) -> i32 {
    let mut value: u32 = 0;
    let attacker = attacker_of(e, this);
    if !e.vcall(attacker, 0x21c, &[]).bool() {
        if e.call(0x0070_31c0, &args![this]).u32() != 0 {
            let weapon = e.call(0x0070_31c0, &args![this]).u32();
            let actor_value = e.call(0x008d_85e0, &args![weapon]).u32();
            let attacker = attacker_of(e, this);
            value = e
                .call(
                    0x0066_ef20,
                    &args![attacker.wrapping_add(0xa4), actor_value],
                )
                .u32();
        } else {
            let attacker = attacker_of(e, this);
            value = e
                .call(0x0066_ef20, &args![attacker.wrapping_add(0xa4), 0x2du32])
                .u32();
        }
    } else {
        let attacker = attacker_of(e, this);
        // Type descriptors `.?AVActor@@` (011846d4) and `.?AVCreature@@`
        // (011a106c).
        let creature = e
            .call(
                RT_DYNAMIC_CAST,
                &args![attacker, 0u32, 0x0118_46d4u32, 0x011a_106cu32, 0u32],
            )
            .u32();
        if creature != 0 {
            let skill = e.call(0x008d_46f0, &args![creature]).f64();
            value = e.call(FTOL, &args![skill]).u32();
        }
    }
    value as i32
}

// Translated from 0097fd40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs `00461580(3)` on `this`.
pub fn fn_0097fd40(e: &mut Engine, this: Ptr) {
    e.call(0x0046_1580, &args![this, 3u32]);
}

// Translated from 0097fd60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards `value` to `004febd0` on the combat state.
pub fn fn_0097fd60(e: &mut Engine, this: Ptr<CombatController>, value: u32) {
    let state = e.get(this, CombatController::pCombatState);
    e.call(0x004f_ebd0, &args![state, value]);
}

// Translated from 0097fd80 (decompiled, FalloutNV.exe 1.4.0.525)
/// The movement of `actor`'s power attack `action` (cdecl): `out` (an
/// `NiPoint3`) starts as the point at `011f426c`; when the action's
/// animation group is a power attack (`TESAnimGroup::IsPowerAttackAction`)
/// and the actor has its sequence, `out` becomes the group's `005f4c90`
/// vector times the time between the group's times 2 and 0 (`0045bb20`),
/// times the actor's scale, and the result is true. False for a null actor.
pub fn fn_0097fd80(e: &mut Engine, actor: Ptr, action: u32, out: Ptr<NiPoint3>) -> bool {
    if actor.is_null() {
        return false;
    }
    for i in 0..3 {
        let word = e.mem.u32(POINT_011F426C + 4 * i);
        e.mem.set_u32(out.addr() + 4 * i, word);
    }
    let group = e
        .call(
            ACTOR_GET_ANIM_GROUP,
            &args![actor, action, 0u32, 0u32, 0u32],
        )
        .u16();
    if !e.call(IS_POWER_ATTACK_ACTION, &args![group as u32]).bool() {
        return false;
    }
    let owner = e.vcall(actor.addr(), 0x1e4, &[]).u32();
    // Locals: the sequence (4 bytes), then three NiPoint3 temporaries.
    e.with_stack(0x28, |e, locals| {
        let sequence_slot = locals;
        let base = locals.byte_add(4);
        let scaled = locals.byte_add(0x10);
        let result = locals.byte_add(0x1c);
        let map = e.call(ANIM_MAP_OF, &args![owner]).u32();
        if !e
            .call(ANIM_MAP_GET_AT, &args![map, group as u32, sequence_slot])
            .bool()
        {
            return false;
        }
        let sequence = e.mem.u32(sequence_slot.addr());
        let sequence_group = e.vcall(sequence, 0x10, &args![u32::MAX]).u32();
        let anim_group = e.call(ANIM_GROUP_OF, &args![sequence_group]).u32();
        let end = e.call(ANIM_GROUP_GET_TIME, &args![anim_group, 2u32]).f64();
        let start = e.call(ANIM_GROUP_GET_TIME, &args![anim_group, 0u32]).f64();
        let duration = (end - start) as f32;
        let vector = e.call(0x005f_4c90, &args![anim_group, base]).u32();
        e.call(0x0045_bb20, &args![vector, scaled, x87_quiet(duration)]);
        let scale = e.call(REFR_GET_SCALE, &args![actor]).f64() as f32;
        let product = e.call(0x0045_bb20, &args![scaled, result, scale]).u32();
        for i in 0..3 {
            let word = e.mem.u32(product + 4 * i);
            e.mem.set_u32(out.addr() + 4 * i, word);
        }
        true
    })
}

// Translated from 0097feb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The hit time of `actor`'s power attack `action` (cdecl): the time
/// between its animation group's times 1 and 0, or -1.0 (the float at
/// `01012054`) when the action is not a power attack or the actor has no
/// sequence for it.
pub fn fn_0097feb0(e: &mut Engine, actor: Ptr, action: u32) -> f32 {
    let group = e
        .call(
            ACTOR_GET_ANIM_GROUP,
            &args![actor, action, 0u32, 0u32, 0u32],
        )
        .u16();
    if e.call(IS_POWER_ATTACK_ACTION, &args![group as u32]).bool() {
        let owner = e.vcall(actor.addr(), 0x1e4, &[]).u32();
        let time = e.with_stack(4, |e, sequence_slot| {
            let map = e.call(ANIM_MAP_OF, &args![owner]).u32();
            if !e
                .call(ANIM_MAP_GET_AT, &args![map, group as u32, sequence_slot])
                .bool()
            {
                return None;
            }
            let sequence = e.mem.u32(sequence_slot.addr());
            let sequence_group = e.vcall(sequence, 0x10, &args![u32::MAX]).u32();
            let anim_group = e.call(ANIM_GROUP_OF, &args![sequence_group]).u32();
            let hit = e.call(ANIM_GROUP_GET_TIME, &args![anim_group, 1u32]).f64();
            let start = e.call(ANIM_GROUP_GET_TIME, &args![anim_group, 0u32]).f64();
            Some((hit - start) as f32)
        });
        if let Some(time) = time {
            return time;
        }
    }
    x87_quiet(e.global(MINUS_ONE))
}

// Translated from 0097ff60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatController::DeterminePowerAttackDeltas` (Xbox PDB): for the five
/// power-attack actions 0x5c..0x60, the range (the forward distance of the
/// attack's movement, `00980090`, times the attacker's scale, when the
/// combat style's `005524b0` byte is positive and the distance is; 0
/// otherwise) and the hit time (`0097feb0`). `bHasForwardAttacks` says
/// whether any range was set; `bDeterminedPowerAttackDeltas` is set.
pub fn combat_controller_determine_power_attack_deltas(
    e: &mut Engine,
    this: Ptr<CombatController>,
) {
    let style = fn_00980070(e, this);
    let allowed = e.call(0x0055_24b0, &args![style]).u8() as i8;
    let zero: f64 = e.global(ZERO);
    // `FILD; FCOMP; TEST AH,0x41; JNZ`: true only when greater.
    let forward_allowed = allowed as f64 > zero;
    e.set(this, CombatController::bHasForwardAttacks, false);
    for index in 0..5u32 {
        let action = index + 0x5c;
        let distance = fn_00980090(e, this, action);
        let range_slot = this.addr() + 0xfc + 4 * index;
        if forward_allowed && distance as f64 > zero {
            let attacker = attacker_of(e, this);
            let scale = e.call(REFR_GET_SCALE, &args![attacker]).f64();
            e.mem.set_f32(range_slot, (scale * distance as f64) as f32);
            e.set(this, CombatController::bHasForwardAttacks, true);
        } else {
            e.mem.set_f32(range_slot, 0.0);
        }
        let attacker = attacker_of(e, this);
        let hit_time = fn_0097feb0(e, Ptr::new(attacker), action);
        e.mem.set_f32(this.addr() + 0x110 + 4 * index, hit_time);
    }
    e.set(this, CombatController::bDeterminedPowerAttackDeltas, true);
}

// Translated from 00980070 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at +0xf4 of `this` (the controller's `pCurrentCombatStyle`).
pub fn fn_00980070(e: &mut Engine, this: Ptr<CombatController>) -> u32 {
    e.mem.u32(this.addr().wrapping_add(0xf4))
}

// Translated from 00980090 (decompiled, FalloutNV.exe 1.4.0.525)
/// The y component of the attacker's power-attack movement for `action`
/// (`009800e0`), 0.0 when it has none.
pub fn fn_00980090(e: &mut Engine, this: Ptr<CombatController>, action: u32) -> f32 {
    e.with_stack(0x0c, |e, point| {
        for i in 0..3 {
            let word = e.mem.u32(POINT_011F426C + 4 * i);
            e.mem.set_u32(point.addr() + 4 * i, word);
        }
        if fn_009800e0(e, this, action, point.cast()) {
            x87_quiet(e.mem.f32(point.addr() + 4))
        } else {
            0.0
        }
    })
}

// Translated from 009800e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `0097fd80` for the attacker (through `0097ae90`): its power-attack
/// movement for `action` into `out`.
pub fn fn_009800e0(
    e: &mut Engine,
    this: Ptr<CombatController>,
    action: u32,
    out: Ptr<NiPoint3>,
) -> bool {
    let attacker = attacker_of(e, this);
    fn_0097fd80(e, Ptr::new(attacker), action, out)
}

// Translated from 00980110 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatController::SetActionProcedure` (Xbox PDB): ends (vtable slot 0xc)
/// and deletes the old action procedure, stores `procedure` and starts it
/// (slot 8 with the controller).
pub fn combat_controller_set_action_procedure(
    e: &mut Engine,
    this: Ptr<CombatController>,
    procedure: Ptr,
) {
    if !e.get(this, CombatController::pActionProcedure).is_null() {
        let old = e.get(this, CombatController::pActionProcedure);
        e.vcall(old.addr(), 0x0c, &[]);
        let old = e.get(this, CombatController::pActionProcedure);
        if !old.is_null() {
            e.vcall(old.addr(), 0x00, &args![1u32]);
        }
    }
    e.set(this, CombatController::pActionProcedure, procedure);
    action_vcall(e, this, 0x08, &args![this]);
}

// Translated from 009801b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatController::SetMovementProcedure` (Xbox PDB): as
/// `SetActionProcedure` for the movement procedure.
pub fn combat_controller_set_movement_procedure(
    e: &mut Engine,
    this: Ptr<CombatController>,
    procedure: Ptr,
) {
    if !e.get(this, CombatController::pMovementProcedure).is_null() {
        let old = e.get(this, CombatController::pMovementProcedure);
        e.vcall(old.addr(), 0x0c, &[]);
        let old = e.get(this, CombatController::pMovementProcedure);
        if !old.is_null() {
            e.vcall(old.addr(), 0x00, &args![1u32]);
        }
    }
    e.set(this, CombatController::pMovementProcedure, procedure);
    movement_vcall(e, this, 0x08, &args![this]);
}

// Translated from 00980250 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears both procedures.
pub fn fn_00980250(e: &mut Engine, this: Ptr<CombatController>) {
    combat_controller_set_action_procedure(e, this, Ptr::NULL);
    combat_controller_set_movement_procedure(e, this, Ptr::NULL);
}

// Translated from 00980270 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatController::AddBackgroundProcedure` (Xbox PDB): ends and deletes
/// the first background procedure of the same kind (vtable slot 0x34) and
/// removes it, appends `procedure` and starts it (slot 8 with the
/// controller).
pub fn combat_controller_add_background_procedure(
    e: &mut Engine,
    this: Ptr<CombatController>,
    procedure: Ptr,
) {
    let mut index: u32 = 0;
    while index < background_count(e, this) {
        let existing = background_at(e, this, index);
        let existing_kind = e.vcall(existing, 0x34, &[]).u32();
        let kind = e.vcall(procedure.addr(), 0x34, &[]).u32();
        if existing_kind == kind {
            release_and_delete(e, existing);
            e.call(ARRAY_REMOVE, &args![background(this), index, 1u32]);
            break;
        }
        index += 1;
    }
    // The array takes the address of the argument's stack slot.
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), procedure.addr());
        e.call(ARRAY_ADD, &args![background(this), slot]);
        let procedure = e.mem.u32(slot.addr());
        e.vcall(procedure, 0x08, &args![this]);
    });
}

// Translated from 00980360 (decompiled, FalloutNV.exe 1.4.0.525)
/// Ends and deletes every background procedure, then empties the array
/// (`008454f0(1)`).
pub fn fn_00980360(e: &mut Engine, this: Ptr<CombatController>) {
    let mut index: u32 = 0;
    while index < background_count(e, this) {
        let procedure = background_at(e, this, index);
        release_and_delete(e, procedure);
        index += 1;
    }
    e.call(0x0084_54f0, &args![background(this), 1u32]);
}

// Translated from 00980400 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatController::GetBackgroundProcedure` (Xbox PDB): the first
/// background procedure whose vtable slot 0x34 returns `kind`, or null.
pub fn combat_controller_get_background_procedure(
    e: &mut Engine,
    this: Ptr<CombatController>,
    kind: u32,
) -> Ptr {
    let mut index: u32 = 0;
    while index < background_count(e, this) {
        let procedure = background_at(e, this, index);
        if e.vcall(procedure, 0x34, &[]).u32() == kind {
            return Ptr::new(procedure);
        }
        index += 1;
    }
    Ptr::NULL
}

// Translated from 00980470 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether either procedure answers `0097e310` (word +8 is 1).
pub fn fn_00980470(e: &mut Engine, this: Ptr<CombatController>) -> bool {
    let action = e.get(this, CombatController::pActionProcedure);
    if !action.is_null() && fn_0097e310(e, action) {
        return true;
    }
    let movement = e.get(this, CombatController::pMovementProcedure);
    if !movement.is_null() && fn_0097e310(e, movement) {
        return true;
    }
    false
}

// Translated from 009804d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether a procedure answers `00666f00` true, or there is no procedure at
/// all.
pub fn fn_009804d0(e: &mut Engine, this: Ptr<CombatController>) -> bool {
    let action = e.get(this, CombatController::pActionProcedure);
    if !action.is_null() && e.call(0x0066_6f00, &args![action]).bool() {
        return true;
    }
    let movement = e.get(this, CombatController::pMovementProcedure);
    if !movement.is_null() && e.call(0x0066_6f00, &args![movement]).bool() {
        return true;
    }
    e.get(this, CombatController::pActionProcedure).is_null()
        && e.get(this, CombatController::pMovementProcedure).is_null()
}

// Translated from 00980550 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deletes the plan: `00980790` first, then the plan's scalar deleting
/// destructor (`0097fc40`), and clears the pointer.
pub fn fn_00980550(e: &mut Engine, this: Ptr<CombatController>) {
    if e.get(this, CombatController::pCombatPlan).is_null() {
        return;
    }
    e.call(0x0098_0790, &args![this]);
    let plan = e.get(this, CombatController::pCombatPlan);
    if !plan.is_null() {
        fn_0097fc40(e, plan, 1);
    }
    e.set(this, CombatController::pCombatPlan, Ptr::NULL);
}

// Translated from 009805b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Carries the planner data's two entries for the plan's index (`0044edb0`
/// of the plan, -1 without a plan) over to the combat state: the first
/// (`00980700`) through `009a0650`, the second (`00980720`) through
/// `009a0370`, then clears both (`00980740`). Without a first entry it
/// checks whether the state already has them (`00778970`/`00825c00`,
/// `0097b050`'s word +4). Then the state drops the first when it is not kept
/// (`009a0680`) and reserves the cover location (`00980770`) or clears it
/// (`CombatState::ClearCoverLocation(0)`).
pub fn fn_009805b0(e: &mut Engine, this: Ptr<CombatController>) {
    let index: u32 = if !e.get(this, CombatController::pCombatPlan).is_null() {
        let plan = e.get(this, CombatController::pCombatPlan);
        e.call(0x0044_edb0, &args![plan]).u32()
    } else {
        u32::MAX
    };
    let mut keep_first = false;
    let mut keep_second = false;
    if index != u32::MAX {
        let data = e.get(this, CombatController::pCombatPlannerData);
        let first = fn_00980700(e, data, index as i32);
        if first != 0 {
            keep_first = true;
            let state = e.get(this, CombatController::pCombatState);
            e.call(0x009a_0650, &args![state, first]);
            let data = e.get(this, CombatController::pCombatPlannerData);
            let second = fn_00980720(e, data, index as i32);
            if second != 0 {
                keep_second = true;
                let state = e.get(this, CombatController::pCombatState);
                e.call(0x009a_0370, &args![state, second]);
            }
            let data = e.get(this, CombatController::pCombatPlannerData);
            fn_00980740(e, data, index as i32);
        } else {
            let state = e.get(this, CombatController::pCombatState);
            let current = e.call(0x0077_8970, &args![state]).u32();
            if current != 0 && e.call(0x0082_5c00, &args![current]).u32() == index {
                keep_first = true;
            }
            let state = e.get(this, CombatController::pCombatState);
            let cover = e.call(0x0097_b050, &args![state]).u32();
            if cover != 0 && e.mem.u32(cover.wrapping_add(4)) == index {
                keep_second = true;
            }
        }
    }
    if !keep_first {
        let state = e.get(this, CombatController::pCombatState);
        e.call(0x009a_0680, &args![state]);
    }
    let state = e.get(this, CombatController::pCombatState);
    if keep_second {
        fn_00980770(e, state);
    } else {
        e.call(0x009a_03d0, &args![state, 0u32]);
    }
}

// Translated from 00980700 (decompiled, FalloutNV.exe 1.4.0.525)
/// Entry `index` of the word array at +8 of `this` (the planner data's
/// first table).
pub fn fn_00980700(e: &mut Engine, this: Ptr, index: i32) -> u32 {
    let offset = (index as u32).wrapping_mul(4).wrapping_add(8);
    e.mem.u32(this.addr().wrapping_add(offset))
}

// Translated from 00980720 (decompiled, FalloutNV.exe 1.4.0.525)
/// Entry `index` of the word array at +0x60 of `this` (the planner data's
/// second table).
pub fn fn_00980720(e: &mut Engine, this: Ptr, index: i32) -> u32 {
    let offset = (index as u32).wrapping_mul(4).wrapping_add(0x60);
    e.mem.u32(this.addr().wrapping_add(offset))
}

// Translated from 00980740 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears entry `index` of both word arrays (+8 and +0x60) of `this`.
pub fn fn_00980740(e: &mut Engine, this: Ptr, index: i32) {
    let scaled = (index as u32).wrapping_mul(4);
    e.mem
        .set_u32(this.addr().wrapping_add(scaled.wrapping_add(8)), 0);
    e.mem
        .set_u32(this.addr().wrapping_add(scaled.wrapping_add(0x60)), 0);
}

// Translated from 00980770 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatCoverMap::ReserveCoverLocation` (the Xbox PDB name of `009a0510`)
/// on `this` (a `CombatState`) with its word at +0xe4.
pub fn fn_00980770(e: &mut Engine, this: Ptr) {
    let location = e.mem.u32(this.addr().wrapping_add(0xe4));
    e.call(0x009a_0510, &args![this, location]);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x0097d3a0,
            combat_controller_combat_controller(
                Ptr<CombatController>,
                Ptr,
                Ptr,
                u32,
            ) -> Ptr<CombatController>
        ),
        entry!(
            0x0097d740,
            fn_0097d740(Ptr<CombatController>, u32) -> Ptr<CombatController>
        ),
        entry!(0x0097d770, fn_0097d770(Ptr<CombatController>)),
        entry!(0x0097d960, fn_0097d960(Ptr, u32) -> Ptr),
        entry!(0x0097d990, fn_0097d990(Ptr<CombatController>)),
        entry!(
            0x0097d9d0,
            combat_controller_calculate_attacker_info(Ptr<CombatController>)
        ),
        entry!(0x0097da50, fn_0097da50(Ptr<CombatController>)),
        entry!(0x0097e310, fn_0097e310(Ptr) -> bool),
        entry!(0x0097e330, fn_0097e330(Ptr) -> Ptr),
        entry!(0x0097e3e0, fn_0097e3e0(Ptr)),
        entry!(0x0097e460, combat_planner_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0097f310, combat_controller_check_has_equip(Ptr, u32) -> bool),
        entry!(0x0097f390, fn_0097f390(Ptr<CombatController>)),
        entry!(0x0097f3f0, fn_0097f3f0(Ptr<CombatController>)),
        entry!(0x0097f450, fn_0097f450(Ptr, u8)),
        entry!(
            0x0097f470,
            combat_controller_check_current_weapon(Ptr<CombatController>)
        ),
        entry!(
            0x0097f4d0,
            combat_controller_stop_combat(Ptr<CombatController>, Ptr) -> bool
        ),
        entry!(0x0097f580, fn_0097f580(Ptr<CombatController>, Ptr, u32)),
        entry!(0x0097f6d0, fn_0097f6d0(Ptr<CombatController>, u32)),
        entry!(0x0097f720, fn_0097f720(Ptr<CombatController>, u32)),
        entry!(0x0097f7a0, fn_0097f7a0(Ptr<CombatController>, u32, u8)),
        entry!(0x0097f7d0, fn_0097f7d0(Ptr<CombatController>, u32)),
        entry!(0x0097f7f0, fn_0097f7f0(Ptr<CombatController>, u32, f32)),
        entry!(
            0x0097f820,
            fn_0097f820(Ptr<CombatController>, u32, u32, u32, u8)
        ),
        entry!(0x0097f8c0, fn_0097f8c0(Ptr<CombatController>)),
        entry!(
            0x0097f930,
            combat_controller_add_target(Ptr<CombatController>, Ptr, u32, u32, f32, f32)
        ),
        entry!(
            0x0097f9c0,
            combat_controller_remove_target(Ptr<CombatController>, Ptr)
        ),
        entry!(
            0x0097fa10,
            combat_controller_is_actora_combat_target(Ptr<CombatController>, Ptr) -> bool
        ),
        entry!(0x0097fa50, fn_0097fa50(Ptr) -> bool),
        entry!(0x0097fa90, fn_0097fa90(Ptr) -> f32),
        entry!(0x0097fae0, fn_0097fae0(Ptr, Ptr) -> Ptr),
        entry!(0x0097fb00, fn_0097fb00(Ptr, Ptr) -> Ptr),
        entry!(0x0097fb20, fn_0097fb20(Ptr) -> u8),
        entry!(0x0097fb40, fn_0097fb40(Ptr, Ptr) -> Ptr),
        entry!(0x0097fb60, fn_0097fb60(Ptr) -> u32),
        entry!(0x0097fb80, fn_0097fb80(Ptr<CombatController>)),
        entry!(0x0097fc40, fn_0097fc40(Ptr, u32) -> Ptr),
        entry!(0x0097fc70, fn_0097fc70(Ptr<CombatController>) -> i32),
        entry!(0x0097fd40, fn_0097fd40(Ptr)),
        entry!(0x0097fd60, fn_0097fd60(Ptr<CombatController>, u32)),
        entry!(0x0097fd80, fn_0097fd80(Ptr, u32, Ptr<NiPoint3>) -> bool),
        entry!(0x0097feb0, fn_0097feb0(Ptr, u32) -> f32),
        entry!(
            0x0097ff60,
            combat_controller_determine_power_attack_deltas(Ptr<CombatController>)
        ),
        entry!(0x00980070, fn_00980070(Ptr<CombatController>) -> u32),
        entry!(0x00980090, fn_00980090(Ptr<CombatController>, u32) -> f32),
        entry!(
            0x009800e0,
            fn_009800e0(Ptr<CombatController>, u32, Ptr<NiPoint3>) -> bool
        ),
        entry!(
            0x00980110,
            combat_controller_set_action_procedure(Ptr<CombatController>, Ptr)
        ),
        entry!(
            0x009801b0,
            combat_controller_set_movement_procedure(Ptr<CombatController>, Ptr)
        ),
        entry!(0x00980250, fn_00980250(Ptr<CombatController>)),
        entry!(
            0x00980270,
            combat_controller_add_background_procedure(Ptr<CombatController>, Ptr)
        ),
        entry!(0x00980360, fn_00980360(Ptr<CombatController>)),
        entry!(
            0x00980400,
            combat_controller_get_background_procedure(Ptr<CombatController>, u32) -> Ptr
        ),
        entry!(0x00980470, fn_00980470(Ptr<CombatController>) -> bool),
        entry!(0x009804d0, fn_009804d0(Ptr<CombatController>) -> bool),
        entry!(0x00980550, fn_00980550(Ptr<CombatController>)),
        entry!(0x009805b0, fn_009805b0(Ptr<CombatController>)),
        entry!(0x00980700, fn_00980700(Ptr, i32) -> u32),
        entry!(0x00980720, fn_00980720(Ptr, i32) -> u32),
        entry!(0x00980740, fn_00980740(Ptr, i32)),
        entry!(0x00980770, fn_00980770(Ptr)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    type Log = Vec<(u32, Vec<u32>)>;

    fn ret(eax: u32) -> Ret {
        Ret {
            eax,
            ..Ret::default()
        }
    }

    /// A double returning `eax`.
    fn stub(e: &mut Engine, addr: u32, eax: u32) {
        e.register_double(addr, move |_, _| ret(eax));
    }

    /// A double returning `st0`.
    fn stub_st0(e: &mut Engine, addr: u32, st0: f64) {
        e.register_double(addr, move |_, _| Ret {
            st0,
            ..Ret::default()
        });
    }

    fn stubs(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            stub(e, *addr, 0);
        }
    }

    /// The argument words of every call to `addr`.
    fn calls(log: &Log, addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, w)| w.clone())
            .collect()
    }

    fn logged(e: &mut Engine, f: impl FnOnce(&mut Engine)) -> Log {
        e.call_log = Some(vec![]);
        f(e);
        e.call_log.take().unwrap()
    }

    /// An object whose vtable slot `s` calls address `base + s`.
    fn object(e: &mut Engine, base: u32, size: u32) -> u32 {
        let object = e.mem.alloc(size.max(0x10));
        let vtable = e.mem.alloc(0x800);
        for slot in (0..0x800).step_by(4) {
            e.mem.set_u32(vtable + slot, base + slot);
        }
        e.mem.set_u32(object, vtable);
        object
    }

    /// Doubles for the getters of other units and a working model of the
    /// `BSSimpleArray` calls; the constants the code reads.
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.register(GET_ATTACKER, |e, a| ret(e.mem.u32(a[0] + 0xbc)));
        e.register(GET_TARGET, |e, a| ret(e.mem.u32(a[0] + 0xc0)));
        e.register(GET_PROCESS, |e, a| ret(e.mem.u32(a[0] + 0x68)));
        e.register(0x0070_31c0, |e, a| ret(e.mem.u32(a[0] + 0xf0)));
        e.register(ARRAY_SIZE, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.register(ARRAY_ELEMENT, |e, a| ret(e.mem.u32(a[0] + 4) + 4 * a[1]));
        e.register(ARRAY_REMOVE, |e, a| {
            let (array, index, count) = (a[0], a[1], a[2]);
            let size = e.mem.u32(array + 8);
            let buffer = e.mem.u32(array + 4);
            for j in index..size - count {
                let word = e.mem.u32(buffer + 4 * (j + count));
                e.mem.set_u32(buffer + 4 * j, word);
            }
            e.mem.set_u32(array + 8, size - count);
            ret(0)
        });
        e.register(ARRAY_ADD, |e, a| {
            let array = a[0];
            let size = e.mem.u32(array + 8);
            let buffer = e.mem.u32(array + 4);
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(buffer + 4 * size, value);
            e.mem.set_u32(array + 8, size + 1);
            ret(0)
        });
        for page in [
            0x0101_2000,
            0x0101_7000,
            0x0101_e000,
            0x0101_f000,
            0x011a_4000,
            0x011c_3000,
            0x011d_d000,
            0x011d_e000,
            0x011f_1000,
            0x011f_4000,
        ] {
            e.map(page, 0x1000);
        }
        e.set_global(ONE_TENTH, 0.1f32 as f64);
        e.set_global(PLAN_TIMER_DELAY, 0.1f32);
        e.set_global(MINUS_ONE, -1.0f32);
        e.set_global(DEFAULT_HEIGHT, 128.0f32);
        e.set_global(DEFAULT_SNEAK_HEIGHT, 64.0f32);
        e.set_global(ZERO, 0.0f64);
        e.set_global(ELAPSED_LIMIT, 1000.0f64);
        e.set_global(UPDATE_ENABLED, 1u8);
        e
    }

    /// A controller with an attacker (vtable base 0x7100_0000, process at
    /// +0x68 with vtable base 0x7200_0000), a combat state, a combat group
    /// and an empty background array.
    fn controller(e: &mut Engine) -> Ptr<CombatController> {
        let this: Ptr<CombatController> = e.new_object();
        let attacker = object(e, 0x7100_0000, 0x200);
        let process = object(e, 0x7200_0000, 0x40);
        e.mem.set_u32(attacker + 0x68, process);
        e.set(this, CombatController::pAttacker, Ptr::new(attacker));
        let state = Ptr::new(e.mem.alloc(0x100));
        e.set(this, CombatController::pCombatState, state);
        let group = Ptr::new(e.mem.alloc(0x40));
        e.set(this, CombatController::pCombatGroup, group);
        let buffer = e.mem.alloc(0x40);
        e.mem.set_u32(this.addr() + 0x90, buffer);
        this
    }

    fn attacker(e: &Engine, this: Ptr<CombatController>) -> u32 {
        e.get(this, CombatController::pAttacker).addr()
    }

    /// A procedure (vtable base `base`) of `kind` whose word +8 is
    /// `flag_word`; its delete, update, start and end slots do nothing.
    fn procedure(e: &mut Engine, base: u32, kind: u32, flag_word: u32) -> u32 {
        let procedure = object(e, base, 0x40);
        e.mem.set_u32(procedure + 8, flag_word);
        stub(e, base + 0x34, kind);
        stubs(e, &[base, base + 0x04, base + 0x08, base + 0x0c]);
        procedure
    }

    fn set_background(e: &mut Engine, this: Ptr<CombatController>, procedures: &[u32]) {
        let buffer = e.mem.u32(this.addr() + 0x90);
        for (i, p) in procedures.iter().enumerate() {
            e.mem.set_u32(buffer + 4 * i as u32, *p);
        }
        e.mem.set_u32(this.addr() + 0x94, procedures.len() as u32);
    }

    fn background_list(e: &Engine, this: Ptr<CombatController>) -> Vec<u32> {
        let buffer = e.mem.u32(this.addr() + 0x90);
        let size = e.mem.u32(this.addr() + 0x94);
        (0..size).map(|i| e.mem.u32(buffer + 4 * i)).collect()
    }

    #[test]
    fn constructor_sets_defaults_and_updates_package_data() {
        let mut e = engine();
        stubs(
            &mut e,
            &[
                0x0067_07c0,
                0x0098_2430,
                0x0090_6f60,
                0x0063_3c90,
                0x0040_1050,
                COMBAT_TIMER_START,
                FILL_BYTES,
                0x0067_0fc0,
                0x0098_4f60,
                0x0098_0ae0,
            ],
        );
        stub(&mut e, OPERATOR_NEW, 0x0050_0000);
        stub(&mut e, 0x0099_7600, 0x0060_0000);
        e.set_global(CONTROLLER_COUNT, 13i32);
        let this: Ptr<CombatController> = e.new_object();
        let attacker = object(&mut e, 0x7100_0000, 0x100);
        let process = object(&mut e, 0x7200_0000, 0x40);
        e.mem.set_u32(attacker + 0x68, process);
        stub(&mut e, 0x7200_022c, 0x55);
        let log = logged(&mut e, |e| {
            let back = e.call(0x0097_d3a0, &args![this, attacker, 0x0070_0000u32, 0u32]);
            assert_eq!(back.u32(), this.addr());
        });
        assert_eq!(e.mem.u32(this.addr()), COMBAT_CONTROLLER_VTABLE);
        assert_eq!(e.global::<i32>(CONTROLLER_COUNT), 14);
        let state_call = &calls(&log, 0x0099_7600)[0];
        assert_eq!(state_call[..2], [0x0050_0000, this.addr()]);
        assert_eq!(
            f32::from_bits(state_call[2]),
            (3.0 * (0.1f32 as f64)) as f32
        );
        assert_eq!(
            e.get(this, CombatController::pCombatState).addr(),
            0x0060_0000
        );
        assert_eq!(
            e.get(this, CombatController::pCombatGroup).addr(),
            0x0070_0000
        );
        assert_eq!(e.get(this, CombatController::pAttacker).addr(), attacker);
        assert_eq!(e.get(this, CombatController::fCombatPosition), -1.0);
        assert_eq!(e.get(this, CombatController::fAttackerHeight), 128.0);
        assert_eq!(e.get(this, CombatController::fAttackerSneakHeight), 64.0);
        assert_eq!(e.get(this, CombatController::cUseIronSights), -1);
        assert!(e.get(this, CombatController::bAttackerCanMove));
        assert_eq!(e.get(this, CombatController::fRunSpeedMult), 1.0);
        assert_eq!(calls(&log, 0x0067_0fc0), vec![vec![this.addr(), 0x12]]);
        assert_eq!(calls(&log, 0x0098_4f60), vec![vec![this.addr(), 0x0c]]);
        assert_eq!(
            calls(&log, 0x0040_1050),
            vec![vec![this.addr() + 0x128, 0x0c, 2, 0x0068_15c0]]
        );
        assert_eq!(
            calls(&log, COMBAT_TIMER_START),
            vec![vec![this.addr() + 0xb4, 0.1f32.to_bits()]]
        );
        assert_eq!(calls(&log, FILL_BYTES).len(), 5);
        // No package given: the process's slot 0x22c supplies it.
        assert_eq!(calls(&log, 0x0098_0ae0), vec![vec![this.addr(), 0x55]]);

        // Without an attacker and with a failed allocation: no state, no
        // package update.
        stub(&mut e, OPERATOR_NEW, 0);
        let other: Ptr<CombatController> = e.new_object();
        let log = logged(&mut e, |e| {
            e.call(0x0097_d3a0, &args![other, 0u32, 0u32, 7u32]);
        });
        assert!(e.get(other, CombatController::pCombatState).is_null());
        assert!(calls(&log, 0x0099_7600).is_empty());
        assert!(calls(&log, 0x0098_0ae0).is_empty());
    }

    /// Doubles for every callee of the destructor.
    fn destructor_doubles(e: &mut Engine) {
        stubs(
            e,
            &[
                0x0042_ce10,
                0x0044_4d20,
                0x0042_26e0,
                SET_TARGET_00980830,
                0x008b_3d30,
                ACTOR_SET_IRON_SIGHTS,
                0x0098_69a0,
                0x0099_7aa0,
                OPERATOR_DELETE,
                0x0045_34f0,
                0x0047_c850,
                ACTOR_GET_BLOCKED,
                ACTOR_SET_BLOCK,
                0x0045_cec0,
                0x0098_2460,
                0x0067_08b0,
                0x0084_54f0,
            ],
        );
    }

    #[test]
    fn destructor_tears_down_in_order() {
        let mut e = engine();
        destructor_doubles(&mut e);
        stub(&mut e, 0x0042_ce10, 1);
        stub(&mut e, ACTOR_GET_BLOCKED, 1);
        let this = controller(&mut e);
        let state = e.get(this, CombatController::pCombatState);
        e.set_global(CONTROLLER_COUNT, 0i32);
        let log = logged(&mut e, |e| {
            e.call(0x0097_d770, &args![this]);
        });
        assert!(e.get(this, CombatController::bDestructing));
        // The save/load singleton answered 0042ce10 yes and 00444d20 no:
        // the group is dropped before the combat-group removal.
        assert!(e.get(this, CombatController::pCombatGroup).is_null());
        assert!(calls(&log, 0x0098_69a0).is_empty());
        assert_eq!(calls(&log, SET_TARGET_00980830), vec![vec![this.addr(), 0]]);
        assert_eq!(calls(&log, ACTOR_SET_IRON_SIGHTS)[0][1..], [0, 0, 0]);
        // The state is deleted through 0097d960.
        assert_eq!(calls(&log, 0x0099_7aa0), vec![vec![state.addr()]]);
        assert_eq!(calls(&log, OPERATOR_DELETE), vec![vec![state.addr()]]);
        assert_eq!(calls(&log, 0x0045_34f0), vec![vec![this.addr(), 1]]);
        assert_eq!(
            calls(&log, ACTOR_SET_BLOCK),
            vec![vec![attacker(&e, this), 0]]
        );
        // The count does not go below 0.
        assert_eq!(e.global::<i32>(CONTROLLER_COUNT), 0);
        let order: Vec<u32> = log
            .iter()
            .map(|(a, _)| *a)
            .filter(|a| [0x0045_cec0, 0x0098_2460, 0x0067_08b0].contains(a))
            .collect();
        assert_eq!(order, [0x0045_cec0, 0x0045_cec0, 0x0098_2460, 0x0067_08b0]);
        assert_eq!(calls(&log, 0x0045_cec0)[0], vec![this.addr() + 0xec]);
    }

    #[test]
    fn destructor_keeps_group_and_skips_clearing_when_asked() {
        let mut e = engine();
        destructor_doubles(&mut e);
        stub(&mut e, 0x0042_26e0, 1);
        stub(&mut e, 0x0047_c850, 1);
        let this = controller(&mut e);
        e.set_global(CONTROLLER_COUNT, 5i32);
        let group = e.get(this, CombatController::pCombatGroup);
        let log = logged(&mut e, |e| {
            e.call(0x0097_d770, &args![this]);
        });
        assert!(calls(&log, SET_TARGET_00980830).is_empty());
        assert_eq!(
            calls(&log, 0x0098_69a0),
            vec![vec![group.addr(), attacker(&e, this)]]
        );
        assert!(calls(&log, ACTOR_GET_BLOCKED).is_empty());
        assert_eq!(e.global::<i32>(CONTROLLER_COUNT), 4);
    }

    #[test]
    fn scalar_deleting_destructor_frees_on_flag() {
        let mut e = engine();
        destructor_doubles(&mut e);
        let this = controller(&mut e);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0097_d740, &args![this, 1u32]).u32(), this.addr());
        });
        assert!(calls(&log, OPERATOR_DELETE).contains(&vec![this.addr()]));
        let log = logged(&mut e, |e| {
            e.call(0x0097_d740, &args![this, 0u32]);
        });
        assert!(!calls(&log, OPERATOR_DELETE).contains(&vec![this.addr()]));
    }

    #[test]
    fn state_deleting_destructor_runs_00997aa0() {
        let mut e = engine();
        stubs(&mut e, &[0x0099_7aa0, OPERATOR_DELETE]);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0097_d960, &args![0x1234u32, 3u32]).u32(), 0x1234);
            e.call(0x0097_d960, &args![0x1238u32, 2u32]);
        });
        assert_eq!(calls(&log, 0x0099_7aa0), vec![vec![0x1234], vec![0x1238]]);
        assert_eq!(calls(&log, OPERATOR_DELETE), vec![vec![0x1234]]);
    }

    #[test]
    fn first_update_recalculates_for_process_type_zero() {
        let mut e = engine();
        let this = controller(&mut e);
        stubs(
            &mut e,
            &[
                0x0099_7c30,
                0x008a_1710,
                0x0088_53a0,
                0x0088_54d0,
                0x008e_ea60,
            ],
        );
        stub(&mut e, 0x0041_81e0, 0x99);
        stub(&mut e, 0x005f_0c80, 0);
        stub(&mut e, 0x0093_1850, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0097_d990, &args![this]);
        });
        let state = e.get(this, CombatController::pCombatState).addr();
        assert_eq!(calls(&log, 0x0099_7c30), vec![vec![state]]);
        assert_eq!(calls(&log, 0x0088_53a0).len(), 1);
        stub(&mut e, 0x0093_1850, 2);
        let log = logged(&mut e, |e| {
            e.call(0x0097_d990, &args![this]);
        });
        assert!(calls(&log, 0x0088_53a0).is_empty());
    }

    #[test]
    fn calculate_attacker_info_stores_heights_and_mobility() {
        let mut e = engine();
        let this = controller(&mut e);
        stub_st0(&mut e, 0x0088_53a0, 120.5);
        stub_st0(&mut e, 0x0088_54d0, 60.25);
        stub(&mut e, 0x008e_ea60, 0x0000_0102);
        stub(&mut e, 0x0041_81e0, 0x99);
        stub(&mut e, 0x005f_0c80, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0097_d9d0, &args![this]);
        });
        assert_eq!(e.get(this, CombatController::fAttackerHeight), 120.5);
        assert_eq!(e.get(this, CombatController::fAttackerSneakHeight), 60.25);
        assert_eq!(e.get(this, CombatController::bAttackerCanSneak), 2);
        assert!(!e.get(this, CombatController::bAttackerCanMove));
        assert_eq!(calls(&log, 0x0088_54d0)[0][1], 0);
        assert_eq!(calls(&log, 0x005f_0c80), vec![vec![0x99]]);
        stub(&mut e, 0x005f_0c80, 0);
        e.call(0x0097_d9d0, &args![this]);
        assert!(e.get(this, CombatController::bAttackerCanMove));
    }

    /// Doubles for a full update that passes every guard and does not
    /// replan; tests override the ones they vary.
    fn update_doubles(e: &mut Engine) {
        stub(e, 0x0098_1450, 0);
        stub(e, 0x7100_01e4, 0x0090_0000);
        stub(e, 0x7100_01d0, 1);
        stub(e, 0x7200_0610, 2);
        stubs(
            e,
            &[
                0x7100_0214,
                0x7100_02e8,
                0x7100_022c,
                0x7100_0434,
                0x7100_0230,
                0x7100_0234,
                0x7200_03e4,
                0x008a_1710,
                0x0099_7c30,
                0x0099_7cb0,
                0x009b_73b0,
                0x0049_3830,
                0x008a_7a40,
                0x008d_7f80,
                0x0066_6f00,
                0x0098_1740,
                ACTOR_SET_IRON_SIGHTS,
                RELEASE_PROCEDURE_0097F270,
            ],
        );
        stub(e, 0x0093_1850, 1);
        stub(e, 0x0049_3bb0, 1);
    }

    #[test]
    fn update_does_nothing_when_disabled_or_stopping() {
        let mut e = engine();
        let this = controller(&mut e);
        e.set_global(UPDATE_ENABLED, 0u8);
        let log = logged(&mut e, |e| {
            e.call(0x0097_da50, &args![this]);
        });
        assert_eq!(log.len(), 1);
        e.set_global(UPDATE_ENABLED, 1u8);
        e.set(this, CombatController::bStoppingCombat, true);
        let log = logged(&mut e, |e| {
            e.call(0x0097_da50, &args![this]);
        });
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn update_hands_over_to_slot_434_when_asked() {
        let mut e = engine();
        let this = controller(&mut e);
        update_doubles(&mut e);
        stub(&mut e, 0x0098_1450, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0097_da50, &args![this]);
        });
        assert_eq!(calls(&log, 0x7100_0434), vec![vec![attacker(&e, this), 0]]);
        assert!(calls(&log, 0x7100_01e4).is_empty());

        // A target that stops the update is passed to slot 0x434.
        stub(&mut e, 0x0098_1450, 0);
        let target = object(&mut e, 0x7300_0000, 0x40);
        stub(&mut e, 0x7300_022c, 1);
        stub(&mut e, 0x7300_02e8, 0);
        e.set(this, CombatController::pTarget, Ptr::new(target));
        let log = logged(&mut e, |e| {
            e.call(0x0097_da50, &args![this]);
        });
        assert_eq!(
            calls(&log, 0x7100_0434),
            vec![vec![attacker(&e, this), target]]
        );
        assert!(e.get(this, CombatController::bFirstUpdate));
        assert!(calls(&log, 0x0099_7cb0).is_empty());
    }

    #[test]
    fn update_runs_procedures_and_deletes_finished_background_ones() {
        let mut e = engine();
        let this = controller(&mut e);
        update_doubles(&mut e);
        e.set(this, CombatController::bFirstUpdate, true);
        let action = procedure(&mut e, 0x7400_0000, 1, 0);
        e.set(this, CombatController::pActionProcedure, Ptr::new(action));
        let finished = procedure(&mut e, 0x7500_0000, 2, 1);
        let running = procedure(&mut e, 0x7600_0000, 3, 0);
        set_background(&mut e, this, &[finished, running]);
        e.set(this, CombatController::cUseIronSights, 1);
        e.set(this, CombatController::eLowState, 7);
        let log = logged(&mut e, |e| {
            e.call(0x0097_da50, &args![this]);
        });
        // Not replanned: the action procedure only updates.
        assert!(calls(&log, PLANNER_PLAN).is_empty());
        assert_eq!(calls(&log, 0x7400_0004), vec![vec![action]]);
        // The background procedure whose word +8 is 1 is released and
        // deleted; the other one stays.
        assert_eq!(
            calls(&log, RELEASE_PROCEDURE_0097F270),
            vec![vec![this.addr(), finished]]
        );
        assert_eq!(calls(&log, 0x7500_0000), vec![vec![finished, 1]]);
        assert_eq!(background_list(&e, this), vec![running]);
        assert_eq!(calls(&log, 0x7600_0004), vec![vec![running]]);
        assert_eq!(
            calls(&log, ACTOR_SET_IRON_SIGHTS),
            vec![vec![attacker(&e, this), 1, 0, 0]]
        );
        assert_eq!(e.get(this, CombatController::eLowState), 0);
    }

    #[test]
    fn update_replans_when_the_plan_timer_fires() {
        let mut e = engine();
        let this = controller(&mut e);
        update_doubles(&mut e);
        e.set(this, CombatController::bFirstUpdate, true);
        stub(&mut e, 0x008d_7f80, 1);
        stub(&mut e, 0x0049_3bb0, 0);
        let plan = e.mem.alloc(0x20);
        stub(&mut e, PLANNER_PLAN, plan);
        stub(&mut e, PLAN_GOAL_0084E3A0, 0x4444);
        stub(&mut e, 0x0044_edb0, u32::MAX);
        stub(&mut e, GROUP_CHOOSE_TARGET, 0x5555);
        stubs(
            &mut e,
            &[
                0x0084_54f0,
                SET_TARGET_00980830,
                0x0048_3710,
                0x0098_2480,
                0x009a_a470,
                0x00aa_53f0,
                0x0099_58b0,
                0x0053_7e90,
                0x0099_5830,
                0x0099_5910,
                0x00aa_5460,
                0x0098_2530,
                0x009a_0680,
                0x009a_03d0,
                COMBAT_TIMER_START,
                PLAN_START_00994050,
                0x0099_a9a0,
            ],
        );
        let log = logged(&mut e, |e| {
            e.call(0x0097_da50, &args![this]);
        });
        assert_eq!(
            calls(&log, SET_TARGET_00980830),
            vec![vec![this.addr(), 0x5555]]
        );
        let plan_call = &calls(&log, PLANNER_PLAN)[0];
        assert_eq!(plan_call[1..], [this.addr(), 0]);
        assert_eq!(e.get(this, CombatController::pCombatPlan).addr(), plan);
        assert_eq!(e.get(this, CombatController::pCurrentGoal).addr(), 0x4444);
        assert_eq!(
            calls(&log, COMBAT_TIMER_START),
            vec![vec![this.addr() + 0xb4, 1.0f32.to_bits()]]
        );
        assert_eq!(
            calls(&log, PLAN_START_00994050),
            vec![vec![plan, this.addr()]]
        );
        assert_eq!(calls(&log, 0x0048_3710).len(), 2);
        // The stack planner was built and destroyed around the plan.
        assert_eq!(calls(&log, 0x0098_2480).len(), 1);
        assert_eq!(calls(&log, 0x0098_2530).len(), 1);
        // Background procedures are not run when 00493bb0 says no.
        assert!(calls(&log, 0x0098_1740).is_empty());
    }

    #[test]
    fn procedure_flag_word_is_one() {
        let mut e = engine();
        let p = e.mem.alloc(0x10);
        e.mem.set_u32(p + 8, 1);
        assert!(e.call(0x0097_e310, &args![p]).bool());
        e.mem.set_u32(p + 8, 2);
        assert!(!e.call(0x0097_e310, &args![p]).bool());
    }

    fn planner_doubles(e: &mut Engine) {
        stubs(
            e,
            &[
                0x0098_2480,
                0x009a_a470,
                0x00aa_53f0,
                0x0099_58b0,
                0x0053_7e90,
                0x0099_5830,
                0x0099_5910,
                0x00aa_5460,
                0x0098_2530,
                OPERATOR_DELETE,
            ],
        );
    }

    #[test]
    fn planner_constructor_builds_members() {
        let mut e = engine();
        planner_doubles(&mut e);
        let planner = e.mem.alloc(COMBAT_PLANNER_STACK_SIZE);
        e.mem.set_u32(planner + 0x2058, 9);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0097_e330, &args![planner]).u32(), planner);
        });
        assert_eq!(e.mem.u32(planner), COMBAT_PLANNER_VTABLE);
        assert_eq!(e.mem.u32(planner + 0x2058), 0);
        assert_eq!(calls(&log, 0x00aa_53f0), vec![vec![planner + 0x20b8]]);
        assert_eq!(
            calls(&log, 0x0053_7e90),
            vec![vec![planner + 0x20c8, planner]]
        );
    }

    #[test]
    fn planner_destructor_and_deleting_destructor() {
        let mut e = engine();
        planner_doubles(&mut e);
        let planner = e.mem.alloc(COMBAT_PLANNER_STACK_SIZE);
        let log = logged(&mut e, |e| {
            e.call(0x0097_e3e0, &args![planner]);
        });
        let order: Vec<u32> = log.iter().skip(1).map(|(a, _)| *a).collect();
        assert_eq!(order, [0x0099_5830, 0x0099_5910, 0x00aa_5460, 0x0098_2530]);
        assert_eq!(e.mem.u32(planner), COMBAT_PLANNER_VTABLE);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0097_e460, &args![planner, 1u32]).u32(), planner);
        });
        assert_eq!(calls(&log, OPERATOR_DELETE), vec![vec![planner]]);
        let log = logged(&mut e, |e| {
            e.call(0x0097_e460, &args![planner, 0u32]);
        });
        assert!(calls(&log, OPERATOR_DELETE).is_empty());
    }

    #[test]
    fn check_has_equip_looks_up_the_equip_group() {
        let mut e = engine();
        let actor = object(&mut e, 0x7100_0000, 0x40);
        stub(&mut e, ACTOR_GET_ANIM_GROUP, 0x0001_0042);
        stub(&mut e, 0x7100_01e4, 0x0800);
        stub(&mut e, ANIM_MAP_OF, 0x0900);
        stub(&mut e, ANIM_MAP_GET_AT, 1);
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0097_f310, &args![actor, 0u32]).bool());
        });
        assert_eq!(
            calls(&log, ACTOR_GET_ANIM_GROUP),
            vec![vec![actor, 0x18, 0, 1, 0]]
        );
        assert_eq!(calls(&log, ANIM_MAP_GET_AT)[0][..2], [0x0900, 0x42]);
        stub(&mut e, ANIM_MAP_GET_AT, 0);
        assert!(!e.call(0x0097_f310, &args![actor, 5u32]).bool());
        stub(&mut e, 0x7100_01e4, 0);
        assert!(!e.call(0x0097_f310, &args![actor, 5u32]).bool());
        stub(&mut e, ACTOR_GET_ANIM_GROUP, 0x0001_0000);
        let log = logged(&mut e, |e| {
            assert!(!e.call(0x0097_f310, &args![actor, 5u32]).bool());
        });
        assert_eq!(calls(&log, ACTOR_GET_ANIM_GROUP)[0][2..], [5, 0, 0]);
        assert!(calls(&log, 0x7100_01e4).is_empty());
    }

    #[test]
    fn current_weapon_comes_from_procedure_five_or_the_actor() {
        let mut e = engine();
        let this = controller(&mut e);
        stub(&mut e, 0x008a_1710, 0xaaaa);
        e.call(0x0097_f390, &args![this]);
        assert_eq!(e.get(this, CombatController::pCurrentWeapon).addr(), 0xaaaa);
        let p = procedure(&mut e, 0x7400_0000, 5, 0);
        set_background(&mut e, this, &[p]);
        stub(&mut e, 0x0044_edb0, 0xbbbb);
        let log = logged(&mut e, |e| {
            e.call(0x0097_f390, &args![this]);
        });
        assert_eq!(e.get(this, CombatController::pCurrentWeapon).addr(), 0xbbbb);
        assert_eq!(calls(&log, 0x0044_edb0), vec![vec![p]]);
    }

    #[test]
    fn combat_style_change_flags_the_state() {
        let mut e = engine();
        let this = controller(&mut e);
        stub(&mut e, 0x008a_7f80, 0x10);
        stub(&mut e, AFTER_TARGET_CHANGE_0097F200, 0);
        // From none: no flag.
        let log = logged(&mut e, |e| {
            e.call(0x0097_f3f0, &args![this]);
        });
        assert!(calls(&log, AFTER_TARGET_CHANGE_0097F200).is_empty());
        // The same style: no flag.
        e.call(0x0097_f3f0, &args![this]);
        let state = e.get(this, CombatController::pCombatState).addr();
        assert_eq!(e.mem.u8(state + 0x77), 0);
        // Changed: the flag and 0097f200.
        stub(&mut e, 0x008a_7f80, 0x20);
        let log = logged(&mut e, |e| {
            e.call(0x0097_f3f0, &args![this]);
        });
        assert_eq!(e.mem.u8(state + 0x77), 1);
        assert_eq!(
            calls(&log, AFTER_TARGET_CHANGE_0097F200),
            vec![vec![this.addr()]]
        );
        assert_eq!(
            e.get(this, CombatController::pCurrentCombatStyle).addr(),
            0x20
        );
    }

    #[test]
    fn byte_77_setter() {
        let mut e = engine();
        let p = e.mem.alloc(0x80);
        e.call(0x0097_f450, &args![p, 0x1ffu32]);
        assert_eq!(e.mem.u8(p + 0x77), 0xff);
    }

    #[test]
    fn check_current_weapon_drops_a_weapon_the_state_does_not_offer() {
        let mut e = engine();
        let this = controller(&mut e);
        stub(&mut e, 0x0052_2c80, 4);
        let log = logged(&mut e, |e| {
            e.call(0x0097_f470, &args![this]);
        });
        assert_eq!(log.len(), 1);
        e.set(this, CombatController::pCurrentWeapon, Ptr::new(0x77));
        stub(&mut e, 0x0097_c150, 0x77);
        let log = logged(&mut e, |e| {
            e.call(0x0097_f470, &args![this]);
        });
        assert_eq!(e.get(this, CombatController::pCurrentWeapon).addr(), 0x77);
        let state = e.get(this, CombatController::pCombatState).addr();
        assert_eq!(calls(&log, 0x0097_c150), vec![vec![state, 4]]);
        stub(&mut e, 0x0097_c150, 0x78);
        e.call(0x0097_f470, &args![this]);
        assert!(e.get(this, CombatController::pCurrentWeapon).is_null());
    }

    #[test]
    fn stop_combat_retargets_and_clears_when_no_target_is_left() {
        let mut e = engine();
        let this = controller(&mut e);
        stubs(
            &mut e,
            &[0x0098_6500, AFTER_TARGET_CHANGE_0097F200, ACTOR_SET_BLOCK],
        );
        e.register(SET_TARGET_00980830, |e, a| {
            e.mem.set_u32(a[0] + 0xc0, a[1]);
            ret(0)
        });
        stub(&mut e, GROUP_CHOOSE_TARGET, 0);
        stub(&mut e, ACTOR_GET_BLOCKED, 1);
        e.set(this, CombatController::pTarget, Ptr::new(0x3333));
        let log = logged(&mut e, |e| {
            assert!(!e.call(0x0097_f4d0, &args![this, 0x3333u32]).bool());
        });
        let group = e.get(this, CombatController::pCombatGroup).addr();
        assert_eq!(calls(&log, 0x0098_6500), vec![vec![group, 0x3333]]);
        assert_eq!(calls(&log, SET_TARGET_00980830), vec![vec![this.addr(), 0]]);
        assert_eq!(calls(&log, AFTER_TARGET_CHANGE_0097F200).len(), 1);
        assert_eq!(calls(&log, ACTOR_SET_BLOCK).len(), 1);
        // A null target: true, only the block is checked.
        stub(&mut e, ACTOR_GET_BLOCKED, 0);
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0097_f4d0, &args![this, 0u32]).bool());
        });
        assert!(calls(&log, 0x0098_6500).is_empty());
        assert!(calls(&log, ACTOR_SET_BLOCK).is_empty());
    }

    #[test]
    fn new_target_is_added_when_the_attacker_should_attack() {
        let mut e = engine();
        let this = controller(&mut e);
        e.set(this, CombatController::bFirstUpdate, true);
        let actor = e.mem.alloc(0x80);
        let process = object(&mut e, 0x7700_0000, 0x40);
        e.mem.set_u32(actor + 0x68, process);
        stubs(&mut e, &[0x0098_9230, 0x0098_6410]);
        stub(&mut e, 0x0098_65b0, 0);
        stub(&mut e, 0x008b_06d0, 1);
        stub_st0(&mut e, 0x7700_0208, 37.9);
        e.register(FTOL, |_, a| ret(f64::take(a, &mut 0) as i32 as u32));
        let log = logged(&mut e, |e| {
            e.call(0x0097_f580, &args![this, actor, 0u32]);
        });
        let group = e.get(this, CombatController::pCombatGroup).addr();
        let me = attacker(&e, this);
        assert_eq!(calls(&log, 0x0098_9230), vec![vec![group, me, actor]]);
        let should = &calls(&log, 0x008b_06d0)[0];
        assert_eq!(
            [should[0], should[1], should[2], should[4]],
            [me, actor, 1, 0]
        );
        assert_eq!(calls(&log, 0x7700_0208), vec![vec![process, actor, me]]);
        assert_eq!(calls(&log, FTOL), vec![args![37.9f64]]);
        assert_eq!(calls(&log, 0x0098_6410), vec![vec![group, actor]]);
        // Already a target: nothing added.
        stub(&mut e, 0x0098_65b0, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0097_f580, &args![this, actor, 0u32]);
        });
        assert!(calls(&log, 0x008b_06d0).is_empty());
        // The attacker itself is never added.
        let log = logged(&mut e, |e| {
            e.call(0x0097_f580, &args![this, me, 0u32]);
        });
        assert!(calls(&log, 0x0098_65b0).is_empty());
    }

    #[test]
    fn state_and_procedure_forwarders() {
        let mut e = engine();
        let this = controller(&mut e);
        let state = e.get(this, CombatController::pCombatState).addr();
        let action = procedure(&mut e, 0x7400_0000, 1, 0);
        let movement = procedure(&mut e, 0x7500_0000, 1, 0);
        e.set(this, CombatController::pActionProcedure, Ptr::new(action));
        e.set(
            this,
            CombatController::pMovementProcedure,
            Ptr::new(movement),
        );
        stubs(
            &mut e,
            &[
                0x0099_a720,
                0x0099_a5e0,
                0x0099_a650,
                0x0099_a680,
                0x0099_a6b0,
                0x0099_a7d0,
                0x0099_a8d0,
                0x004f_ebd0,
                0x7400_001c,
                0x7400_0020,
                0x7500_0020,
                0x7400_0024,
                0x7500_0024,
                0x7400_0028,
                0x7500_0028,
            ],
        );
        e.set(this, CombatController::bDeterminedPowerAttackDeltas, true);
        let signalling = f32::from_bits(0x7f80_0001);
        let log = logged(&mut e, |e| {
            e.call(0x0097_f6d0, &args![this, 5u32]);
            e.call(0x0097_f720, &args![this, 6u32]);
            e.call(0x0097_f7a0, &args![this, 7u32, 0x1ffu32]);
            e.call(0x0097_f7d0, &args![this, 8u32]);
            e.call(0x0097_f7f0, &args![this, 9u32, signalling]);
            e.call(0x0097_f820, &args![this, 1u32, 2u32, 3u32, 0x104u32]);
            e.call(0x0097_f8c0, &args![this]);
            e.call(0x0097_fd60, &args![this, 10u32]);
        });
        assert!(!e.get(this, CombatController::bDeterminedPowerAttackDeltas));
        assert_eq!(calls(&log, 0x0099_a720), vec![vec![state]]);
        assert_eq!(calls(&log, 0x7400_001c), vec![vec![action, 5]]);
        assert_eq!(calls(&log, 0x0099_a5e0), vec![vec![state, 6]]);
        assert_eq!(calls(&log, 0x7500_0020), vec![vec![movement, 6]]);
        assert_eq!(calls(&log, 0x0099_a650), vec![vec![state, 7, 0xff]]);
        assert_eq!(calls(&log, 0x0099_a680), vec![vec![state, 8]]);
        // The float goes through the x87 stack: a signalling NaN comes out
        // quiet.
        assert_eq!(calls(&log, 0x0099_a6b0), vec![vec![state, 9, 0x7fc0_0001]]);
        assert_eq!(calls(&log, 0x0099_a7d0), vec![vec![state, 1, 2, 3, 4]]);
        assert_eq!(calls(&log, 0x7500_0028), vec![vec![movement, 1, 2, 3, 4]]);
        assert_eq!(calls(&log, 0x7400_0024), vec![vec![action]]);
        assert_eq!(calls(&log, 0x0099_a8d0), vec![vec![state]]);
        assert_eq!(calls(&log, 0x004f_ebd0), vec![vec![state, 10]]);
        // Without procedures only the state hears about it.
        e.set(this, CombatController::pActionProcedure, Ptr::NULL);
        e.set(this, CombatController::pMovementProcedure, Ptr::NULL);
        let log = logged(&mut e, |e| {
            e.call(0x0097_f720, &args![this, 6u32]);
        });
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn add_target_splits_the_player_group() {
        let mut e = engine();
        let this = controller(&mut e);
        let player = object(&mut e, 0x7800_0000, 0x40);
        e.set_global(PLAYER_CHARACTER, player);
        e.set_global(COMBAT_MANAGER, 0x0abc_u32);
        let group = e.get(this, CombatController::pCombatGroup).addr();
        stub(&mut e, 0x7800_03f8, group);
        stubs(&mut e, &[CHANGE_PERCEIVED_HOSTILE_STATUS, 0x0098_6410]);
        stub(&mut e, 0x0099_1f80, 0x0def);
        let log = logged(&mut e, |e| {
            e.call(
                0x0097_f930,
                &args![this, player, 1u32, 2u32, 0.0f32, 0.0f32],
            );
        });
        let me = attacker(&e, this);
        assert_eq!(
            calls(&log, CHANGE_PERCEIVED_HOSTILE_STATUS),
            vec![vec![player, me, 1]]
        );
        assert_eq!(calls(&log, 0x0099_1f80), vec![vec![0x0abc, me]]);
        assert_eq!(calls(&log, 0x0098_6410), vec![vec![0x0def, player]]);
        // Another actor goes straight to the group; null is ignored.
        let log = logged(&mut e, |e| {
            e.call(
                0x0097_f930,
                &args![this, 0x4444u32, 0u32, 0u32, 0.0f32, 0.0f32],
            );
            e.call(0x0097_f930, &args![this, 0u32, 0u32, 0u32, 0.0f32, 0.0f32]);
        });
        assert_eq!(calls(&log, 0x0098_6410), vec![vec![0x0def, 0x4444]]);
        assert!(calls(&log, CHANGE_PERCEIVED_HOSTILE_STATUS).is_empty());
    }

    #[test]
    fn remove_target_and_is_target() {
        let mut e = engine();
        let this = controller(&mut e);
        let player = 0x0000_9000u32;
        e.set_global(PLAYER_CHARACTER, player);
        stubs(&mut e, &[CHANGE_PERCEIVED_HOSTILE_STATUS, 0x0098_6500]);
        stub(&mut e, 0x0098_65b0, 1);
        let group = e.get(this, CombatController::pCombatGroup).addr();
        let log = logged(&mut e, |e| {
            e.call(0x0097_f9c0, &args![this, player]);
            assert!(e.call(0x0097_fa10, &args![this, 0x55u32]).bool());
        });
        let me = attacker(&e, this);
        assert_eq!(
            calls(&log, CHANGE_PERCEIVED_HOSTILE_STATUS),
            vec![vec![player, me, 0]]
        );
        assert_eq!(calls(&log, 0x0098_6500), vec![vec![group, player]]);
        assert_eq!(calls(&log, 0x0098_65b0), vec![vec![group, 0x55]]);
        e.set(this, CombatController::pCombatGroup, Ptr::NULL);
        let log = logged(&mut e, |e| {
            e.call(0x0097_f9c0, &args![this, 0x55u32]);
            assert!(!e.call(0x0097_fa10, &args![this, 0x55u32]).bool());
        });
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn float_at_4_is_nonzero() {
        let mut e = engine();
        let p = e.mem.alloc(8);
        assert!(!e.call(0x0097_fa50, &args![p]).bool());
        e.mem.set_f32(p + 4, -0.0);
        assert!(!e.call(0x0097_fa50, &args![p]).bool());
        e.mem.set_f32(p + 4, 0.5);
        assert!(e.call(0x0097_fa50, &args![p]).bool());
        e.mem.set_f32(p + 4, f32::NAN);
        assert!(e.call(0x0097_fa50, &args![p]).bool());
    }

    #[test]
    fn elapsed_time_is_capped() {
        let mut e = engine();
        let p = e.mem.alloc(8);
        e.mem.set_f32(p, 100.0);
        stub_st0(&mut e, 0x0043_5dd0, 350.5);
        assert_eq!(e.call(0x0097_fa90, &args![p]).f32(), 250.5);
        stub_st0(&mut e, 0x0043_5dd0, 1100.5);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0097_fa90, &args![p]).f32(), 1000.0);
        });
        assert_eq!(calls(&log, 0x0043_5dd0).len(), 1);
        // Exactly at the cap reads the clock again.
        stub_st0(&mut e, 0x0043_5dd0, 1100.0);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0097_fa90, &args![p]).f32(), 1000.0);
        });
        assert_eq!(calls(&log, 0x0043_5dd0).len(), 2);
    }

    #[test]
    fn small_getters() {
        let mut e = engine();
        let p = e.mem.alloc(0x100);
        let out = e.mem.alloc(4);
        e.mem.set_u32(p + 0x94, 1);
        e.mem.set_u32(p + 0x98, 2);
        e.mem.set_u32(p + 0x9c, 3);
        e.mem.set_u32(p + 0xe8, 4);
        e.mem.set_u8(p + 0x71, 5);
        e.mem.set_u32(p + 0xf4, 6);
        assert_eq!(e.call(0x0097_fae0, &args![p, out]).u32(), out);
        assert_eq!(e.mem.u32(out), 1);
        e.call(0x0097_fb00, &args![p, out]);
        assert_eq!(e.mem.u32(out), 2);
        e.call(0x0097_fb40, &args![p, out]);
        assert_eq!(e.mem.u32(out), 3);
        assert_eq!(e.call(0x0097_fb60, &args![p]).u32(), 4);
        assert_eq!(e.call(0x0097_fb20, &args![p]).u8(), 5);
        assert_eq!(e.call(0x0098_0070, &args![p]).u32(), 6);
    }

    #[test]
    fn stack_planner_plan_is_deleted() {
        let mut e = engine();
        planner_doubles(&mut e);
        let this = controller(&mut e);
        stub(&mut e, PLANNER_PLAN, 0x6000);
        stub(&mut e, 0x0099_3f70, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0097_fb80, &args![this]);
        });
        let plan = &calls(&log, PLANNER_PLAN)[0];
        assert_eq!(plan[1..], [this.addr(), 1]);
        assert_eq!(calls(&log, 0x0099_3f70), vec![vec![0x6000]]);
        assert_eq!(calls(&log, OPERATOR_DELETE), vec![vec![0x6000]]);
        assert_eq!(calls(&log, 0x0098_2530).len(), 1);
        stub(&mut e, PLANNER_PLAN, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0097_fb80, &args![this]);
        });
        assert!(calls(&log, 0x0099_3f70).is_empty());
    }

    #[test]
    fn plan_deleting_destructor() {
        let mut e = engine();
        stubs(&mut e, &[0x0099_3f70, OPERATOR_DELETE]);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0097_fc40, &args![0x60u32, 1u32]).u32(), 0x60);
            e.call(0x0097_fc40, &args![0x64u32, 0u32]);
        });
        assert_eq!(calls(&log, 0x0099_3f70).len(), 2);
        assert_eq!(calls(&log, OPERATOR_DELETE), vec![vec![0x60]]);
    }

    #[test]
    fn skill_value_for_actors_and_creatures() {
        let mut e = engine();
        let this = controller(&mut e);
        let me = attacker(&e, this);
        stub(&mut e, 0x7100_021c, 0);
        stub(&mut e, 0x0066_ef20, 42);
        stub(&mut e, 0x008d_85e0, 0x21);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0097_fc70, &args![this]).i32(), 42);
        });
        assert_eq!(calls(&log, 0x0066_ef20), vec![vec![me + 0xa4, 0x2d]]);
        e.set(this, CombatController::pCurrentWeapon, Ptr::new(0x900));
        let log = logged(&mut e, |e| {
            e.call(0x0097_fc70, &args![this]);
        });
        assert_eq!(calls(&log, 0x008d_85e0), vec![vec![0x900]]);
        assert_eq!(calls(&log, 0x0066_ef20), vec![vec![me + 0xa4, 0x21]]);
        // A creature: the truncated 008d46f0.
        stub(&mut e, 0x7100_021c, 1);
        stub(&mut e, RT_DYNAMIC_CAST, me);
        stub_st0(&mut e, 0x008d_46f0, 55.75);
        e.register(FTOL, |_, a| ret(f64::take(a, &mut 0) as i32 as u32));
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0097_fc70, &args![this]).i32(), 55);
        });
        assert_eq!(
            calls(&log, RT_DYNAMIC_CAST),
            vec![vec![me, 0, 0x0118_46d4, 0x011a_106c, 0]]
        );
        stub(&mut e, RT_DYNAMIC_CAST, 0);
        assert_eq!(e.call(0x0097_fc70, &args![this]).i32(), 0);
    }

    #[test]
    fn forwards_three_to_00461580() {
        let mut e = engine();
        stub(&mut e, 0x0046_1580, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0097_fd40, &args![0x88u32]);
        });
        assert_eq!(calls(&log, 0x0046_1580), vec![vec![0x88, 3]]);
    }

    /// Doubles for the power-attack lookups of an actor with vtable base
    /// 0x7100_0000: group 0x55, a sequence whose slot 0x10 gives 0x66,
    /// animation group 0x77 with times 0, 0.25 and 0.75 for 0, 1 and 2; the
    /// `005f4c90` vector (1, 2, 3); `0045bb20` scales; the scale is 2.
    fn power_attack_doubles(e: &mut Engine) {
        stub(e, ACTOR_GET_ANIM_GROUP, 0x55);
        stub(e, IS_POWER_ATTACK_ACTION, 1);
        stub(e, 0x7100_01e4, 0x700);
        stub(e, ANIM_MAP_OF, 0x800);
        let sequence = object(e, 0x7900_0000, 0x10);
        e.register_double(ANIM_MAP_GET_AT, move |e, a| {
            e.mem.set_u32(a[2], sequence);
            ret(1)
        });
        stub(e, 0x7900_0010, 0x66);
        stub(e, ANIM_GROUP_OF, 0x77);
        e.register(ANIM_GROUP_GET_TIME, |_, a| Ret {
            st0: [0.0, 0.25, 0.75][a[1] as usize],
            ..Ret::default()
        });
        e.register(0x005f_4c90, |e, a| {
            for (i, v) in [1.0f32, 2.0, 3.0].iter().enumerate() {
                e.mem.set_f32(a[1] + 4 * i as u32, *v);
            }
            ret(a[1])
        });
        e.register(0x0045_bb20, |e, a| {
            let factor = f32::from_bits(a[2]);
            for i in 0..3 {
                let v = e.mem.f32(a[0] + 4 * i) * factor;
                e.mem.set_f32(a[1] + 4 * i, v);
            }
            ret(a[1])
        });
        stub_st0(e, REFR_GET_SCALE, 2.0);
    }

    #[test]
    fn power_attack_movement_scales_the_vector() {
        let mut e = engine();
        let actor = object(&mut e, 0x7100_0000, 0x80);
        power_attack_doubles(&mut e);
        let out = e.mem.alloc(12);
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0097_fd80, &args![actor, 0x5cu32, out]).bool());
        });
        // (1, 2, 3) * 0.75 * 2.0.
        assert_eq!(e.mem.f32(out), 1.5);
        assert_eq!(e.mem.f32(out + 4), 3.0);
        assert_eq!(e.mem.f32(out + 8), 4.5);
        assert_eq!(
            calls(&log, ACTOR_GET_ANIM_GROUP),
            vec![vec![actor, 0x5c, 0, 0, 0]]
        );
        assert_eq!(calls(&log, IS_POWER_ATTACK_ACTION), vec![vec![0x55]]);
        assert_eq!(calls(&log, 0x7900_0010)[0][1], u32::MAX);
        // Not a power attack: the starting point and false.
        stub(&mut e, IS_POWER_ATTACK_ACTION, 0);
        e.set_global(POINT_011F426C + 4, 9.0f32);
        assert!(!e.call(0x0097_fd80, &args![actor, 0x5cu32, out]).bool());
        assert_eq!(e.mem.f32(out + 4), 9.0);
        assert!(!e.call(0x0097_fd80, &args![0u32, 0x5cu32, out]).bool());
    }

    #[test]
    fn power_attack_hit_time() {
        let mut e = engine();
        let actor = object(&mut e, 0x7100_0000, 0x80);
        power_attack_doubles(&mut e);
        assert_eq!(e.call(0x0097_feb0, &args![actor, 0x5du32]).f32(), 0.25);
        stub(&mut e, ANIM_MAP_GET_AT, 0);
        assert_eq!(e.call(0x0097_feb0, &args![actor, 0x5du32]).f32(), -1.0);
        stub(&mut e, IS_POWER_ATTACK_ACTION, 0);
        assert_eq!(e.call(0x0097_feb0, &args![actor, 0x5du32]).f32(), -1.0);
    }

    #[test]
    fn power_attack_deltas_for_five_actions() {
        let mut e = engine();
        let this = controller(&mut e);
        power_attack_doubles(&mut e);
        e.set(this, CombatController::pCurrentCombatStyle, Ptr::new(0x500));
        stub(&mut e, 0x0055_24b0, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0097_ff60, &args![this]);
        });
        assert_eq!(calls(&log, 0x0055_24b0), vec![vec![0x500]]);
        let actions: Vec<u32> = calls(&log, ACTOR_GET_ANIM_GROUP)
            .iter()
            .map(|w| w[1])
            .collect();
        assert_eq!(
            actions,
            [0x5c, 0x5c, 0x5d, 0x5d, 0x5e, 0x5e, 0x5f, 0x5f, 0x60, 0x60]
        );
        for i in 0..5 {
            // y = 2 * 0.75 * 2 (scale), times the scale again.
            assert_eq!(e.mem.f32(this.addr() + 0xfc + 4 * i), 6.0);
            assert_eq!(e.mem.f32(this.addr() + 0x110 + 4 * i), 0.25);
        }
        assert!(e.get(this, CombatController::bHasForwardAttacks));
        assert!(e.get(this, CombatController::bDeterminedPowerAttackDeltas));
        // A style byte that is not positive (0x80 is -128): no ranges.
        stub(&mut e, 0x0055_24b0, 0x80);
        e.call(0x0097_ff60, &args![this]);
        assert_eq!(e.mem.f32(this.addr() + 0xfc), 0.0);
        assert!(!e.get(this, CombatController::bHasForwardAttacks));
    }

    #[test]
    fn forward_distance_and_attacker_query() {
        let mut e = engine();
        let this = controller(&mut e);
        let me = attacker(&e, this);
        power_attack_doubles(&mut e);
        assert_eq!(e.call(0x0098_0090, &args![this, 0x5cu32]).f32(), 3.0);
        let out = e.mem.alloc(12);
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0098_00e0, &args![this, 0x5cu32, out]).bool());
        });
        assert_eq!(calls(&log, ACTOR_GET_ANIM_GROUP)[0][0], me);
        stub(&mut e, IS_POWER_ATTACK_ACTION, 0);
        assert_eq!(e.call(0x0098_0090, &args![this, 0x5cu32]).f32(), 0.0);
    }

    #[test]
    fn set_procedures_end_delete_and_start() {
        let mut e = engine();
        let this = controller(&mut e);
        let old = procedure(&mut e, 0x7400_0000, 1, 0);
        let new = procedure(&mut e, 0x7500_0000, 1, 0);
        e.set(this, CombatController::pActionProcedure, Ptr::new(old));
        let log = logged(&mut e, |e| {
            e.call(0x0098_0110, &args![this, new]);
        });
        let order: Log = log.into_iter().skip(1).collect();
        assert_eq!(
            order,
            vec![
                (0x7400_000c, vec![old]),
                (0x7400_0000, vec![old, 1]),
                (0x7500_0008, vec![new, this.addr()]),
            ]
        );
        assert_eq!(e.get(this, CombatController::pActionProcedure).addr(), new);
        let log = logged(&mut e, |e| {
            e.call(0x0098_01b0, &args![this, old]);
        });
        assert_eq!(calls(&log, 0x7400_0008), vec![vec![old, this.addr()]]);
        assert_eq!(
            e.get(this, CombatController::pMovementProcedure).addr(),
            old
        );
        let log = logged(&mut e, |e| {
            e.call(0x0098_0250, &args![this]);
        });
        assert_eq!(calls(&log, 0x7500_0000), vec![vec![new, 1]]);
        assert_eq!(calls(&log, 0x7400_0000), vec![vec![old, 1]]);
        assert!(e.get(this, CombatController::pActionProcedure).is_null());
        assert!(e.get(this, CombatController::pMovementProcedure).is_null());
    }

    #[test]
    fn background_procedures_replace_find_and_clear() {
        let mut e = engine();
        let this = controller(&mut e);
        let first = procedure(&mut e, 0x7400_0000, 1, 0);
        let second = procedure(&mut e, 0x7500_0000, 2, 0);
        let replacement = procedure(&mut e, 0x7600_0000, 1, 0);
        set_background(&mut e, this, &[first, second]);
        let log = logged(&mut e, |e| {
            e.call(0x0098_0270, &args![this, replacement]);
        });
        assert_eq!(background_list(&e, this), vec![second, replacement]);
        assert_eq!(calls(&log, 0x7400_0000), vec![vec![first, 1]]);
        assert_eq!(
            calls(&log, 0x7600_0008),
            vec![vec![replacement, this.addr()]]
        );
        assert_eq!(e.call(0x0098_0400, &args![this, 1u32]).u32(), replacement);
        assert_eq!(e.call(0x0098_0400, &args![this, 3u32]).u32(), 0);
        // Clearing deletes every one, then empties the array.
        e.register(0x0084_54f0, |e, a| {
            e.mem.set_u32(a[0] + 8, 0);
            ret(0)
        });
        let log = logged(&mut e, |e| {
            e.call(0x0098_0360, &args![this]);
        });
        assert_eq!(calls(&log, 0x7500_0000), vec![vec![second, 1]]);
        assert_eq!(calls(&log, 0x7600_0000), vec![vec![replacement, 1]]);
        assert_eq!(calls(&log, 0x0084_54f0), vec![vec![this.addr() + 0x8c, 1]]);
        assert!(background_list(&e, this).is_empty());
    }

    #[test]
    fn procedure_state_queries() {
        let mut e = engine();
        let this = controller(&mut e);
        stub(&mut e, 0x0066_6f00, 0);
        // No procedures: none answers 0097e310, and "all done".
        assert!(!e.call(0x0098_0470, &args![this]).bool());
        assert!(e.call(0x0098_04d0, &args![this]).bool());
        let movement = procedure(&mut e, 0x7500_0000, 1, 1);
        e.set(
            this,
            CombatController::pMovementProcedure,
            Ptr::new(movement),
        );
        assert!(e.call(0x0098_0470, &args![this]).bool());
        assert!(!e.call(0x0098_04d0, &args![this]).bool());
        stub(&mut e, 0x0066_6f00, 1);
        assert!(e.call(0x0098_04d0, &args![this]).bool());
    }

    #[test]
    fn plan_is_deleted_after_00980790() {
        let mut e = engine();
        let this = controller(&mut e);
        stubs(&mut e, &[0x0098_0790, 0x0099_3f70, OPERATOR_DELETE]);
        let log = logged(&mut e, |e| {
            e.call(0x0098_0550, &args![this]);
        });
        assert_eq!(log.len(), 1);
        e.set(this, CombatController::pCombatPlan, Ptr::new(0x6000));
        let log = logged(&mut e, |e| {
            e.call(0x0098_0550, &args![this]);
        });
        let order: Vec<u32> = log.iter().skip(1).map(|(a, _)| *a).collect();
        assert_eq!(order, [0x0098_0790, 0x0099_3f70, OPERATOR_DELETE]);
        assert!(e.get(this, CombatController::pCombatPlan).is_null());
    }

    #[test]
    fn planner_data_is_carried_to_the_state() {
        let mut e = engine();
        let this = controller(&mut e);
        let state = e.get(this, CombatController::pCombatState).addr();
        e.mem.set_u32(state + 0xe4, 0xc0c0);
        let data = e.mem.alloc(0xc0);
        e.mem.set_u32(data + 8 + 4 * 2, 0x111);
        e.mem.set_u32(data + 0x60 + 4 * 2, 0x222);
        e.set(this, CombatController::pCombatPlannerData, Ptr::new(data));
        e.set(this, CombatController::pCombatPlan, Ptr::new(0x6000));
        stub(&mut e, 0x0044_edb0, 2);
        stubs(
            &mut e,
            &[
                0x009a_0650,
                0x009a_0370,
                0x009a_0680,
                0x009a_0510,
                0x009a_03d0,
            ],
        );
        let log = logged(&mut e, |e| {
            e.call(0x0098_05b0, &args![this]);
        });
        assert_eq!(calls(&log, 0x009a_0650), vec![vec![state, 0x111]]);
        assert_eq!(calls(&log, 0x009a_0370), vec![vec![state, 0x222]]);
        assert!(calls(&log, 0x009a_0680).is_empty());
        assert_eq!(calls(&log, 0x009a_0510), vec![vec![state, 0xc0c0]]);
        assert_eq!(e.mem.u32(data + 8 + 4 * 2), 0);
        assert_eq!(e.mem.u32(data + 0x60 + 4 * 2), 0);
        // Nothing to carry and nothing kept: drop and clear.
        stub(&mut e, 0x0077_8970, 0);
        stub(&mut e, 0x0097_b050, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0098_05b0, &args![this]);
        });
        assert_eq!(calls(&log, 0x009a_0680), vec![vec![state]]);
        assert_eq!(calls(&log, 0x009a_03d0), vec![vec![state, 0]]);
        // The state already holds the entries for this index.
        let cover = e.mem.alloc(8);
        e.mem.set_u32(cover + 4, 2);
        stub(&mut e, 0x0077_8970, 0x900);
        stub(&mut e, 0x0082_5c00, 2);
        stub(&mut e, 0x0097_b050, cover);
        let log = logged(&mut e, |e| {
            e.call(0x0098_05b0, &args![this]);
        });
        assert!(calls(&log, 0x009a_0680).is_empty());
        assert_eq!(calls(&log, 0x009a_0510).len(), 1);
        // Without a plan the index is -1: drop and clear.
        e.set(this, CombatController::pCombatPlan, Ptr::NULL);
        let log = logged(&mut e, |e| {
            e.call(0x0098_05b0, &args![this]);
        });
        assert!(calls(&log, 0x0044_edb0).is_empty());
        assert_eq!(calls(&log, 0x009a_0680).len(), 1);
    }

    #[test]
    fn planner_data_tables() {
        let mut e = engine();
        let data = e.mem.alloc(0xc0);
        e.mem.set_u32(data + 8 + 12, 7);
        e.mem.set_u32(data + 0x60 + 12, 8);
        assert_eq!(e.call(0x0098_0700, &args![data, 3u32]).u32(), 7);
        assert_eq!(e.call(0x0098_0720, &args![data, 3u32]).u32(), 8);
        e.call(0x0098_0740, &args![data, 3u32]);
        assert_eq!(e.mem.u32(data + 8 + 12), 0);
        assert_eq!(e.mem.u32(data + 0x60 + 12), 0);
    }

    #[test]
    fn reserves_cover_with_word_e4() {
        let mut e = engine();
        let state = e.mem.alloc(0x100);
        e.mem.set_u32(state + 0xe4, 0x1234);
        stub(&mut e, 0x009a_0510, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0098_0770, &args![state]);
        });
        assert_eq!(calls(&log, 0x009a_0510), vec![vec![state, 0x1234]]);
    }
}
