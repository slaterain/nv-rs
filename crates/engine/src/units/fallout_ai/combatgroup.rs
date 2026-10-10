//! `fallout/ai/combat/combatgroup.cpp` (Xbox PDB source unit), subsystem `fallout/ai`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit holds `CombatGroup` (a group of combat actors with the targets
//! they fight), the two element classes of its arrays (`CombatTarget`,
//! `CombatMember`) and the small `CombatGroupCluster` / `CombatSearchLocation`
//! records. The unit is translated over several sessions, in address order:
//! this part covers `0069cfe0` and `00985410` to `00986c60`; the next
//! function of the queue is `009871c0`.
//!
//! Conventions of this file:
//! - Layouts first (below); embedded structs that other units own
//!   (`BGSWorldLocation`, 0x10 bytes; `CombatTimer`, 8 bytes;
//!   `CombatTimeStamp`, a `float`) are addressed through offset constants.
//! - Tiny accessors the game calls (array size, element `i`, a form's ID)
//!   are called by address like the game does; they are folded template
//!   instances, so their constants are named by what the body does.
//! - `CombatGroup`'s `BSSimpleArray<CombatTarget>` is at +0x08 (elements of
//!   0x68 bytes), its `BSSimpleArray<CombatMember>` at +0x18 (0x14 bytes).
//! - The compiler's exception-unwinding frames in the constructor,
//!   destructor and `009862e0` are not translated.
//! - x87 note: comparisons of `float`s follow the disassembly's flag tests
//!   (strict `<` / `>`, and a NaN fails them); sums stay in `f32`.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::BSSimpleArray;

/// Global holding the `CombatManager*` singleton.
const COMBAT_MANAGER: u32 = 0x011f_1958;
/// `BSSimpleArray<T>::GetSize`-like accessor (`this->iSize` at +8).
const ARRAY_SIZE: u32 = 0x0044_ddc0;
/// The group's target count (`TargetArray.iSize`), taking the group.
const TARGET_COUNT: u32 = 0x005a_4320;
/// The group's member count (`MemberArray.iSize`), taking the group.
const MEMBER_COUNT: u32 = 0x0099_0890;
/// Element `i` of a `BSSimpleArray<CombatTarget>` (address of the element).
const TARGET_AT: u32 = 0x0096_a2b0;
/// Element `i` of a `BSSimpleArray<CombatMember>` (address of the element).
const MEMBER_AT: u32 = 0x006d_ad90;
/// The actor of the group's target `i`, taking the group.
const TARGET_ACTOR_AT: u32 = 0x0096_8630;
/// The actor of the group's member `i`, taking the group.
const MEMBER_ACTOR_AT: u32 = 0x0090_3600;
/// Element `i` of a `BSSimpleArray<CombatGroupCluster *>` (address of the
/// slot).
const CLUSTER_AT: u32 = 0x006a_7ad0;
/// `SetAt(i, value)` of a `BSSimpleArray<T *>`.
const ARRAY_SET_AT: u32 = 0x0047_a110;
/// `BSSimpleArray<T *>::SetSize(size, 1)`.
const POINTER_ARRAY_SET_SIZE: u32 = 0x006f_2290;
/// `BSSimpleArray<CombatSearchDoor>::SetSize(size, 1)`.
const SEARCH_DOOR_ARRAY_SET_SIZE: u32 = 0x0069_cea0;
/// Element `i` of a `BSSimpleArray<CombatSearchDoor>`.
const SEARCH_DOOR_AT: u32 = 0x006a_1440;
/// `TESForm::iFormID` getter (`this + 0xc`); the same body reads
/// `BGSWorldLocation::pSpace`.
const FORM_ID: u32 = 0x0084_e3a0;
/// `CombatTimeStamp` value (`float` at `this`), in ST0.
const TIME_STAMP_VALUE: u32 = 0x006a_7f50;
/// Game time now (`float` global at `011f1bf0`), in ST0.
const GAME_TIME_NOW: u32 = 0x0043_5dd0;
/// `CombatTimeStamp` setter (`this = value`).
const TIME_STAMP_SET: u32 = 0x0043_5de0;
/// `CombatTimeStamp` zero constructor.
const TIME_STAMP_ZERO: u32 = 0x0043_00a0;
/// Game time now minus the stamp at `this`, in ST0.
const TIME_STAMP_AGE: u32 = 0x0043_5e00;
/// `CombatTimer` constructor (zeroes both floats).
const TIMER_CONSTRUCT: u32 = 0x0090_6f60;
/// `CombatTimer` has expired (game time minus start reaches the target
/// time), in AL.
const TIMER_EXPIRED: u32 = 0x008d_7f80;
/// `CombatTimer::Start(delay)`: start time now, target time `delay`.
const TIMER_START: u32 = 0x008d_7f40;
/// `CombatTimer` is running (target time not zero), in AL.
const TIMER_RUNNING: u32 = 0x0097_fa50;
/// `BGSWorldLocation` constructor (copies a default point, space null).
const WORLD_LOCATION_CONSTRUCT: u32 = 0x0097_ede0;
/// `BGSWorldLocation::GetDistanceSquared` (this, other), in ST0.
const DISTANCE_SQUARED: u32 = 0x0052_7a40;
/// `BGSWorldLocation::SaveGame` (this, buffer).
const WORLD_LOCATION_SAVE: u32 = 0x0052_7b70;
/// `BGSWorldLocation::LoadGame` (this, buffer).
const WORLD_LOCATION_LOAD: u32 = 0x0052_7ba0;
/// `CombatTimeStamp::SaveGame` (this, buffer).
const TIME_STAMP_SAVE: u32 = 0x009a_5f90;
/// `CombatTimeStamp::LoadGame` (this, buffer).
const TIME_STAMP_LOAD: u32 = 0x009a_5ff0;
/// `BGSSaveGameBuffer::SaveFormID_ov2` (buffer, form, 0).
const SAVE_FORM_ID: u32 = 0x0086_5df0;
/// `BGSSaveGameBuffer` raw write (buffer, data, size, 0).
const SAVE_BYTES: u32 = 0x0086_5e50;
/// `BGSLoadGameBuffer::LoadFormID_ov2` (buffer, address of the field).
const LOAD_FORM_ID: u32 = 0x0086_48e0;
/// `BGSLoadGameBuffer` raw read (buffer, data, size).
const LOAD_BYTES: u32 = 0x0086_4980;
/// `TESObjectREFR::GetWorldLocation(this, out)`.
const GET_WORLD_LOCATION: u32 = 0x0057_5650;
/// `Setting` value pointer (`this + 4`, or a static zero when `this` is
/// null): the setting's `float`.
const SETTING_FLOAT_VALUE: u32 = 0x0040_3e20;
/// The same for a `bool` setting.
const SETTING_BOOL_VALUE: u32 = 0x0040_8d60;
/// `Actor::IsFleeing(this, 0)`.
const ACTOR_IS_FLEEING: u32 = 0x008a_6650;
/// The actor test `008b06d0(this, target, 0, &out, 0)` (the engine map
/// calls it `Actor::GetShouldAttackActor`).
const ACTOR_GET_SHOULD_ATTACK: u32 = 0x008b_06d0;
/// Whether `this` accepts `other` as a combat partner (`this`, `other`),
/// in AL.
const ACTOR_COMBAT_COMPATIBLE: u32 = 0x008b_0670;
/// `CombatManager` calls: (manager; group, actor).
const MANAGER_ADD_TARGET: u32 = 0x0099_26c0;
const MANAGER_REMOVE_TARGET: u32 = 0x0099_26e0;
const MANAGER_ADD_MEMBER: u32 = 0x0099_2700;
const MANAGER_REMOVE_MEMBER: u32 = 0x008d_0600;
/// `BSSimpleArray<CombatTarget>::Add(element)` (returns the index).
const TARGET_ARRAY_ADD: u32 = 0x0098_eec0;
/// `BSSimpleArray<CombatTarget>::RemoveAt(index, 1)`.
const TARGET_ARRAY_REMOVE_AT: u32 = 0x0098_ef10;
/// `BSSimpleArray<CombatMember>::Add(element)`.
const MEMBER_ARRAY_ADD: u32 = 0x0098_f180;
/// `BSSimpleArray<CombatMember>::RemoveAt(index, 1)`.
const MEMBER_ARRAY_REMOVE_AT: u32 = 0x0098_f1f0;
/// The game's `TintScenegraph(node, color, 0)` / `UnTintScenegraph(node)`.
const TINT_SCENEGRAPH: u32 = 0x004b_6360;
const UNTINT_SCENEGRAPH: u32 = 0x004b_6630;
/// `printf`-style game log (`format, ...`).
const LOG_MESSAGE: u32 = 0x005b_5e40;
/// Setting holding the "tint combat members" flag (`bool`).
const TINT_SETTING: u32 = 0x011f_1878;
/// Setting holding the "only targets in the actor's cell" flag (`bool`).
const SAME_CELL_SETTING: u32 = 0x011f_1840;
/// `FLT_MAX` (`-FLT_MAX` is its negation).
const MAX_FLOAT: u32 = 0x0108_d540;
/// `-1.0f`.
const MINUS_ONE: u32 = 0x0101_2054;
/// `0.0` as a `double`.
const ZERO_DOUBLE: u32 = 0x0101_2060;

layout! {
    /// `CombatTarget` (Xbox PDB), 0x68 bytes on both builds. The four
    /// `BGSWorldLocation`s (0x10 bytes each) are at +0x08, +0x18, +0x28 and
    /// +0x38; see the `*_LOCATION` constants.
    pub struct CombatTarget: 0x68 {
        /// `pActor` (Xbox PDB): `Actor*`.
        0x00 pActor: Ptr,
        /// `iDetectionLevel` (Xbox PDB).
        0x04 iDetectionLevel: i32,
        /// `sLastSearchNoticed` (Xbox PDB).
        0x48 sLastSearchNoticed: u16,
        /// `sAttackerCount` (Xbox PDB).
        0x4a sAttackerCount: u16,
        /// `fLastSeenTimeStamp` (Xbox PDB): a `CombatTimeStamp`.
        0x4c fLastSeenTimeStamp: f32,
        /// `fLastDetectedTimeStamp` (Xbox PDB).
        0x50 fLastDetectedTimeStamp: f32,
        /// `fDetectionLevelUpdateTimeStamp` (Xbox PDB).
        0x54 fDetectionLevelUpdateTimeStamp: f32,
        /// `fLastNoticedTimeStamp` (Xbox PDB).
        0x58 fLastNoticedTimeStamp: f32,
        /// `fDetectionEventTimeStamp` (Xbox PDB).
        0x5c fDetectionEventTimeStamp: f32,
        /// `fLastAttackedMemberTimeStamp` (Xbox PDB).
        0x60 fLastAttackedMemberTimeStamp: f32,
        /// `cMemberLOSCount` (Xbox PDB).
        0x64 cMemberLOSCount: u8,
        /// `cMember360LOSCount` (Xbox PDB).
        0x65 cMember360LOSCount: u8,
    }

    /// `CombatMember` (Xbox PDB), 0x14 bytes.
    pub struct CombatMember: 0x14 {
        /// `pActor` (Xbox PDB): `Actor*`.
        0x00 pActor: Ptr,
        /// `iGroupStrategyAssignment` (Xbox PDB).
        0x04 iGroupStrategyAssignment: u32,
        /// `fDamagePerSecond` (Xbox PDB).
        0x08 fDamagePerSecond: f32,
        /// `fCombatStrength` (Xbox PDB).
        0x0c fCombatStrength: f32,
        /// `pCluster` (Xbox PDB): `CombatGroupCluster*`.
        0x10 pCluster: Ptr,
    }

    /// `CombatSearchLocation` (Xbox PDB), 0x1c bytes: a `BGSWorldLocation`
    /// at +0 followed by these fields.
    pub struct CombatSearchLocation: 0x1c {
        /// `fTimeStamp` (Xbox PDB): a `CombatTimeStamp`.
        0x10 fTimeStamp: f32,
        /// `iTargetID` (Xbox PDB).
        0x14 iTargetID: u32,
        /// `fScore` (Xbox PDB).
        0x18 fScore: f32,
    }

    /// `CombatGroup` (Xbox PDB), 0x15c bytes, every scalar field the
    /// constructor sets. Embedded `CombatTimer`s, `BGSWorldLocation`s and
    /// `BSSimpleArray`s are addressed with the offset constants of
    /// `impl CombatGroup`.
    pub struct CombatGroup: 0x15c {
        /// `iGroupNum` (Xbox PDB).
        0x00 iGroupNum: u32,
        /// `iGroupID` (Xbox PDB).
        0x04 iGroupID: u32,
        /// `pGroupStrategy` (Xbox PDB): `CombatGroupStrategy*`.
        0x28 pGroupStrategy: Ptr,
        /// `bStrategyForced` (Xbox PDB).
        0x2c bStrategyForced: bool,
        /// `iLastGroupStrategyChosenIndex` (Xbox PDB).
        0x40 iLastGroupStrategyChosenIndex: u32,
        /// `fLastGroupStrategyChosenTimeStamp` (Xbox PDB).
        0x44 fLastGroupStrategyChosenTimeStamp: f32,
        /// `fMemberCombatStrength` (Xbox PDB).
        0xc0 fMemberCombatStrength: f32,
        /// `fAverageMemberDamagePerSecond` (Xbox PDB).
        0xc4 fAverageMemberDamagePerSecond: f32,
        /// `fTargetCombatStrength` (Xbox PDB).
        0xc8 fTargetCombatStrength: f32,
        /// `fAverageTargetCombatStrength` (Xbox PDB).
        0xcc fAverageTargetCombatStrength: f32,
        /// `iSearchCount` (Xbox PDB).
        0xd0 iSearchCount: u32,
        /// `spPathingLOSGridMap` (Xbox PDB): an `NiPointer`.
        0xd4 spPathingLOSGridMap: Ptr,
        /// `fSearchStartedTimeStamp` (Xbox PDB).
        0xe8 fSearchStartedTimeStamp: f32,
        /// `pSearchingMember` (Xbox PDB): `Actor*`.
        0xec pSearchingMember: Ptr,
        /// `fSearchRadius` (Xbox PDB).
        0x10c fSearchRadius: f32,
        /// `spSearchDebugGeometry` (Xbox PDB): `NiPointer<NiNode>`.
        0x130 spSearchDebugGeometry: Ptr,
        /// `bUpdateSearchDebugGeometry` (Xbox PDB).
        0x134 bUpdateSearchDebugGeometry: bool,
        /// `iInitializedMemberCount` (Xbox PDB).
        0x148 iInitializedMemberCount: u32,
        /// `iFleeingMemberCount` (Xbox PDB).
        0x14c iFleeingMemberCount: u32,
        /// `iNonFleeingMemberCount` (Xbox PDB).
        0x150 iNonFleeingMemberCount: u32,
        /// `cCombatMusicState` (Xbox PDB); -1 until the music timer decides.
        0x154 cCombatMusicState: i8,
        /// `spDebugGeometry` (Xbox PDB): `NiPointer<NiNode>`.
        0x158 spDebugGeometry: Ptr,
    }
}

impl CombatTarget {
    /// `LastNoticedLocation` (a `BGSWorldLocation`).
    pub const LAST_NOTICED_LOCATION: u32 = 0x08;
    /// `LastDetectedLocation`.
    pub const LAST_DETECTED_LOCATION: u32 = 0x18;
    /// `LastSeenLocation`.
    pub const LAST_SEEN_LOCATION: u32 = 0x28;
    /// `LastAttackedMemberLocation`.
    pub const LAST_ATTACKED_MEMBER_LOCATION: u32 = 0x38;
}

impl CombatGroup {
    /// `TargetArray` (`BSSimpleArray<CombatTarget, 1024>`).
    pub const TARGET_ARRAY: u32 = 0x08;
    /// `MemberArray` (`BSSimpleArray<CombatMember, 1024>`).
    pub const MEMBER_ARRAY: u32 = 0x18;
    /// `ChooseStrategyTimer`, `UpdateStrategyTimer` (`CombatTimer`s).
    pub const CHOOSE_STRATEGY_TIMER: u32 = 0x30;
    pub const UPDATE_STRATEGY_TIMER: u32 = 0x38;
    /// `AvoidThreatDialogueTimer`, `CombatStrengthTimer`.
    pub const AVOID_THREAT_DIALOGUE_TIMER: u32 = 0x48;
    pub const COMBAT_STRENGTH_TIMER: u32 = 0x50;
    /// `DetectionDialogueTimers`: ten `CombatTimer`s.
    pub const DETECTION_DIALOGUE_TIMERS: u32 = 0x58;
    /// `TargetUpdateTimer`, `ClusterUpdateTimer`, `CombatMusicUpdateTimer`.
    pub const TARGET_UPDATE_TIMER: u32 = 0xa8;
    pub const CLUSTER_UPDATE_TIMER: u32 = 0xb0;
    pub const COMBAT_MUSIC_UPDATE_TIMER: u32 = 0xb8;
    /// `SearchUpdateTimer`, `SearchAreaUpdateTimer`.
    pub const SEARCH_UPDATE_TIMER: u32 = 0xd8;
    pub const SEARCH_AREA_UPDATE_TIMER: u32 = 0xe0;
    /// `SearchCenter` (`BGSWorldLocation`).
    pub const SEARCH_CENTER: u32 = 0xf0;
    /// `SearchFocalPoint` (`NiPoint3`).
    pub const SEARCH_FOCAL_POINT: u32 = 0x100;
    /// `SearchLocations` (`BSSimpleArray<CombatSearchLocation, 1024>`).
    pub const SEARCH_LOCATIONS: u32 = 0x110;
    /// `SearchTeleportDoors` (`BSSimpleArray<CombatSearchDoor, 1024>`).
    pub const SEARCH_TELEPORT_DOORS: u32 = 0x120;
    /// `ClusterArray` (`BSSimpleArray<CombatGroupCluster *, 1024>`).
    pub const CLUSTER_ARRAY: u32 = 0x138;
}

/// The singleton `CombatManager*` (read each time, as the game does).
fn combat_manager(e: &Engine) -> u32 {
    e.global::<u32>(COMBAT_MANAGER)
}

fn time_stamp(e: &mut Engine, stamp: Ptr) -> f32 {
    e.call(TIME_STAMP_VALUE, &args![stamp]).f32()
}

/// Copies one `BGSWorldLocation` (four words, as the game does).
fn copy_location(e: &mut Engine, to: Ptr, from: Ptr) {
    for word in 0..4 {
        let value = e.mem.u32(from.addr() + 4 * word);
        e.mem.set_u32(to.addr() + 4 * word, value);
    }
}

/// Reads the byte a `bool` setting accessor points at.
fn bool_setting(e: &mut Engine, setting: u32) -> bool {
    let value = e.call(SETTING_BOOL_VALUE, &args![setting]).u32();
    e.mem.u8(value) != 0
}

/// Reads the `float` a `float` setting accessor points at.
fn float_setting(e: &mut Engine, setting: u32) -> f32 {
    let value = e.call(SETTING_FLOAT_VALUE, &args![setting]).u32();
    e.mem.f32(value)
}

// Translated from 0069cfe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies a `BSSimpleArray<CombatSearchDoor, 1024>` (12-byte elements)
/// into `other`: sizes `other` to this array's size, then copies element by
/// element. The engine map names it
/// `BSSimpleArray<CombatSearchDoor_1024>::SetSize` (Xbox PDB), but the body
/// is the copy.
pub fn fn_0069cfe0(e: &mut Engine, this: Ptr<BSSimpleArray>, other: Ptr<BSSimpleArray>) {
    let size = e.get(this, BSSimpleArray::iSize);
    e.call(SEARCH_DOOR_ARRAY_SET_SIZE, &args![other, size, 1u32]);
    let mut index = 0;
    while index < e.get(this, BSSimpleArray::iSize) {
        let from = e.get(this, BSSimpleArray::pBuffer) + index * 0xc;
        let to = e.call(SEARCH_DOOR_AT, &args![other, index]).u32();
        for word in 0..3 {
            let value = e.mem.u32(from + 4 * word);
            e.mem.set_u32(to + 4 * word, value);
        }
        index += 1;
    }
}

// Translated from 00985410 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatTarget` constructor (the map has no name; identified by the
/// layout): the actor, detection level 0, four default locations, the
/// search-noticed count, attacker count 0, the time stamps (-FLT_MAX for
/// last seen / last detected / last attacked member, the current game time
/// for last noticed, 0 for the other two), both line-of-sight counts 0, and
/// `LastNoticedLocation` set to the actor's current world location.
pub fn fn_00985410(
    e: &mut Engine,
    this: Ptr<CombatTarget>,
    actor: Ptr,
    search_noticed: u16,
) -> Ptr<CombatTarget> {
    e.set(this, CombatTarget::pActor, actor);
    e.set(this, CombatTarget::iDetectionLevel, 0);
    for location in [
        CombatTarget::LAST_NOTICED_LOCATION,
        CombatTarget::LAST_DETECTED_LOCATION,
        CombatTarget::LAST_SEEN_LOCATION,
        CombatTarget::LAST_ATTACKED_MEMBER_LOCATION,
    ] {
        e.call(WORLD_LOCATION_CONSTRUCT, &args![this.byte_add(location)]);
    }
    e.set(this, CombatTarget::sLastSearchNoticed, search_noticed);
    e.set(this, CombatTarget::sAttackerCount, 0);
    let lowest = -e.global::<f32>(MAX_FLOAT);
    e.call(
        TIME_STAMP_SET,
        &args![this.byte_add(CombatTarget::fLastSeenTimeStamp.off), lowest],
    );
    e.call(
        TIME_STAMP_SET,
        &args![
            this.byte_add(CombatTarget::fLastDetectedTimeStamp.off),
            lowest
        ],
    );
    e.call(
        TIME_STAMP_ZERO,
        &args![this.byte_add(CombatTarget::fDetectionLevelUpdateTimeStamp.off)],
    );
    let now = e.call(GAME_TIME_NOW, &args![]).f32();
    e.call(
        TIME_STAMP_SET,
        &args![this.byte_add(CombatTarget::fLastNoticedTimeStamp.off), now],
    );
    e.call(
        TIME_STAMP_ZERO,
        &args![this.byte_add(CombatTarget::fDetectionEventTimeStamp.off)],
    );
    e.call(
        TIME_STAMP_SET,
        &args![
            this.byte_add(CombatTarget::fLastAttackedMemberTimeStamp.off),
            lowest
        ],
    );
    e.set(this, CombatTarget::cMemberLOSCount, 0);
    e.set(this, CombatTarget::cMember360LOSCount, 0);
    e.with_stack(0x10, |e, scratch| {
        let world = e
            .call(GET_WORLD_LOCATION, &args![actor, scratch])
            .ptr::<()>();
        copy_location(e, this.byte_add(CombatTarget::LAST_NOTICED_LOCATION), world);
    });
    this
}

// Translated from 00985520 (decompiled, FalloutNV.exe 1.4.0.525)
/// Merges `other` into this `CombatTarget`: the higher detection level; the
/// newer of each pair of time stamps (last detected with its location, last
/// seen with its location, detection level update, last noticed, last
/// attacked member with its location); and the sums of the attacker count
/// and of the two line-of-sight counts.
pub fn fn_00985520(e: &mut Engine, this: Ptr<CombatTarget>, other: Ptr<CombatTarget>) {
    if e.get(this, CombatTarget::iDetectionLevel) < e.get(other, CombatTarget::iDetectionLevel) {
        let level = e.get(other, CombatTarget::iDetectionLevel);
        e.set(this, CombatTarget::iDetectionLevel, level);
    }
    // (time stamp offset, location taken along with it)
    let pairs = [
        (
            CombatTarget::fLastDetectedTimeStamp.off,
            Some(CombatTarget::LAST_DETECTED_LOCATION),
        ),
        (
            CombatTarget::fLastSeenTimeStamp.off,
            Some(CombatTarget::LAST_SEEN_LOCATION),
        ),
        (CombatTarget::fDetectionLevelUpdateTimeStamp.off, None),
        (CombatTarget::fLastNoticedTimeStamp.off, None),
        (
            CombatTarget::fLastAttackedMemberTimeStamp.off,
            Some(CombatTarget::LAST_ATTACKED_MEMBER_LOCATION),
        ),
    ];
    for (stamp, location) in pairs {
        let mine = time_stamp(e, this.byte_add(stamp));
        let theirs = time_stamp(e, other.byte_add(stamp));
        if theirs > mine {
            let value = e.mem.u32(other.addr() + stamp);
            e.mem.set_u32(this.addr() + stamp, value);
            if let Some(location) = location {
                copy_location(e, this.byte_add(location), other.byte_add(location));
            }
        }
    }
    let attackers = e
        .get(this, CombatTarget::sAttackerCount)
        .wrapping_add(e.get(other, CombatTarget::sAttackerCount));
    e.set(this, CombatTarget::sAttackerCount, attackers);
    let los = e
        .get(this, CombatTarget::cMemberLOSCount)
        .wrapping_add(e.get(other, CombatTarget::cMemberLOSCount));
    e.set(this, CombatTarget::cMemberLOSCount, los);
    let los_360 = e
        .get(this, CombatTarget::cMember360LOSCount)
        .wrapping_add(e.get(other, CombatTarget::cMember360LOSCount));
    e.set(this, CombatTarget::cMember360LOSCount, los_360);
}

// Translated from 009856e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatTarget::SaveGame` (Xbox PDB): writes the actor as a form ID, the
/// detection level (4 bytes), the search-noticed and attacker counts (2
/// bytes each), the two line-of-sight counts, four locations and six time
/// stamps. The second stack word is never read.
pub fn combat_target_save_game(
    e: &mut Engine,
    this: Ptr<CombatTarget>,
    buffer: Ptr,
    _unused_2: u32,
) {
    let actor = e.get(this, CombatTarget::pActor);
    e.call(SAVE_FORM_ID, &args![buffer, actor, 0u32]);
    for (offset, size) in [(0x04u32, 4u32), (0x48, 2), (0x4a, 2), (0x64, 1), (0x65, 1)] {
        e.call(
            SAVE_BYTES,
            &args![buffer, this.byte_add(offset), size, 0u32],
        );
    }
    for location in [0x08u32, 0x18, 0x28, 0x38] {
        e.call(WORLD_LOCATION_SAVE, &args![this.byte_add(location), buffer]);
    }
    for stamp in [0x50u32, 0x54, 0x58, 0x4c, 0x5c, 0x60] {
        e.call(TIME_STAMP_SAVE, &args![this.byte_add(stamp), buffer]);
    }
}

// Translated from 00985800 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatTarget::LoadGame` (Xbox PDB): reads what `SaveGame` wrote. The
/// fourth location (`LastAttackedMemberLocation`) is only in saves of
/// version 14 or later, the last time stamp (`LastAttackedMemberTimeStamp`)
/// in version 13 or later (the version is the result of the buffer's
/// virtual method at slot 0). The second stack word is never read.
pub fn combat_target_load_game(
    e: &mut Engine,
    this: Ptr<CombatTarget>,
    buffer: Ptr,
    _unused_2: u32,
) {
    e.call(LOAD_FORM_ID, &args![buffer, this]);
    for (offset, size) in [(0x04u32, 4u32), (0x48, 2), (0x4a, 2), (0x64, 1), (0x65, 1)] {
        e.call(LOAD_BYTES, &args![buffer, this.byte_add(offset), size]);
    }
    for location in [0x08u32, 0x18, 0x28] {
        e.call(WORLD_LOCATION_LOAD, &args![this.byte_add(location), buffer]);
    }
    if e.vcall(buffer.addr(), 0, &args![]).u8() >= 0x0e {
        e.call(WORLD_LOCATION_LOAD, &args![this.byte_add(0x38), buffer]);
    }
    for stamp in [0x50u32, 0x54, 0x58, 0x4c, 0x5c] {
        e.call(TIME_STAMP_LOAD, &args![this.byte_add(stamp), buffer]);
    }
    if e.vcall(buffer.addr(), 0, &args![]).u8() >= 0x0d {
        e.call(TIME_STAMP_LOAD, &args![this.byte_add(0x60), buffer]);
    }
}

// Translated from 00985930 (decompiled, FalloutNV.exe 1.4.0.525)
/// Replaces the value in the word at `this` by an `Actor*`: zero stays
/// zero; otherwise the word is looked up with `004839c0` and the result is
/// down-cast with `__RTDynamicCast` (`TESForm` type descriptor `01183028`
/// to the one at `011846d4`, no reference). Both stack words are never read
/// (the shape of a "resolve references" callback).
pub fn fn_00985930(e: &mut Engine, this: Ptr, _unused_1: u32, _unused_2: u32) {
    let value = e.mem.u32(this.addr());
    let resolved = if value == 0 {
        0
    } else {
        let form = e.call(0x0048_39c0, &args![value]).u32();
        e.call(
            0x00ec_43fb,
            &args![form, 0u32, 0x0118_3028u32, 0x0118_46d4u32, 0u32],
        )
        .u32()
    };
    e.mem.set_u32(this.addr(), resolved);
}

// Translated from 00985980 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the damage per second, combat strength and cluster of another
/// `CombatMember` into this one (not the actor or the strategy assignment).
pub fn fn_00985980(e: &mut Engine, this: Ptr<CombatMember>, other: Ptr<CombatMember>) {
    let damage = e.get(other, CombatMember::fDamagePerSecond);
    e.set(this, CombatMember::fDamagePerSecond, damage);
    let strength = e.get(other, CombatMember::fCombatStrength);
    e.set(this, CombatMember::fCombatStrength, strength);
    let cluster = e.get(other, CombatMember::pCluster);
    e.set(this, CombatMember::pCluster, cluster);
}

// Translated from 009859c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatMember::SaveGame` (Xbox PDB): the actor as a form ID, the three
/// 4-byte fields after it, and the index of the member's cluster in the
/// group's cluster array (`0xff` when it has none).
pub fn combat_member_save_game(
    e: &mut Engine,
    this: Ptr<CombatMember>,
    buffer: Ptr,
    group: Ptr<CombatGroup>,
) {
    let actor = e.get(this, CombatMember::pActor);
    e.call(SAVE_FORM_ID, &args![buffer, actor, 0u32]);
    for offset in [0x04u32, 0x08, 0x0c] {
        e.call(
            SAVE_BYTES,
            &args![buffer, this.byte_add(offset), 4u32, 0u32],
        );
    }
    let mut cluster_index = 0xffu8;
    let cluster = e.get(this, CombatMember::pCluster);
    if !cluster.is_null() {
        cluster_index = e.call(0x0098_de30, &args![group, cluster]).u8();
    }
    e.with_stack(4, |e, byte| {
        e.mem.set_u8(byte.addr(), cluster_index);
        e.call(SAVE_BYTES, &args![buffer, byte, 1u32, 0u32]);
    });
}

// Translated from 00985a50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatMember::LoadGame` (Xbox PDB): reads what `SaveGame` wrote and
/// turns the cluster index back into the group's cluster (`0098de60`; null
/// for `0xff`).
pub fn combat_member_load_game(
    e: &mut Engine,
    this: Ptr<CombatMember>,
    buffer: Ptr,
    group: Ptr<CombatGroup>,
) {
    e.call(LOAD_FORM_ID, &args![buffer, this]);
    for offset in [0x04u32, 0x08, 0x0c] {
        e.call(LOAD_BYTES, &args![buffer, this.byte_add(offset), 4u32]);
    }
    let cluster_index = e.with_stack(4, |e, byte| {
        e.mem.set_u8(byte.addr(), 0xff);
        e.call(LOAD_BYTES, &args![buffer, byte, 1u32]);
        e.mem.i8(byte.addr())
    });
    let cluster = if cluster_index == -1 {
        Ptr::NULL
    } else {
        e.call(0x0098_de60, &args![group, cluster_index as u8])
            .ptr::<()>()
    };
    e.set(this, CombatMember::pCluster, cluster);
}

// Translated from 00985ae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatGroupCluster` save: its `Center` location as 0x10 raw bytes, then
/// the member count (4 bytes). The second stack word is never read.
pub fn fn_00985ae0(e: &mut Engine, this: Ptr, buffer: Ptr, _unused_2: u32) {
    e.call(SAVE_BYTES, &args![buffer, this, 0x10u32, 0u32]);
    e.call(SAVE_BYTES, &args![buffer, this.byte_add(0x10), 4u32, 0u32]);
}

// Translated from 00985b10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatGroupCluster` load: reads what `fn_00985ae0` wrote. The second
/// stack word is never read.
pub fn fn_00985b10(e: &mut Engine, this: Ptr, buffer: Ptr, _unused_2: u32) {
    e.call(LOAD_BYTES, &args![buffer, this, 0x10u32]);
    e.call(LOAD_BYTES, &args![buffer, this.byte_add(0x10), 4u32]);
}

// Translated from 00985b40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatSearchLocation` score seen from `location`: the time left of the
/// entry (`fn_00985bb0`) minus the squared distance to `location` (capped
/// by `0040ebd0` at the `float` at `0108d5a0`) divided by the `double` at
/// `0108d598`, plus the `double` at `01020758` (10) when the entry has a
/// target ID.
pub fn fn_00985b40(e: &mut Engine, this: Ptr<CombatSearchLocation>, location: Ptr) -> f32 {
    let remaining = fn_00985bb0(e, this);
    let distance = e.call(DISTANCE_SQUARED, &args![location, this]).f32();
    let cap: f32 = e.global(0x0108_d5a0);
    let distance = e.call(0x0040_ebd0, &args![distance, cap]).f32();
    let divisor: f64 = e.global(0x0108_d598);
    let mut score = (remaining as f64 - distance as f64 / divisor) as f32;
    if e.get(this, CombatSearchLocation::iTargetID) != 0 {
        let bonus: f64 = e.global(0x0102_0758);
        score = (score as f64 + bonus) as f32;
    }
    score
}

// Translated from 00985bb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatSearchLocation`: the time left of the entry, `fScore` minus the
/// age of its time stamp (`fScore` holds a duration here).
pub fn fn_00985bb0(e: &mut Engine, this: Ptr<CombatSearchLocation>) -> f32 {
    let age = e
        .call(
            TIME_STAMP_AGE,
            &args![this.byte_add(CombatSearchLocation::fTimeStamp.off)],
        )
        .f32();
    (e.get(this, CombatSearchLocation::fScore) as f64 - age as f64) as f32
}

// Translated from 00985be0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatSearchLocation` debug colour, written to `result` (an `NiColorA`)
/// and returned, by the time left (`fn_00985bb0`): (1, 0, 1) above the
/// `double` at `0101e2c0` (50), (0, 0, 1) above the one at `0101db88` (30),
/// otherwise (1, 1, 0); the alpha is the `float` at `01016248`.
pub fn fn_00985be0(e: &mut Engine, this: Ptr<CombatSearchLocation>, result: Ptr) -> Ptr {
    let alpha: f32 = e.global(0x0101_6248);
    e.with_stack(0x20, |e, scratch| {
        // The game first builds an all-zero colour that is overwritten.
        e.call(0x0041_4430, &args![scratch, 0.0f32, 0.0f32, 0.0f32, 0.0f32]);
        let remaining = fn_00985bb0(e, this) as f64;
        let upper: f64 = e.global(0x0101_e2c0);
        let lower: f64 = e.global(0x0101_db88);
        let chosen = scratch.byte_add(0x10);
        let (red, green, blue) = if remaining > upper {
            (1.0f32, 0.0f32, 1.0f32)
        } else if remaining > lower {
            (0.0, 0.0, 1.0)
        } else {
            (1.0, 1.0, 0.0)
        };
        e.call(0x0041_4430, &args![chosen, red, green, blue, alpha]);
        for word in 0..4 {
            let value = e.mem.u32(chosen.addr() + 4 * word);
            e.mem.set_u32(result.addr() + 4 * word, value);
        }
    });
    result
}

// Translated from 00985d10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatGroup` constructor (the map has no name): group number 0, group
/// ID `group_id`, empty target and member arrays, no strategy, all timers
/// zeroed, strength figures -1, no search state, no clusters, music state
/// -1 (undecided). The compiler's unwinding frame is not translated.
pub fn fn_00985d10(e: &mut Engine, this: Ptr<CombatGroup>, group_id: u32) -> Ptr<CombatGroup> {
    e.set(this, CombatGroup::iGroupNum, 0);
    e.set(this, CombatGroup::iGroupID, group_id);
    e.call(
        0x0098_ed30,
        &args![this.byte_add(CombatGroup::TARGET_ARRAY)],
    );
    e.call(
        0x0098_eff0,
        &args![this.byte_add(CombatGroup::MEMBER_ARRAY)],
    );
    e.set(this, CombatGroup::pGroupStrategy, Ptr::NULL);
    e.set(this, CombatGroup::bStrategyForced, false);
    for timer in [
        CombatGroup::CHOOSE_STRATEGY_TIMER,
        CombatGroup::UPDATE_STRATEGY_TIMER,
    ] {
        e.call(TIMER_CONSTRUCT, &args![this.byte_add(timer)]);
    }
    e.set(this, CombatGroup::iLastGroupStrategyChosenIndex, 0);
    e.call(
        TIME_STAMP_ZERO,
        &args![this.byte_add(CombatGroup::fLastGroupStrategyChosenTimeStamp.off)],
    );
    for timer in [
        CombatGroup::AVOID_THREAT_DIALOGUE_TIMER,
        CombatGroup::COMBAT_STRENGTH_TIMER,
    ] {
        e.call(TIMER_CONSTRUCT, &args![this.byte_add(timer)]);
    }
    // `_vector_constructor_iterator_(base, size, count, constructor)`.
    e.call(
        0x0040_1050,
        &args![
            this.byte_add(CombatGroup::DETECTION_DIALOGUE_TIMERS),
            8u32,
            10u32,
            TIMER_CONSTRUCT
        ],
    );
    for timer in [
        CombatGroup::TARGET_UPDATE_TIMER,
        CombatGroup::CLUSTER_UPDATE_TIMER,
        CombatGroup::COMBAT_MUSIC_UPDATE_TIMER,
    ] {
        e.call(TIMER_CONSTRUCT, &args![this.byte_add(timer)]);
    }
    let unset: f32 = e.global(MINUS_ONE);
    e.set(this, CombatGroup::fMemberCombatStrength, unset);
    e.set(this, CombatGroup::fAverageMemberDamagePerSecond, unset);
    e.set(this, CombatGroup::fTargetCombatStrength, unset);
    e.set(this, CombatGroup::fAverageTargetCombatStrength, unset);
    e.set(this, CombatGroup::iSearchCount, 0);
    e.call(
        0x0063_3c90,
        &args![this.byte_add(CombatGroup::spPathingLOSGridMap.off), 0u32],
    );
    for timer in [
        CombatGroup::SEARCH_UPDATE_TIMER,
        CombatGroup::SEARCH_AREA_UPDATE_TIMER,
    ] {
        e.call(TIMER_CONSTRUCT, &args![this.byte_add(timer)]);
    }
    let lowest = -e.global::<f32>(MAX_FLOAT);
    e.call(
        TIME_STAMP_SET,
        &args![
            this.byte_add(CombatGroup::fSearchStartedTimeStamp.off),
            lowest
        ],
    );
    e.set(this, CombatGroup::pSearchingMember, Ptr::NULL);
    e.call(
        WORLD_LOCATION_CONSTRUCT,
        &args![this.byte_add(CombatGroup::SEARCH_CENTER)],
    );
    for word in 0..3 {
        let value = e.global::<u32>(0x011f_426c + 4 * word);
        e.mem.set_u32(
            this.addr() + CombatGroup::SEARCH_FOCAL_POINT + 4 * word,
            value,
        );
    }
    e.set(this, CombatGroup::fSearchRadius, 0.0);
    e.call(
        0x0098_f2a0,
        &args![this.byte_add(CombatGroup::SEARCH_LOCATIONS)],
    );
    e.call(
        0x0098_f4c0,
        &args![this.byte_add(CombatGroup::SEARCH_TELEPORT_DOORS)],
    );
    e.call(
        0x0063_3c90,
        &args![this.byte_add(CombatGroup::spSearchDebugGeometry.off), 0u32],
    );
    e.set(this, CombatGroup::bUpdateSearchDebugGeometry, false);
    e.call(
        0x0098_f510,
        &args![this.byte_add(CombatGroup::CLUSTER_ARRAY)],
    );
    e.set(this, CombatGroup::iInitializedMemberCount, 0);
    e.set(this, CombatGroup::iFleeingMemberCount, 0);
    e.set(this, CombatGroup::iNonFleeingMemberCount, 0);
    e.set(this, CombatGroup::cCombatMusicState, -1);
    e.call(
        0x0063_3c90,
        &args![this.byte_add(CombatGroup::spDebugGeometry.off), 0u32],
    );
    this
}

// Translated from 00985f90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatGroup` destructor (the map has no name): takes every target and
/// every member out of the combat manager, destroys the clusters (`00401030`
/// per cluster), calls `009878b0(this, 1)` (a clean-up the map misnames
/// `PathingDebugGeometry::MakeSmoothingData`), then destroys the members in
/// reverse order: the debug geometry pointer, the cluster array, the search
/// debug geometry pointer, the teleport door array, the search location
/// array, the line-of-sight grid pointer, the member array and the target
/// array. Exception unwinding states are not translated.
pub fn fn_00985f90(e: &mut Engine, this: Ptr<CombatGroup>) {
    let targets = e.call(TARGET_COUNT, &args![this]).u32();
    for index in 0..targets {
        let actor = e.call(TARGET_ACTOR_AT, &args![this, index]).u32();
        let manager = combat_manager(e);
        e.call(MANAGER_REMOVE_TARGET, &args![manager, this, actor]);
    }
    let members = e.call(MEMBER_COUNT, &args![this]).u32();
    for index in 0..members {
        let actor = e.call(MEMBER_ACTOR_AT, &args![this, index]).u32();
        let manager = combat_manager(e);
        e.call(MANAGER_REMOVE_MEMBER, &args![manager, this, actor]);
    }
    let clusters = this.byte_add(CombatGroup::CLUSTER_ARRAY);
    let mut index = 0;
    while index < e.call(ARRAY_SIZE, &args![clusters]).u32() {
        let slot = e.call(CLUSTER_AT, &args![clusters, index]).u32();
        let cluster = e.mem.u32(slot);
        e.call(0x0040_1030, &args![cluster]);
        index += 1;
    }
    e.call(0x0098_78b0, &args![this, 1u32]);
    e.call(
        0x0045_cec0,
        &args![this.byte_add(CombatGroup::spDebugGeometry.off)],
    );
    e.call(0x0098_f540, &args![clusters]);
    e.call(
        0x0045_cec0,
        &args![this.byte_add(CombatGroup::spSearchDebugGeometry.off)],
    );
    e.call(
        0x0098_f4f0,
        &args![this.byte_add(CombatGroup::SEARCH_TELEPORT_DOORS)],
    );
    e.call(
        0x0098_f2d0,
        &args![this.byte_add(CombatGroup::SEARCH_LOCATIONS)],
    );
    e.call(
        0x0045_cec0,
        &args![this.byte_add(CombatGroup::spPathingLOSGridMap.off)],
    );
    e.call(
        0x0098_f020,
        &args![this.byte_add(CombatGroup::MEMBER_ARRAY)],
    );
    e.call(
        0x0098_ed60,
        &args![this.byte_add(CombatGroup::TARGET_ARRAY)],
    );
}

// Translated from 00986150 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatGroup::Update` (Xbox PDB): refreshes the counts (`UpdateCounts`);
/// when the target update timer (`+0xa8`) has expired, runs `0098cf70` and
/// `0098d030` and restarts it (delay: the `float` at `010162c0`); runs
/// `0098d1a0` and `0098a580`; when the combat strength timer (`+0x50`) has
/// expired, runs `UpdateCombatStrength` and restarts it with the setting at
/// `011cfe84`; with more than one member runs `0098d2c0`; and decides the
/// combat music state (`+0x154`, -1 until decided): while the music timer
/// (`+0xb8`) is not running and `0097ef30` says no search was started, it is
/// armed with the setting at `011ceb44`; once it expires the state becomes 1
/// when member strength over target strength lies strictly between the
/// settings at `011cee6c` and `011cf944` (and the target strength is
/// positive), else 0.
pub fn combat_group_update(e: &mut Engine, this: Ptr<CombatGroup>) {
    e.call(0x0098_ce80, &args![this]);
    let target_timer = this.byte_add(CombatGroup::TARGET_UPDATE_TIMER);
    if e.call(TIMER_EXPIRED, &args![target_timer]).bool() {
        e.call(0x0098_cf70, &args![this]);
        e.call(0x0098_d030, &args![this]);
        let delay: f32 = e.global(0x0101_62c0);
        e.call(TIMER_START, &args![target_timer, delay]);
    }
    e.call(0x0098_d1a0, &args![this]);
    e.call(0x0098_a580, &args![this]);
    let strength_timer = this.byte_add(CombatGroup::COMBAT_STRENGTH_TIMER);
    if e.call(TIMER_EXPIRED, &args![strength_timer]).bool() {
        e.call(0x0098_c870, &args![this]);
        let delay = float_setting(e, 0x011c_fe84);
        e.call(TIMER_START, &args![strength_timer, delay]);
    }
    if e.call(MEMBER_COUNT, &args![this]).u32() > 1 {
        e.call(0x0098_d2c0, &args![this]);
    }
    if e.get(this, CombatGroup::cCombatMusicState) != -1 {
        return;
    }
    let music_timer = this.byte_add(CombatGroup::COMBAT_MUSIC_UPDATE_TIMER);
    if !e.call(TIMER_RUNNING, &args![music_timer]).bool() {
        if !e.call(0x0097_ef30, &args![this]).bool() {
            let delay = float_setting(e, 0x011c_eb44);
            e.call(TIMER_START, &args![music_timer, delay]);
        }
    } else if e.call(TIMER_EXPIRED, &args![music_timer]).bool() {
        e.set(this, CombatGroup::cCombatMusicState, 0);
        let target_strength = e.get(this, CombatGroup::fTargetCombatStrength);
        if 0.0 < target_strength as f64 {
            let ratio = e.get(this, CombatGroup::fMemberCombatStrength) / target_strength;
            let low = float_setting(e, 0x011c_ee6c);
            if low < ratio {
                let high = float_setting(e, 0x011c_f944);
                if ratio < high {
                    e.set(this, CombatGroup::cCombatMusicState, 1);
                }
            }
        }
    }
}

// Translated from 009862e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the group's strategy (the map has no name). A null strategy only
/// clears `bStrategyForced`. Otherwise it builds a temporary
/// (`00990320(temp, group)`), runs `0098a3c0(group)`, calls the strategy's
/// virtual method at slot 4 with the group and the temporary, stores the
/// strategy's `00726070` result in `iLastGroupStrategyChosenIndex` and the
/// game time in `fLastGroupStrategyChosenTimeStamp`, sets the flag, and
/// destroys the temporary (`fn_009863b0`). Exception unwinding is not
/// translated.
pub fn fn_009862e0(e: &mut Engine, this: Ptr<CombatGroup>, strategy: Ptr) {
    e.set(this, CombatGroup::pGroupStrategy, strategy);
    if e.get(this, CombatGroup::pGroupStrategy).is_null() {
        e.set(this, CombatGroup::bStrategyForced, false);
        return;
    }
    e.with_stack(0x40, |e, temporary| {
        e.call(0x0099_0320, &args![temporary, this]);
        e.call(0x0098_a3c0, &args![this]);
        let strategy = e.get(this, CombatGroup::pGroupStrategy);
        e.vcall(strategy.addr(), 4, &args![this, temporary]);
        let index = e.call(0x0072_6070, &args![strategy]).u32();
        e.set(this, CombatGroup::iLastGroupStrategyChosenIndex, index);
        let now = e.call(GAME_TIME_NOW, &args![]).f32();
        let stored = e.with_stack(4, |e, local| {
            e.call(TIME_STAMP_SET, &args![local, now]);
            e.mem.f32(local.addr())
        });
        e.set(this, CombatGroup::fLastGroupStrategyChosenTimeStamp, stored);
        e.set(this, CombatGroup::bStrategyForced, true);
        fn_009863b0(e, temporary);
    });
}

// Translated from 009863b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of the temporary `fn_009862e0` builds (the decompiler's
/// `~_ATL_WIN_MODULE70` label is wrong): destroys the two arrays at +0x28
/// and +0x18 with `0098f560`. Exception unwinding is not translated.
pub fn fn_009863b0(e: &mut Engine, this: Ptr) {
    e.call(0x0098_f560, &args![this.byte_add(0x28)]);
    e.call(0x0098_f560, &args![this.byte_add(0x18)]);
}

// Translated from 00986410 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatGroup::AddTarget` (Xbox PDB): false for null; true when the actor
/// already is a target; false when it cannot be added (`CanAddTarget`);
/// false (with a log message) when it is a member; otherwise appends a new
/// `CombatTarget` (built with the group's search count), tells the combat
/// manager and returns true.
pub fn combat_group_add_target(e: &mut Engine, this: Ptr<CombatGroup>, actor: Ptr) -> bool {
    if actor.is_null() {
        return false;
    }
    if combat_group_is_target(e, this, actor) {
        return true;
    }
    if !combat_group_can_add_target(e, this, actor) {
        return false;
    }
    if combat_group_is_member(e, this, actor) {
        log_actor_message(e, actor, 0x0108_d5a8);
        return false;
    }
    let search_count = e.get(this, CombatGroup::iSearchCount);
    e.with_stack(CombatTarget::SIZE, |e, target| {
        fn_00985410(e, target.cast(), actor, search_count as u16);
        e.call(
            TARGET_ARRAY_ADD,
            &args![this.byte_add(CombatGroup::TARGET_ARRAY), target],
        );
    });
    let manager = combat_manager(e);
    e.call(MANAGER_ADD_TARGET, &args![manager, this, actor]);
    true
}

/// The "COMBAT: Trying to add actor %08X %s to Combat Group ..." log line
/// (`format` is the string's address): the actor's form ID and its name
/// (the virtual method at slot 0x130 when `00474cb0` says the actor has
/// one, else `0055d520`).
fn log_actor_message(e: &mut Engine, actor: Ptr, format: u32) {
    let name = if e.call(0x0047_4cb0, &args![actor]).u32() != 0 {
        e.vcall(actor.addr(), 0x130, &args![]).u32()
    } else {
        e.call(0x0055_d520, &args![actor]).u32()
    };
    let form_id = e.call(FORM_ID, &args![actor]).u32();
    e.call(LOG_MESSAGE, &args![format, form_id, name]);
}

// Translated from 00986500 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatGroup::RemoveTarget` (Xbox PDB): removes the first target whose
/// actor is `actor` (`fn_00986560`).
pub fn combat_group_remove_target(e: &mut Engine, this: Ptr<CombatGroup>, actor: Ptr) {
    let targets = this.byte_add(CombatGroup::TARGET_ARRAY);
    let mut index = 0;
    while index < e.call(ARRAY_SIZE, &args![targets]).u32() {
        let target = e
            .call(TARGET_AT, &args![targets, index])
            .ptr::<CombatTarget>();
        if e.get(target, CombatTarget::pActor) == actor {
            fn_00986560(e, this, index);
            return;
        }
        index += 1;
    }
}

// Translated from 00986560 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes target `index`: tells the combat manager the actor is no longer
/// a target of the group, then removes the array entry.
pub fn fn_00986560(e: &mut Engine, this: Ptr<CombatGroup>, index: u32) {
    let targets = this.byte_add(CombatGroup::TARGET_ARRAY);
    let target = e
        .call(TARGET_AT, &args![targets, index])
        .ptr::<CombatTarget>();
    let actor = e.get(target, CombatTarget::pActor);
    let manager = combat_manager(e);
    e.call(MANAGER_REMOVE_TARGET, &args![manager, this, actor]);
    e.call(TARGET_ARRAY_REMOVE_AT, &args![targets, index, 1u32]);
}

// Translated from 009865b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatGroup::IsTarget` (Xbox PDB): whether the actor is one of the
/// group's targets.
pub fn combat_group_is_target(e: &mut Engine, this: Ptr<CombatGroup>, actor: Ptr) -> bool {
    !fn_009865d0(e, this, actor).is_null()
}

// Translated from 009865d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `CombatTarget` of an actor (by form ID, `fn_00986610`), or null.
pub fn fn_009865d0(e: &mut Engine, this: Ptr<CombatGroup>, actor: Ptr) -> Ptr<CombatTarget> {
    let form_id = e.call(FORM_ID, &args![actor]).u32();
    fn_00986610(e, this, form_id)
}

// Translated from 009865f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `CombatMember` of an actor (by form ID, `fn_00986670`), or null.
pub fn fn_009865f0(e: &mut Engine, this: Ptr<CombatGroup>, actor: Ptr) -> Ptr<CombatMember> {
    let form_id = e.call(FORM_ID, &args![actor]).u32();
    fn_00986670(e, this, form_id)
}

// Translated from 00986610 (decompiled, FalloutNV.exe 1.4.0.525)
/// The first `CombatTarget` whose actor has form ID `form_id`, or null.
pub fn fn_00986610(e: &mut Engine, this: Ptr<CombatGroup>, form_id: u32) -> Ptr<CombatTarget> {
    let targets = this.byte_add(CombatGroup::TARGET_ARRAY);
    let mut index = 0;
    while index < e.call(ARRAY_SIZE, &args![targets]).u32() {
        let target = e
            .call(TARGET_AT, &args![targets, index])
            .ptr::<CombatTarget>();
        let actor = e.get(target, CombatTarget::pActor);
        if e.call(FORM_ID, &args![actor]).u32() == form_id {
            return target;
        }
        index += 1;
    }
    Ptr::NULL
}

// Translated from 00986670 (decompiled, FalloutNV.exe 1.4.0.525)
/// The first `CombatMember` whose actor has form ID `form_id`, or null.
pub fn fn_00986670(e: &mut Engine, this: Ptr<CombatGroup>, form_id: u32) -> Ptr<CombatMember> {
    let members = this.byte_add(CombatGroup::MEMBER_ARRAY);
    let mut index = 0;
    while index < e.call(ARRAY_SIZE, &args![members]).u32() {
        let member = e
            .call(MEMBER_AT, &args![members, index])
            .ptr::<CombatMember>();
        let actor = e.get(member, CombatMember::pActor);
        if e.call(FORM_ID, &args![actor]).u32() == form_id {
            return member;
        }
        index += 1;
    }
    Ptr::NULL
}

// Translated from 009866d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatGroup::CanAddTarget` (Xbox PDB): true when every member actor
/// (null ones are skipped) accepts `target` (`008b0670(member, target)`).
pub fn combat_group_can_add_target(e: &mut Engine, this: Ptr<CombatGroup>, target: Ptr) -> bool {
    let members = e.call(MEMBER_COUNT, &args![this]).u32();
    let mut index = 0;
    while index < members {
        let member = e.call(MEMBER_ACTOR_AT, &args![this, index]).u32();
        if member != 0
            && !e
                .call(ACTOR_COMBAT_COMPATIBLE, &args![member, target])
                .bool()
        {
            return false;
        }
        index += 1;
    }
    true
}

// Translated from 00986740 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `0098f580` with `other`'s target array as `this` and this group's
/// target array as the argument (an assignment between the two
/// `BSSimpleArray<CombatTarget>`s; the callee is not in this session's
/// range).
pub fn fn_00986740(e: &mut Engine, this: Ptr<CombatGroup>, other: Ptr<CombatGroup>) {
    e.call(
        0x0098_f580,
        &args![
            other.byte_add(CombatGroup::TARGET_ARRAY),
            this.byte_add(CombatGroup::TARGET_ARRAY)
        ],
    );
}

// Translated from 00986760 (decompiled, FalloutNV.exe 1.4.0.525)
/// Fills the pointer array `other` with the address of each of this group's
/// `CombatTarget`s, in order (sized first with `SetSize(count, 1)`).
pub fn fn_00986760(e: &mut Engine, this: Ptr<CombatGroup>, other: Ptr) {
    let targets = this.byte_add(CombatGroup::TARGET_ARRAY);
    let count = e.call(ARRAY_SIZE, &args![targets]).u32();
    e.call(POINTER_ARRAY_SET_SIZE, &args![other, count, 1u32]);
    let mut index = 0;
    while index < e.call(ARRAY_SIZE, &args![targets]).u32() {
        let target = e.call(TARGET_AT, &args![targets, index]).u32();
        e.call(ARRAY_SET_AT, &args![other, index, target]);
        index += 1;
    }
}

// Translated from 009867d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatGroup::AddMember` (Xbox PDB): false for null; true when the actor
/// already is a member; false when it cannot be added (`CanAddMember`);
/// false (with a log message) when it is a target. Otherwise appends a new
/// `CombatMember`, tells the combat manager, and, when the tint setting
/// (`011f1878`) is on and the actor has a 3D root (virtual method at slot
/// 0x1d0), tints it with the colour of the group number (`005d8550`);
/// finally restarts the cluster update timer (`+0xb0`, `0097f220`) and
/// returns true.
pub fn combat_group_add_member(e: &mut Engine, this: Ptr<CombatGroup>, actor: Ptr) -> bool {
    if actor.is_null() {
        return false;
    }
    if combat_group_is_member(e, this, actor) {
        return true;
    }
    if !combat_group_can_add_member(e, this, actor) {
        return false;
    }
    if combat_group_is_target(e, this, actor) {
        log_actor_message(e, actor, 0x0108_d600);
        return false;
    }
    e.with_stack(CombatMember::SIZE, |e, member| {
        fn_00986950(e, member.cast(), actor);
        e.call(
            MEMBER_ARRAY_ADD,
            &args![this.byte_add(CombatGroup::MEMBER_ARRAY), member],
        );
    });
    let manager = combat_manager(e);
    e.call(MANAGER_ADD_MEMBER, &args![manager, this, actor]);
    if bool_setting(e, TINT_SETTING) && e.vcall(actor.addr(), 0x1d0, &args![]).u32() != 0 {
        let group_number = e.get(this, CombatGroup::iGroupNum);
        let colour = e.call(0x005d_8550, &args![group_number]).u32();
        let (red, green, blue) = (
            e.mem.f32(colour),
            e.mem.f32(colour + 4),
            e.mem.f32(colour + 8),
        );
        e.with_stack(0xc, |e, rgb| {
            e.call(0x0041_6870, &args![rgb, red, green, blue]);
            let node = e.vcall(actor.addr(), 0x1d0, &args![]).u32();
            e.call(TINT_SCENEGRAPH, &args![node, rgb, 0u32]);
        });
    }
    e.call(
        0x0097_f220,
        &args![this.byte_add(CombatGroup::CLUSTER_UPDATE_TIMER)],
    );
    true
}

// Translated from 00986950 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatMember` constructor (the map has no name): the actor, group
/// strategy assignment 0, damage per second and combat strength -1.0, no
/// cluster.
pub fn fn_00986950(e: &mut Engine, this: Ptr<CombatMember>, actor: Ptr) -> Ptr<CombatMember> {
    e.set(this, CombatMember::pActor, actor);
    e.set(this, CombatMember::iGroupStrategyAssignment, 0);
    let unset: f32 = e.global(MINUS_ONE);
    e.set(this, CombatMember::fDamagePerSecond, unset);
    e.set(this, CombatMember::fCombatStrength, unset);
    e.set(this, CombatMember::pCluster, Ptr::NULL);
    this
}

// Translated from 009869a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes `actor` from the group's members (the map has no name): tells the
/// combat manager, removes the entry, clears `pSearchingMember` when it was
/// that actor, and (tint setting on and a 3D root) removes the tint with
/// `UnTintScenegraph`. Does nothing when the actor is not a member.
pub fn fn_009869a0(e: &mut Engine, this: Ptr<CombatGroup>, actor: Ptr) {
    let members = this.byte_add(CombatGroup::MEMBER_ARRAY);
    let mut index = 0;
    while index < e.call(ARRAY_SIZE, &args![members]).u32() {
        let member = e
            .call(MEMBER_AT, &args![members, index])
            .ptr::<CombatMember>();
        if e.get(member, CombatMember::pActor) == actor {
            let manager = combat_manager(e);
            e.call(MANAGER_REMOVE_MEMBER, &args![manager, this, actor]);
            e.call(MEMBER_ARRAY_REMOVE_AT, &args![members, index, 1u32]);
            if e.get(this, CombatGroup::pSearchingMember) == actor {
                e.set(this, CombatGroup::pSearchingMember, Ptr::NULL);
            }
            if bool_setting(e, TINT_SETTING) && e.vcall(actor.addr(), 0x1d0, &args![]).u32() != 0 {
                let node = e.vcall(actor.addr(), 0x1d0, &args![]).u32();
                e.call(UNTINT_SCENEGRAPH, &args![node]);
            }
            return;
        }
        index += 1;
    }
}

// Translated from 00986a80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatGroup::IsMember` (Xbox PDB): whether the actor is one of the
/// group's members.
pub fn combat_group_is_member(e: &mut Engine, this: Ptr<CombatGroup>, actor: Ptr) -> bool {
    !fn_009865f0(e, this, actor).is_null()
}

// Translated from 00986aa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CombatGroup::CanAddMember` (Xbox PDB): true when `member` accepts every
/// target actor of the group (`008b0670(member, target actor)`).
pub fn combat_group_can_add_member(e: &mut Engine, this: Ptr<CombatGroup>, member: Ptr) -> bool {
    let targets = e.call(TARGET_COUNT, &args![this]).u32();
    let mut index = 0;
    while index < targets {
        let target = e.call(TARGET_ACTOR_AT, &args![this, index]).u32();
        if !e
            .call(ACTOR_COMBAT_COMPATIBLE, &args![member, target])
            .bool()
        {
            return false;
        }
        index += 1;
    }
    true
}

// Translated from 00986b00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Fills the pointer array `other` with the address of each of this group's
/// `CombatMember`s, in order (sized first with `SetSize(count, 1)`).
pub fn fn_00986b00(e: &mut Engine, this: Ptr<CombatGroup>, other: Ptr) {
    let members = this.byte_add(CombatGroup::MEMBER_ARRAY);
    let count = e.call(ARRAY_SIZE, &args![members]).u32();
    e.call(POINTER_ARRAY_SET_SIZE, &args![other, count, 1u32]);
    let mut index = 0;
    while index < e.call(ARRAY_SIZE, &args![members]).u32() {
        let member = e.call(MEMBER_AT, &args![members, index]).u32();
        e.call(ARRAY_SET_AT, &args![other, index, member]);
        index += 1;
    }
}

// Translated from 00986b70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls virtual method 0x3fc with argument 0 on every member actor.
pub fn fn_00986b70(e: &mut Engine, this: Ptr<CombatGroup>) {
    let members = e.call(MEMBER_COUNT, &args![this]).u32();
    for index in 0..members {
        let actor = e.call(MEMBER_ACTOR_AT, &args![this, index]).u32();
        e.vcall(actor, 0x3fc, &args![0u32]);
    }
}

// Translated from 00986bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Takes every member actor out of the combat manager's lists
/// (`008d0600(manager; group, actor)`), then calls `008454f0(member array,
/// 0)`.
pub fn fn_00986bd0(e: &mut Engine, this: Ptr<CombatGroup>) {
    let members = e.call(MEMBER_COUNT, &args![this]).u32();
    for index in 0..members {
        let actor = e.call(MEMBER_ACTOR_AT, &args![this, index]).u32();
        let manager = combat_manager(e);
        e.call(MANAGER_REMOVE_MEMBER, &args![manager, this, actor]);
    }
    e.call(
        0x0084_54f0,
        &args![this.byte_add(CombatGroup::MEMBER_ARRAY), 0u32],
    );
}

// Translated from 00986c40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the group has no members (`0076b610` on the member array).
pub fn fn_00986c40(e: &mut Engine, this: Ptr<CombatGroup>) -> bool {
    e.call(
        0x0076_b610,
        &args![this.byte_add(CombatGroup::MEMBER_ARRAY)],
    )
    .bool()
}

// Translated from 00986c60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Chooses the target `actor` (a member of this group) should fight (the map
/// has no name; returns the `Actor*`, or null).
///
/// Null when the group has no targets. An actor without a process
/// (virtual method at slot 0x428) is removed from the group
/// (`fn_009869a0`) and null is returned. Otherwise every target is examined
/// in order: skipped when it lies outside the process' view angle (when
/// that is positive; `009a6d70` with the position from the target's
/// virtual method at slot 0x1f4), when the cell setting (`011f1840`) is on
/// and it is in another cell, or when `008b06d0` says the actor should not
/// attack it (only asked when `00566950` is true for the actor). The
/// detection call `009887b0` gives a level (forced to 1 when negative for
/// the process' current target when `0097ee90` allows), two flags and a
/// time stamp. With a level above zero the score is 0, or, with more than
/// one target, the sum of the settings `011a4d78` .. `011a4d50` (second
/// flag, first flag, being the current target and a recency bonus, same
/// weapon style, distance capped and scaled, fewer attackers than the
/// current-target flag, minus penalties for unreachable locations,
/// locations the actor cannot exist in, a target that is unfit and one that
/// is fleeing); the best score wins. With a level of zero or less the
/// target is a fallback ranked by its last noticed time stamp (30 seconds
/// in the past, at least the previous best); the result is the best scored
/// target, else the best fallback, else the last target examined.
pub fn fn_00986c60(e: &mut Engine, this: Ptr<CombatGroup>, actor: Ptr) -> Ptr {
    if e.call(
        0x0076_b610,
        &args![this.byte_add(CombatGroup::TARGET_ARRAY)],
    )
    .bool()
    {
        return Ptr::NULL;
    }
    // The game's locals, in one block: +0x00 the fallback time stamp,
    // +0x04 the stamp from the detection call, +0x08 its level, +0x0c / +0x0d
    // its two flags, +0x10 the actor's location, +0x20 the target's
    // location, +0x30 a stamp temporary, +0x34 the age buffer, +0x38 the
    // `008b06d0` output.
    e.with_stack(0x40, |e, frame| choose_target(e, this, actor, frame))
}

fn choose_target(e: &mut Engine, this: Ptr<CombatGroup>, actor: Ptr, frame: Ptr) -> Ptr {
    let fallback_stamp = frame;
    let detected_stamp = frame.byte_add(0x04);
    let detected_level = frame.byte_add(0x08);
    let first_flag = frame.byte_add(0x0c);
    let second_flag = frame.byte_add(0x0d);
    let actor_location = frame.byte_add(0x10);
    let target_location = frame.byte_add(0x20);
    let stamp_temporary = frame.byte_add(0x30);
    let age_buffer = frame.byte_add(0x34);
    let attack_output = frame.byte_add(0x38);

    let mut best_score = -e.global::<f32>(MAX_FLOAT);
    let lowest = -e.global::<f32>(MAX_FLOAT);
    e.call(TIME_STAMP_SET, &args![fallback_stamp, lowest]);
    let mut best_actor = Ptr::NULL;
    let mut best_by_stamp = Ptr::NULL;
    let mut last_examined = Ptr::NULL;
    e.call(GET_WORLD_LOCATION, &args![actor, actor_location]);
    let process = e.vcall(actor.addr(), 0x428, &args![]).u32();
    if process == 0 {
        fn_009869a0(e, this, actor);
        return Ptr::NULL;
    }
    let current_target = e.call(0x0040_30b0, &args![process]).u32();
    let process_part = e.call(0x0058_6150, &args![process]).u32();
    let view = e.call(0x0098_0070, &args![process]).u32();
    let view_angle = e.call(0x0063_9aa0, &args![view]).f32();
    let cell = e.call(0x0056_7d90, &args![actor]).u32();
    let uses_melee = e.call(0x009a_9630, &args![actor]).u8();
    let count = e.call(TARGET_COUNT, &args![this]).u32();

    let mut index = 0;
    while index < count {
        let entry = e
            .call(0x0098_71c0, &args![this, index])
            .ptr::<CombatTarget>();
        index += 1;
        let target = e.get(entry, CombatTarget::pActor);

        if view_angle as f64 > e.global::<f64>(ZERO_DOUBLE) {
            let radians = (view_angle as f64 * e.global::<f64>(0x0102_3128)) as f32;
            let full_circle: f32 = e.global(0x0101_ff50);
            let position = e.vcall(target.addr(), 0x1f4, &args![]).u32();
            if !e
                .call(0x009a_6d70, &args![actor, position, radians, full_circle])
                .bool()
            {
                continue;
            }
        }
        if bool_setting(e, SAME_CELL_SETTING) {
            let target_cell = e.call(0x0056_7d90, &args![target]).u32();
            if cell != target_cell {
                continue;
            }
        }
        if e.call(0x0056_6950, &args![actor]).bool() {
            e.mem.set_u32(attack_output.addr(), 0);
            let wanted = e
                .call(
                    ACTOR_GET_SHOULD_ATTACK,
                    &args![actor, target, 0u32, attack_output, 0u32],
                )
                .bool();
            if !wanted {
                continue;
            }
        }

        e.mem.set_u8(second_flag.addr(), 0);
        e.mem.set_i32(detected_level.addr(), 0);
        e.mem.set_u8(first_flag.addr(), 0);
        e.call(TIME_STAMP_ZERO, &args![detected_stamp]);
        e.call(
            0x0098_87b0,
            &args![
                this,
                actor,
                target,
                detected_level,
                detected_stamp,
                first_flag,
                second_flag
            ],
        );
        if e.mem.i32(detected_level.addr()) < 0
            && target.addr() == current_target
            && e.call(0x0097_ee90, &args![process_part]).bool()
        {
            e.mem.set_i32(detected_level.addr(), 1);
        }

        if e.mem.i32(detected_level.addr()) > 0 {
            let mut score = 0.0f32;
            if count > 1 {
                let is_current = target.addr() == current_target;
                if e.mem.u8(second_flag.addr()) != 0 {
                    score += e.global::<f32>(0x011a_4d78);
                }
                if e.mem.u8(first_flag.addr()) != 0 {
                    score += e.global::<f32>(0x011a_4d74);
                }
                if is_current {
                    score += e.global::<f32>(0x011a_4d70);
                    if e.mem.u8(second_flag.addr()) == 0 {
                        let part = e.call(0x0058_6150, &args![process]).u32();
                        let stamp = e.call(0x0097_fb00, &args![part, age_buffer]).u32();
                        let age = e.call(TIME_STAMP_AGE, &args![stamp]).f32();
                        let window: f32 = e.global(0x011a_4d6c);
                        if age < window {
                            score += e.global::<f32>(0x011a_4d78);
                        }
                    }
                }
                let target_uses_melee = e.call(0x009a_9630, &args![target]).u8();
                if uses_melee == target_uses_melee {
                    score += e.global::<f32>(0x011a_4d68);
                }
                e.call(GET_WORLD_LOCATION, &args![target, target_location]);
                let target_space = e.call(FORM_ID, &args![target_location]).u32();
                let actor_space = e.call(FORM_ID, &args![actor_location]).u32();
                let other_space = target_space != actor_space;
                let distance = if other_space {
                    if is_current {
                        0.0
                    } else {
                        let far: f32 = e.global(0x011a_4d64);
                        far * far
                    }
                } else {
                    e.call(DISTANCE_SQUARED, &args![actor_location, target_location])
                        .f32()
                };
                let cap: f32 = e.global(0x0108_d660);
                let distance = e.call(0x0040_ebd0, &args![cap, distance]).f32();
                let divisor: f64 = e.global(0x0108_d658);
                let weight: f32 = e.global(0x011a_4d60);
                score = ((1.0 - distance as f64 / divisor) * weight as f64 + score as f64) as f32;
                let current_attackers = is_current as u16;
                if e.get(entry, CombatTarget::sAttackerCount) > current_attackers {
                    score += e.global::<f32>(0x011f_18b0);
                }
                if e.call(0x0098_71e0, &args![process_part]).bool() || other_space {
                    let reach: f32 = e.global(0x0103_2adc);
                    if e.call(0x009a_0f40, &args![process_part, target_location, reach])
                        .bool()
                    {
                        score -= e.global::<f32>(0x011a_4d5c);
                    }
                }
                if other_space && !e.call(0x009a_96f0, &args![actor, target_location]).bool() {
                    score -= e.global::<f32>(0x011a_4d58);
                }
                if e.vcall(target.addr(), 0x22c, &args![0u32]).bool()
                    || e.vcall(target.addr(), 0x230, &args![]).bool()
                {
                    score -= e.global::<f32>(0x011a_4d54);
                }
                if e.call(ACTOR_IS_FLEEING, &args![target, 0u32]).bool() {
                    score -= e.global::<f32>(0x011a_4d50);
                }
            }
            if best_score < score {
                best_score = score;
                best_actor = target;
            }
        } else {
            let noticed = time_stamp(e, entry.byte_add(CombatTarget::fLastNoticedTimeStamp.off));
            let past = (noticed as f64 - e.global::<f64>(0x0101_db88)) as f32;
            let current = time_stamp(e, detected_stamp);
            let newer = e.call(0x0040_4010, &args![past, current]).f32();
            e.call(TIME_STAMP_SET, &args![stamp_temporary, newer]);
            let newer = e.mem.u32(stamp_temporary.addr());
            e.mem.set_u32(detected_stamp.addr(), newer);
            let candidate = time_stamp(e, detected_stamp);
            if time_stamp(e, fallback_stamp) < candidate {
                best_by_stamp = target;
                let value = e.mem.u32(detected_stamp.addr());
                e.mem.set_u32(fallback_stamp.addr(), value);
            }
        }
        last_examined = target;
    }
    if best_actor.is_null() {
        best_actor = best_by_stamp;
    }
    if best_actor.is_null() {
        best_actor = last_examined;
    }
    best_actor
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x0069cfe0,
            fn_0069cfe0(Ptr<BSSimpleArray>, Ptr<BSSimpleArray>)
        ),
        entry!(
            0x00985410,
            fn_00985410(Ptr<CombatTarget>, Ptr, u16) -> Ptr<CombatTarget>
        ),
        entry!(
            0x00985520,
            fn_00985520(Ptr<CombatTarget>, Ptr<CombatTarget>)
        ),
        entry!(
            0x009856e0,
            combat_target_save_game(Ptr<CombatTarget>, Ptr, u32)
        ),
        entry!(
            0x00985800,
            combat_target_load_game(Ptr<CombatTarget>, Ptr, u32)
        ),
        entry!(0x00985930, fn_00985930(Ptr, u32, u32)),
        entry!(
            0x00985980,
            fn_00985980(Ptr<CombatMember>, Ptr<CombatMember>)
        ),
        entry!(
            0x009859c0,
            combat_member_save_game(Ptr<CombatMember>, Ptr, Ptr<CombatGroup>)
        ),
        entry!(
            0x00985a50,
            combat_member_load_game(Ptr<CombatMember>, Ptr, Ptr<CombatGroup>)
        ),
        entry!(0x00985ae0, fn_00985ae0(Ptr, Ptr, u32)),
        entry!(0x00985b10, fn_00985b10(Ptr, Ptr, u32)),
        entry!(
            0x00985b40,
            fn_00985b40(Ptr<CombatSearchLocation>, Ptr) -> f32
        ),
        entry!(0x00985bb0, fn_00985bb0(Ptr<CombatSearchLocation>) -> f32),
        entry!(
            0x00985be0,
            fn_00985be0(Ptr<CombatSearchLocation>, Ptr) -> Ptr
        ),
        entry!(
            0x00985d10,
            fn_00985d10(Ptr<CombatGroup>, u32) -> Ptr<CombatGroup>
        ),
        entry!(0x00985f90, fn_00985f90(Ptr<CombatGroup>)),
        entry!(0x00986150, combat_group_update(Ptr<CombatGroup>)),
        entry!(0x009862e0, fn_009862e0(Ptr<CombatGroup>, Ptr)),
        entry!(0x009863b0, fn_009863b0(Ptr)),
        entry!(
            0x00986410,
            combat_group_add_target(Ptr<CombatGroup>, Ptr) -> bool
        ),
        entry!(
            0x00986500,
            combat_group_remove_target(Ptr<CombatGroup>, Ptr)
        ),
        entry!(0x00986560, fn_00986560(Ptr<CombatGroup>, u32)),
        entry!(
            0x009865b0,
            combat_group_is_target(Ptr<CombatGroup>, Ptr) -> bool
        ),
        entry!(
            0x009865d0,
            fn_009865d0(Ptr<CombatGroup>, Ptr) -> Ptr<CombatTarget>
        ),
        entry!(
            0x009865f0,
            fn_009865f0(Ptr<CombatGroup>, Ptr) -> Ptr<CombatMember>
        ),
        entry!(
            0x00986610,
            fn_00986610(Ptr<CombatGroup>, u32) -> Ptr<CombatTarget>
        ),
        entry!(
            0x00986670,
            fn_00986670(Ptr<CombatGroup>, u32) -> Ptr<CombatMember>
        ),
        entry!(
            0x009866d0,
            combat_group_can_add_target(Ptr<CombatGroup>, Ptr) -> bool
        ),
        entry!(0x00986740, fn_00986740(Ptr<CombatGroup>, Ptr<CombatGroup>)),
        entry!(0x00986760, fn_00986760(Ptr<CombatGroup>, Ptr)),
        entry!(
            0x009867d0,
            combat_group_add_member(Ptr<CombatGroup>, Ptr) -> bool
        ),
        entry!(
            0x00986950,
            fn_00986950(Ptr<CombatMember>, Ptr) -> Ptr<CombatMember>
        ),
        entry!(0x009869a0, fn_009869a0(Ptr<CombatGroup>, Ptr)),
        entry!(
            0x00986a80,
            combat_group_is_member(Ptr<CombatGroup>, Ptr) -> bool
        ),
        entry!(
            0x00986aa0,
            combat_group_can_add_member(Ptr<CombatGroup>, Ptr) -> bool
        ),
        entry!(0x00986b00, fn_00986b00(Ptr<CombatGroup>, Ptr)),
        entry!(0x00986b70, fn_00986b70(Ptr<CombatGroup>)),
        entry!(0x00986bd0, fn_00986bd0(Ptr<CombatGroup>)),
        entry!(0x00986c40, fn_00986c40(Ptr<CombatGroup>) -> bool),
        entry!(0x00986c60, fn_00986c60(Ptr<CombatGroup>, Ptr) -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game time `00435dd0` returns in these tests.
    const NOW: f32 = 100.0;
    /// Where the fake actors' vtable lives.
    const ACTOR_VTABLE: u32 = 0x7100_0000;
    /// Address of the fake virtual method at `slot` of the actors' vtable.
    const fn fake(slot: u32) -> u32 {
        0x7000_0000 + slot
    }

    /// Engine with the data the code reads mapped, the array, form and time
    /// stamp accessors doubled over memory, and an actor vtable. The tests
    /// double everything else they reach.
    ///
    /// Fake actor layout: +0x0c form ID, +0x30 3D root, +0x34 / +0x35 the
    /// results of virtual methods 0x22c / 0x230, +0x38 process, +0x40 the
    /// position slot 0x1f4 returns.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_2000u32,
            0x0101_6000,
            0x0101_d000,
            0x0101_e000,
            0x0101_f000,
            0x0102_0000,
            0x0102_3000,
            0x0103_2000,
            0x0108_d000,
            0x011a_4000,
            0x011c_e000,
            0x011c_f000,
            0x011f_1000,
            0x011f_4000,
        ] {
            e.map(page, 0x1000);
        }
        e.set_global(0x011f_1bf0, NOW);
        e.set_global(MAX_FLOAT, f32::MAX);
        e.set_global(MINUS_ONE, -1.0f32);
        e.set_global(ZERO_DOUBLE, 0.0f64);

        e.register(ARRAY_SIZE, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(TARGET_COUNT, |e, a| e.mem.u32(a[0] + 0x10).into_ret());
        e.register(MEMBER_COUNT, |e, a| e.mem.u32(a[0] + 0x20).into_ret());
        e.register(TARGET_AT, |e, a| {
            (e.mem.u32(a[0] + 4) + a[1] * 0x68).into_ret()
        });
        e.register(MEMBER_AT, |e, a| {
            (e.mem.u32(a[0] + 4) + a[1] * 0x14).into_ret()
        });
        e.register(TARGET_ACTOR_AT, |e, a| {
            e.mem.u32(e.mem.u32(a[0] + 0x0c) + a[1] * 0x68).into_ret()
        });
        e.register(MEMBER_ACTOR_AT, |e, a| {
            e.mem.u32(e.mem.u32(a[0] + 0x1c) + a[1] * 0x14).into_ret()
        });
        e.register(CLUSTER_AT, |e, a| {
            (e.mem.u32(a[0] + 4) + a[1] * 4).into_ret()
        });
        e.register(FORM_ID, |e, a| e.mem.u32(a[0] + 0x0c).into_ret());
        e.register(TIME_STAMP_VALUE, |e, a| e.mem.f32(a[0]).into_ret());
        e.register(GAME_TIME_NOW, |e, _| {
            e.global::<f32>(0x011f_1bf0).into_ret()
        });
        e.register(TIME_STAMP_SET, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        e.register(TIME_STAMP_ZERO, |e, a| {
            e.mem.set_f32(a[0], 0.0);
            a[0].into_ret()
        });
        e.register(TIME_STAMP_AGE, |e, a| (NOW - e.mem.f32(a[0])).into_ret());
        // `0040ebd0` is the smaller of its two floats (ties: the second),
        // `00404010` the larger (ties: the second).
        e.register(0x0040_ebd0, |_, a| {
            let (x, y) = (f32::from_bits(a[0]), f32::from_bits(a[1]));
            (if y <= x { y } else { x }).into_ret()
        });
        e.register(0x0040_4010, |_, a| {
            let (x, y) = (f32::from_bits(a[0]), f32::from_bits(a[1]));
            (if y < x { x } else { y }).into_ret()
        });
        // A setting accessor answers with the address it was given, so a
        // test writes the setting's value there.
        e.register(SETTING_BOOL_VALUE, |_, a| a[0].into_ret());
        e.register(SETTING_FLOAT_VALUE, |_, a| a[0].into_ret());
        e.register(LOG_MESSAGE, |_, _| Ret::default());

        let mut slots = vec![0u32; 0x120];
        for slot in [0x130u32, 0x1d0, 0x1f4, 0x22c, 0x230, 0x3fc, 0x428] {
            slots[(slot / 4) as usize] = fake(slot);
        }
        e.put_vtable(ACTOR_VTABLE, &slots);
        e.register(fake(0x130), |_, _| 0xbeefu32.into_ret());
        e.register(fake(0x1d0), |e, a| e.mem.u32(a[0] + 0x30).into_ret());
        e.register(fake(0x1f4), |_, a| (a[0] + 0x40).into_ret());
        e.register(fake(0x22c), |e, a| e.mem.u8(a[0] + 0x34).into_ret());
        e.register(fake(0x230), |e, a| e.mem.u8(a[0] + 0x35).into_ret());
        e.register(fake(0x3fc), |_, _| Ret::default());
        e.register(fake(0x428), |e, a| e.mem.u32(a[0] + 0x38).into_ret());
        e
    }

    fn actor(e: &mut Engine, form_id: u32) -> Ptr {
        let actor = e.mem.alloc(0x80);
        e.mem.set_u32(actor, ACTOR_VTABLE);
        e.mem.set_u32(actor + 0x0c, form_id);
        Ptr::new(actor)
    }

    /// A group whose arrays hold the given target and member actors. The
    /// fake accessors read the arrays at +0x08 (buffer at +0x0c, size at
    /// +0x10) and +0x18 (buffer +0x1c, size +0x20).
    fn group_with(e: &mut Engine, targets: &[Ptr], members: &[Ptr]) -> Ptr<CombatGroup> {
        let group = e.new_object::<CombatGroup>();
        let target_buffer = e.mem.alloc(0x68 * (targets.len() as u32 + 4));
        for (i, target) in targets.iter().enumerate() {
            e.mem
                .set_u32(target_buffer + 0x68 * i as u32, target.addr());
        }
        e.mem.set_u32(group.addr() + 0x0c, target_buffer);
        e.mem.set_u32(group.addr() + 0x10, targets.len() as u32);
        let member_buffer = e.mem.alloc(0x14 * (members.len() as u32 + 4));
        for (i, member) in members.iter().enumerate() {
            e.mem
                .set_u32(member_buffer + 0x14 * i as u32, member.addr());
        }
        e.mem.set_u32(group.addr() + 0x1c, member_buffer);
        e.mem.set_u32(group.addr() + 0x20, members.len() as u32);
        group
    }

    fn target_entry(e: &Engine, group: Ptr<CombatGroup>, index: u32) -> Ptr<CombatTarget> {
        Ptr::new(e.mem.u32(group.addr() + 0x0c) + 0x68 * index)
    }

    /// Doubles answering 0 for the given addresses, and starts the call log.
    fn record(e: &mut Engine, addrs: &[u32]) {
        for &addr in addrs {
            e.register(addr, |_, _| Ret::default());
        }
        e.call_log = Some(vec![]);
    }

    fn calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    fn addresses(e: &Engine) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(a, _)| *a)
            .collect()
    }

    #[test]
    fn search_door_array_copy_sizes_the_other_array_then_copies() {
        let mut e = engine();
        let source = e.new_object::<BSSimpleArray>();
        let buffer = e.mem.alloc(24);
        for word in 0..6 {
            e.mem.set_u32(buffer + 4 * word, 0x100 + word);
        }
        e.set(source, BSSimpleArray::pBuffer, buffer);
        e.set(source, BSSimpleArray::iSize, 2);
        let other = e.new_object::<BSSimpleArray>();
        let destination = e.mem.alloc(24);
        e.set(other, BSSimpleArray::pBuffer, destination);
        e.register(SEARCH_DOOR_AT, |e, a| {
            (e.mem.u32(a[0] + 4) + a[1] * 12).into_ret()
        });
        record(&mut e, &[SEARCH_DOOR_ARRAY_SET_SIZE]);
        fn_0069cfe0(&mut e, source, other);
        assert_eq!(
            calls(&e, SEARCH_DOOR_ARRAY_SET_SIZE),
            vec![vec![other.addr(), 2, 1]]
        );
        for word in 0..6 {
            assert_eq!(e.mem.u32(destination + 4 * word), 0x100 + word);
        }
    }

    #[test]
    fn combat_target_constructor_sets_defaults_and_the_noticed_location() {
        let mut e = engine();
        e.register(WORLD_LOCATION_CONSTRUCT, |e, a| {
            for word in 0..3 {
                e.mem.set_u32(a[0] + 4 * word, 0x5555);
            }
            a[0].into_ret()
        });
        e.register(GET_WORLD_LOCATION, |e, a| {
            for word in 0..4 {
                e.mem.set_u32(a[1] + 4 * word, 0x700 + word);
            }
            a[1].into_ret()
        });
        let target = e.new_object::<CombatTarget>();
        let who = actor(&mut e, 7);
        let result = fn_00985410(&mut e, target, who, 9);
        assert_eq!(result, target);
        assert_eq!(e.get(target, CombatTarget::pActor), who);
        assert_eq!(e.get(target, CombatTarget::iDetectionLevel), 0);
        assert_eq!(e.get(target, CombatTarget::sLastSearchNoticed), 9);
        assert_eq!(e.get(target, CombatTarget::sAttackerCount), 0);
        assert_eq!(e.get(target, CombatTarget::fLastSeenTimeStamp), -f32::MAX);
        assert_eq!(
            e.get(target, CombatTarget::fLastDetectedTimeStamp),
            -f32::MAX
        );
        assert_eq!(
            e.get(target, CombatTarget::fDetectionLevelUpdateTimeStamp),
            0.0
        );
        assert_eq!(e.get(target, CombatTarget::fLastNoticedTimeStamp), NOW);
        assert_eq!(e.get(target, CombatTarget::fDetectionEventTimeStamp), 0.0);
        assert_eq!(
            e.get(target, CombatTarget::fLastAttackedMemberTimeStamp),
            -f32::MAX
        );
        // The other locations keep the constructor's default; the noticed
        // one is the actor's.
        assert_eq!(e.mem.u32(target.addr() + 0x18), 0x5555);
        for word in 0..4 {
            assert_eq!(e.mem.u32(target.addr() + 8 + 4 * word), 0x700 + word);
        }
    }

    #[test]
    fn combat_target_merge_takes_newer_stamps_with_their_locations_and_sums_counts() {
        let mut e = engine();
        let mine = e.new_object::<CombatTarget>();
        let theirs = e.new_object::<CombatTarget>();
        e.set(mine, CombatTarget::iDetectionLevel, 2);
        e.set(theirs, CombatTarget::iDetectionLevel, 5);
        // Last detected: theirs is newer (copied with its location).
        e.set(mine, CombatTarget::fLastDetectedTimeStamp, 1.0);
        e.set(theirs, CombatTarget::fLastDetectedTimeStamp, 2.0);
        e.mem.set_u32(theirs.addr() + 0x18, 0xd1);
        // Last seen: mine is newer (kept).
        e.set(mine, CombatTarget::fLastSeenTimeStamp, 9.0);
        e.set(theirs, CombatTarget::fLastSeenTimeStamp, 3.0);
        e.mem.set_u32(theirs.addr() + 0x28, 0xd2);
        // Detection level update: equal (kept).
        e.set(mine, CombatTarget::fDetectionLevelUpdateTimeStamp, 4.0);
        e.set(theirs, CombatTarget::fDetectionLevelUpdateTimeStamp, 4.0);
        // Last noticed: theirs newer (no location).
        e.set(mine, CombatTarget::fLastNoticedTimeStamp, 1.0);
        e.set(theirs, CombatTarget::fLastNoticedTimeStamp, 6.0);
        // Last attacked member: theirs newer (with location).
        e.set(mine, CombatTarget::fLastAttackedMemberTimeStamp, 1.0);
        e.set(theirs, CombatTarget::fLastAttackedMemberTimeStamp, 8.0);
        e.mem.set_u32(theirs.addr() + 0x38, 0xd3);
        e.set(mine, CombatTarget::sAttackerCount, 1);
        e.set(theirs, CombatTarget::sAttackerCount, 2);
        e.set(mine, CombatTarget::cMemberLOSCount, 250);
        e.set(theirs, CombatTarget::cMemberLOSCount, 10);
        e.set(mine, CombatTarget::cMember360LOSCount, 1);
        e.set(theirs, CombatTarget::cMember360LOSCount, 1);
        fn_00985520(&mut e, mine, theirs);
        assert_eq!(e.get(mine, CombatTarget::iDetectionLevel), 5);
        assert_eq!(e.get(mine, CombatTarget::fLastDetectedTimeStamp), 2.0);
        assert_eq!(e.mem.u32(mine.addr() + 0x18), 0xd1);
        assert_eq!(e.get(mine, CombatTarget::fLastSeenTimeStamp), 9.0);
        assert_eq!(e.mem.u32(mine.addr() + 0x28), 0);
        assert_eq!(e.get(mine, CombatTarget::fLastNoticedTimeStamp), 6.0);
        assert_eq!(e.get(mine, CombatTarget::fLastAttackedMemberTimeStamp), 8.0);
        assert_eq!(e.mem.u32(mine.addr() + 0x38), 0xd3);
        assert_eq!(e.get(mine, CombatTarget::sAttackerCount), 3);
        assert_eq!(e.get(mine, CombatTarget::cMemberLOSCount), 4); // wraps
        assert_eq!(e.get(mine, CombatTarget::cMember360LOSCount), 2);
        // A lower detection level on the other side is not taken.
        e.set(theirs, CombatTarget::iDetectionLevel, 1);
        fn_00985520(&mut e, mine, theirs);
        assert_eq!(e.get(mine, CombatTarget::iDetectionLevel), 5);
    }

    #[test]
    fn combat_target_save_writes_fields_in_order() {
        let mut e = engine();
        record(
            &mut e,
            &[
                SAVE_FORM_ID,
                SAVE_BYTES,
                WORLD_LOCATION_SAVE,
                TIME_STAMP_SAVE,
            ],
        );
        let target = e.new_object::<CombatTarget>();
        let who = actor(&mut e, 7);
        e.set(target, CombatTarget::pActor, who);
        let buffer = Ptr::new(0x6000_0000);
        combat_target_save_game(&mut e, target, buffer, 0);
        let t = target.addr();
        let b = buffer.addr();
        assert_eq!(calls(&e, SAVE_FORM_ID), vec![vec![b, who.addr(), 0]]);
        assert_eq!(
            calls(&e, SAVE_BYTES),
            vec![
                vec![b, t + 4, 4, 0],
                vec![b, t + 0x48, 2, 0],
                vec![b, t + 0x4a, 2, 0],
                vec![b, t + 0x64, 1, 0],
                vec![b, t + 0x65, 1, 0],
            ]
        );
        assert_eq!(
            calls(&e, WORLD_LOCATION_SAVE),
            vec![
                vec![t + 8, b],
                vec![t + 0x18, b],
                vec![t + 0x28, b],
                vec![t + 0x38, b]
            ]
        );
        let stamps: Vec<u32> = calls(&e, TIME_STAMP_SAVE)
            .iter()
            .map(|c| c[0] - t)
            .collect();
        assert_eq!(stamps, vec![0x50, 0x54, 0x58, 0x4c, 0x5c, 0x60]);
    }

    /// A save buffer whose version (virtual method at slot 0) is `version`.
    fn buffer_with_version(e: &mut Engine, version: u32) -> Ptr {
        let vtable = 0x7200_0000;
        e.put_vtable(vtable, &[0x7200_0100]);
        e.register(0x7200_0100, |e, a| e.mem.u32(a[0] + 4).into_ret());
        let buffer = e.mem.alloc(8);
        e.mem.set_u32(buffer, vtable);
        e.mem.set_u32(buffer + 4, version);
        Ptr::new(buffer)
    }

    #[test]
    fn combat_target_load_reads_newer_fields_only_in_newer_saves() {
        for (version, locations, stamps) in [(12u32, 3usize, 5usize), (13, 3, 6), (14, 4, 6)] {
            let mut e = engine();
            record(
                &mut e,
                &[
                    LOAD_FORM_ID,
                    LOAD_BYTES,
                    WORLD_LOCATION_LOAD,
                    TIME_STAMP_LOAD,
                ],
            );
            let buffer = buffer_with_version(&mut e, version);
            let target = e.new_object::<CombatTarget>();
            combat_target_load_game(&mut e, target, buffer, 0);
            let t = target.addr();
            assert_eq!(calls(&e, LOAD_FORM_ID), vec![vec![buffer.addr(), t]]);
            assert_eq!(calls(&e, LOAD_BYTES).len(), 5);
            assert_eq!(calls(&e, LOAD_BYTES)[1], vec![buffer.addr(), t + 0x48, 2]);
            assert_eq!(calls(&e, WORLD_LOCATION_LOAD).len(), locations, "{version}");
            assert_eq!(calls(&e, TIME_STAMP_LOAD).len(), stamps, "{version}");
            if version >= 14 {
                assert_eq!(
                    calls(&e, WORLD_LOCATION_LOAD)[3],
                    vec![t + 0x38, buffer.addr()]
                );
            }
            if version >= 13 {
                assert_eq!(calls(&e, TIME_STAMP_LOAD)[5][0], t + 0x60);
            }
        }
    }

    #[test]
    fn fn_00985930_resolves_a_form_into_an_actor_or_keeps_null() {
        let mut e = engine();
        record(&mut e, &[]);
        e.register(0x0048_39c0, |_, a| (a[0] + 1).into_ret());
        e.register(0x00ec_43fb, |_, a| (a[0] + 2).into_ret());
        let slot = e.mem.alloc(4);
        fn_00985930(&mut e, Ptr::new(slot), 0, 0);
        assert_eq!(e.mem.u32(slot), 0);
        assert!(calls(&e, 0x0048_39c0).is_empty());
        e.mem.set_u32(slot, 0x100);
        fn_00985930(&mut e, Ptr::new(slot), 0, 0);
        assert_eq!(e.mem.u32(slot), 0x103);
        assert_eq!(calls(&e, 0x0048_39c0), vec![vec![0x100]]);
        assert_eq!(
            calls(&e, 0x00ec_43fb),
            vec![vec![0x101, 0, 0x0118_3028, 0x0118_46d4, 0]]
        );
    }

    #[test]
    fn fn_00985980_copies_strengths_and_cluster() {
        let mut e = engine();
        let from = e.new_object::<CombatMember>();
        let to = e.new_object::<CombatMember>();
        e.set(from, CombatMember::pActor, Ptr::new(0x1111));
        e.set(from, CombatMember::fDamagePerSecond, 2.5);
        e.set(from, CombatMember::fCombatStrength, 7.0);
        e.set(from, CombatMember::pCluster, Ptr::new(0x2222));
        e.set(to, CombatMember::pActor, Ptr::new(0x3333));
        fn_00985980(&mut e, to, from);
        assert_eq!(e.get(to, CombatMember::fDamagePerSecond), 2.5);
        assert_eq!(e.get(to, CombatMember::fCombatStrength), 7.0);
        assert_eq!(e.get(to, CombatMember::pCluster), Ptr::new(0x2222));
        assert_eq!(e.get(to, CombatMember::pActor), Ptr::new(0x3333));
    }

    #[test]
    fn combat_member_save_writes_the_cluster_index_or_ff() {
        let mut e = engine();
        record(&mut e, &[SAVE_FORM_ID]);
        // The raw writer remembers the last single byte it was asked to
        // write (in a spare global).
        e.register(SAVE_BYTES, |e, a| {
            if a[2] == 1 {
                let byte = e.mem.u8(a[1]);
                e.set_global(0x011f_1000, byte);
            }
            Ret::default()
        });
        e.register(0x0098_de30, |_, a| {
            assert_eq!(a[1], 0xc1c1);
            3u32.into_ret()
        });
        let member = e.new_object::<CombatMember>();
        let who = actor(&mut e, 1);
        e.set(member, CombatMember::pActor, who);
        let group = e.new_object::<CombatGroup>();
        let buffer = Ptr::new(0x6000_0000);
        e.call_log = Some(vec![]);
        combat_member_save_game(&mut e, member, buffer, group);
        assert_eq!(e.global::<u8>(0x011f_1000), 0xff);
        assert_eq!(
            calls(&e, SAVE_FORM_ID),
            vec![vec![buffer.addr(), who.addr(), 0]]
        );
        e.set(member, CombatMember::pCluster, Ptr::new(0xc1c1));
        combat_member_save_game(&mut e, member, buffer, group);
        assert_eq!(e.global::<u8>(0x011f_1000), 3);
        let m = member.addr();
        let words: Vec<Vec<u32>> = calls(&e, SAVE_BYTES)
            .into_iter()
            .filter(|c| c[2] == 4)
            .take(3)
            .collect();
        assert_eq!(
            words,
            vec![
                vec![buffer.addr(), m + 4, 4, 0],
                vec![buffer.addr(), m + 8, 4, 0],
                vec![buffer.addr(), m + 0xc, 4, 0],
            ]
        );
    }

    #[test]
    fn combat_member_load_turns_the_index_back_into_a_cluster() {
        let mut e = engine();
        record(&mut e, &[LOAD_FORM_ID]);
        e.register(LOAD_BYTES, |e, a| {
            if a[2] == 1 {
                let value = e.global::<u8>(0x011f_1000);
                e.mem.set_u8(a[1], value);
            }
            Ret::default()
        });
        e.register(0x0098_de60, |_, a| (0xc000 + (a[1] & 0xff)).into_ret());
        let member = e.new_object::<CombatMember>();
        let group = e.new_object::<CombatGroup>();
        e.set(member, CombatMember::pCluster, Ptr::new(0x1234));
        e.set_global(0x011f_1000, 0xffu8);
        combat_member_load_game(&mut e, member, Ptr::new(0x6000_0000), group);
        assert!(e.get(member, CombatMember::pCluster).is_null());
        e.set_global(0x011f_1000, 2u8);
        combat_member_load_game(&mut e, member, Ptr::new(0x6000_0000), group);
        assert_eq!(e.get(member, CombatMember::pCluster), Ptr::new(0xc002));
        assert_eq!(calls(&e, LOAD_FORM_ID)[0], vec![0x6000_0000, member.addr()]);
    }

    #[test]
    fn cluster_save_and_load_use_sixteen_plus_four_bytes() {
        let mut e = engine();
        record(&mut e, &[SAVE_BYTES, LOAD_BYTES]);
        let cluster = Ptr::new(0x5000_0000);
        let buffer = Ptr::new(0x6000_0000);
        fn_00985ae0(&mut e, cluster, buffer, 0);
        fn_00985b10(&mut e, cluster, buffer, 0);
        assert_eq!(
            calls(&e, SAVE_BYTES),
            vec![
                vec![0x6000_0000, 0x5000_0000, 0x10, 0],
                vec![0x6000_0000, 0x5000_0010, 4, 0]
            ]
        );
        assert_eq!(
            calls(&e, LOAD_BYTES),
            vec![
                vec![0x6000_0000, 0x5000_0000, 0x10],
                vec![0x6000_0000, 0x5000_0010, 4]
            ]
        );
    }

    #[test]
    fn search_location_time_left_score_and_colour() {
        let mut e = engine();
        e.set_global(0x0108_d5a0, 16777216.0f32);
        e.set_global(0x0108_d598, 360000.0f64);
        e.set_global(0x0102_0758, 10.0f64);
        e.set_global(0x0101_6248, 0.5f32);
        e.set_global(0x0101_e2c0, 50.0f64);
        e.set_global(0x0101_db88, 30.0f64);
        e.register(DISTANCE_SQUARED, |e, a| {
            (e.mem.f32(a[0]) - e.mem.f32(a[1])).powi(2).into_ret()
        });
        e.register(0x0041_4430, |e, a| {
            for word in 0..4 {
                e.mem.set_u32(a[0] + 4 * word, a[1 + word as usize]);
            }
            a[0].into_ret()
        });
        let entry = e.new_object::<CombatSearchLocation>();
        // Stamped 20 seconds ago with 60 seconds of duration: 40 left.
        e.set(entry, CombatSearchLocation::fTimeStamp, NOW - 20.0);
        e.set(entry, CombatSearchLocation::fScore, 60.0);
        assert_eq!(fn_00985bb0(&mut e, entry), 40.0);

        let from = e.mem.alloc(16);
        e.mem.set_f32(from, 600.0); // the entry's point is at 0: 360000 away
        assert_eq!(fn_00985b40(&mut e, entry, Ptr::new(from)), 39.0);
        e.set(entry, CombatSearchLocation::iTargetID, 5);
        assert_eq!(fn_00985b40(&mut e, entry, Ptr::new(from)), 49.0);

        let colour = e.mem.alloc(16);
        for (score, want) in [
            (80.0f32, [1.0f32, 0.0, 1.0, 0.5]),
            (60.0, [0.0, 0.0, 1.0, 0.5]),
            (45.0, [1.0, 1.0, 0.0, 0.5]),
        ] {
            // 20 seconds old: 60, 40 and 25 seconds left.
            e.set(entry, CombatSearchLocation::fScore, score);
            let out = fn_00985be0(&mut e, entry, Ptr::new(colour));
            assert_eq!(out.addr(), colour);
            let got: Vec<f32> = (0..4).map(|i| e.mem.f32(colour + 4 * i)).collect();
            assert_eq!(got, want, "{score}");
        }
    }

    /// The combat manager singleton of these tests.
    const MANAGER: u32 = 0x7300_0000;

    fn with_manager(e: &mut Engine) {
        e.set_global(COMBAT_MANAGER, MANAGER);
    }

    #[test]
    fn group_constructor_sets_every_field_and_constructs_the_parts() {
        let mut e = engine();
        let timers = [0x0090_6f60u32];
        record(
            &mut e,
            &[
                0x0098_ed30,
                0x0098_eff0,
                timers[0],
                0x0040_1050,
                0x0063_3c90,
                WORLD_LOCATION_CONSTRUCT,
                0x0098_f2a0,
                0x0098_f4c0,
                0x0098_f510,
            ],
        );
        for word in 0..3 {
            e.set_global(0x011f_426c + 4 * word, 0x4000_0000 + word);
        }
        let group = e.new_object::<CombatGroup>();
        e.mem.write(group.addr(), &[0xaa; 0x15c]);
        let result = fn_00985d10(&mut e, group, 12);
        assert_eq!(result, group);
        assert_eq!(e.get(group, CombatGroup::iGroupNum), 0);
        assert_eq!(e.get(group, CombatGroup::iGroupID), 12);
        assert!(e.get(group, CombatGroup::pGroupStrategy).is_null());
        assert!(!e.get(group, CombatGroup::bStrategyForced));
        assert_eq!(e.get(group, CombatGroup::iLastGroupStrategyChosenIndex), 0);
        assert_eq!(
            e.get(group, CombatGroup::fLastGroupStrategyChosenTimeStamp),
            0.0
        );
        assert_eq!(e.get(group, CombatGroup::fMemberCombatStrength), -1.0);
        assert_eq!(
            e.get(group, CombatGroup::fAverageMemberDamagePerSecond),
            -1.0
        );
        assert_eq!(e.get(group, CombatGroup::fTargetCombatStrength), -1.0);
        assert_eq!(
            e.get(group, CombatGroup::fAverageTargetCombatStrength),
            -1.0
        );
        assert_eq!(e.get(group, CombatGroup::iSearchCount), 0);
        assert_eq!(
            e.get(group, CombatGroup::fSearchStartedTimeStamp),
            -f32::MAX
        );
        assert!(e.get(group, CombatGroup::pSearchingMember).is_null());
        for word in 0..3 {
            assert_eq!(
                e.mem.u32(group.addr() + 0x100 + 4 * word),
                0x4000_0000 + word
            );
        }
        assert_eq!(e.get(group, CombatGroup::fSearchRadius), 0.0);
        assert!(!e.get(group, CombatGroup::bUpdateSearchDebugGeometry));
        assert_eq!(e.get(group, CombatGroup::iInitializedMemberCount), 0);
        assert_eq!(e.get(group, CombatGroup::iFleeingMemberCount), 0);
        assert_eq!(e.get(group, CombatGroup::iNonFleeingMemberCount), 0);
        assert_eq!(e.get(group, CombatGroup::cCombatMusicState), -1);

        let g = group.addr();
        let timer_offsets: Vec<u32> = calls(&e, 0x0090_6f60).iter().map(|c| c[0] - g).collect();
        assert_eq!(
            timer_offsets,
            vec![0x30, 0x38, 0x48, 0x50, 0xa8, 0xb0, 0xb8, 0xd8, 0xe0]
        );
        assert_eq!(
            calls(&e, 0x0040_1050),
            vec![vec![g + 0x58, 8, 10, 0x0090_6f60]]
        );
        assert_eq!(calls(&e, 0x0098_ed30), vec![vec![g + 0x08]]);
        assert_eq!(calls(&e, 0x0098_eff0), vec![vec![g + 0x18]]);
        assert_eq!(calls(&e, 0x0098_f2a0), vec![vec![g + 0x110]]);
        assert_eq!(calls(&e, 0x0098_f4c0), vec![vec![g + 0x120]]);
        assert_eq!(calls(&e, 0x0098_f510), vec![vec![g + 0x138]]);
        assert_eq!(
            calls(&e, 0x0063_3c90),
            vec![vec![g + 0xd4, 0], vec![g + 0x130, 0], vec![g + 0x158, 0]]
        );
        assert_eq!(calls(&e, WORLD_LOCATION_CONSTRUCT), vec![vec![g + 0xf0]]);
    }

    #[test]
    fn group_destructor_leaves_the_manager_then_destroys_parts_in_reverse() {
        let mut e = engine();
        with_manager(&mut e);
        let t0 = actor(&mut e, 1);
        let t1 = actor(&mut e, 2);
        let m0 = actor(&mut e, 3);
        let group = group_with(&mut e, &[t0, t1], &[m0]);
        // Two clusters in the cluster array (buffer +0x13c, size +0x140).
        let clusters = e.mem.alloc(8);
        e.mem.set_u32(clusters, 0xc100);
        e.mem.set_u32(clusters + 4, 0xc200);
        e.mem.set_u32(group.addr() + 0x13c, clusters);
        e.mem.set_u32(group.addr() + 0x140, 2);
        let parts = [
            MANAGER_REMOVE_TARGET,
            MANAGER_REMOVE_MEMBER,
            0x0040_1030,
            0x0098_78b0,
            0x0045_cec0,
            0x0098_f540,
            0x0098_f4f0,
            0x0098_f2d0,
            0x0098_f020,
            0x0098_ed60,
        ];
        record(&mut e, &parts);
        fn_00985f90(&mut e, group);
        let g = group.addr();
        assert_eq!(
            calls(&e, MANAGER_REMOVE_TARGET),
            vec![vec![MANAGER, g, t0.addr()], vec![MANAGER, g, t1.addr()]]
        );
        assert_eq!(
            calls(&e, MANAGER_REMOVE_MEMBER),
            vec![vec![MANAGER, g, m0.addr()]]
        );
        assert_eq!(calls(&e, 0x0040_1030), vec![vec![0xc100], vec![0xc200]]);
        let order: Vec<(u32, Vec<u32>)> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| {
                [
                    0x0098_78b0,
                    0x0045_cec0,
                    0x0098_f540,
                    0x0098_f4f0,
                    0x0098_f2d0,
                    0x0098_f020,
                    0x0098_ed60,
                ]
                .contains(a)
            })
            .cloned()
            .collect();
        assert_eq!(
            order,
            vec![
                (0x0098_78b0, vec![g, 1]),
                (0x0045_cec0, vec![g + 0x158]),
                (0x0098_f540, vec![g + 0x138]),
                (0x0045_cec0, vec![g + 0x130]),
                (0x0098_f4f0, vec![g + 0x120]),
                (0x0098_f2d0, vec![g + 0x110]),
                (0x0045_cec0, vec![g + 0xd4]),
                (0x0098_f020, vec![g + 0x18]),
                (0x0098_ed60, vec![g + 0x08]),
            ]
        );
    }

    /// Timers for the update tests: a timer whose first float is 1.0 has
    /// expired, one whose second float is not zero is running; starting one
    /// clears the first and stores the delay in the second.
    fn update_engine() -> Engine {
        let mut e = engine();
        e.register(TIMER_EXPIRED, |e, a| (e.mem.f32(a[0]) == 1.0).into_ret());
        e.register(TIMER_RUNNING, |e, a| {
            (e.mem.f32(a[0] + 4) != 0.0).into_ret()
        });
        e.register(TIMER_START, |e, a| {
            e.mem.set_f32(a[0], 0.0);
            e.mem.set_u32(a[0] + 4, a[1]);
            Ret::default()
        });
        // `0097ef30`: whether a search was started (`iSearchCount`).
        e.register(0x0097_ef30, |e, a| (e.mem.u32(a[0] + 0xd0) != 0).into_ret());
        record(
            &mut e,
            &[
                0x0098_ce80,
                0x0098_cf70,
                0x0098_d030,
                0x0098_d1a0,
                0x0098_a580,
                0x0098_c870,
                0x0098_d2c0,
            ],
        );
        e.set_global(0x0101_62c0, 2.0f32);
        e.set_global(0x011c_fe84, 5.0f32);
        e.set_global(0x011c_eb44, 9.0f32);
        e.set_global(0x011c_ee6c, 0.5f32);
        e.set_global(0x011c_f944, 0.8f32);
        e
    }

    /// A group whose music state is still undecided (-1).
    fn music_group(e: &mut Engine) -> Ptr<CombatGroup> {
        let group = group_with(e, &[], &[]);
        e.set(group, CombatGroup::cCombatMusicState, -1);
        group
    }

    fn expire(e: &mut Engine, group: Ptr<CombatGroup>, timer: u32) {
        e.mem.set_f32(group.addr() + timer, 1.0);
    }

    #[test]
    fn update_with_nothing_due_only_refreshes() {
        let mut e = update_engine();
        let group = group_with(&mut e, &[], &[]);
        e.set(group, CombatGroup::cCombatMusicState, 0);
        combat_group_update(&mut e, group);
        assert_eq!(
            addresses(&e)
                .into_iter()
                .filter(|a| *a != TIMER_EXPIRED && *a != MEMBER_COUNT)
                .collect::<Vec<_>>(),
            vec![0x0098_ce80, 0x0098_d1a0, 0x0098_a580]
        );
    }

    #[test]
    fn update_restarts_the_expired_timers_and_runs_the_extras() {
        let mut e = update_engine();
        let m0 = actor(&mut e, 1);
        let m1 = actor(&mut e, 2);
        let group = group_with(&mut e, &[], &[m0, m1]);
        e.set(group, CombatGroup::cCombatMusicState, 0);
        expire(&mut e, group, CombatGroup::TARGET_UPDATE_TIMER);
        expire(&mut e, group, CombatGroup::COMBAT_STRENGTH_TIMER);
        combat_group_update(&mut e, group);
        let g = group.addr();
        assert_eq!(
            calls(&e, TIMER_START),
            vec![
                vec![g + 0xa8, 2.0f32.to_bits()],
                vec![g + 0x50, 5.0f32.to_bits()]
            ]
        );
        assert_eq!(calls(&e, 0x0098_cf70).len(), 1);
        assert_eq!(calls(&e, 0x0098_d030).len(), 1);
        assert_eq!(calls(&e, 0x0098_c870).len(), 1);
        assert_eq!(calls(&e, 0x0098_d2c0).len(), 1, "two members");
    }

    #[test]
    fn update_decides_the_music_state() {
        // Not running: armed with the setting, unless a search started.
        let mut e = update_engine();
        let group = music_group(&mut e);
        combat_group_update(&mut e, group);
        assert_eq!(
            calls(&e, TIMER_START),
            vec![vec![group.addr() + 0xb8, 9.0f32.to_bits()]]
        );
        assert_eq!(e.get(group, CombatGroup::cCombatMusicState), -1);
        let mut e = update_engine();
        let group = music_group(&mut e);
        e.set(group, CombatGroup::iSearchCount, 1);
        combat_group_update(&mut e, group);
        assert!(calls(&e, TIMER_START).is_empty());

        // Running and expired: the strength ratio picks 1 or 0.
        for (member, target, state) in [
            (6.0f32, 10.0f32, 1i8),
            (9.0, 10.0, 0),
            (4.0, 10.0, 0),
            (5.0, 10.0, 0),
            (5.0, 0.0, 0),
        ] {
            let mut e = update_engine();
            let group = music_group(&mut e);
            e.mem.set_f32(group.addr() + 0xb8, 1.0);
            e.mem.set_f32(group.addr() + 0xbc, 3.0);
            e.set(group, CombatGroup::fMemberCombatStrength, member);
            e.set(group, CombatGroup::fTargetCombatStrength, target);
            combat_group_update(&mut e, group);
            assert_eq!(
                e.get(group, CombatGroup::cCombatMusicState),
                state,
                "{member}/{target}"
            );
        }
        // Running but not expired: still undecided.
        let mut e = update_engine();
        let group = music_group(&mut e);
        e.mem.set_f32(group.addr() + 0xbc, 3.0);
        combat_group_update(&mut e, group);
        assert_eq!(e.get(group, CombatGroup::cCombatMusicState), -1);
    }

    #[test]
    fn set_strategy_clears_the_flag_for_null_and_runs_the_strategy_otherwise() {
        let mut e = engine();
        record(
            &mut e,
            &[0x0099_0320, 0x0098_a3c0, 0x0098_f560, 0x0072_6070],
        );
        let group = e.new_object::<CombatGroup>();
        e.set(group, CombatGroup::bStrategyForced, true);
        fn_009862e0(&mut e, group, Ptr::NULL);
        assert!(!e.get(group, CombatGroup::bStrategyForced));
        assert!(calls(&e, 0x0099_0320).is_empty());

        e.register(0x0072_6070, |_, _| 7u32.into_ret());
        let strategy = e.mem.alloc(8);
        e.put_vtable(0x7400_0000, &[0, 0x7400_0100]);
        e.mem.set_u32(strategy, 0x7400_0000);
        e.register(0x7400_0100, |_, _| Ret::default());
        fn_009862e0(&mut e, group, Ptr::new(strategy));
        let g = group.addr();
        let temporary = calls(&e, 0x0099_0320)[0][0];
        assert_eq!(calls(&e, 0x0099_0320), vec![vec![temporary, g]]);
        assert_eq!(calls(&e, 0x0098_a3c0), vec![vec![g]]);
        assert_eq!(calls(&e, 0x7400_0100), vec![vec![strategy, g, temporary]]);
        assert_eq!(calls(&e, 0x0072_6070), vec![vec![strategy]]);
        assert_eq!(e.get(group, CombatGroup::iLastGroupStrategyChosenIndex), 7);
        assert_eq!(
            e.get(group, CombatGroup::fLastGroupStrategyChosenTimeStamp),
            NOW
        );
        assert!(e.get(group, CombatGroup::bStrategyForced));
        assert_eq!(
            e.get(group, CombatGroup::pGroupStrategy),
            Ptr::new(strategy)
        );
        assert_eq!(
            calls(&e, 0x0098_f560),
            vec![vec![temporary + 0x28], vec![temporary + 0x18]]
        );
    }

    #[test]
    fn strategy_temporary_destructor_destroys_two_arrays() {
        let mut e = engine();
        record(&mut e, &[0x0098_f560]);
        fn_009863b0(&mut e, Ptr::new(0x5000_0000));
        assert_eq!(
            calls(&e, 0x0098_f560),
            vec![vec![0x5000_0028], vec![0x5000_0018]]
        );
    }

    /// Doubles that make the array adders and removers real: element
    /// `size` is copied in, resp. elements after `index` are shifted down.
    fn array_engine() -> Engine {
        let mut e = engine();
        with_manager(&mut e);
        e.register(TARGET_ARRAY_ADD, |e, a| {
            let (buffer, size) = (e.mem.u32(a[0] + 4), e.mem.u32(a[0] + 8));
            let bytes = e.mem.bytes(a[1], 0x68);
            e.mem.write(buffer + size * 0x68, &bytes);
            e.mem.set_u32(a[0] + 8, size + 1);
            size.into_ret()
        });
        e.register(MEMBER_ARRAY_ADD, |e, a| {
            let (buffer, size) = (e.mem.u32(a[0] + 4), e.mem.u32(a[0] + 8));
            let bytes = e.mem.bytes(a[1], 0x14);
            e.mem.write(buffer + size * 0x14, &bytes);
            e.mem.set_u32(a[0] + 8, size + 1);
            size.into_ret()
        });
        e.register(TARGET_ARRAY_REMOVE_AT, |e, a| {
            let (buffer, size) = (e.mem.u32(a[0] + 4), e.mem.u32(a[0] + 8));
            let tail = e
                .mem
                .bytes(buffer + (a[1] + 1) * 0x68, (size - a[1] - 1) * 0x68);
            e.mem.write(buffer + a[1] * 0x68, &tail);
            e.mem.set_u32(a[0] + 8, size - 1);
            Ret::default()
        });
        e.register(MEMBER_ARRAY_REMOVE_AT, |e, a| {
            let (buffer, size) = (e.mem.u32(a[0] + 4), e.mem.u32(a[0] + 8));
            let tail = e
                .mem
                .bytes(buffer + (a[1] + 1) * 0x14, (size - a[1] - 1) * 0x14);
            e.mem.write(buffer + a[1] * 0x14, &tail);
            e.mem.set_u32(a[0] + 8, size - 1);
            Ret::default()
        });
        // An actor is incompatible with everything when its byte at +0x3c
        // is set.
        e.register(ACTOR_COMBAT_COMPATIBLE, |e, a| {
            (e.mem.u8(a[0] + 0x3c) == 0).into_ret()
        });
        e.register(WORLD_LOCATION_CONSTRUCT, |_, a| a[0].into_ret());
        e.register(GET_WORLD_LOCATION, |_, a| a[1].into_ret());
        e.register(0x0047_4cb0, |e, a| e.mem.u8(a[0] + 0x3d).into_ret());
        e.register(0x0055_d520, |_, _| 0xcafeu32.into_ret());
        e.register(MANAGER_ADD_TARGET, |_, _| Ret::default());
        e.register(MANAGER_REMOVE_TARGET, |_, _| Ret::default());
        e.register(MANAGER_ADD_MEMBER, |_, _| Ret::default());
        e.register(MANAGER_REMOVE_MEMBER, |_, _| Ret::default());
        e.register(0x0097_f220, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e
    }

    fn array_len(e: &Engine, group: Ptr<CombatGroup>, array: u32) -> u32 {
        e.mem.u32(group.addr() + array + 8)
    }

    #[test]
    fn add_target_handles_each_refusal_and_appends_otherwise() {
        let mut e = array_engine();
        let existing = actor(&mut e, 1);
        let member = actor(&mut e, 2);
        let newcomer = actor(&mut e, 3);
        let group = group_with(&mut e, &[existing], &[member]);
        e.set(group, CombatGroup::iSearchCount, 4);
        assert!(!combat_group_add_target(&mut e, group, Ptr::NULL));
        assert!(combat_group_add_target(&mut e, group, existing));
        assert_eq!(array_len(&e, group, 8), 1);

        // A member of the group: refused with a log line.
        assert!(!combat_group_add_target(&mut e, group, member));
        assert_eq!(calls(&e, LOG_MESSAGE), vec![vec![0x0108_d5a8, 2, 0xcafe]]);
        e.mem.set_u8(member.addr() + 0x3d, 1);
        assert!(!combat_group_add_target(&mut e, group, member));
        assert_eq!(calls(&e, LOG_MESSAGE)[1], vec![0x0108_d5a8, 2, 0xbeef]);

        // A member that cannot fight the newcomer: refused.
        e.mem.set_u8(member.addr() + 0x3c, 1);
        assert!(!combat_group_add_target(&mut e, group, newcomer));
        assert_eq!(array_len(&e, group, 8), 1);
        e.mem.set_u8(member.addr() + 0x3c, 0);

        assert!(combat_group_add_target(&mut e, group, newcomer));
        assert_eq!(array_len(&e, group, 8), 2);
        let entry = target_entry(&e, group, 1);
        assert_eq!(e.get(entry, CombatTarget::pActor), newcomer);
        assert_eq!(e.get(entry, CombatTarget::sLastSearchNoticed), 4);
        assert_eq!(e.get(entry, CombatTarget::fLastNoticedTimeStamp), NOW);
        assert_eq!(
            calls(&e, MANAGER_ADD_TARGET),
            vec![vec![MANAGER, group.addr(), newcomer.addr()]]
        );
    }

    #[test]
    fn remove_target_removes_the_matching_entry() {
        let mut e = array_engine();
        let (a, b, c) = (actor(&mut e, 1), actor(&mut e, 2), actor(&mut e, 3));
        let group = group_with(&mut e, &[a, b, c], &[]);
        let stranger = actor(&mut e, 9);
        combat_group_remove_target(&mut e, group, stranger);
        assert_eq!(array_len(&e, group, 8), 3);
        combat_group_remove_target(&mut e, group, b);
        assert_eq!(array_len(&e, group, 8), 2);
        assert_eq!(e.get(target_entry(&e, group, 0), CombatTarget::pActor), a);
        assert_eq!(e.get(target_entry(&e, group, 1), CombatTarget::pActor), c);
        assert_eq!(
            calls(&e, MANAGER_REMOVE_TARGET),
            vec![vec![MANAGER, group.addr(), b.addr()]]
        );
        assert_eq!(
            calls(&e, TARGET_ARRAY_REMOVE_AT),
            vec![vec![group.addr() + 8, 1, 1]]
        );
    }

    #[test]
    fn find_helpers_match_by_form_id() {
        let mut e = array_engine();
        let (t, m) = (actor(&mut e, 10), actor(&mut e, 20));
        let group = group_with(&mut e, &[t], &[m]);
        let same_id_as_target = actor(&mut e, 10);
        let same_id_as_member = actor(&mut e, 20);
        let stranger = actor(&mut e, 30);
        let entry = target_entry(&e, group, 0);
        let member_entry: Ptr = Ptr::new(e.mem.u32(group.addr() + 0x1c));
        assert!(combat_group_is_target(&mut e, group, t));
        assert!(combat_group_is_target(&mut e, group, same_id_as_target));
        assert!(!combat_group_is_target(&mut e, group, m));
        assert!(!combat_group_is_target(&mut e, group, stranger));
        assert!(combat_group_is_member(&mut e, group, same_id_as_member));
        assert!(!combat_group_is_member(&mut e, group, t));
        assert_eq!(fn_009865d0(&mut e, group, t), entry);
        assert!(fn_009865d0(&mut e, group, stranger).is_null());
        assert_eq!(fn_009865f0(&mut e, group, m), member_entry.cast());
        assert!(fn_009865f0(&mut e, group, stranger).is_null());
        assert_eq!(fn_00986610(&mut e, group, 10), entry);
        assert!(fn_00986610(&mut e, group, 20).is_null());
        assert_eq!(fn_00986670(&mut e, group, 20), member_entry.cast());
        assert!(fn_00986670(&mut e, group, 10).is_null());
    }

    #[test]
    fn remove_target_at_tells_the_manager_then_removes() {
        let mut e = array_engine();
        let (a, b) = (actor(&mut e, 1), actor(&mut e, 2));
        let group = group_with(&mut e, &[a, b], &[]);
        fn_00986560(&mut e, group, 0);
        assert_eq!(array_len(&e, group, 8), 1);
        assert_eq!(e.get(target_entry(&e, group, 0), CombatTarget::pActor), b);
        let order = addresses(&e);
        let manager_at = order
            .iter()
            .position(|a| *a == MANAGER_REMOVE_TARGET)
            .unwrap();
        let remove_at = order
            .iter()
            .position(|a| *a == TARGET_ARRAY_REMOVE_AT)
            .unwrap();
        assert!(manager_at < remove_at);
        assert_eq!(
            calls(&e, MANAGER_REMOVE_TARGET),
            vec![vec![MANAGER, group.addr(), a.addr()]]
        );
    }

    #[test]
    fn can_add_target_asks_every_non_null_member() {
        let mut e = array_engine();
        let (m0, m1) = (actor(&mut e, 1), actor(&mut e, 2));
        let newcomer = actor(&mut e, 3);
        let group = group_with(&mut e, &[], &[m0, Ptr::NULL, m1]);
        assert!(combat_group_can_add_target(&mut e, group, newcomer));
        assert_eq!(
            calls(&e, ACTOR_COMBAT_COMPATIBLE),
            vec![
                vec![m0.addr(), newcomer.addr()],
                vec![m1.addr(), newcomer.addr()]
            ]
        );
        e.mem.set_u8(m1.addr() + 0x3c, 1);
        assert!(!combat_group_can_add_target(&mut e, group, newcomer));
        let empty = group_with(&mut e, &[], &[]);
        assert!(combat_group_can_add_target(&mut e, empty, newcomer));
    }

    #[test]
    fn can_add_member_asks_the_candidate_about_every_target() {
        let mut e = array_engine();
        let (t0, t1) = (actor(&mut e, 1), actor(&mut e, 2));
        let candidate = actor(&mut e, 3);
        let group = group_with(&mut e, &[t0, t1], &[]);
        assert!(combat_group_can_add_member(&mut e, group, candidate));
        assert_eq!(
            calls(&e, ACTOR_COMBAT_COMPATIBLE),
            vec![
                vec![candidate.addr(), t0.addr()],
                vec![candidate.addr(), t1.addr()]
            ]
        );
        e.mem.set_u8(candidate.addr() + 0x3c, 1);
        assert!(!combat_group_can_add_member(&mut e, group, candidate));
        let empty = group_with(&mut e, &[], &[]);
        assert!(combat_group_can_add_member(&mut e, empty, candidate));
    }

    #[test]
    fn target_array_assignment_and_pointer_lists() {
        let mut e = array_engine();
        let (a, b) = (actor(&mut e, 1), actor(&mut e, 2));
        let group = group_with(&mut e, &[a, b], &[b]);
        let other = group_with(&mut e, &[], &[]);
        record(&mut e, &[0x0098_f580, POINTER_ARRAY_SET_SIZE, ARRAY_SET_AT]);
        fn_00986740(&mut e, group, other);
        assert_eq!(
            calls(&e, 0x0098_f580),
            vec![vec![other.addr() + 8, group.addr() + 8]]
        );
        let list = 0x5000_0000;
        fn_00986760(&mut e, group, Ptr::new(list));
        assert_eq!(calls(&e, POINTER_ARRAY_SET_SIZE), vec![vec![list, 2, 1]]);
        let first = target_entry(&e, group, 0).addr();
        assert_eq!(
            calls(&e, ARRAY_SET_AT),
            vec![vec![list, 0, first], vec![list, 1, first + 0x68]]
        );
        fn_00986b00(&mut e, group, Ptr::new(list));
        assert_eq!(calls(&e, POINTER_ARRAY_SET_SIZE)[1], vec![list, 1, 1]);
        let member = e.mem.u32(group.addr() + 0x1c);
        assert_eq!(calls(&e, ARRAY_SET_AT)[2], vec![list, 0, member]);
    }

    #[test]
    fn member_constructor_defaults() {
        let mut e = engine();
        let member = e.new_object::<CombatMember>();
        e.mem.write(member.addr(), &[0xaa; 0x14]);
        let who = actor(&mut e, 1);
        assert_eq!(fn_00986950(&mut e, member, who), member);
        assert_eq!(e.get(member, CombatMember::pActor), who);
        assert_eq!(e.get(member, CombatMember::iGroupStrategyAssignment), 0);
        assert_eq!(e.get(member, CombatMember::fDamagePerSecond), -1.0);
        assert_eq!(e.get(member, CombatMember::fCombatStrength), -1.0);
        assert!(e.get(member, CombatMember::pCluster).is_null());
    }

    #[test]
    fn add_member_refusals_and_tinting() {
        let mut e = array_engine();
        let target = actor(&mut e, 1);
        let existing = actor(&mut e, 2);
        let newcomer = actor(&mut e, 3);
        let group = group_with(&mut e, &[target], &[existing]);
        e.set(group, CombatGroup::iGroupNum, 6);
        assert!(!combat_group_add_member(&mut e, group, Ptr::NULL));
        assert!(combat_group_add_member(&mut e, group, existing));
        // A target of the group: refused with a log line.
        assert!(!combat_group_add_member(&mut e, group, target));
        assert_eq!(calls(&e, LOG_MESSAGE), vec![vec![0x0108_d600, 1, 0xcafe]]);
        // Incompatible with a target: refused.
        e.mem.set_u8(newcomer.addr() + 0x3c, 1);
        assert!(!combat_group_add_member(&mut e, group, newcomer));
        e.mem.set_u8(newcomer.addr() + 0x3c, 0);

        // Appended; the tint setting is off.
        record(&mut e, &[TINT_SCENEGRAPH]);
        assert!(combat_group_add_member(&mut e, group, newcomer));
        assert_eq!(array_len(&e, group, 0x18), 2);
        let entry = Ptr::new(e.mem.u32(group.addr() + 0x1c) + 0x14);
        assert_eq!(e.get(entry, CombatMember::pActor), newcomer);
        assert_eq!(e.get(entry, CombatMember::fCombatStrength), -1.0);
        assert!(calls(&e, TINT_SCENEGRAPH).is_empty());
        assert_eq!(
            calls(&e, MANAGER_ADD_MEMBER),
            vec![vec![MANAGER, group.addr(), newcomer.addr()]]
        );
        assert_eq!(calls(&e, 0x0097_f220), vec![vec![group.addr() + 0xb0]]);

        // With the tint setting on and a 3D root: tinted with the group's
        // colour.
        e.mem.set_u8(TINT_SETTING, 1);
        let fourth = actor(&mut e, 4);
        e.mem.set_u32(fourth.addr() + 0x30, 0x9999);
        let colour = e.mem.alloc(16);
        for (i, v) in [0.25f32, 0.5, 0.75, 1.0].iter().enumerate() {
            e.mem.set_f32(colour + 4 * i as u32, *v);
        }
        e.set_global(0x011f_1b00, colour);
        e.register(0x005d_8550, |e, a| {
            assert_eq!(a[0], 6);
            e.global::<u32>(0x011f_1b00).into_ret()
        });
        e.register(0x0041_6870, |e, a| {
            for i in 0..3 {
                e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
            }
            a[0].into_ret()
        });
        e.register_double(TINT_SCENEGRAPH, |e, a| {
            assert_eq!(a[0], 0x9999);
            assert_eq!(a[2], 0);
            let rgb: Vec<f32> = (0..3).map(|i| e.mem.f32(a[1] + 4 * i)).collect();
            assert_eq!(rgb, vec![0.25, 0.5, 0.75]);
            e.set_global(0x011f_1b04, 1u32);
            Ret::default()
        });
        assert!(combat_group_add_member(&mut e, group, fourth));
        assert_eq!(e.global::<u32>(0x011f_1b04), 1);
        // No 3D root: not tinted.
        let fifth = actor(&mut e, 5);
        e.set_global(0x011f_1b04, 0u32);
        assert!(combat_group_add_member(&mut e, group, fifth));
        assert_eq!(e.global::<u32>(0x011f_1b04), 0);
    }

    #[test]
    fn remove_member_clears_search_state_and_the_tint() {
        let mut e = array_engine();
        let (a, b) = (actor(&mut e, 1), actor(&mut e, 2));
        let group = group_with(&mut e, &[], &[a, b]);
        e.set(group, CombatGroup::pSearchingMember, a);
        e.mem.set_u8(TINT_SETTING, 1);
        e.mem.set_u32(a.addr() + 0x30, 0x7777);
        record(&mut e, &[UNTINT_SCENEGRAPH]);
        // Not a member: nothing happens.
        let stranger = actor(&mut e, 9);
        fn_009869a0(&mut e, group, stranger);
        assert!(calls(&e, MANAGER_REMOVE_MEMBER).is_empty());
        fn_009869a0(&mut e, group, a);
        assert_eq!(array_len(&e, group, 0x18), 1);
        assert_eq!(Ptr::new(e.mem.u32(e.mem.u32(group.addr() + 0x1c))), b);
        assert!(e.get(group, CombatGroup::pSearchingMember).is_null());
        assert_eq!(
            calls(&e, MANAGER_REMOVE_MEMBER),
            vec![vec![MANAGER, group.addr(), a.addr()]]
        );
        assert_eq!(calls(&e, UNTINT_SCENEGRAPH), vec![vec![0x7777]]);
        // The other member without a 3D root, search member untouched.
        e.set(group, CombatGroup::pSearchingMember, Ptr::new(0x1234));
        fn_009869a0(&mut e, group, b);
        assert_eq!(calls(&e, UNTINT_SCENEGRAPH).len(), 1);
        assert_eq!(
            e.get(group, CombatGroup::pSearchingMember),
            Ptr::new(0x1234)
        );
    }

    #[test]
    fn member_loops_call_the_actors_and_the_manager() {
        let mut e = array_engine();
        let (a, b) = (actor(&mut e, 1), actor(&mut e, 2));
        let group = group_with(&mut e, &[], &[a, b]);
        record(&mut e, &[0x0084_54f0]);
        fn_00986b70(&mut e, group);
        assert_eq!(
            calls(&e, fake(0x3fc)),
            vec![vec![a.addr(), 0], vec![b.addr(), 0]]
        );
        fn_00986bd0(&mut e, group);
        assert_eq!(
            calls(&e, MANAGER_REMOVE_MEMBER),
            vec![
                vec![MANAGER, group.addr(), a.addr()],
                vec![MANAGER, group.addr(), b.addr()]
            ]
        );
        assert_eq!(calls(&e, 0x0084_54f0), vec![vec![group.addr() + 0x18, 0]]);
    }

    #[test]
    fn has_no_members_asks_the_member_array() {
        let mut e = engine();
        e.register(0x0076_b610, |e, a| (e.mem.u32(a[0] + 8) == 0).into_ret());
        let a = actor(&mut e, 1);
        let with = group_with(&mut e, &[], &[a]);
        let without = group_with(&mut e, &[], &[]);
        assert!(!fn_00986c40(&mut e, with));
        assert!(fn_00986c40(&mut e, without));
    }

    /// The scene of the target-choice tests. Fake fields (see `engine`):
    /// actors: +0x14 x position, +0x10 space, +0x18 cell, +0x1c melee,
    /// +0x1d force-check flag, +0x1e should-attack, +0x1f fleeing, +0x24 /
    /// +0x28 / +0x2c / +0x2d the level, stamp and two flags the detection
    /// call answers with, +0x40 byte: inside the firing arc. Process: +0
    /// current target, +0x100 byte answering `0097ee90`, +0x200 view angle.
    struct Scene {
        e: Engine,
        group: Ptr<CombatGroup>,
        actor: Ptr,
        targets: Vec<Ptr>,
        process: u32,
    }

    fn scene(target_count: usize) -> Scene {
        let mut e = engine();
        with_manager(&mut e);
        let me = actor(&mut e, 100);
        let targets: Vec<Ptr> = (0..target_count)
            .map(|i| {
                let target = actor(&mut e, 200 + i as u32);
                e.mem.set_u8(target.addr() + 0x1e, 1); // should attack
                e.mem.set_u8(target.addr() + 0x40, 1); // inside the arc
                e.mem.set_i32(target.addr() + 0x24, 1); // detected
                target
            })
            .collect();
        let group = group_with(&mut e, &targets, &[me]);
        let process = e.mem.alloc(0x300);
        e.mem.set_u32(me.addr() + 0x38, process);

        e.register(0x0076_b610, |e, a| (e.mem.u32(a[0] + 8) == 0).into_ret());
        e.register(GET_WORLD_LOCATION, |e, a| {
            let (x, space) = (e.mem.u32(a[0] + 0x14), e.mem.u32(a[0] + 0x10));
            e.mem.set_u32(a[1], x);
            e.mem.set_u32(a[1] + 0x0c, space);
            a[1].into_ret()
        });
        e.register(0x0040_30b0, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(0x0058_6150, |_, a| (a[0] + 0x100).into_ret());
        e.register(0x0098_0070, |_, a| (a[0] + 0x200).into_ret());
        e.register(0x0063_9aa0, |e, a| e.mem.f32(a[0]).into_ret());
        e.register(0x0056_7d90, |e, a| e.mem.u32(a[0] + 0x18).into_ret());
        e.register(0x009a_9630, |e, a| e.mem.u8(a[0] + 0x1c).into_ret());
        e.register(0x0098_71c0, |e, a| {
            (e.mem.u32(a[0] + 0x0c) + a[1] * 0x68).into_ret()
        });
        e.register(0x009a_6d70, |e, a| (e.mem.u8(a[1]) != 0).into_ret());
        e.register(0x0056_6950, |e, a| e.mem.u8(a[0] + 0x1d).into_ret());
        e.register(ACTOR_GET_SHOULD_ATTACK, |e, a| {
            e.mem.u8(a[1] + 0x1e).into_ret()
        });
        e.register(0x0098_87b0, |e, a| {
            let target = a[2];
            let level = e.mem.i32(target + 0x24);
            e.mem.set_i32(a[3], level);
            let stamp = e.mem.u32(target + 0x28);
            e.mem.set_u32(a[4], stamp);
            let (first, second) = (e.mem.u8(target + 0x2c), e.mem.u8(target + 0x2d));
            e.mem.set_u8(a[5], first);
            e.mem.set_u8(a[6], second);
            Ret::default()
        });
        e.register(0x0097_ee90, |e, a| e.mem.u8(a[0]).into_ret());
        e.register(0x0098_71e0, |_, _| Ret::default());
        e.register(0x009a_0f40, |_, _| Ret::default());
        e.register(0x009a_96f0, |_, _| 1u32.into_ret());
        e.register(DISTANCE_SQUARED, |e, a| {
            (e.mem.f32(a[1]) - e.mem.f32(a[0])).powi(2).into_ret()
        });
        e.register(0x0097_fb00, |e, a| {
            e.mem.set_f32(a[1], NOW - 1.0);
            a[1].into_ret()
        });
        e.register(ACTOR_IS_FLEEING, |e, a| e.mem.u8(a[0] + 0x1f).into_ret());
        e.register(MANAGER_REMOVE_MEMBER, |_, _| Ret::default());
        e.register(MEMBER_ARRAY_REMOVE_AT, |e, a| {
            e.mem.set_u32(a[0] + 8, 0);
            Ret::default()
        });
        for (address, value) in [
            (0x011a_4d78u32, 100.0f32),
            (0x011a_4d74, 10.0),
            (0x011a_4d70, 1000.0),
            (0x011a_4d6c, 5.0),
            (0x011a_4d68, 1.0),
            (0x011a_4d64, 10.0),
            (0x011a_4d60, 50.0),
            (0x011a_4d5c, 7.0),
            (0x011a_4d58, 5.0),
            (0x011a_4d54, 3.0),
            (0x011a_4d50, 2.0),
            (0x011f_18b0, 0.5),
            (0x0108_d660, 4194304.0),
            (0x0101_ff50, std::f32::consts::TAU),
            (0x0103_2adc, 256.0),
        ] {
            e.set_global(address, value);
        }
        e.set_global(0x0108_d658, 4194304.0f64);
        e.set_global(0x0102_3128, std::f64::consts::PI / 180.0);
        e.set_global(0x0101_db88, 30.0f64);
        e.call_log = Some(vec![]);
        Scene {
            e,
            group,
            actor: me,
            targets,
            process,
        }
    }

    fn choose(s: &mut Scene) -> Ptr {
        fn_00986c60(&mut s.e, s.group, s.actor)
    }

    #[test]
    fn choose_target_without_targets_or_process() {
        let mut s = scene(0);
        assert!(choose(&mut s).is_null());
        assert!(calls(&s.e, fake(0x428)).is_empty());

        // An actor without a process leaves the group.
        let mut s = scene(2);
        s.e.mem.set_u32(s.actor.addr() + 0x38, 0);
        assert!(choose(&mut s).is_null());
        assert_eq!(
            calls(&s.e, MANAGER_REMOVE_MEMBER),
            vec![vec![MANAGER, s.group.addr(), s.actor.addr()]]
        );
        assert_eq!(
            calls(&s.e, MEMBER_ARRAY_REMOVE_AT),
            vec![vec![s.group.addr() + 0x18, 0, 1]]
        );
    }

    #[test]
    fn choose_target_scores_flags_and_distance() {
        // A single target scores 0 and wins over the starting -FLT_MAX.
        let mut s = scene(1);
        assert_eq!(choose(&mut s), s.targets[0]);

        // Two targets, level above zero: the second has the first flag
        // (+10). Same space, same position: 50 for distance, 1 for the same
        // weapon style.
        let mut s = scene(2);
        s.e.mem.set_u8(s.targets[1].addr() + 0x2c, 1);
        assert_eq!(choose(&mut s), s.targets[1]);
        // The second flag (+100) of the first outweighs it.
        s.e.mem.set_u8(s.targets[0].addr() + 0x2d, 1);
        assert_eq!(choose(&mut s), s.targets[0]);

        // Distance: 1024 units away scores 50 * (1 - 1048576 / 4194304).
        let mut s = scene(2);
        s.e.mem
            .set_u32(s.targets[0].addr() + 0x14, 1024.0f32.to_bits());
        assert_eq!(choose(&mut s), s.targets[1]);
        // Both far, the nearer wins.
        s.e.mem
            .set_u32(s.targets[1].addr() + 0x14, 2048.0f32.to_bits());
        assert_eq!(choose(&mut s), s.targets[0]);

        // Same weapon style as the actor adds 1 to the other target only.
        let mut s = scene(2);
        s.e.mem.set_u8(s.targets[0].addr() + 0x1c, 1);
        assert_eq!(choose(&mut s), s.targets[1]);
    }

    #[test]
    fn choose_target_current_target_bonus_and_penalties() {
        // The process' current target gets 1000, plus 100 while its
        // recency (one second) is inside the window of 5.
        let mut s = scene(2);
        s.e.mem.set_u32(s.process, s.targets[1].addr());
        assert_eq!(choose(&mut s), s.targets[1]);
        // A fleeing current target loses 2: still ahead.
        s.e.mem.set_u8(s.targets[1].addr() + 0x1f, 1);
        assert_eq!(choose(&mut s), s.targets[1]);
        // An unfit one (virtual method 0x22c) loses 3, a target in another
        // space that the actor cannot exist in loses the same again; with a
        // current-target bonus of 1000 it still wins, without it it loses
        // to the other target.
        let mut s = scene(2);
        s.e.mem.set_u8(s.targets[0].addr() + 0x34, 1);
        assert_eq!(choose(&mut s), s.targets[1]);
        let mut s = scene(2);
        s.e.mem.set_u8(s.targets[0].addr() + 0x35, 1);
        assert_eq!(choose(&mut s), s.targets[1]);
        let mut s = scene(2);
        s.e.mem.set_u8(s.targets[0].addr() + 0x1f, 1);
        assert_eq!(choose(&mut s), s.targets[1]);
        // More attackers than the current-target flag: +0.5.
        let mut s = scene(2);
        let entry = target_entry(&s.e, s.group, 0);
        s.e.set(entry, CombatTarget::sAttackerCount, 1);
        assert_eq!(choose(&mut s), s.targets[0]);
    }

    #[test]
    fn choose_target_other_space_uses_the_far_distance() {
        // The actor is in space 1, the first target in space 2 (squared
        // far distance 100, term 50 * (1 - 100 / 4194304)); the second in
        // the actor's space but 1024 away (37.5): the first wins. The
        // actor cannot exist there (-5) and the location is unreachable
        // (-7): the second wins.
        let mut s = scene(2);
        s.e.mem.set_u32(s.actor.addr() + 0x10, 1);
        s.e.mem.set_u32(s.targets[0].addr() + 0x10, 2);
        s.e.mem.set_u32(s.targets[1].addr() + 0x10, 1);
        s.e.mem
            .set_u32(s.targets[1].addr() + 0x14, 1024.0f32.to_bits());
        assert_eq!(choose(&mut s), s.targets[0]);
        s.e.register(0x009a_96f0, |_, _| Ret::default());
        s.e.set_global(0x011a_4d58, 20.0f32);
        assert_eq!(choose(&mut s), s.targets[1]);
        s.e.register(0x009a_96f0, |_, _| 1u32.into_ret());
        s.e.register(0x009a_0f40, |_, _| 1u32.into_ret());
        s.e.set_global(0x011a_4d5c, 20.0f32);
        assert_eq!(choose(&mut s), s.targets[1]);
    }

    #[test]
    fn choose_target_skips_and_fallbacks() {
        // Outside the firing arc (view angle above zero): skipped.
        let mut s = scene(2);
        s.e.mem.set_f32(s.process + 0x200, 90.0);
        s.e.mem.set_u8(s.targets[0].addr() + 0x40, 0);
        s.e.mem.set_u8(s.targets[1].addr() + 0x2c, 1);
        assert_eq!(choose(&mut s), s.targets[1]);
        let arc = calls(&s.e, 0x009a_6d70);
        assert_eq!(arc.len(), 2);
        assert_eq!(arc[0][0], s.actor.addr());
        assert_eq!(arc[0][1], s.targets[0].addr() + 0x40);
        assert_eq!(f32::from_bits(arc[0][2]), (90.0f64.to_radians()) as f32);
        assert_eq!(f32::from_bits(arc[0][3]), std::f32::consts::TAU);

        // Another cell with the same-cell setting on: skipped; every
        // target skipped gives null.
        let mut s = scene(2);
        s.e.mem.set_u8(SAME_CELL_SETTING, 1);
        s.e.mem.set_u32(s.actor.addr() + 0x18, 5);
        s.e.mem.set_u32(s.targets[0].addr() + 0x18, 6);
        s.e.mem.set_u32(s.targets[1].addr() + 0x18, 5);
        assert_eq!(choose(&mut s), s.targets[1]);
        s.e.mem.set_u32(s.targets[1].addr() + 0x18, 6);
        assert!(choose(&mut s).is_null());

        // The force-check flag makes the attack test decide.
        let mut s = scene(2);
        s.e.mem.set_u8(s.actor.addr() + 0x1d, 1);
        s.e.mem.set_u8(s.targets[0].addr() + 0x1e, 0);
        s.e.mem.set_u8(s.targets[0].addr() + 0x2c, 1);
        assert_eq!(choose(&mut s), s.targets[1]);
        assert_eq!(calls(&s.e, ACTOR_GET_SHOULD_ATTACK).len(), 2);

        // Level zero or less: ranked by the last noticed stamp, 30 seconds
        // in the past (the newest stamp wins).
        let mut s = scene(2);
        for (i, noticed) in [50.0f32, 70.0].into_iter().enumerate() {
            s.e.mem.set_i32(s.targets[i].addr() + 0x24, 0);
            let entry = target_entry(&s.e, s.group, i as u32);
            s.e.set(entry, CombatTarget::fLastNoticedTimeStamp, noticed);
        }
        assert_eq!(choose(&mut s), s.targets[1]);
        let entry = target_entry(&s.e, s.group, 1);
        s.e.set(entry, CombatTarget::fLastNoticedTimeStamp, 40.0);
        assert_eq!(choose(&mut s), s.targets[0]);
        // Stamps long before the detection call's own (0) are lifted to it
        // by the maximum: the first target stays the best.
        for i in 0..2 {
            let entry = target_entry(&s.e, s.group, i);
            s.e.set(entry, CombatTarget::fLastNoticedTimeStamp, -1000.0);
        }
        assert_eq!(choose(&mut s), s.targets[0]);
    }

    #[test]
    fn choose_target_negative_level_for_the_current_target_may_become_one() {
        let mut s = scene(2);
        s.e.mem.set_i32(s.targets[0].addr() + 0x24, -1);
        s.e.mem.set_u32(s.process, s.targets[0].addr());
        // The process says it may not be forced: level stays negative and
        // the target is only a fallback; the other one is scored.
        assert_eq!(choose(&mut s), s.targets[1]);
        // Forced to 1: it scores with the current-target bonus.
        s.e.mem.set_u8(s.process + 0x100, 1);
        assert_eq!(choose(&mut s), s.targets[0]);
        assert_eq!(calls(&s.e, 0x0097_ee90).len(), 2);
    }
}
