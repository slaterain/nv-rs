//! `fallout/ai/playercharacter.cpp` (Xbox PDB source unit), subsystem `fallout/ai`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The PC `PlayerCharacter` is 0xE50 bytes. Like every class derived from
//! `TESForm` its fields sit 0x10 lower than in the Xbox PDB (`TESForm` is
//! 0x18 on PC), which holds for all of `PlayerCharacter`'s own fields up to
//! the end of the class; the PC build adds one word at +0xE4C. The layout
//! below lists the fields the translations use, with the PDB names at the PC
//! offsets.
//!
//! Several functions of this unit are the `MagicCaster` / `MagicTarget`
//! overrides of the player: the game calls them with the address of that base
//! subobject (`PlayerCharacter + 0x88` / `+ 0x94`) as `this`, so their
//! offsets are relative to it ([`MagicCasterBase`], [`MagicTargetBase`]).
//!
//! Not translated: the compiler's exception-unwinding state writes of the
//! constructor and destructor (SEH frames).

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::{BSSimpleArray, BSSimpleList, NiPoint3};
use crate::units::fallout_ai::actor::Actor;

/// `operator new` (cdecl, size).
pub(crate) const OPERATOR_NEW: u32 = 0x0040_1000;
/// `operator delete` (cdecl, block).
pub(crate) const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `_ftol2_sse` (`00ec62c0`): truncates the float in ST0 to an integer in
/// EAX; the uniform form passes it as a leading `f64` argument.
pub(crate) const FTOL: u32 = 0x00ec_62c0;
/// `_eh_vector_constructor_iterator_` (stdcall: array, element size, count,
/// constructor, destructor).
const VECTOR_CONSTRUCTOR_ITERATOR: u32 = 0x00ec_782f;
/// `_eh_vector_destructor_iterator_` (stdcall: array, element size, count,
/// destructor).
const VECTOR_DESTRUCTOR_ITERATOR: u32 = 0x00ec_5fce;
/// `memset` (cdecl: destination, byte, count).
const MEMSET: u32 = 0x0040_3d30;

/// `ProcessLists` instance (the `this` of `0096d470`).
const PROCESS_LISTS: u32 = 0x011e_0e80;
/// `CombatManager` singleton pointer (`CombatManager::CreateCombatGroup`'s `this`).
const COMBAT_MANAGER: u32 = 0x011f_1958;
/// A global pointer whose target is the `this` of `0042ce10`.
const GLOBAL_OBJECT_0042CE10: u32 = 0x011d_df38;
/// `0.0` as a `double` in the exe's constants.
const ZERO_DOUBLE: u32 = 0x0101_2060;
/// A `float` constant the constructor stores into the steal-warning timers
/// and other timers.
const TIMER_RESET: u32 = 0x0101_2054;
/// A `float` constant stored into `fBlockActivateTimer`.
const BLOCK_ACTIVATE_TIMER_RESET: u32 = 0x0101_712c;
/// A `float` constant stored into `fEyeHeight`.
const EYE_HEIGHT_DEFAULT: u32 = 0x0108_a8dc;
/// The three words (`NiPoint3`, zero in the exe's data) copied into every
/// vector member the constructor initializes.
const ZERO_VECTOR: u32 = 0x011f_426c;
/// Table the achievement entries read a flag byte from (stride 0x28).
const ACHIEVEMENT_TABLE: u32 = 0x011a_3ad4;

/// The getter (`00403e20`, returns a pointer to its `float` value) of the
/// game settings the constructor reads.
const SETTING_FLOAT_GETTER: u32 = 0x0040_3e20;
/// The setting getter that returns a pointer to a byte value.
const SETTING_BYTE_GETTER: u32 = 0x0040_8d60;
/// The setting getter that returns a pointer to a word value.
const SETTING_INT_GETTER: u32 = 0x0043_d4d0;

/// `BSSoundHandle` constructor (returns the handle), assignment from another
/// handle, destructor, validity test and stop.
const SOUND_HANDLE_CONSTRUCT: u32 = 0x0041_a250;
const SOUND_HANDLE_ASSIGN: u32 = 0x0041_8900;
const SOUND_HANDLE_DESTRUCT: u32 = 0x0048_3710;
const SOUND_HANDLE_IS_VALID: u32 = 0x00ad_8ce0;
const SOUND_HANDLE_STOP: u32 = 0x00ad_88f0;
/// `BSSimpleList<T>` constructor, destructor, "clear", and the deleting
/// destructor of a heap list (`this`, flags).
const LIST_CONSTRUCT: u32 = 0x0096_a2d0;
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
const LIST_CLEAR: u32 = 0x0047_0470;
const LIST_DELETING_DESTRUCT: u32 = 0x0047_02f0;
/// `BSSimpleList` node accessors: the address of a node's item word; the
/// next node; remove the head node; whether the list is empty.
const LIST_ITEM_ADDRESS: u32 = 0x0068_15c0;
const LIST_NEXT: u32 = 0x0072_6070;
const LIST_REMOVE_HEAD: u32 = 0x0063_f7b0;
const LIST_IS_EMPTY: u32 = 0x0082_56d0;
/// `NiPointer<T>` constructor (with the pointer), destructor, and assignment.
const NI_POINTER_CONSTRUCT: u32 = 0x0063_3c90;
const NI_POINTER_DESTRUCT: u32 = 0x0045_cec0;
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;

layout! {
    /// `NiTMapItem<unsigned int, unsigned char>` (Xbox PDB), 0xC bytes.
    pub struct NiTMapItem: 0x0C {
        /// `m_pkNext` (Xbox PDB).
        0x00 m_pkNext: Ptr,
        /// `m_key` (Xbox PDB).
        0x04 m_key: u32,
        /// `m_val` (Xbox PDB).
        0x08 m_val: u8,
    }

    /// `PlayerCharacter` (Xbox PDB), 0xE50 bytes on PC. Only the fields the
    /// translations use are listed; each is at the PDB offset minus 0x10.
    /// Members that are objects embedded by value are `Inline<..>`: their
    /// address is what the code uses.
    pub struct PlayerCharacter: 0xE50 {
        /// `PlayerAchievements` (Xbox PDB): three `AchievementInstance` of
        /// 0xC bytes (a flag byte, a word, a flag byte).
        0x1C8 PlayerAchievements: Inline<()>,
        /// `pQueuedTargetLoc` (Xbox PDB).
        0x1EC pQueuedTargetLoc: Ptr,
        /// `pQueuedWeaponAttach` (Xbox PDB).
        0x1F0 pQueuedWeaponAttach: Ptr,
        /// `fTimeSinceLastAmmoRegenTick` (Xbox PDB).
        0x1F4 fTimeSinceLastAmmoRegenTick: f32,
        /// `cShotsSinceLastAmmoRegen` (Xbox PDB).
        0x1F8 cShotsSinceLastAmmoRegen: u8,
        /// `iSandmanDetection` (Xbox PDB).
        0x1FC iSandmanDetection: u32,
        /// `iCombatPersue` (Xbox PDB).
        0x200 iCombatPersue: u32,
        /// `bTravelUseDoor` (Xbox PDB).
        0x204 bTravelUseDoor: bool,
        /// `bOnElevator` (Xbox PDB).
        0x205 bOnElevator: bool,
        /// `pClosestConversation` (Xbox PDB).
        0x208 pClosestConversation: Ptr,
        /// `btransporting` (Xbox PDB).
        0x20C btransporting: bool,
        /// `pActiveEffectList` (Xbox PDB): `BSSimpleList<ActiveEffect *> *`.
        0x210 pActiveEffectList: Ptr,
        /// `pCurrentSpell` (Xbox PDB).
        0x214 pCurrentSpell: Ptr,
        /// `pDesiredTarget` (Xbox PDB).
        0x218 pDesiredTarget: Ptr,
        /// `pCameraCaster` (Xbox PDB): `CameraCaster *`.
        0x21C pCameraCaster: Ptr,
        /// `pAIConversationRunning` (Xbox PDB).
        0x224 pAIConversationRunning: Ptr,
        /// `iNumberofStealWarnings` (Xbox PDB).
        0x228 iNumberofStealWarnings: u32,
        /// `fStealWarningTimer` (Xbox PDB).
        0x22C fStealWarningTimer: f32,
        /// `iNumberofPickpocketWarnings` (Xbox PDB).
        0x230 iNumberofPickpocketWarnings: u32,
        /// `fPickPocketWarningTimer` (Xbox PDB).
        0x234 fPickPocketWarningTimer: f32,
        /// `EatDrinkItems` (Xbox PDB): `BSSimpleList<MagicItem *> *`.
        0x238 EatDrinkItems: Ptr,
        /// `QueuedWornEnchantments` (Xbox PDB): `BSSimpleList<TESBoundObject *> *`.
        0x23C QueuedWornEnchantments: Ptr,
        /// `bShowQuestItemsInInventory` (Xbox PDB).
        0x240 bShowQuestItemsInInventory: bool,
        /// `TemporaryActorValueModifiers` (Xbox PDB): 77 floats.
        0x244 TemporaryActorValueModifiers: Inline<()>,
        /// `ScriptActorValueModifiers` (Xbox PDB): 77 floats.
        0x378 ScriptActorValueModifiers: Inline<()>,
        /// `fHealthModifier` (Xbox PDB).
        0x4AC fHealthModifier: f32,
        /// `DamageActorValueModifiers` (Xbox PDB): 77 floats.
        0x4B0 DamageActorValueModifiers: Inline<()>,
        /// `listNotes` (Xbox PDB): `BSSimpleList<BGSNote *>`.
        0x5E4 listNotes: Inline<BSSimpleList>,
        /// `pIronsightsDOFInstance` (Xbox PDB).
        0x5EC pIronsightsDOFInstance: Ptr,
        /// `pVatsDOFInstance` (Xbox PDB).
        0x5F0 pVatsDOFInstance: Ptr,
        /// `pVatsDRBInstance` (Xbox PDB).
        0x5F4 pVatsDRBInstance: Ptr,
        /// `bHostileDetection` (Xbox PDB).
        0x5F8 bHostileDetection: bool,
        /// `bIsAcousticSpaceTransition` (Xbox PDB).
        0x5F9 bIsAcousticSpaceTransition: bool,
        /// `pListOfTeammates` (Xbox PDB): `BSSimpleList<Actor *>`.
        0x5FC pListOfTeammates: Inline<BSSimpleList>,
        /// `pLastExtDoorActivated` (Xbox PDB).
        0x604 pLastExtDoorActivated: Ptr,
        /// `bSpeaking` (Xbox PDB).
        0x608 bSpeaking: bool,
        /// `pListofActions` (Xbox PDB): `BSSimpleList<PlayerActionObject *> *`.
        0x60C pListofActions: Ptr,
        /// `pListofCasinoData` (Xbox PDB): `BSSimpleList<CasinoData *> *`.
        0x610 pListofCasinoData: Ptr,
        /// `pInactiveListofCaravanCards` (Xbox PDB).
        0x614 pInactiveListofCaravanCards: Ptr,
        /// `pActiveListofCaravanCards` (Xbox PDB).
        0x618 pActiveListofCaravanCards: Ptr,
        /// `iCaravanCapWinnings` (Xbox PDB).
        0x61C iCaravanCapWinnings: u32,
        /// `iCaravanCapLosses` (Xbox PDB).
        0x620 iCaravanCapLosses: u32,
        /// `iCaravanWinnings` (Xbox PDB).
        0x624 iCaravanWinnings: u32,
        /// `iCaravanLosses` (Xbox PDB).
        0x628 iCaravanLosses: u32,
        /// `iCaravanLargestWinning` (Xbox PDB).
        0x62C iCaravanLargestWinning: u32,
        /// `iCasinoCheatLevel` (Xbox PDB).
        0x630 iCasinoCheatLevel: u32,
        /// `spGrabSpring` (Xbox PDB): an embedded `NiPointer`.
        0x634 spGrabSpring: Inline<()>,
        /// `pGrabbedObject` (Xbox PDB).
        0x638 pGrabbedObject: Ptr,
        /// `eGrabType` (Xbox PDB).
        0x63C eGrabType: u32,
        /// `fGrabObjectWeight` (Xbox PDB).
        0x640 fGrabObjectWeight: f32,
        /// `fGrabDistance` (Xbox PDB).
        0x644 fGrabDistance: f32,
        /// `bsave3rdPerson` (Xbox PDB).
        0x648 bsave3rdPerson: bool,
        /// `b3rdPersonSaved` (Xbox PDB).
        0x649 b3rdPersonSaved: bool,
        /// `b3rdPerson` (Xbox PDB).
        0x64A b3rdPerson: bool,
        /// `bActually3rdPerson` (Xbox PDB).
        0x64B bActually3rdPerson: bool,
        /// `bWant3rdPerson` (Xbox PDB).
        0x64C bWant3rdPerson: bool,
        /// `bTemp3rdPerson` (Xbox PDB).
        0x64D bTemp3rdPerson: bool,
        /// `bTemp3rdPersonSwitchBack` (Xbox PDB).
        0x64E bTemp3rdPersonSwitchBack: bool,
        /// `bTemp1stPerson` (Xbox PDB).
        0x64F bTemp1stPerson: bool,
        /// `bTemp1stPersonSwitchBack` (Xbox PDB).
        0x650 bTemp1stPersonSwitchBack: bool,
        /// `bAlwaysRun` (Xbox PDB).
        0x651 bAlwaysRun: bool,
        /// `bAutoMove` (Xbox PDB).
        0x652 bAutoMove: bool,
        /// `iSleepTime` (Xbox PDB).
        0x654 iSleepTime: u32,
        /// `bIsSleeping` (Xbox PDB).
        0x658 bIsSleeping: bool,
        /// `fsecondRunning` (Xbox PDB).
        0x660 fsecondRunning: f32,
        /// `fsecondSwimming` (Xbox PDB).
        0x664 fsecondSwimming: f32,
        /// `fsecondSneaking` (Xbox PDB).
        0x668 fsecondSneaking: f32,
        /// `bActorinSneakRange` (Xbox PDB).
        0x66C bActorinSneakRange: bool,
        /// `bCanFastTravel` (Xbox PDB).
        0x66D bCanFastTravel: bool,
        /// `bCanWait` (Xbox PDB).
        0x66E bCanWait: bool,
        /// `fWorldFOV` (Xbox PDB).
        0x670 fWorldFOV: f32,
        /// `f1stPersonFOV` (Xbox PDB).
        0x674 f1stPersonFOV: f32,
        /// `f3rdPersonFOV` (Xbox PDB).
        0x678 f3rdPersonFOV: f32,
        /// `iNumberTraining` (Xbox PDB).
        0x67C iNumberTraining: u32,
        /// `ucControlsDisabled` (Xbox PDB).
        0x680 ucControlsDisabled: u8,
        /// `bBlockActivate` (Xbox PDB).
        0x681 bBlockActivate: bool,
        /// `fBlockActivateTimer` (Xbox PDB).
        0x684 fBlockActivateTimer: f32,
        /// `p1stPersonBipedAnim` (Xbox PDB).
        0x68C p1stPersonBipedAnim: Ptr,
        /// `p1stPersonAnimation` (Xbox PDB).
        0x690 p1stPersonAnimation: Ptr,
        /// `sp1stPerson3D` (Xbox PDB): an embedded `NiPointer`.
        0x694 sp1stPerson3D: Inline<()>,
        /// `fEyeHeight` (Xbox PDB).
        0x698 fEyeHeight: f32,
        /// `spInventoryPC` (Xbox PDB): an embedded `NiPointer`.
        0x69C spInventoryPC: Inline<()>,
        /// `pInventoryAnimation` (Xbox PDB).
        0x6A0 pInventoryAnimation: Ptr,
        /// `pInventoryWeaponEffect` (Xbox PDB).
        0x6A4 pInventoryWeaponEffect: Ptr,
        /// `listTopics` (Xbox PDB): `BSSimpleList<TESTopic *>`.
        0x6A8 listTopics: Inline<BSSimpleList>,
        /// `listQuestLog` (Xbox PDB).
        0x6B0 listQuestLog: Inline<BSSimpleList>,
        /// `pActiveQuest` (Xbox PDB).
        0x6B8 pActiveQuest: Ptr,
        /// `listObjectives` (Xbox PDB).
        0x6BC listObjectives: Inline<BSSimpleList>,
        /// `listQuestTargets` (Xbox PDB).
        0x6C4 listQuestTargets: Inline<BSSimpleList>,
        /// `bGreetingPlayer` (Xbox PDB).
        0x6CC bGreetingPlayer: bool,
        /// `ihourstosleep` (Xbox PDB).
        0x6D4 ihourstosleep: u32,
        /// `cMurder` (Xbox PDB).
        0x6D8 cMurder: u8,
        /// `iAmountStolenSold` (Xbox PDB).
        0x6DC iAmountStolenSold: u32,
        /// `fSortActorDistanceTimer` (Xbox PDB).
        0x6E0 fSortActorDistanceTimer: f32,
        /// `fSitHeadingDelta` (Xbox PDB).
        0x6E4 fSitHeadingDelta: f32,
        /// `bBeenAttacked` (Xbox PDB).
        0x6E8 bBeenAttacked: bool,
        /// `pSelectedSpell` (Xbox PDB).
        0x6EC pSelectedSpell: Ptr,
        /// `pSelectedScroll` (Xbox PDB).
        0x6F0 pSelectedScroll: Ptr,
        /// `pPlayerMapMarker` (Xbox PDB).
        0x6F4 pPlayerMapMarker: Ptr,
        /// `PlayerMarkerPath` (Xbox PDB): an embedded `TeleportPath`.
        0x6F8 PlayerMarkerPath: Inline<()>,
        /// `fProjectileReleaseTimer` (Xbox PDB).
        0x730 fProjectileReleaseTimer: f32,
        /// `iNumAdvance` (Xbox PDB).
        0x734 iNumAdvance: u32,
        /// `eskilladvance` (Xbox PDB).
        0x738 eskilladvance: u32,
        /// `pDefaultClass` (Xbox PDB).
        0x73C pDefaultClass: Ptr,
        /// `pClassBasedOn` (Xbox PDB).
        0x740 pClassBasedOn: Ptr,
        /// `pCrimeCounts` (Xbox PDB): five words.
        0x744 pCrimeCounts: Inline<()>,
        /// `pPendingPoison` (Xbox PDB).
        0x758 pPendingPoison: Ptr,
        /// `bChargen` (Xbox PDB).
        0x75C bChargen: bool,
        /// `bAllowEGMCacheClear` (Xbox PDB).
        0x75D bAllowEGMCacheClear: bool,
        /// `bTelekinesisSelected` (Xbox PDB).
        0x75E bTelekinesisSelected: bool,
        /// `pOccupiedRegion` (Xbox PDB).
        0x760 pOccupiedRegion: Ptr,
        /// `m_AllOccupiedRegions` (Xbox PDB): an embedded `TESRegionList`.
        0x764 m_AllOccupiedRegions: Inline<()>,
        /// `CurrentRegionSoundList` (Xbox PDB).
        0x774 CurrentRegionSoundList: Inline<BSSimpleList>,
        /// `StatusSoundHandle` (Xbox PDB): an embedded `BSSoundHandle`.
        0x77C StatusSoundHandle: Inline<()>,
        /// `pInitialStateBuffer` (Xbox PDB).
        0x788 pInitialStateBuffer: Ptr,
        /// `iTotalPlayingTime` (Xbox PDB).
        0x790 iTotalPlayingTime: u32,
        /// `iCharacterSeed` (Xbox PDB).
        0x794 iCharacterSeed: u32,
        /// `bAiControlledToPos` (Xbox PDB).
        0x798 bAiControlledToPos: bool,
        /// `bAiControlledFromPos` (Xbox PDB).
        0x799 bAiControlledFromPos: bool,
        /// `bAiControlledActivate` (Xbox PDB).
        0x79A bAiControlledActivate: bool,
        /// `bAiControlledPackage` (Xbox PDB).
        0x79B bAiControlledPackage: bool,
        /// `bInBorderContainedCell` (Xbox PDB).
        0x79C bInBorderContainedCell: bool,
        /// `bReturnToLastKnownGoodPosition` (Xbox PDB).
        0x79D bReturnToLastKnownGoodPosition: bool,
        /// `LastKnownGoodPosition` (Xbox PDB).
        0x7A0 LastKnownGoodPosition: Inline<NiPoint3>,
        /// `pLastKnownGoodLocation` (Xbox PDB).
        0x7AC pLastKnownGoodLocation: Ptr,
        /// `pBorderRegions` (Xbox PDB).
        0x7B0 pBorderRegions: Ptr,
        /// `pLastKnownMusicType` (Xbox PDB).
        0x7B4 pLastKnownMusicType: Ptr,
        /// `eDifficultyLevel` (Xbox PDB).
        0x7B8 eDifficultyLevel: u32,
        /// `eKillCameraSetting` (Xbox PDB).
        0x7C0 eKillCameraSetting: u32,
        /// `bBeingChased` (Xbox PDB).
        0x7C4 bBeingChased: bool,
        /// `bIsYoung` (Xbox PDB).
        0x7C5 bIsYoung: bool,
        /// `bIsToddler` (Xbox PDB).
        0x7C6 bIsToddler: bool,
        /// `bCanUsePowerArmor` (Xbox PDB).
        0x7C7 bCanUsePowerArmor: bool,
        /// `MapMarkerList` (Xbox PDB).
        0x7C8 MapMarkerList: Inline<BSSimpleList>,
        /// `pMapWorld` (Xbox PDB).
        0x7D0 pMapWorld: Ptr,
        /// `AudioMarkerList` (Xbox PDB).
        0x7D4 AudioMarkerList: Inline<BSSimpleList>,
        /// `pClosestAudioMarkerInfo` (Xbox PDB).
        0x7DC pClosestAudioMarkerInfo: Ptr,
        /// `fUFOCameraHeading` (Xbox PDB).
        0x7E0 fUFOCameraHeading: f32,
        /// `fUFOCameraPitch` (Xbox PDB).
        0x7E4 fUFOCameraPitch: f32,
        /// `UFOCameraPos` (Xbox PDB).
        0x7E8 UFOCameraPos: Inline<NiPoint3>,
        /// `iSelectedSpellCastSoundID` (Xbox PDB).
        0x7F4 iSelectedSpellCastSoundID: u32,
        /// `SelectedSpellCastSound` (Xbox PDB): an embedded `BSSoundHandle`.
        0x7F8 SelectedSpellCastSound: Inline<()>,
        /// `MagicFailureSounds` (Xbox PDB): six `BSSoundHandle` of 0xC bytes.
        0x804 MagicFailureSounds: Inline<()>,
        /// `DroppedRefList` (Xbox PDB): `BSSimpleList<TESObjectREFR *>`.
        0x84C DroppedRefList: Inline<BSSimpleList>,
        /// `RandomDoorSpaceMap` (Xbox PDB): an embedded `NiTMap`.
        0x854 RandomDoorSpaceMap: Inline<()>,
        /// `sp1stPersonLight` (Xbox PDB): an embedded `NiPointer`.
        0x864 sp1stPersonLight: Inline<()>,
        /// `sp3rdPersonLight` (Xbox PDB): an embedded `NiPointer`.
        0x868 sp3rdPersonLight: Inline<()>,
        /// `fDropAngleMod` (Xbox PDB).
        0x870 fDropAngleMod: f32,
        /// `fLastDropAngleMod` (Xbox PDB).
        0x874 fLastDropAngleMod: f32,
        /// `CharacterProgressionInfo` (Xbox PDB): an embedded `CharacterProgression`.
        0x878 CharacterProgressionInfo: Inline<()>,
        /// `Perks` (Xbox PDB): `BSSimpleList<PerkRankData *>`.
        0x87C Perks: Inline<BSSimpleList>,
        /// `PerkEntryLists` (Xbox PDB): 74 lists of 8 bytes.
        0x884 PerkEntryLists: Inline<BSSimpleList>,
        /// `CompanionPerks` (Xbox PDB).
        0xAD4 CompanionPerks: Inline<BSSimpleList>,
        /// `CompanionPerkEntryLists` (Xbox PDB): 74 lists of 8 bytes.
        0xADC CompanionPerkEntryLists: Inline<BSSimpleList>,
        /// `pAutoAimActor` (Xbox PDB).
        0xD2C pAutoAimActor: Ptr,
        /// `BulletAutoAim` (Xbox PDB).
        0xD30 BulletAutoAim: Inline<NiPoint3>,
        /// `spTargeted3D` (Xbox PDB): an embedded `NiPointer`.
        0xD3C spTargeted3D: Inline<()>,
        /// `bTarget3DDistant` (Xbox PDB).
        0xD40 bTarget3DDistant: bool,
        /// `pPlayersTargetActor` (Xbox PDB).
        0xD44 pPlayersTargetActor: Ptr,
        /// `pListofPercievedActors` (Xbox PDB).
        0xD48 pListofPercievedActors: Ptr,
        /// `fMenuModeButtonTimer` (Xbox PDB).
        0xD4C fMenuModeButtonTimer: f32,
        /// `fAmmoSwapButtonTimer` (Xbox PDB).
        0xD50 fAmmoSwapButtonTimer: f32,
        /// `bMenuModeButtonClicked` (Xbox PDB).
        0xD54 bMenuModeButtonClicked: bool,
        /// `kCamera3rdPersonShoulderOffset` (Xbox PDB).
        0xD58 kCamera3rdPersonShoulderOffset: Inline<NiPoint3>,
        /// `pCombatGroup` (Xbox PDB): `CombatGroup *`.
        0xD64 pCombatGroup: Ptr,
        /// `iTeammateCount` (Xbox PDB).
        0xD68 iTeammateCount: u32,
        /// `fCombatTimer` (Xbox PDB).
        0xD6C fCombatTimer: f32,
        /// `fYieldTimer` (Xbox PDB).
        0xD70 fYieldTimer: f32,
        /// `pWobbleNodes` (Xbox PDB): 0x60 bytes of node pointers.
        0xD74 pWobbleNodes: Inline<()>,
        /// `Cached1stPersonCameraPos` (Xbox PDB).
        0xDD4 Cached1stPersonCameraPos: Inline<NiPoint3>,
        /// `CachedWorldCameraPos` (Xbox PDB).
        0xDE0 CachedWorldCameraPos: Inline<NiPoint3>,
        /// `spCameraRigidBody` (Xbox PDB): an embedded `NiPointer`.
        0xDEC spCameraRigidBody: Inline<()>,
        /// `bPlayerInCombat` (Xbox PDB).
        0xDF0 bPlayerInCombat: bool,
        /// `bAllCombatTargetsSearching` (Xbox PDB).
        0xDF1 bAllCombatTargetsSearching: bool,
        /// `RockItLauncherAmmoList` (Xbox PDB): `BSSimpleArray<ItemChange *, 1024>`.
        0xDF4 RockItLauncherAmmoList: Inline<BSSimpleArray>,
        /// `bNightVisionOn` (Xbox PDB).
        0xE08 bNightVisionOn: bool,
        /// `pReputationUpdate` (Xbox PDB).
        0xE0C pReputationUpdate: Ptr,
        /// `fTimeInSlowMoCam` (Xbox PDB).
        0xE18 fTimeInSlowMoCam: f32,
        /// `fKillCamCooldown` (Xbox PDB).
        0xE1C fKillCamCooldown: f32,
        /// `bIgnoresGTM` (Xbox PDB).
        0xE20 bIgnoresGTM: bool,
        /// `bTurboISM` (Xbox PDB).
        0xE21 bTurboISM: bool,
        /// `fLastHelloTime` (Xbox PDB).
        0xE24 fLastHelloTime: f32,
        /// `fCounterAttackTimer` (Xbox PDB).
        0xE28 fCounterAttackTimer: f32,
        /// `bCounterAttackCamera` (Xbox PDB).
        0xE2C bCounterAttackCamera: bool,
        /// `bHasCateyeActive` (Xbox PDB).
        0xE2D bHasCateyeActive: bool,
        /// `bHasSpotterActive` (Xbox PDB).
        0xE2E bHasSpotterActive: bool,
        /// `fCheckForItems` (Xbox PDB).
        0xE30 fCheckForItems: f32,
        /// `bAlwaysHardcore` (Xbox PDB).
        0xE38 bAlwaysHardcore: bool,
        /// `HotKeyLastAmmo` (Xbox PDB): `BSSimpleArray<TESAmmo *, 1024>`.
        0xE3C HotKeyLastAmmo: Inline<BSSimpleArray>,
        /// PC only (not in the Xbox PDB): set to 0 by the constructor.
        0xE4C pcOnlyWord: u32,
    }

    /// `PlayerCharacter` seen through its `MagicCaster` base subobject
    /// (`PlayerCharacter + 0x88`): the offsets are relative to that
    /// subobject.
    pub struct MagicCasterBase: 0x194 {
        /// `PlayerCharacter::pCurrentSpell` (Xbox PDB), at base-relative +0x18C.
        0x18C pCurrentSpell: u32,
        /// `PlayerCharacter::pDesiredTarget` (Xbox PDB), at base-relative +0x190.
        0x190 pDesiredTarget: u32,
    }

    /// `PlayerCharacter` seen through its `MagicTarget` base subobject
    /// (`PlayerCharacter + 0x94`).
    pub struct MagicTargetBase: 0x180 {
        /// `PlayerCharacter::pActiveEffectList` (Xbox PDB), at base-relative +0x17C.
        0x17C pActiveEffectList: Ptr,
    }
}

/// Offset of the word `00939860` stores into its object (the constructor
/// stores 0x3E8 into the object `009306d0` returns).
const WORD_STORED_BY_00939860: u32 = 0x604;
/// Offset of the float `00939880` clears in its 0x1A0-byte object.
const FLOAT_CLEARED_BY_00939880: u32 = 0x198;
/// Offset in the `Actor` base of `pCurrentProcess` (`BaseProcess *`).
const ACTOR_CURRENT_PROCESS: u32 = 0x68;
/// Offset in the `Actor` base of `pContinuousBeamPersistant`.
const ACTOR_CONTINUOUS_BEAM_PERSISTANT: u32 = 0x1A0;
/// Offset of the `MagicCaster` base within `PlayerCharacter`.
const MAGIC_CASTER_BASE: u32 = 0x88;
/// Offset of the `ActorValueOwner` base within `PlayerCharacter`.
const ACTOR_VALUE_OWNER_BASE: u32 = 0xA4;

/// The address of a member of the player.
fn member<T>(this: Ptr<PlayerCharacter>, field: Field<PlayerCharacter, T>) -> u32 {
    this.addr().wrapping_add(field.off)
}

/// Copies the three words of the zero vector into the vector at `dst`.
fn set_zero_vector(e: &mut Engine, dst: u32) {
    for i in 0..3 {
        let word = e.mem.u32(ZERO_VECTOR + 4 * i);
        e.mem.set_u32(dst + 4 * i, word);
    }
}

// Translated from 004a9950 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiTMapBase<DFALL<NiTMapItem<unsigned int, unsigned char>>, unsigned int, unsigned char>::SetValue`
/// (Xbox PDB): stores `key` and `value` into the map item. The map itself
/// (`this`) is not read.
pub fn ni_tmap_base_set_value(
    e: &mut Engine,
    _this: Ptr,
    item: Ptr<NiTMapItem>,
    key: u32,
    value: u8,
) {
    e.set(item, NiTMapItem::m_key, key);
    e.set(item, NiTMapItem::m_val, value);
}

// Translated from 005f73f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The engine map names it `PlayerCharacter::GetPermanentActorValue` (Xbox
/// PDB), but the body is shared by seven vtables (linker folding): it calls
/// the object's virtual at `+0x20` with `actor_value`, passes the float result
/// through `00406ce0` and truncates the result to an integer.
pub fn fn_005f73f0(e: &mut Engine, this: Ptr, actor_value: u32) -> i32 {
    let value = e.vcall(this.addr(), 0x20, &args![actor_value]).f32();
    let converted = e.call(0x0040_6ce0, &args![value]).f64();
    e.call(FTOL, &args![converted]).i32()
}

// Translated from 006bf8f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSSimpleArray<ItemChange *, 1024>::Remove` (Xbox PDB): removes the
/// pointer at `index`, shifting the later ones down. With `shrink` set and
/// `006f3170` agreeing, the array is first moved to a buffer of the size
/// `00869600` asks for (the shifted tail copy keeps the size minus one as its
/// count, as the code does).
pub fn bs_simple_array_item_change_remove(
    e: &mut Engine,
    this: Ptr<BSSimpleArray>,
    index: u32,
    shrink: bool,
) {
    if shrink && e.call(0x006f_3170, &args![this]).bool() {
        let new_reserved = e.call(0x0086_9600, &args![this]).u32();
        let new_buffer = e.vcall(this.addr(), 4, &args![new_reserved]).u32();
        let buffer = e.get(this, BSSimpleArray::pBuffer);
        e.call(0x0042_fb60, &args![this, new_buffer, buffer, index]);
        let buffer = e.get(this, BSSimpleArray::pBuffer);
        e.call(
            0x0072_ba80,
            &args![this, buffer.wrapping_add(index.wrapping_mul(4)), 1u32],
        );
        let size = e.get(this, BSSimpleArray::iSize);
        let buffer = e.get(this, BSSimpleArray::pBuffer);
        e.call(
            0x0042_fb60,
            &args![
                this,
                new_buffer.wrapping_add(index.wrapping_mul(4)),
                buffer.wrapping_add(4).wrapping_add(index.wrapping_mul(4)),
                size.wrapping_sub(1)
            ],
        );
        e.call(0x006a_8500, &args![this]);
        e.set(this, BSSimpleArray::pBuffer, new_buffer);
        e.set(this, BSSimpleArray::iReservedSize, new_reserved);
    } else {
        let buffer = e.get(this, BSSimpleArray::pBuffer);
        e.call(
            0x0072_ba80,
            &args![this, buffer.wrapping_add(index.wrapping_mul(4)), 1u32],
        );
        let size = e.get(this, BSSimpleArray::iSize);
        let buffer = e.get(this, BSSimpleArray::pBuffer);
        e.call(
            0x0042_fb60,
            &args![
                this,
                buffer.wrapping_add(index.wrapping_mul(4)),
                buffer.wrapping_add(4).wrapping_add(index.wrapping_mul(4)),
                size.wrapping_sub(index).wrapping_sub(1)
            ],
        );
    }
    let size = e.get(this, BSSimpleArray::iSize);
    e.set(this, BSSimpleArray::iSize, size.wrapping_sub(1));
}

// Translated from 00815340 (decompiled, FalloutNV.exe 1.4.0.525)
/// The engine map names it `PlayerCharacter::StartAim` (Xbox PDB); the body
/// only calls [`fn_00815360`].
pub fn fn_00815340(e: &mut Engine, this: Ptr) {
    fn_00815360(e, this);
}

// Translated from 00815360 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the object's virtual at `+0x20` (no arguments).
pub fn fn_00815360(e: &mut Engine, this: Ptr) {
    e.vcall(this.addr(), 0x20, &args![]);
}

// Translated from 00939680 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MagicCaster` base override: calls the object's virtual at `+0x20` (no
/// arguments).
pub fn fn_00939680(e: &mut Engine, this: Ptr) {
    e.vcall(this.addr(), 0x20, &args![]);
}

// Translated from 009396a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter` virtual `+0x42C`: returns `pPlayersTargetActor`.
pub fn fn_009396a0(e: &mut Engine, this: Ptr<PlayerCharacter>) -> Ptr {
    e.get(this, PlayerCharacter::pPlayersTargetActor)
}

// Translated from 009396c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter` virtual `+0x4E4`: takes seven words and returns false.
#[allow(clippy::too_many_arguments)]
pub fn fn_009396c0(
    _e: &mut Engine,
    _this: Ptr,
    _unused_1: u32,
    _unused_2: u32,
    _unused_3: u32,
    _unused_4: u32,
    _unused_5: u32,
    _unused_6: u32,
    _unused_7: u32,
) -> bool {
    false
}

// Translated from 009396d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter` virtual `+0x3F8`: returns `pCombatGroup`.
pub fn fn_009396d0(e: &mut Engine, this: Ptr<PlayerCharacter>) -> Ptr {
    e.get(this, PlayerCharacter::pCombatGroup)
}

// Translated from 009396f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter` virtual `+0x3FC`: stores `pCombatGroup`.
pub fn fn_009396f0(e: &mut Engine, this: Ptr<PlayerCharacter>, group: Ptr) {
    e.set(this, PlayerCharacter::pCombatGroup, group);
}

// Translated from 00939710 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MagicTarget` base override (`this` is the player plus 0x94): returns the
/// player's `pActiveEffectList`.
pub fn fn_00939710(e: &mut Engine, this: Ptr<MagicTargetBase>) -> Ptr {
    e.get(this, MagicTargetBase::pActiveEffectList)
}

// Translated from 00939730 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter` virtual `+0x328`: stores `iNumAdvance`.
pub fn fn_00939730(e: &mut Engine, this: Ptr<PlayerCharacter>, value: u32) {
    e.set(this, PlayerCharacter::iNumAdvance, value);
}

// Translated from 00939750 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter` virtual `+0x32C`: returns `iNumAdvance`.
pub fn fn_00939750(e: &mut Engine, this: Ptr<PlayerCharacter>) -> u32 {
    e.get(this, PlayerCharacter::iNumAdvance)
}

// Translated from 00939770 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter` virtual `+0x330`: stores `eskilladvance`.
pub fn fn_00939770(e: &mut Engine, this: Ptr<PlayerCharacter>, value: u32) {
    e.set(this, PlayerCharacter::eskilladvance, value);
}

// Translated from 00939790 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter` virtual `+0x334`: returns `eskilladvance`.
pub fn fn_00939790(e: &mut Engine, this: Ptr<PlayerCharacter>) -> u32 {
    e.get(this, PlayerCharacter::eskilladvance)
}

// Translated from 009397b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter` virtual `+0x4E8`: the address of the companion perk list
/// (`CompanionPerks`) when `companion` is non-zero, else of the player's own
/// (`Perks`).
pub fn fn_009397b0(e: &mut Engine, this: Ptr<PlayerCharacter>, companion: u8) -> Ptr {
    let _ = e;
    if companion == 0 {
        Ptr::new(member(this, PlayerCharacter::Perks))
    } else {
        Ptr::new(member(this, PlayerCharacter::CompanionPerks))
    }
}

// Translated from 009397e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter` virtual `+0x4B4`: calls `005a03f0` on the player with 1.
pub fn fn_009397e0(e: &mut Engine, this: Ptr) {
    e.call(0x005a_03f0, &args![this, 1u32]);
}

// Translated from 00939800 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MagicCaster` base override (`this` is the player plus 0x88): stores the
/// player's `pCurrentSpell`.
pub fn fn_00939800(e: &mut Engine, this: Ptr<MagicCasterBase>, spell: u32) {
    e.set(this, MagicCasterBase::pCurrentSpell, spell);
}

// Translated from 00939820 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MagicCaster` base override: returns the player's `pDesiredTarget`.
pub fn fn_00939820(e: &mut Engine, this: Ptr<MagicCasterBase>) -> u32 {
    e.get(this, MagicCasterBase::pDesiredTarget)
}

// Translated from 00939840 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MagicCaster` base override: stores the player's `pDesiredTarget`.
pub fn fn_00939840(e: &mut Engine, this: Ptr<MagicCasterBase>, target: u32) {
    e.set(this, MagicCasterBase::pDesiredTarget, target);
}

// Translated from 00939860 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` into the word at `+0x604` of its object (the constructor
/// calls it with 0x3E8 on the object `009306d0` returns).
pub fn fn_00939860(e: &mut Engine, this: Ptr, value: u32) {
    e.mem.set_u32(this.addr() + WORD_STORED_BY_00939860, value);
}

// Translated from 00939880 (decompiled, FalloutNV.exe 1.4.0.525)
/// Initializer of a 0x1A0-byte object (the constructor allocates two): clears
/// the float at `+0x198` and returns the object.
pub fn fn_00939880(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_f32(this.addr() + FLOAT_CLEARED_BY_00939880, 0.0);
    this
}

// Translated from 009398a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter` vector deleting destructor (`__vecDelDtor`, vtable slot
/// `0x10`): runs the destructor body [`fn_009398d0`], then frees the object
/// when bit 0 of `flags` is set.
pub fn player_character_vector_deleting_destructor(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    flags: u32,
) -> Ptr<PlayerCharacter> {
    fn_009398d0(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0093a5a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CameraCaster` vector deleting destructor: runs `CameraCaster::~CameraCaster`
/// (Xbox PDB, `006208d0`), then frees the object when bit 0 of `flags` is set.
pub fn camera_caster_vector_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x0062_08d0, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0093a5d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns `bAiControlledToPos`.
pub fn fn_0093a5d0(e: &mut Engine, this: Ptr<PlayerCharacter>) -> bool {
    e.get(this, PlayerCharacter::bAiControlledToPos)
}

// Translated from 0093a5f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `bAiControlledToPos`, then calls the virtual at `+0x258` with 0 when
/// any of the four AI-control flags is set ([`fn_0093a740`]) and with 1 when
/// none is.
pub fn fn_0093a5f0(e: &mut Engine, this: Ptr<PlayerCharacter>, value: bool) {
    e.set(this, PlayerCharacter::bAiControlledToPos, value);
    let controlled = fn_0093a740(e, this);
    e.vcall(this.addr(), 0x258, &args![!controlled as u32]);
}

// Translated from 0093a640 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns `bAiControlledFromPos`.
pub fn fn_0093a640(e: &mut Engine, this: Ptr<PlayerCharacter>) -> bool {
    e.get(this, PlayerCharacter::bAiControlledFromPos)
}

// Translated from 0093a660 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::RemoveActorFromPlayercombatList` (Xbox PDB): when the
/// player has a combat group, removes `actor` from it (`00986500`).
pub fn player_character_remove_actor_from_playercombat_list(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor: Ptr,
) {
    let group = e.get(this, PlayerCharacter::pCombatGroup);
    if !group.is_null() {
        e.call(0x0098_6500, &args![group, actor]);
    }
}

// Translated from 0093a690 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::AddActorToPlayerCombatList` (Xbox PDB): creates the
/// player's combat group on first use (`CombatManager::CreateCombatGroup`,
/// `00991e80`, and adds the player to it with `009867d0`), then adds `actor` as
/// a target (`00986410`).
pub fn player_character_add_actor_to_player_combat_list(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor: Ptr,
) {
    if e.get(this, PlayerCharacter::pCombatGroup).is_null() {
        let manager = e.global::<u32>(COMBAT_MANAGER);
        let group: Ptr = e.call(0x0099_1e80, &args![manager]).ptr();
        e.set(this, PlayerCharacter::pCombatGroup, group);
        let group = e.get(this, PlayerCharacter::pCombatGroup);
        e.call(0x0098_67d0, &args![group, this]);
    }
    let group = e.get(this, PlayerCharacter::pCombatGroup);
    e.call(0x0098_6410, &args![group, actor]);
}

// Translated from 0093a6f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `bAiControlledFromPos`, then calls the virtual at `+0x258` as
/// [`fn_0093a5f0`] does.
pub fn fn_0093a6f0(e: &mut Engine, this: Ptr<PlayerCharacter>, value: bool) {
    e.set(this, PlayerCharacter::bAiControlledFromPos, value);
    let controlled = fn_0093a740(e, this);
    e.vcall(this.addr(), 0x258, &args![!controlled as u32]);
}

// Translated from 0093a740 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when any of `bAiControlledToPos`, `bAiControlledFromPos`,
/// `bAiControlledActivate` and `bAiControlledPackage` is set.
pub fn fn_0093a740(e: &mut Engine, this: Ptr<PlayerCharacter>) -> bool {
    e.get(this, PlayerCharacter::bAiControlledFromPos)
        || e.get(this, PlayerCharacter::bAiControlledToPos)
        || e.get(this, PlayerCharacter::bAiControlledActivate)
        || e.get(this, PlayerCharacter::bAiControlledPackage)
}

// Translated from 0093a7a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `pOccupiedRegion`.
pub fn fn_0093a7a0(e: &mut Engine, this: Ptr<PlayerCharacter>, region: Ptr) {
    e.set(this, PlayerCharacter::pOccupiedRegion, region);
}

/// The common tail of the actor-value setters: tells the interface the value
/// changed (`Interface::UpdateActorValue`, `00704e10`, cdecl) and calls
/// `008808f0` on the player.
fn notify_actor_value_changed(e: &mut Engine, this: Ptr<PlayerCharacter>, actor_value: u32) {
    e.call(0x0070_4e10, &args![actor_value]);
    e.call(0x0088_08f0, &args![this, actor_value, 1u32]);
}

/// Asks the player's virtual at `+0x48C` whether `actor_value` has an
/// override (the byte it writes through its second argument).
fn actor_value_has_override(e: &mut Engine, this: Ptr<PlayerCharacter>, actor_value: u32) -> bool {
    e.with_stack(4, |e, flag| {
        e.vcall(this.addr(), 0x48c, &args![actor_value, flag]);
        e.mem.u8(flag.addr()) != 0
    })
}

// Translated from 0093a7c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::SetActorBaseValue` (Xbox PDB): sets the base value of
/// `actor_value` to the integer `value`. When the virtual at `+0x48C` reports
/// an override, the override (virtual `+0x490`, as a float) is set; otherwise
/// the object `004181e0` returns gets its virtual `+0x194` called. The
/// interface is then notified.
pub fn player_character_set_actor_base_value(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor_value: u32,
    value: i32,
) {
    if !actor_value_has_override(e, this, actor_value) {
        let target: Ptr = e.call(0x0041_81e0, &args![this]).ptr();
        e.vcall(target.addr(), 0x194, &args![actor_value, value]);
    } else {
        e.vcall(
            this.addr(),
            0x490,
            &args![actor_value, (value as f64) as f32],
        );
    }
    notify_actor_value_changed(e, this, actor_value);
}

// Translated from 0093a850 (decompiled, FalloutNV.exe 1.4.0.525)
/// Like [`player_character_set_actor_base_value`] with a float value: the
/// object `004181e0` returns gets its virtual `+0x190` called when there is no
/// override.
pub fn fn_0093a850(e: &mut Engine, this: Ptr<PlayerCharacter>, actor_value: u32, value: f32) {
    if !actor_value_has_override(e, this, actor_value) {
        let target: Ptr = e.call(0x0041_81e0, &args![this]).ptr();
        e.vcall(target.addr(), 0x190, &args![actor_value, value]);
    } else {
        e.vcall(this.addr(), 0x490, &args![actor_value, value]);
    }
    notify_actor_value_changed(e, this, actor_value);
}

/// The value of `actor_value` the `ActorValueOwner` base reports (its virtual
/// `+0xC`), or 0 when `0066ee10` says the value does not apply.
fn owner_actor_value_or_zero(e: &mut Engine, this: Ptr<PlayerCharacter>, actor_value: u32) -> f32 {
    if e.call(0x0066_ee10, &args![actor_value]).bool() {
        e.vcall(
            this.addr() + ACTOR_VALUE_OWNER_BASE,
            0xc,
            &args![actor_value],
        )
        .f32()
    } else {
        0.0
    }
}

/// Reports a change of `actor_value` from `old` by `delta` (`0066ee50`,
/// cdecl) to the `ActorValueOwner` base.
fn report_actor_value_change(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor_value: u32,
    old: f32,
    delta: f32,
) {
    let owner = if this.is_null() {
        0
    } else {
        this.addr() + ACTOR_VALUE_OWNER_BASE
    };
    e.call(0x0066_ee50, &args![owner, actor_value, old, delta, 0u32]);
}

// Translated from 0093a8f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds the integer `delta` to the base value of `actor_value`: the sum with
/// the current value goes to the override (virtual `+0x490`) when the virtual
/// `+0x48C` reports one, otherwise `delta` goes to virtual `+0x19C` of the
/// object `004181e0` returns. The interface and the `ActorValueOwner` are told.
pub fn fn_0093a8f0(e: &mut Engine, this: Ptr<PlayerCharacter>, actor_value: u32, delta: i32) {
    let old = owner_actor_value_or_zero(e, this, actor_value);
    if !actor_value_has_override(e, this, actor_value) {
        let target: Ptr = e.call(0x0041_81e0, &args![this]).ptr();
        e.vcall(target.addr(), 0x19c, &args![actor_value, delta]);
    } else {
        let sum = (delta as f64 + old as f64) as f32;
        e.vcall(this.addr(), 0x490, &args![actor_value, sum]);
    }
    notify_actor_value_changed(e, this, actor_value);
    report_actor_value_change(e, this, actor_value, old, (delta as f64) as f32);
}

// Translated from 0093aa10 (decompiled, FalloutNV.exe 1.4.0.525)
/// [`fn_0093a8f0`] with a float delta (virtual `+0x198` of the object
/// `004181e0` returns when there is no override).
pub fn fn_0093aa10(e: &mut Engine, this: Ptr<PlayerCharacter>, actor_value: u32, delta: f32) {
    let old = owner_actor_value_or_zero(e, this, actor_value);
    if !actor_value_has_override(e, this, actor_value) {
        let target: Ptr = e.call(0x0041_81e0, &args![this]).ptr();
        e.vcall(target.addr(), 0x198, &args![actor_value, delta]);
    } else {
        let sum = (old as f64 + delta as f64) as f32;
        e.vcall(this.addr(), 0x490, &args![actor_value, sum]);
    }
    notify_actor_value_changed(e, this, actor_value);
    report_actor_value_change(e, this, actor_value, old, delta);
}

// Translated from 0093ab30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Computes the action-point cost of `actor_value` (`0066dca0`, with the flag
/// `00525430` reads from the object at `011f2250`) and passes it to the
/// virtual at `+0x33C` (which spends the action points).
pub fn fn_0093ab30(e: &mut Engine, this: Ptr<PlayerCharacter>, actor_value: u32) {
    let flag = e
        .call(0x0052_5430, &args![0x011f_2250u32, 0.0f32, 0u32])
        .bool();
    let cost = e.call(0x0066_dca0, &args![actor_value, flag]).f32();
    e.vcall(this.addr(), 0x33c, &args![cost]);
}

// Translated from 0093ab80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Spends `amount` action points: unless the player is in god mode
/// (`009526b0`) or `0042ce10` (on the object at global `011ddf38`) says
/// otherwise, and `amount` is positive, calls virtual `+0x3AC` with
/// (12, `-amount`, 0).
pub fn fn_0093ab80(e: &mut Engine, this: Ptr<PlayerCharacter>, amount: f32) {
    if e.call(0x0095_26b0, &args![this]).bool() {
        return;
    }
    let object = e.global::<u32>(GLOBAL_OBJECT_0042CE10);
    if e.call(0x0042_ce10, &args![object]).bool() {
        return;
    }
    let zero: f64 = e.global(ZERO_DOUBLE);
    if amount as f64 > zero {
        e.vcall(this.addr(), 0x3ac, &args![0xcu32, -amount, 0u32]);
    }
}

/// `PlayerCharacter` global singleton pointer (`011dea3c`).
const PLAYER_SINGLETON: u32 = 0x011d_ea3c;
/// `TES` global singleton pointer (`011dea10`): the `this` of the cell-grid
/// functions.
const TES_SINGLETON: u32 = 0x011d_ea10;
/// The limit (a `double`) above which the water-cell search radius is
/// clamped, and the radius it is clamped to (a `float`).
const WATER_RADIUS_LIMIT: u32 = 0x0101_6968;
const WATER_RADIUS_CLAMPED: u32 = 0x0101_8bfc;
/// The cell tests: `00425fd0` is bit 0 of the cell flag byte at +0x24 (what
/// `TESObjectREFR::GetInterior` returns for the parent cell, so the
/// interior-cell test), `004518e0` bit 1 (value 2) of the same byte.
const CELL_IS_INTERIOR: u32 = 0x0042_5fd0;
const CELL_FLAG_2_TEST: u32 = 0x0045_18e0;
/// The width of a grid cell as a `double` (4096.0).
const CELL_SIZE: u32 = 0x0101_7a10;
/// The setting objects (read through `00403e20`) the heartbeat sound of the
/// health meter compares the health fraction with.
const HEARTBEAT_SETTING_LOW: u32 = 0x011d_00b4;
const HEARTBEAT_SETTING_HIGH: u32 = 0x011d_04a8;
/// `BSAudio::QInstance` (`00453a70`) returns the audio singleton; its
/// `GetSoundHandleByName` (`00ad7550`) fills a handle from a sound name.
const AUDIO_INSTANCE: u32 = 0x0045_3a70;
const AUDIO_GET_SOUND_HANDLE_BY_NAME: u32 = 0x00ad_7550;
const SOUND_HANDLE_RELEASE: u32 = 0x00ad_8d10;
const SOUND_HANDLE_PLAY: u32 = 0x00ad_8830;
const SOUND_HANDLE_FADE_OUT_AND_RELEASE: u32 = 0x00ad_8da0;
/// The sound names the health heartbeat switches between.
const HEARTBEAT_SOUND_ALP: u32 = 0x0108_af40;
const HEARTBEAT_SOUND_BLP: u32 = 0x0108_af28;

// Translated from 0093abe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes two floats through the pointers: `out_second` gets the result of
/// `008a0c20` on the player and `out_first` gets the `ActorValueOwner` base's
/// virtual `+0x8` for actor value `0x2E`, converted from an integer.
pub fn fn_0093abe0(e: &mut Engine, this: Ptr<PlayerCharacter>, out_first: Ptr, out_second: Ptr) {
    let second = e.call(0x008a_0c20, &args![this]).f32();
    e.mem.set_f32(out_second.addr(), second);
    let first = e
        .vcall(this.addr() + ACTOR_VALUE_OWNER_BASE, 8, &args![0x2eu32])
        .i32();
    e.mem.set_f32(out_first.addr(), first as f32);
}

// Translated from 0093ac20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ActorValueOwner` base override (`this` is the player + `0xA4`): when
/// `004181e0` finds the object for the player, returns the base's virtual
/// `+0x0` for `actor_value` plus the three values `0094c3d0` (kinds 0, 1 and
/// 2) gives for it, each truncated to an integer; otherwise 0.
pub fn fn_0093ac20(e: &mut Engine, this: Ptr, actor_value: u32) -> i32 {
    let player = this.addr().wrapping_sub(ACTOR_VALUE_OWNER_BASE);
    if e.call(0x0041_81e0, &args![player]).u32() == 0 {
        return 0;
    }
    let mut sum = e.vcall(this.addr(), 0, &args![actor_value]).i32();
    for kind in 0..3u32 {
        let part = e.call(0x0094_c3d0, &args![player, kind, actor_value]).f64();
        sum = sum.wrapping_add(e.call(FTOL, &args![part]).i32());
    }
    sum
}

// Translated from 0093acb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Float version of [`fn_0093ac20`]: the base's virtual `+0x4` plus the three
/// values `0094c3d0` gives, summed in that order.
pub fn fn_0093acb0(e: &mut Engine, this: Ptr, actor_value: u32) -> f32 {
    let player = this.addr().wrapping_sub(ACTOR_VALUE_OWNER_BASE);
    let mut sum = e.vcall(this.addr(), 4, &args![actor_value]).f64();
    for kind in 0..3u32 {
        let part = e.call(0x0094_c3d0, &args![player, kind, actor_value]).f64();
        sum += part;
    }
    sum as f32
}

// Translated from 0093ad30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the object's virtual at `+0x20` with `actor_value`, passes the float
/// through `00406ce0` and truncates the result to an integer (the same body as
/// [`fn_005f73f0`]).
pub fn fn_0093ad30(e: &mut Engine, this: Ptr, actor_value: u32) -> i32 {
    let value = e.vcall(this.addr(), 0x20, &args![actor_value]).f32();
    let converted = e.call(0x0040_6ce0, &args![value]).f64();
    e.call(FTOL, &args![converted]).i32()
}

// Translated from 0093ad60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::GetPermanentActorFloatValue` (Xbox PDB): `this` is the
/// `ActorValueOwner` base (player + `0xA4`). The permanent value of
/// `actor_value`, clamped by `ActorValue::ClampActorValue` (`0066f190`),
/// whose result is returned. For a skill (`0047f060`) it is the base's
/// virtual `+0x18` plus the player's virtual `+0x48C` value plus the derived
/// skill of the player singleton's `ActorValueOwner` (`AiFormulas::AVDeriveSkill`);
/// otherwise the base's virtual `+0x18` plus its virtual `+0x4`.
pub fn player_character_get_permanent_actor_float_value(
    e: &mut Engine,
    this: Ptr,
    actor_value: u32,
) -> f32 {
    let value = if e.call(0x0047_f060, &args![actor_value]).bool() {
        let singleton = e.global::<u32>(PLAYER_SINGLETON);
        let derive_this = if singleton == 0 {
            0
        } else {
            singleton.wrapping_add(ACTOR_VALUE_OWNER_BASE)
        };
        let skill = e
            .call(0x0064_3c90, &args![derive_this, actor_value, 1u32])
            .f64();
        let player = this.addr().wrapping_sub(ACTOR_VALUE_OWNER_BASE);
        let modifier = e.with_stack(4, |e, flag| {
            e.vcall(player, 0x48c, &args![actor_value, flag]).f64()
        });
        let sum = modifier + skill;
        let base = e.vcall(this.addr(), 0x18, &args![actor_value]).f64();
        (base + sum) as f32
    } else {
        let first = e.vcall(this.addr(), 4, &args![actor_value]).f64();
        let base = e.vcall(this.addr(), 0x18, &args![actor_value]).f64();
        (base + first) as f32
    };
    e.call(0x0066_f190, &args![actor_value, value]).f32()
}

// Translated from 0093ae40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Passes `actor_value` and `value` to `00880700`, then does what the other
/// actor-value setters do afterwards (interface update, `008808f0`).
pub fn fn_0093ae40(e: &mut Engine, this: Ptr<PlayerCharacter>, actor_value: u32, value: f32) {
    e.call(0x0088_0700, &args![this, actor_value, value]);
    notify_actor_value_changed(e, this, actor_value);
}

/// Reports a change of `actor_value` from `old` by `delta` to the
/// `ActorValueOwner` bases of the player and of `source` (`0066ee50`).
fn report_change_from(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor_value: u32,
    old: f32,
    delta: f32,
    source: Ptr,
) {
    let source_owner = if source.is_null() {
        0
    } else {
        source.addr().wrapping_add(ACTOR_VALUE_OWNER_BASE)
    };
    let owner = if this.is_null() {
        0
    } else {
        this.addr().wrapping_add(ACTOR_VALUE_OWNER_BASE)
    };
    e.call(
        0x0066_ee50,
        &args![owner, actor_value, old, delta, source_owner],
    );
}

/// The common tail of the four damage-style setters `0093ae80`, `0093afb0`,
/// `0093b0f0` and `0093b240`: reads the old value, records the change in
/// `0094c4f0` (kind `kind`, the last argument 2), updates the interface, for
/// health lost calls the player's virtual `+0x4B8` with `source`, and
/// reports the change.
fn apply_actor_value_change(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor_value: u32,
    delta: f32,
    lost: bool,
    kind: u32,
    source: Ptr,
) {
    let old = owner_actor_value_or_zero(e, this, actor_value);
    e.call(0x0094_c4f0, &args![this, kind, actor_value, delta, 2u32]);
    e.call(0x0070_4e10, &args![actor_value]);
    if actor_value == 0x10 && lost {
        e.vcall(this.addr(), 0x4b8, &args![source, delta]);
    }
    e.call(0x0088_08f0, &args![this, actor_value, 0u32]);
    report_change_from(e, this, actor_value, old, delta, source);
}

// Translated from 0093ae80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unless `00406d70` says `actor_value` has flag `0x100`, applies a change of
/// the actor value: `00880850` gives the integer delta (see
/// [`apply_actor_value_change`], kind 0).
pub fn fn_0093ae80(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor_value: u32,
    amount: i32,
    source: Ptr,
) {
    if e.call(0x0040_6d70, &args![actor_value, 0x100u32]).bool() {
        return;
    }
    let delta = e
        .call(0x0088_0850, &args![this, actor_value, amount, source])
        .i32();
    apply_actor_value_change(e, this, actor_value, delta as f32, delta < 0, 0, source);
}

// Translated from 0093afb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Float version of [`fn_0093ae80`] (`00880890` gives the delta).
pub fn fn_0093afb0(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor_value: u32,
    amount: f32,
    source: Ptr,
) {
    if e.call(0x0040_6d70, &args![actor_value, 0x100u32]).bool() {
        return;
    }
    let delta = e
        .call(0x0088_0890, &args![this, actor_value, amount, source])
        .f32();
    let zero: f64 = e.global(ZERO_DOUBLE);
    apply_actor_value_change(
        e,
        this,
        actor_value,
        delta,
        (delta as f64) < zero,
        0,
        source,
    );
}

// Translated from 0093b0f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// [`fn_0093ae80`] preceded by the check `00952730` (called with the amount
/// as a float) and with kind 1.
pub fn fn_0093b0f0(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor_value: u32,
    amount: i32,
    source: Ptr,
) {
    if !e
        .call(0x0095_2730, &args![this, actor_value, amount as f32])
        .bool()
    {
        return;
    }
    if e.call(0x0040_6d70, &args![actor_value, 0x100u32]).bool() {
        return;
    }
    let delta = e
        .call(0x0088_0850, &args![this, actor_value, amount, source])
        .i32();
    apply_actor_value_change(e, this, actor_value, delta as f32, delta < 0, 1, source);
}

// Translated from 0093b240 (decompiled, FalloutNV.exe 1.4.0.525)
/// Float version of [`fn_0093b0f0`].
pub fn fn_0093b240(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor_value: u32,
    amount: f32,
    source: Ptr,
) {
    if !e
        .call(0x0095_2730, &args![this, actor_value, amount])
        .bool()
    {
        return;
    }
    if e.call(0x0040_6d70, &args![actor_value, 0x100u32]).bool() {
        return;
    }
    let delta = e
        .call(0x0088_0890, &args![this, actor_value, amount, source])
        .f32();
    let zero: f64 = e.global(ZERO_DOUBLE);
    apply_actor_value_change(
        e,
        this,
        actor_value,
        delta,
        (delta as f64) < zero,
        1,
        source,
    );
}

/// The health of the player as a fraction of its maximum: the
/// `ActorValueOwner` base's virtual `+0xC` over its virtual `+0x20`, both for
/// actor value `0x10`, divided in double precision.
fn health_ratio(e: &mut Engine, this: Ptr<PlayerCharacter>) -> f64 {
    let owner = this.addr().wrapping_add(ACTOR_VALUE_OWNER_BASE);
    let current = e.vcall(owner, 0xc, &args![0x10u32]).f32() as f64;
    let maximum = e.vcall(owner, 0x20, &args![0x10u32]).f32() as f64;
    current / maximum
}

/// The float the settings getter returns for the setting object `setting`.
fn setting_float(e: &mut Engine, setting: u32) -> f32 {
    let value = e.call(SETTING_FLOAT_GETTER, &args![setting]).u32();
    e.mem.f32(value)
}

/// Whether the x87 comparison of `a` with `b` is unordered (a NaN).
fn unordered(a: f32, b: f32) -> bool {
    a.is_nan() || b.is_nan()
}

/// `!(a < b)`: false only when `a` is below `b` (true for NaNs).
fn not_below(a: f32, b: f32) -> bool {
    a.partial_cmp(&b) != Some(std::cmp::Ordering::Less)
}

/// Stops the status sound and starts the sound called `name` in its place.
fn play_status_sound(e: &mut Engine, this: Ptr<PlayerCharacter>, name: u32) {
    let handle = member(this, PlayerCharacter::StatusSoundHandle);
    e.call(SOUND_HANDLE_STOP, &args![handle]);
    e.call(SOUND_HANDLE_RELEASE, &args![handle]);
    e.with_stack(12, |e, found| {
        let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
        let result = e
            .call(
                AUDIO_GET_SOUND_HANDLE_BY_NAME,
                &args![audio, found, name, 0x31u32],
            )
            .u32();
        e.call(SOUND_HANDLE_ASSIGN, &args![handle, result]);
        e.call(SOUND_HANDLE_DESTRUCT, &args![found]);
    });
    e.call(SOUND_HANDLE_PLAY, &args![handle, 1u32]);
}

/// The heartbeat sound of the health meter: given the health fraction before
/// (`before`) and after (`after`) a change, fades the status sound out, or
/// switches it to the "ALP" or "BLP" heartbeat according to the two health
/// thresholds of the settings at `011d00b4` and `011d04a8`. The branches
/// follow the x87 comparisons of the code, NaN included (one test pair can
/// never fall through for ordered values; it is still written out).
fn update_health_heartbeat(e: &mut Engine, this: Ptr<PlayerCharacter>, before: f32, after: f32) {
    let zero: f64 = e.global(ZERO_DOUBLE);
    let handle = member(this, PlayerCharacter::StatusSoundHandle);
    if (after as f64) <= zero || setting_float(e, HEARTBEAT_SETTING_LOW) < after {
        e.call(SOUND_HANDLE_FADE_OUT_AND_RELEASE, &args![handle, 1000u32]);
        return;
    }
    let low = setting_float(e, HEARTBEAT_SETTING_LOW);
    let mut check_blp = low < after || unordered(low, after);
    if !check_blp {
        let high = setting_float(e, HEARTBEAT_SETTING_HIGH);
        if not_below(high, after) {
            check_blp = true;
        } else {
            let high = setting_float(e, HEARTBEAT_SETTING_HIGH);
            let alp = if not_below(high, before) && !unordered(high, before) {
                true
            } else {
                let low = setting_float(e, HEARTBEAT_SETTING_LOW);
                if low < before {
                    true
                } else {
                    check_blp = true;
                    false
                }
            };
            if alp {
                play_status_sound(e, this, HEARTBEAT_SOUND_ALP);
                return;
            }
        }
    }
    if check_blp {
        let high = setting_float(e, HEARTBEAT_SETTING_HIGH);
        if high < before {
            let high = setting_float(e, HEARTBEAT_SETTING_HIGH);
            if !(high < after || unordered(high, after)) {
                play_status_sound(e, this, HEARTBEAT_SOUND_BLP);
            }
        }
    }
}

/// The part shared by `0093b3a0` and `0093b7a0`: `before` is the health
/// fraction before the change (1.0 for other actor values).
fn apply_health_change(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor_value: u32,
    before: f32,
    delta: f32,
    lost: bool,
    source: Ptr,
) {
    let old = owner_actor_value_or_zero(e, this, actor_value);
    let flagged = e.call(0x0040_6d70, &args![actor_value, 0x200u32]).bool() as u32;
    e.call(0x0094_c4f0, &args![this, 2u32, actor_value, delta, flagged]);
    e.call(0x0070_4e10, &args![actor_value]);
    if actor_value == 0x10 {
        let after = health_ratio(e, this) as f32;
        if lost {
            e.vcall(this.addr(), 0x4b8, &args![source, delta]);
        }
        update_health_heartbeat(e, this, before, after);
    }
    e.call(0x0088_08f0, &args![this, actor_value, 0u32]);
    report_change_from(e, this, actor_value, old, delta, source);
}

// Translated from 0093b3a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Like [`fn_0093b0f0`] with kind 2: when the actor value is health (`0x10`)
/// the heartbeat sound follows the change ([`update_health_heartbeat`]), and
/// the change is recorded with the flag `00406d70` reports for `0x200`. The
/// SEH frame (for the sound-handle temporaries) is not translated.
pub fn fn_0093b3a0(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor_value: u32,
    amount: i32,
    source: Ptr,
) {
    if !e
        .call(0x0095_2730, &args![this, actor_value, amount as f32])
        .bool()
    {
        return;
    }
    if e.call(0x0040_6d70, &args![actor_value, 0x100u32]).bool() {
        return;
    }
    let before = if actor_value == 0x10 {
        health_ratio(e, this) as f32
    } else {
        1.0
    };
    let delta = e
        .call(0x0088_0850, &args![this, actor_value, amount, source])
        .i32();
    apply_health_change(
        e,
        this,
        actor_value,
        before,
        delta as f32,
        delta < 0,
        source,
    );
}

// Translated from 0093b7a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Float version of [`fn_0093b3a0`] (`00880890` gives the delta).
pub fn fn_0093b7a0(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor_value: u32,
    amount: f32,
    source: Ptr,
) {
    if !e
        .call(0x0095_2730, &args![this, actor_value, amount])
        .bool()
    {
        return;
    }
    if e.call(0x0040_6d70, &args![actor_value, 0x100u32]).bool() {
        return;
    }
    let before = if actor_value == 0x10 {
        health_ratio(e, this) as f32
    } else {
        1.0
    };
    let delta = e
        .call(0x0088_0890, &args![this, actor_value, amount, source])
        .f32();
    let zero: f64 = e.global(ZERO_DOUBLE);
    apply_health_change(
        e,
        this,
        actor_value,
        before,
        delta,
        (delta as f64) < zero,
        source,
    );
}

/// `TESDataHandler` singleton pointer (`011c3f2c`).
const DATA_HANDLER: u32 = 0x011c_3f2c;
/// `ExteriorCellLoader` singleton pointer (`011c9618`).
const EXTERIOR_CELL_LOADER: u32 = 0x011c_9618;
/// The `Main` singleton pointer (`011dea0c`).
const MAIN_SINGLETON: u32 = 0x011d_ea0c;
/// The `MagicTarget` base of the player: its offset in `PlayerCharacter`.
const MAGIC_TARGET_BASE: u32 = 0x94;
/// A pointer-sized global that `0093ccd0` returns.
const GLOBAL_RETURNED_BY_0093CCD0: u32 = 0x011c_358c;
/// The 3x3 identity matrix (nine words) the player's matrix starts from.
const IDENTITY_MATRIX: u32 = 0x011a_9448;
/// The constant vector (three words) passed as the second vector to the
/// listener orientation.
const LISTENER_UP_VECTOR: u32 = 0x011a_9484;
/// pi, 2 pi and -pi as `double`s.
const PI_DOUBLE: u32 = 0x0101_ff40;
const TWO_PI_DOUBLE: u32 = 0x0101_ff48;
const NEGATIVE_PI_DOUBLE: u32 = 0x0101_ff58;
/// 32.0 as a `double`: the height above the land the player is lifted to.
const LAND_CLEARANCE: u32 = 0x0102_f070;
/// 0.1 as a `double`: the vertical movement below which the settling loop stops.
const SETTLE_LIMIT: u32 = 0x0101_ffa0;
/// A `float` (0.16): the time step of the settling moves.
const SETTLE_TIME_STEP: u32 = 0x0108_aff0;
/// A `float` global of the process lists' fader (`01202d98` is the IO
/// manager, `011d8804` the fader manager).
const IO_MANAGER: u32 = 0x0120_2d98;
const FADER_MANAGER: u32 = 0x011d_8804;
/// The `MoviePlayer` singleton pointer.
const MOVIE_PLAYER: u32 = 0x0126_fac4;
/// The `HUDMainMenu` loading-screen flag byte.
const LOADING_SCREEN_FLAG: u32 = 0x011d_8907;
/// The `BSTimer` global (`011f6394`); `0084d030` also reads a float from it.
const GAME_TIMER: u32 = 0x011f_6394;
/// Object whose `00403df0` result is the movie to play.
const MOVIE_OBJECT: u32 = 0x011d_e71c;
/// The `Calendar` global (`011de7b8`): `00867e30` gives its state, `00867950` its time scale.
const CALENDAR_OBJECT: u32 = 0x011d_e7b8;

/// Water-search tile step in cells: `-1`, `0` or `1`, from the position of
/// the player relative to the edges of the cell's `[low, high]` range
/// widened by `radius` (the x87 comparisons: a NaN selects `0`).
fn tile_step(low: f32, high: f32, position: f32, radius: f32) -> i32 {
    if (low as f64) >= position as f64 - radius as f64 {
        -1
    } else if (high as f64) <= position as f64 + radius as f64 {
        1
    } else {
        0
    }
}

// Translated from 0093bba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::GetWaterCell` (Xbox PDB): finds the cell to look for water
/// in within `radius` of the player (clamped to a limit). The search ends with
/// the player's parent cell (`008d6f30`) when it is an interior cell
/// ([`CELL_IS_INTERIOR`]), passes [`CELL_FLAG_2_TEST`], or no world space is
/// current (`TES::GetWorldSpace`); otherwise the grid cells next to the
/// player's grid cell are tried, toward the edges the radius reaches (x, then
/// y, then the diagonal one; a clamped search finally falls back to
/// `TES::FindFirstGridCellWithWater`), stopping at the first that passes
/// [`CELL_FLAG_2_TEST`]. Returns the cell if it passes that test, else 0.
pub fn player_character_get_water_cell(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    radius: f32,
) -> u32 {
    let mut radius = radius;
    let mut clamped = false;
    let limit: f64 = e.global(WATER_RADIUS_LIMIT);
    if (radius as f64) > limit {
        radius = e.global(WATER_RADIUS_CLAMPED);
        clamped = true;
    }
    let mut cell = e.call(0x008d_6f30, &args![this]).u32();
    'done: {
        if cell != 0 {
            if e.call(CELL_IS_INTERIOR, &args![cell]).bool() {
                break 'done;
            }
            if e.call(CELL_FLAG_2_TEST, &args![cell]).bool() {
                break 'done;
            }
            let tes = e.global::<u32>(TES_SINGLETON);
            if e.call(0x004f_d3e0, &args![tes]).u32() == 0 {
                break 'done;
            }
        }
        let position = e.call(0x0089_1170, &args![this]).u32().wrapping_add(0x10);
        let x = e.mem.f32(position);
        let y = e.mem.f32(position.wrapping_add(4));
        let cell_x = e.call(0x0040_6d90, &args![x]).i32() >> 12;
        let cell_y = e.call(0x0040_6d90, &args![y]).i32() >> 12;
        let (low_x, low_y, high_x, high_y) = e.with_stack(16, |e, bounds| {
            let low = bounds.addr();
            let high = low + 8;
            e.call(
                0x0045_2dc0,
                &args![
                    low,
                    cell_x.wrapping_shl(12) as f32,
                    cell_y.wrapping_shl(12) as f32
                ],
            );
            let size: f64 = e.global(CELL_SIZE);
            e.call(
                0x0045_2dc0,
                &args![
                    high,
                    (cell_x.wrapping_shl(12) as f64 + size) as f32,
                    (cell_y.wrapping_shl(12) as f64 + size) as f32
                ],
            );
            (
                e.mem.f32(low),
                e.mem.f32(low + 4),
                e.mem.f32(high),
                e.mem.f32(high + 4),
            )
        });
        let tes = e.global::<u32>(TES_SINGLETON);
        let step_x = tile_step(low_x, high_x, x, radius);
        if step_x != 0 {
            cell = e
                .call(
                    0x0045_1900,
                    &args![tes, cell_x.wrapping_add(step_x), cell_y],
                )
                .u32();
        }
        if cell != 0 && e.call(CELL_FLAG_2_TEST, &args![cell]).bool() {
            break 'done;
        }
        let step_y = tile_step(low_y, high_y, y, radius);
        if step_y != 0 {
            cell = e
                .call(
                    0x0045_1900,
                    &args![tes, cell_x, cell_y.wrapping_add(step_y)],
                )
                .u32();
        }
        if cell != 0 && e.call(CELL_FLAG_2_TEST, &args![cell]).bool() {
            break 'done;
        }
        if step_x != 0 && step_y != 0 {
            cell = e
                .call(
                    0x0045_1900,
                    &args![
                        tes,
                        cell_x.wrapping_add(step_x),
                        cell_y.wrapping_add(step_y)
                    ],
                )
                .u32();
            if cell != 0 && e.call(CELL_FLAG_2_TEST, &args![cell]).bool() {
                break 'done;
            }
            if clamped {
                let found = e.call(0x0045_16d0, &args![tes]).u32();
                if found != 0 {
                    cell = e.mem.u32(found);
                }
            }
        }
    }
    if cell != 0 && !e.call(CELL_FLAG_2_TEST, &args![cell]).bool() {
        cell = 0;
    }
    cell
}

layout! {
    /// A queued `PositionPlayer` request (the object `pQueuedTargetLoc`
    /// points to; the Xbox PDB has no type for it), 0x34 bytes: the fields
    /// are named after how `HandlePositionPlayerRequest` uses them.
    pub struct PositionPlayerRequest: 0x34 {
        /// The world space of an exterior target (0 when the target is a cell).
        0x00 worldspace: Ptr,
        /// The target cell (used when `worldspace` is 0).
        0x04 cell: Ptr,
        /// The target position (three floats).
        0x08 position: Inline<NiPoint3>,
        /// The target rotation (three floats).
        0x14 rotation: Inline<NiPoint3>,
        /// The last argument of `PositionPlayer` (resets the weather).
        0x20 reset_weather: u8,
        /// A callback (an exe address) called with `callback_argument` after
        /// the player is placed; 0 for none.
        0x24 callback: u32,
        /// The argument of the callback.
        0x28 callback_argument: u32,
        /// A furniture reference the player is put into afterwards.
        0x2C furniture: Ptr,
        /// A reference to fast travel to (when set, nothing else is used).
        0x30 fast_travel_target: Ptr,
    }
}

// Translated from 0093be30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::RequestPositionPlayer` (Xbox PDB): queues `request` in
/// `pQueuedTargetLoc`, freeing (and, if the new request is not null,
/// reporting that it bashes) an earlier one; when `00451530` says the cell
/// grid can take it, the request is handled at once
/// ([`player_character_handle_position_player_request`]).
pub fn player_character_request_position_player(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    request: Ptr<PositionPlayerRequest>,
) {
    let earlier = e.get(this, PlayerCharacter::pQueuedTargetLoc);
    if !earlier.is_null() {
        if !request.is_null() {
            e.call(0x005b_5e40, &args![0x0108_af58u32]);
        }
        let earlier = e.get(this, PlayerCharacter::pQueuedTargetLoc);
        e.call(OPERATOR_DELETE, &args![earlier]);
    }
    e.set(this, PlayerCharacter::pQueuedTargetLoc, request.cast());
    let tes = e.global::<u32>(TES_SINGLETON);
    if e.call(0x0045_1530, &args![tes]).bool() {
        player_character_handle_position_player_request(e, this);
    }
}

/// The queued request, read afresh (the callees may replace it).
fn queued_request(e: &Engine, this: Ptr<PlayerCharacter>) -> Ptr<PositionPlayerRequest> {
    e.get(this, PlayerCharacter::pQueuedTargetLoc).cast()
}

// Translated from 0093bea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::HandlePositionPlayerRequest` (Xbox PDB): carries out the
/// queued request, if any, and returns whether the player was moved.
/// A request with a fast-travel target travels there ([`player_character_fast_travel`],
/// the `00569b80` result taking precedence over the target); one with a world
/// space calls [`player_character_position_player_exterior`], one with only a
/// cell [`player_character_position_player`], and neither is reported as
/// invalid; after a move the character controller gets the target's height
/// (`00573f20`), the callback runs, and a furniture reference puts the player
/// in a chair or bed (`Actor::PutActorInChairBedQuick`). The request is freed
/// afterwards, the garbage collector cleared, and, unless `0042ce10` or the
/// fast-travel flag (`0093c1e0`) forbids it, `005d14d0` is called on the
/// player.
pub fn player_character_handle_position_player_request(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
) -> bool {
    let mut moved = false;
    if !e.get(this, PlayerCharacter::pQueuedTargetLoc).is_null() {
        let tes = e.global::<u32>(TES_SINGLETON);
        e.call(0x0045_2eb0, &args![tes]);
        e.call(
            NI_POINTER_ASSIGN,
            &args![member(this, PlayerCharacter::spTargeted3D), 0u32],
        );
        let request = queued_request(e, this);
        let travel = e.get(request, PositionPlayerRequest::fast_travel_target);
        if !travel.is_null() {
            let mut target = e.call(0x0056_9b80, &args![travel]).u32();
            if target == 0 {
                let request = queued_request(e, this);
                target = e
                    .get(request, PositionPlayerRequest::fast_travel_target)
                    .addr();
            }
            let player = e.global::<u32>(PLAYER_SINGLETON);
            player_character_fast_travel(e, Ptr::new(player), Ptr::new(target));
            moved = true;
        } else {
            let worldspace = e.get(request, PositionPlayerRequest::worldspace);
            let cell = e.get(request, PositionPlayerRequest::cell);
            if worldspace.is_null() && cell.is_null() {
                e.call(0x005b_5e40, &args![0x0108_afa0u32]);
            } else {
                let position = read_floats(e, request.addr() + 0x08);
                let rotation = read_floats(e, request.addr() + 0x14);
                let flag = e.get(request, PositionPlayerRequest::reset_weather) != 0;
                if !worldspace.is_null() {
                    player_character_position_player_exterior(
                        e,
                        this,
                        position[0],
                        position[1],
                        position[2],
                        rotation[0],
                        rotation[1],
                        rotation[2],
                        worldspace,
                        flag,
                    );
                } else {
                    player_character_position_player(
                        e,
                        this,
                        position[0],
                        position[1],
                        position[2],
                        rotation[0],
                        rotation[1],
                        rotation[2],
                        cell,
                        flag,
                    );
                }
                let controller = e.call(0x0093_06d0, &args![this]).u32();
                let request = queued_request(e, this);
                let height = e.mem.f32(request.addr() + 0x10);
                e.call(0x0057_3f20, &args![controller, height]);
                let request = queued_request(e, this);
                let callback = e.get(request, PositionPlayerRequest::callback);
                if callback != 0 {
                    let request = queued_request(e, this);
                    let argument = e.get(request, PositionPlayerRequest::callback_argument);
                    e.call(callback, &args![argument]);
                }
                moved = true;
            }
            let request = queued_request(e, this);
            let furniture = e.get(request, PositionPlayerRequest::furniture);
            if !furniture.is_null() && e.call(0x0056_8680, &args![furniture]).bool() {
                let process = e.call(0x008d_8520, &args![this]).u32();
                let marker_owner = e.vcall(process, 0x4d4, &args![]).u32();
                let request = queued_request(e, this);
                let furniture = e.get(request, PositionPlayerRequest::furniture);
                let form = e.call(0x007a_f430, &args![furniture]).u32();
                let can_sleep = e.call(0x0050_9420, &args![form]).bool();
                let request = queued_request(e, this);
                let furniture = e.get(request, PositionPlayerRequest::furniture);
                let index = e.call(0x0056_82c0, &args![furniture, 1u32]).i32();
                if index != -1 {
                    let request = queued_request(e, this);
                    let furniture = e.get(request, PositionPlayerRequest::furniture);
                    if e.call(0x0056_8500, &args![furniture, index, marker_owner])
                        .bool()
                    {
                        let request = queued_request(e, this);
                        let furniture = e.get(request, PositionPlayerRequest::furniture);
                        e.call(
                            0x0088_d2f0,
                            &args![this, furniture, marker_owner, index, can_sleep],
                        );
                    }
                }
            }
        }
        e.call(0x0086_8d70, &args![1u32]);
        let request = e.get(this, PlayerCharacter::pQueuedTargetLoc);
        e.call(OPERATOR_DELETE, &args![request]);
        e.set(this, PlayerCharacter::pQueuedTargetLoc, Ptr::NULL);
    }
    if moved {
        let object = e.global::<u32>(GLOBAL_OBJECT_0042CE10);
        if !e.call(0x0042_ce10, &args![object]).bool() {
            let player = e.global::<u32>(PLAYER_SINGLETON);
            if !fn_0093c1e0(e, Ptr::new(player)) {
                e.call(0x005d_14d0, &args![player, 1u32]);
            }
        }
    }
    moved
}

/// The three floats at `addr`.
fn read_floats(e: &Engine, addr: u32) -> [f32; 3] {
    [
        e.mem.f32(addr),
        e.mem.f32(addr.wrapping_add(4)),
        e.mem.f32(addr.wrapping_add(8)),
    ]
}

/// The three words at `addr`.
fn read_words(e: &Engine, addr: u32) -> [u32; 3] {
    [
        e.mem.u32(addr),
        e.mem.u32(addr.wrapping_add(4)),
        e.mem.u32(addr.wrapping_add(8)),
    ]
}

// Translated from 0093c1e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit `0x2` of the byte at `+0x66D` (the byte `bCanFastTravel` sits in).
pub fn fn_0093c1e0(e: &mut Engine, this: Ptr<PlayerCharacter>) -> bool {
    e.mem.u8(this.addr().wrapping_add(0x66d)) & 2 != 0
}

// Translated from 0093ccd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the global word at `011c358c`.
pub fn fn_0093ccd0(e: &mut Engine) -> u32 {
    e.global(GLOBAL_RETURNED_BY_0093CCD0)
}

/// Copies nine words (a 3x3 matrix) from `source` to `destination`.
fn copy_matrix(e: &mut Engine, destination: u32, source: u32) {
    for i in 0..9 {
        let word = e.mem.u32(source.wrapping_add(4 * i));
        e.mem.set_u32(destination.wrapping_add(4 * i), word);
    }
}

/// The spell target the player's `MagicTarget` base is asked about: the
/// object `0093ccd0` returns, at +0x18 (0 when there is none).
fn magic_source(e: &mut Engine) -> u32 {
    let object = fn_0093ccd0(e);
    if object == 0 {
        0
    } else {
        object.wrapping_add(0x18)
    }
}

// Translated from 0093c200 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::PositionPlayer` (Xbox PDB): moves the player to the
/// position and rotation (three floats each; the third rotation float is the
/// heading, the first two pitch and roll) in `cell`. In order: the menu
/// background is rendered when the cell grid cannot take requests
/// (`00451530`), the process lists are prepared (`009785d0`) and a loading
/// menu's tile menu deleted; the encounter zones of the old and new cell are
/// switched (`00526070`/`005260a0`); the player leaves its parent cell, takes
/// the position (virtual `+0x2A8`) and rotation (`SetAngleOnReference`) and
/// enters `cell`; an interior cell ([`CELL_IS_INTERIOR`]) is entered through
/// `00453dc0` and clears the canopy shadows, any other keeps them (when the
/// world space has them) or clears the mask and sets the world space; a player
/// in move mode (`005f36f0`) has a spell target dispelled and the pipboy
/// light shown; sounds of kind `0x1000` stop and the weather resets when
/// `reset_weather`; the fader starts for a player without 3D; the player's
/// matrix is rebuilt from heading, pitch and roll and given to the 3D root and
/// the player's node; the audio listener is placed and oriented; for a player
/// with first-person 3D the position is lifted above the land and settled by
/// moving it down (at most 100 `Move`s) and the animations are refreshed;
/// the camera rigid body, terrain and world-space text are updated. The SEH
/// frame (stack-cookie check) is not translated.
#[allow(clippy::too_many_arguments)]
pub fn player_character_position_player(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    position_x: f32,
    position_y: f32,
    position_z: f32,
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,
    cell: Ptr,
    reset_weather: bool,
) {
    // The arguments live on the game's stack: position at +0x0, rotation at
    // +0xC, with the locals behind them.
    e.with_stack(0x180, |e, frame| {
        let f = frame.addr();
        e.mem.set_f32(f, position_x);
        e.mem.set_f32(f + 4, position_y);
        e.mem.set_f32(f + 8, position_z);
        e.mem.set_f32(f + 0xc, rotation_x);
        e.mem.set_f32(f + 0x10, rotation_y);
        e.mem.set_f32(f + 0x14, rotation_z);
        position_player_in_frame(e, this, f, cell, reset_weather);
    });
}

/// Frame offsets used by [`position_player_in_frame`].
const FRAME_POSITION: u32 = 0x00;
const FRAME_ROTATION: u32 = 0x0c;
const FRAME_PLAYER_MATRIX: u32 = 0x18;
const FRAME_TURN_MATRIX: u32 = 0x3c;
const FRAME_PRODUCT_A: u32 = 0x60;
const FRAME_PRODUCT_B: u32 = 0x84;
const FRAME_UPDATE_DATA: u32 = 0xa8;
const FRAME_FORWARD: u32 = 0xb8;
const FRAME_START_POSITION: u32 = 0xc4;
const FRAME_LAND_HEIGHT: u32 = 0xd0;
const FRAME_MOVE_VECTOR: u32 = 0xd4;
const FRAME_CAMERA_POSITION: u32 = 0xe0;
const FRAME_CAMERA_MATRIX: u32 = 0xf0;

fn position_player_in_frame(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    f: u32,
    cell: Ptr,
    reset_weather: bool,
) {
    let position = f + FRAME_POSITION;
    let rotation = f + FRAME_ROTATION;
    let tes = e.global::<u32>(TES_SINGLETON);
    if !e.call(0x0045_1530, &args![tes]).bool() {
        let main = e.global::<u32>(MAIN_SINGLETON);
        e.call(0x0087_1dc0, &args![main]);
    }
    e.call(0x0097_85d0, &args![PROCESS_LISTS]);
    if e.call(0x0070_edf0, &args![]).bool() {
        e.call(0x0070_5e30, &args![]);
        let tile = e.call(0x0070_ede0, &args![]).u32();
        let menu_class = e.call(0x00a0_9030, &args![tile]).u32();
        let menu = e.call(0x00a0_3c90, &args![menu_class]).u32();
        if menu != 0 {
            e.vcall(menu, 0, &args![1u32]);
        }
    }
    let uses_move_mode = e.call(0x005f_36f0, &args![tes]).u32() != 0;
    let world = e.call(0x004f_d3e0, &args![tes]).u32();
    e.call(0x0054_ddd0, &args![cell]);
    let new_zone = e.call(0x0054_6c20, &args![cell]).u32();
    let mut old_zone = 0;
    if e.call(0x008d_6f30, &args![this]).u32() != 0 {
        let parent = e.call(0x008d_6f30, &args![this]).u32();
        old_zone = e.call(0x0054_6c20, &args![parent]).u32();
    } else if world != 0 {
        old_zone = e.call(0x0045_8400, &args![world]).u32();
    }
    if old_zone != new_zone {
        let state = e.call(0x0086_7e30, &args![CALENDAR_OBJECT]).u32();
        if old_zone != 0 {
            e.call(0x0052_6070, &args![old_zone, state]);
        }
        if new_zone != 0 {
            e.call(0x0052_60a0, &args![new_zone, state]);
        }
    }
    let movie = e.global::<u32>(MOVIE_PLAYER);
    let playing_sequence = e.call(0x00ec_17c0, &args![movie]).bool();
    if e.call(0x008d_6f30, &args![this]).u32() != 0 {
        let parent = e.call(0x008d_6f30, &args![this]).u32();
        e.call(0x0054_ca90, &args![parent, this]);
    }
    e.vcall(this.addr(), 0x2a8, &args![position]);
    e.call(0x0086_d490, &args![this, rotation]);
    e.call(0x0057_5bb0, &args![this, cell]);
    'audio: {
        if !cell.is_null() && e.call(CELL_IS_INTERIOR, &args![cell]).bool() {
            let music = e.call(0x0054_74b0, &args![cell, 0u32]).u32();
            e.set(this, PlayerCharacter::pLastKnownMusicType, Ptr::new(music));
            e.call(0x0045_3dc0, &args![tes, cell, position]);
            e.call(0x0066_5860, &args![]);
            break 'audio;
        }
        if !cell.is_null() {
            let music = e.call(0x0054_74b0, &args![cell, 0u32]).u32();
            e.set(this, PlayerCharacter::pLastKnownMusicType, Ptr::new(music));
            let mut clear_canopy = true;
            if e.call(0x0045_2480, &args![]).bool() {
                let world = e.call(0x0054_ddd0, &args![cell]).u32();
                let flag = e.call(0x0054_8210, &args![world]).u32();
                if e.mem.i8(flag) != 0 {
                    e.call(0x0066_5610, &args![]);
                    clear_canopy = false;
                }
            }
            if clear_canopy {
                e.call(0x0066_5860, &args![]);
                let grid = e.call(0x0044_ddc0, &args![tes]).u32();
                e.call(0x004b_aec0, &args![grid]);
            }
            let world = e.call(0x0054_ddd0, &args![cell]).u32();
            e.call(0x0045_8200, &args![tes, world]);
        }
        if uses_move_mode {
            let source = magic_source(e);
            if e.call(
                0x0082_2b90,
                &args![this.addr() + MAGIC_TARGET_BASE, source, 1u32],
            )
            .bool()
            {
                let pipboy = e.call(0x0070_5990, &args![]).u32();
                e.call(0x007f_a310, &args![pipboy, 1u32, 0u32, 1u32]);
                let pipboy = e.call(0x0070_5990, &args![]).u32();
                e.call(0x007f_a310, &args![pipboy, 0u32, 0u32, 1u32]);
                let source = magic_source(e);
                e.call(
                    0x0082_4400,
                    &args![this.addr() + MAGIC_TARGET_BASE, source, 0u32, 0u32],
                );
            }
        }
        e.call(0x0045_4450, &args![tes, position]);
    }
    let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
    e.call(SOUND_HANDLE_DESTRUCT, &args![audio]);
    let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
    e.call(0x00ad_8780, &args![audio, 0x1000u32]);
    if reset_weather {
        let sky = e.call(0x0046_dd00, &args![]).u32();
        e.call(0x0063_d060, &args![sky]);
    }
    e.call(0x0057_5bb0, &args![this, 0u32]);
    e.call(0x0054_8230, &args![cell, this, 0u32]);
    if e.vcall(this.addr(), 0x1d0, &args![]).u32() == 0 {
        let object = e.global::<u32>(GLOBAL_OBJECT_0042CE10);
        if !e.call(0x0042_ce10, &args![object]).bool() {
            let io = e.global::<u32>(IO_MANAGER);
            e.call(0x0045_6520, &args![io]);
            let fader = e.global::<u32>(FADER_MANAGER);
            e.call(0x0070_0960, &args![fader, 1u32, 0.0f32, 0u32]);
        }
    }

    // The player's 3D: heading, pitch and roll rotations.
    let player_matrix = f + FRAME_PLAYER_MATRIX;
    let turn_matrix = f + FRAME_TURN_MATRIX;
    copy_matrix(e, player_matrix, IDENTITY_MATRIX);
    e.call(LIST_ITEM_ADDRESS, &args![turn_matrix]);
    let pi: f64 = e.global(PI_DOUBLE);
    let two_pi: f64 = e.global(TWO_PI_DOUBLE);
    let negative_pi: f64 = e.global(NEGATIVE_PI_DOUBLE);
    let mut heading = e.mem.f32(rotation + 8);
    while heading as f64 > pi {
        heading = (heading as f64 - two_pi) as f32;
        e.mem.set_f32(rotation + 8, heading);
    }
    while (heading as f64) < negative_pi {
        heading = (heading as f64 + two_pi) as f32;
        e.mem.set_f32(rotation + 8, heading);
    }
    e.call(0x004a_0c90, &args![turn_matrix, heading]);
    let product = e
        .call(
            0x0043_f8d0,
            &args![player_matrix, f + FRAME_PRODUCT_A, turn_matrix],
        )
        .u32();
    copy_matrix(e, player_matrix, product);
    let rotation_x = e.mem.f32(rotation);
    let rotation_y = e.mem.f32(rotation + 4);
    let tilt = if heading >= 0.0 {
        (rotation_x as f64 - rotation_y as f64) as f32
    } else {
        (rotation_x as f64 + rotation_y as f64) as f32
    };
    e.call(0x0052_4ac0, &args![turn_matrix, tilt]);
    if e.vcall(this.addr(), 0x1d0, &args![]).u32() != 0 {
        let node = e.vcall(this.addr(), 0x1d0, &args![]).u32();
        e.call(0x0043_fa80, &args![node, turn_matrix]);
    }
    let product = e
        .call(
            0x0043_f8d0,
            &args![turn_matrix, f + FRAME_PRODUCT_B, player_matrix],
        )
        .u32();
    copy_matrix(e, player_matrix, product);
    let root = e.call(0x0045_c670, &args![]).u32();
    let node = e.call(0x0055_8310, &args![root]).u32();
    e.call(0x0043_fa80, &args![node, player_matrix]);
    let root = e.call(0x0045_c670, &args![]).u32();
    let node = e.call(0x0055_8310, &args![root]).u32();
    e.call(0x0044_0460, &args![node, position]);
    let update_data = f + FRAME_UPDATE_DATA;
    e.call(0x0043_d410, &args![update_data, 0.0f32, 0u32, 0u32]);
    let root = e.call(0x0045_c670, &args![]).u32();
    let node = e.call(0x0055_8310, &args![root]).u32();
    e.call(0x00a5_9c60, &args![node, update_data]);
    let scene = e.call(0x0084_e3a0, &args![tes]).u32();
    e.call(0x00a5_a040, &args![scene]);

    // The audio listener.
    let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
    let [px, py, pz] = read_words(e, position);
    e.call(0x00ad_78b0, &args![audio, px, py, pz]);
    let up = read_words(e, LISTENER_UP_VECTOR);
    let angle = e.vcall(this.addr(), 0x2bc, &args![0u32]).f32();
    let y = e.call(0x005b_9e80, &args![angle]).f32();
    let angle = e.vcall(this.addr(), 0x2bc, &args![0u32]).f32();
    let x = e.call(0x005c_53d0, &args![angle]).f32();
    let forward = e
        .call(0x0041_6870, &args![f + FRAME_FORWARD, x, y, 0.0f32])
        .u32();
    let [fx, fy, fz] = read_words(e, forward);
    let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
    e.call(0x00ad_7940, &args![audio, fx, fy, fz, up[0], up[1], up[2]]);
    e.call(0x0095_2c30, &args![this, this]);
    let accumulator = e.call(0x00b4_f5c0, &args![]).u32();
    if accumulator != 0 {
        e.call(0x00b6_55b0, &args![accumulator]);
    }

    // The first-person 3D: settle the position and refresh the animations.
    if e.call(
        0x0055_9450,
        &args![member(this, PlayerCharacter::sp1stPerson3D)],
    )
    .u32()
        != 0
    {
        let pipboy = e.call(0x0070_5990, &args![]).u32();
        e.call(0x007f_a200, &args![pipboy]);
        let tes_object = e.call(0x0045_0b80, &args![0u32]).u32();
        e.call(0x00b5_fd60, &args![tes_object]);
        let object = e.global::<u32>(GLOBAL_OBJECT_0042CE10);
        if !e.call(0x0042_ce10, &args![object]).bool() {
            settle_player_position(e, this, f, cell);
        }
        let toggled = !e.get(this, PlayerCharacter::b3rdPerson);
        e.set(this, PlayerCharacter::b3rdPerson, toggled);
        refresh_animation(e, this);
        let toggled = !e.get(this, PlayerCharacter::b3rdPerson);
        e.set(this, PlayerCharacter::b3rdPerson, toggled);
        refresh_animation(e, this);
        e.call(0x0094_ae40, &args![this, 0u32, 0u32]);
    }
    if e.call(
        0x0055_9450,
        &args![member(this, PlayerCharacter::spCameraRigidBody)],
    )
    .u32()
        != 0
    {
        let camera_matrix = f + FRAME_CAMERA_MATRIX;
        let camera_position = f + FRAME_CAMERA_POSITION;
        e.call(LIST_ITEM_ADDRESS, &args![camera_matrix]);
        let current = e.vcall(this.addr(), 0x1f4, &args![]).u32();
        let words = read_words(e, current);
        for (i, word) in words.iter().enumerate() {
            e.mem.set_u32(camera_position + 4 * i as u32, *word);
        }
        e.call(0x004a_3e00, &args![camera_matrix, camera_position]);
        let body = e
            .call(
                0x0055_9450,
                &args![member(this, PlayerCharacter::spCameraRigidBody)],
            )
            .u32();
        e.call(0x0056_10f0, &args![body, camera_position]);
    }
    let weapon = e.call(0x0052_4c90, &args![]).u32();
    e.call(0x0066_52e0, &args![weapon, 0u32]);
    let fade = e.call(0x0084_d030, &args![GAME_TIMER]).f32();
    e.call(0x0045_3550, &args![tes, fade]);
    let object = e.global::<u32>(GLOBAL_OBJECT_0042CE10);
    if !e.call(0x0042_ce10, &args![object]).bool() {
        e.call(0x0078_cfc0, &args![]);
    }
    if playing_sequence {
        let movie = e.global::<u32>(MOVIE_PLAYER);
        e.call(0x00ec_1800, &args![movie]);
    }
    e.call(0x0096_1f90, &args![this]);
    e.call(0x0045_a750, &args![tes]);
    if e.call(0x0057_5d70, &args![this]).u32() != 0 {
        let current = e.vcall(this.addr(), 0x1f4, &args![]).u32();
        let world = e.call(0x0057_5d70, &args![this]).u32();
        let manager = e.call(0x0058_6170, &args![world]).u32();
        e.call(0x006f_ca90, &args![manager, current, 0x0fu32]);
        let current = e.vcall(this.addr(), 0x1f4, &args![]).u32();
        let world = e.call(0x0057_5d70, &args![this]).u32();
        let manager = e.call(0x0058_6170, &args![world]).u32();
        e.call(0x006f_cdb0, &args![manager, current]);
    }
    let main = e.global::<u32>(MAIN_SINGLETON);
    e.call(SOUND_HANDLE_DESTRUCT, &args![main]);
    if e.global::<u8>(LOADING_SCREEN_FLAG) != 0 {
        e.call(0x0045_7d70, &args![tes, 0u32, 0u32, 0u32]);
        e.call(0x0077_1700, &args![0x18u32]);
        e.call(0x0083_04a0, &args![]);
        let movie_name = e.call(0x0040_3df0, &args![MOVIE_OBJECT]).u32();
        let movie = e.global::<u32>(MOVIE_PLAYER);
        e.call(
            0x00ec_2320,
            &args![movie, movie_name, 1u32, 1u32, 0u32, 1u32, 1u32, 0u32, 1u32, 1u32],
        );
        e.call(0x0077_1700, &args![0x0cu32]);
    }
    let player = e.global::<u32>(PLAYER_SINGLETON);
    let name = if e.call(0x0057_5d70, &args![player]).u32() != 0 {
        let world = e.call(0x0057_5d70, &args![player]).u32();
        e.call(0x0040_8da0, &args![world.wrapping_add(0x18)]).u32()
    } else {
        0
    };
    e.call(0x0077_2c30, &args![name]);
    e.call(0x0097_3de0, &args![PROCESS_LISTS]);
}

/// The player is lifted onto the land when it is below it (for a cell that
/// is not an interior), then moved down in 100 steps at most until it no longer falls;
/// when the loop does not settle, it is put back at its start position.
fn settle_player_position(e: &mut Engine, this: Ptr<PlayerCharacter>, f: u32, cell: Ptr) {
    let start = f + FRAME_START_POSITION;
    let current = e.vcall(this.addr(), 0x1f4, &args![]).u32();
    let words = read_words(e, current);
    for (i, word) in words.iter().enumerate() {
        e.mem.set_u32(start + 4 * i as u32, *word);
    }
    if !cell.is_null() && !e.call(CELL_IS_INTERIOR, &args![cell]).bool() {
        let height = f + FRAME_LAND_HEIGHT;
        e.mem.set_f32(height, 0.0);
        if e.call(0x0055_47c0, &args![cell, start, height]).bool() {
            let clearance: f64 = e.global(LAND_CLEARANCE);
            let land = e.mem.f32(height);
            let z = e.mem.f32(start + 8);
            if (z as f64) < land as f64 + clearance {
                e.mem.set_f32(start + 8, (land as f64 + clearance) as f32);
                e.vcall(this.addr(), 0x2a8, &args![start]);
            }
        }
    }
    let limit: f64 = e.global(SETTLE_LIMIT);
    let mut step = 0;
    while step < 100 {
        let before = e.vcall(this.addr(), 0x1f4, &args![]).u32();
        let before_z = e.mem.f32(before + 8);
        let vector = f + FRAME_MOVE_VECTOR;
        let zero_vector = read_words(e, ZERO_VECTOR);
        for (i, word) in zero_vector.iter().enumerate() {
            e.mem.set_u32(vector + 4 * i as u32, *word);
        }
        let time_step = e.global::<f32>(SETTLE_TIME_STEP);
        e.call(0x0092_f260, &args![this, time_step, vector, 0u32]);
        let after = e.vcall(this.addr(), 0x1f4, &args![]).u32();
        let after_z = e.mem.f32(after + 8);
        let moved = (after_z as f64 - before_z as f64) as f32;
        let magnitude = e.call(0x0040_8840, &args![moved]).f64();
        if magnitude < limit {
            break;
        }
        step += 1;
    }
    if step == 100 {
        e.vcall(this.addr(), 0x2a8, &args![start]);
    }
}

/// Refreshes the animation after the third-person flag changed: `008d3550`
/// with 0.0, then `Actor::UpdateAnimationMovement` with the animation
/// `GetAnimation` gives for the opposite of the flag.
fn refresh_animation(e: &mut Engine, this: Ptr<PlayerCharacter>) {
    e.call(0x008d_3550, &args![this, 0.0f32]);
    let first_person = !e.get(this, PlayerCharacter::b3rdPerson);
    let animation = e.call(0x0095_0a60, &args![this, first_person]).u32();
    e.call(0x0088_85e0, &args![this, animation, 0.0f32]);
}

// Translated from 0093cce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::PositionPlayerExterior` (Xbox PDB): with a world space,
/// finds (or loads, or creates) the cell at the grid cell the position is in
/// (cancelling pending exterior loads first and clearing the cells that are
/// not needed, [`player_character_clear_cells_if_target_not_loaded`]) and
/// moves the player singleton there ([`player_character_position_player`]);
/// without a world space nothing happens.
#[allow(clippy::too_many_arguments)]
pub fn player_character_position_player_exterior(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    position_x: f32,
    position_y: f32,
    position_z: f32,
    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,
    world_space: Ptr,
    reset_weather: bool,
) {
    let mut cell = 0u32;
    if !world_space.is_null() {
        let loader = e.global::<u32>(EXTERIOR_CELL_LOADER);
        if loader != 0 {
            e.call(0x0052_8540, &args![loader]);
        }
        let cell_x = e.call(0x0040_6d90, &args![position_x]).i32() >> 12;
        let cell_y = e.call(0x0040_6d90, &args![position_y]).i32() >> 12;
        cell = e
            .call(0x0058_75a0, &args![world_space, cell_x, cell_y])
            .u32();
        player_character_clear_cells_if_target_not_loaded(e, this, Ptr::new(cell), world_space);
        if cell == 0 {
            cell = e
                .call(0x0058_5b30, &args![world_space, cell_x, cell_y])
                .u32();
        }
        if cell == 0 {
            let handler = e.global::<u32>(DATA_HANDLER);
            cell = e
                .call(
                    0x0046_1330,
                    &args![handler, 0u32, cell_x, cell_y, world_space],
                )
                .u32();
        }
    }
    if cell != 0 {
        let player = e.global::<u32>(PLAYER_SINGLETON);
        player_character_position_player(
            e,
            Ptr::new(player),
            position_x,
            position_y,
            position_z,
            rotation_x,
            rotation_y,
            rotation_z,
            Ptr::new(cell),
            reset_weather,
        );
    }
}

/// The two statics of `FastTravel`: the guard bits (bit 0 for the timestamp
/// below) and the game time of the last `00850a40` call.
const FAST_TRAVEL_STATIC_FLAGS: u32 = 0x011e_0ba4;
const FAST_TRAVEL_LAST_TIME: u32 = 0x011e_0ba0;
/// A `byte` setting object (`00408d60`'s `this`): whether to ask `00850a40`
/// to react to a long pause between fast travels.
const FAST_TRAVEL_SETTING: u32 = 0x011d_8980;
/// A pointer global whose object `FastTravel` passes to `00850a40`.
const FAST_TRAVEL_REACTOR: u32 = 0x011d_e134;
/// The fast-travel flag byte (`011d8906`) other code reads.
const FAST_TRAVELLING: u32 = 0x011d_8906;
/// 3600.0 as a `double` and as a `float` (seconds per hour).
const SECONDS_PER_HOUR_DOUBLE: u32 = 0x0101_2640;
const SECONDS_PER_HOUR_FLOAT: u32 = 0x0108_4838;
/// The `float` maximum (a `double`) `Pathing::ComputeTeleportDoorPathLength`
/// returns when there is no path.
const NO_PATH_LENGTH: u32 = 0x0102_31b0;
/// 2.0 as a `float`: the duration of the messages and health restored per
/// fast-travel hour.
const TWO_FLOAT: u32 = 0x0101_62c0;
/// The VATS object (`011f2250`).
const VATS_OBJECT: u32 = 0x011f_2250;
/// The object whose `0040fbf0`/`0040fba0` bracket each simulated hour.
const HOUR_BRACKET_OBJECT: u32 = 0x011f_11a0;

// Translated from 0093cdf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::FastTravel` (Xbox PDB): travels to `destination`, a
/// reference. Does nothing when the entry point `0x33` (`005e58f0`) leaves the
/// delay at 0 and the player's virtual `+0x358` agrees. Otherwise the cell grid
/// is flushed, the player leaves any furniture, the path length to the
/// destination is computed (`Pathing::ComputeTeleportDoorPathLength`; with no
/// path a message naming the destination's cell is printed and the length is
/// 0), the travel time is that length over the run speed (in hours, scaled by
/// the multi-bound radius `00526ac0` gives for the setting `0x3a`), the player's
/// sleep time (`iSleepTime`) is set to its whole hours, and time is advanced:
/// hour by hour with the actors updated and the player healed when there are
/// whole hours, otherwise by the fraction at once. Then the player is placed at
/// the destination (`PositionPlayerExterior` with the destination's world
/// space, position and rotation) and the beds, chairs and actors are set up
/// for the new place. The SEH frame is not translated.
pub fn player_character_fast_travel(e: &mut Engine, this: Ptr<PlayerCharacter>, destination: Ptr) {
    let zero: f64 = e.global(ZERO_DOUBLE);
    let delay = e.with_stack(4, |e, slot| {
        e.mem.set_f32(slot.addr(), 0.0);
        e.call(0x005e_58f0, &args![0x33u32, this, slot]);
        e.mem.f32(slot.addr())
    });
    if e.vcall(this.addr(), 0x358, &args![]).bool() && delay as f64 == zero {
        return;
    }
    let flags: u32 = e.global(FAST_TRAVEL_STATIC_FLAGS);
    if flags & 1 == 0 {
        e.set_global(FAST_TRAVEL_STATIC_FLAGS, flags | 1);
        let now = e.call(0x0082_5c00, &args![GAME_TIMER]).u32();
        e.set_global(FAST_TRAVEL_LAST_TIME, now);
    }
    let now = e.call(0x0082_5c00, &args![GAME_TIMER]).u32();
    let elapsed = now.wrapping_sub(e.global::<u32>(FAST_TRAVEL_LAST_TIME));
    let setting = e.call(0x0040_8d60, &args![FAST_TRAVEL_SETTING]).u32();
    if e.mem.u8(setting) != 0 && (elapsed == 0 || elapsed > 20000) {
        let reactor = e.global::<u32>(FAST_TRAVEL_REACTOR);
        e.call(0x0085_0a40, &args![reactor]);
        let now = e.call(0x0082_5c00, &args![GAME_TIMER]).u32();
        e.set_global(FAST_TRAVEL_LAST_TIME, now);
    }
    let player = e.global::<u32>(PLAYER_SINGLETON);
    e.mem.set_u8(player + 0x20c, 1);
    let tes = e.global::<u32>(TES_SINGLETON);
    e.with_stack(16, |e, state| {
        e.call(0x0087_8160, &args![state, 1u32, 1u32, 1u32]);
        e.call(0x0045_39a0, &args![tes, 1u32, 0u32]);
        let byte = e.mem.u8(state.addr() + 5);
        e.call(0x0087_8250, &args![byte as u32]);
        e.call(0x0087_8200, &args![state]);
    });
    let mover = e
        .get(
            this.cast::<crate::units::fallout_ai::actor::Actor>(),
            crate::units::fallout_ai::actor::Actor::pActorMover,
        )
        .addr();
    e.call(0x009e_a3b0, &args![mover, 0x800u32]);
    let controller = e.call(0x0093_06d0, &args![this]).u32();
    e.call(0x0088_b0a0, &args![controller, 0u32]);
    e.call(0x0088_d640, &args![this]);

    // Frame: the path length (+0), the lock data (+0x10), two pathing
    // locations (+0x20, +0x50), a string (+0x80) and the message (+0x90).
    e.with_stack(0x490, |e, frame| {
        let f = frame.addr();
        let length_slot = f;
        let lock_data = f + 0x10;
        let destination_location = f + 0x20;
        let player_location = f + 0x50;
        let name = f + 0x80;
        let message = f + 0x90;
        e.mem.set_f32(length_slot, 0.0);
        e.call(LIST_ITEM_ADDRESS, &args![f + 0x88]);
        let mut position = [0u32; 3];
        for (i, word) in position.iter_mut().enumerate() {
            let current = e.vcall(player, 0x1f4, &args![]).u32();
            *word = e.mem.u32(current + 4 * i as u32);
        }
        e.call(
            0x008d_0500,
            &args![
                this,
                destination,
                length_slot,
                position[0],
                position[1],
                position[2]
            ],
        );
        e.call(0x0050_2670, &args![lock_data, player]);
        let destination_loc = e
            .call(0x006d_cd70, &args![destination_location, destination])
            .u32();
        let player_loc = e.call(0x006d_cd70, &args![player_location, player]).u32();
        let length = e
            .call(
                0x006d_4eb0,
                &args![player_loc, destination_loc, 0.0f32, lock_data, 0u32],
            )
            .f32();
        e.mem.set_f32(length_slot, length);
        e.call(0x004f_f7e0, &args![player_location]);
        e.call(0x004f_f7e0, &args![destination_location]);
        let mut length = length;
        let no_path: f64 = e.global(NO_PATH_LENGTH);
        if length as f64 == no_path {
            e.call(0x0040_6d00, &args![message, 0x400u32, 0x0108_aff4u32]);
            if e.call(0x008d_6f30, &args![destination]).u32() != 0 {
                e.call(0x0040_37b0, &args![name]);
                let cell = e.call(0x008d_6f30, &args![destination]).u32();
                e.vcall(cell, 0x90, &args![name]);
                let text = e.call(0x0055_9450, &args![name]).u32();
                e.call(0x0040_6d50, &args![message, 0x400u32, text]);
                e.call(0x0040_37d0, &args![name]);
            }
            length = 0.0;
            e.call(0x005b_5e40, &args![message]);
        }
        let setting = e.call(0x0048_39c0, &args![0x3au32]).u32();
        let speed = e.call(0x0088_4eb0, &args![player]).f32();
        let per_second = (length as f64 / speed as f64) as f32;
        let per_hour: f64 = e.global(SECONDS_PER_HOUR_DOUBLE);
        let mut hours = (per_second as f64 / per_hour) as f32;
        let radius = e.call(0x0052_6ac0, &args![setting]).f32();
        hours = (radius as f64 * hours as f64) as f32;
        let whole_hours = e.call(FTOL, &args![hours as f64]).u32();
        e.set(this, PlayerCharacter::iSleepTime, whole_hours);
        e.set(this, PlayerCharacter::bIsSleeping, false);
        e.call(0x0045_7d70, &args![tes, 1u32, destination, 0u32]);
        if e.get(this, PlayerCharacter::iSleepTime) != 0 {
            loop {
                let remaining = e.get(this, PlayerCharacter::iSleepTime) as i32;
                if remaining <= 0 {
                    break;
                }
                let scale = e.call(0x0086_7950, &args![CALENDAR_OBJECT]).f32();
                let step = (per_hour / scale as f64) as f32;
                let clock = e.call(0x0096_d490, &args![PROCESS_LISTS]).f32();
                let advanced = (clock as f64 + step as f64) as f32;
                e.call(0x0096_d4b0, &args![PROCESS_LISTS, advanced]);
                e.call(0x0086_7a40, &args![CALENDAR_OBJECT, step]);
                e.call(0x0040_fbf0, &args![HOUR_BRACKET_OBJECT, 0u32]);
                let hour: f32 = e.global(SECONDS_PER_HOUR_FLOAT);
                e.call(0x0096_bcd0, &args![PROCESS_LISTS, hour, 1u32]);
                e.call(0x0096_db30, &args![PROCESS_LISTS, hour, 1u32, 0u32]);
                e.call(0x0096_b810, &args![PROCESS_LISTS, hour, 1u32]);
                e.call(0x0096_b470, &args![PROCESS_LISTS, hour, 1u32]);
                e.call(0x0096_b050, &args![PROCESS_LISTS, hour, 1u32]);
                e.call(0x0096_eb40, &args![PROCESS_LISTS]);
                e.call(0x0040_fba0, &args![HOUR_BRACKET_OBJECT]);
                let remaining = e.get(this, PlayerCharacter::iSleepTime);
                e.set(this, PlayerCharacter::iSleepTime, remaining.wrapping_sub(1));
                let restore: f32 = e.global(TWO_FLOAT);
                e.call(0x0088_b510, &args![player, restore]);
            }
        } else {
            let scale_hours = hours as f64 * per_hour;
            let scale = e.call(0x0086_7950, &args![CALENDAR_OBJECT]).f32();
            let step = (scale_hours / scale as f64) as f32;
            let clock = e.call(0x0096_d490, &args![PROCESS_LISTS]).f32();
            let advanced = (clock as f64 + step as f64) as f32;
            e.call(0x0096_d4b0, &args![PROCESS_LISTS, advanced]);
            e.call(0x0086_7a40, &args![CALENDAR_OBJECT, step]);
        }
    });
    e.call(0x0097_3ee0, &args![PROCESS_LISTS, destination]);
    if fn_0093d4f0(e) {
        e.call(0x0045_aee0, &args![tes, 0u32]);
    }
    if e.call(0x0065_2110, &args![]).u32() != 0 {
        let cache = e.call(0x0065_2110, &args![]).u32();
        e.call(0x0065_0a30, &args![cache, 0u32]);
    }
    e.call(0x00aa_7030, &args![]);
    let sky = e.call(0x0046_dd00, &args![]).u32();
    e.call(0x0063_e8f0, &args![sky, 1u32]);
    e.set_global::<u8>(FAST_TRAVELLING, 1);
    let world_space = e.call(0x0057_5d70, &args![destination]).u32();
    let rotation = e.call(0x0043_0830, &args![destination]).u32();
    let rotation = read_floats(e, rotation);
    let current = e.vcall(destination.addr(), 0x1f4, &args![]).u32();
    let position = read_floats(e, current);
    player_character_position_player_exterior(
        e,
        Ptr::new(player),
        position[0],
        position[1],
        position[2],
        rotation[0],
        rotation[1],
        rotation[2],
        Ptr::new(world_space),
        true,
    );
    e.set_global::<u8>(FAST_TRAVELLING, 0);
    e.set(this, PlayerCharacter::iSleepTime, 0);
    let sky = e.call(0x0046_dd00, &args![]).u32();
    e.call(0x0063_e8f0, &args![sky, 0u32]);
    if fn_0093d4f0(e) {
        e.call(0x0045_aee0, &args![tes, 1u32]);
    }
    e.call(0x0048_3710, &args![PROCESS_LISTS]);
    e.call(0x0045_9870, &args![tes]);
    e.call(0x0097_2d30, &args![PROCESS_LISTS]);
    e.call(0x0096_df40, &args![PROCESS_LISTS]);
    let player = e.global::<u32>(PLAYER_SINGLETON);
    e.mem.set_u8(player + 0x20c, 0);
    e.call(0x0097_5d10, &args![PROCESS_LISTS]);
    e.vcall(this.addr(), 0x3f4, &args![]);
    e.call(0x0096_1f90, &args![this]);
    let player = e.global::<u32>(PLAYER_SINGLETON);
    e.mem.set_u8(player + 0x204, 0);
}

// Translated from 0093d4f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00454af0` on the object at `011c3c10` and returns its byte result.
pub fn fn_0093d4f0(e: &mut Engine) -> bool {
    e.call(0x0045_4af0, &args![0x011c_3c10u32]).bool()
}

// Translated from 0093d500 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::ClearCellsIfTargetNotLoaded` (Xbox PDB): unless `cell` is
/// already loaded (`TES::IsCellLoaded`), flushes the cell grid for the new
/// target (`cell`, or `world_space` when there is none): the player leaves its
/// parent cell, the object `00950bb0` finds is told (virtual `+0xE8` of the
/// object `009611e0` finds), the process lists are updated and the freed
/// objects released, and the face-gen model cache and `00aa7030` are cleaned
/// up.
pub fn player_character_clear_cells_if_target_not_loaded(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    cell: Ptr,
    world_space: Ptr,
) {
    let tes = e.global::<u32>(TES_SINGLETON);
    if !cell.is_null() && e.call(0x0045_11e0, &args![tes, cell, 0u32]).bool() {
        return;
    }
    let target = if cell.is_null() { world_space } else { cell };
    e.call(0x0045_7d70, &args![tes, 1u32, target, 0u32]);
    let player = e.global::<u32>(PLAYER_SINGLETON);
    if e.call(0x008d_6f30, &args![player]).u32() != 0 {
        let player = e.global::<u32>(PLAYER_SINGLETON);
        let parent = e.call(0x008d_6f30, &args![player]).u32();
        e.call(0x0054_ca90, &args![parent, player]);
    }
    let found = e.call(0x0095_0bb0, &args![this, 1u32]).u32();
    let inner = if found != 0 {
        e.vcall(found, 0xc, &args![]).u32()
    } else {
        0
    };
    if inner != 0 && e.call(0x0096_11e0, &args![inner]).u32() != 0 {
        let handler = e.call(0x0096_11e0, &args![inner]).u32();
        e.vcall(handler, 0xe8, &args![found]);
    }
    e.with_stack(16, |e, state| {
        e.call(0x0087_8160, &args![state, 0u32, 1u32, 0u32]);
        e.call(0x0045_39a0, &args![tes, 0u32, 1u32]);
        e.set_global::<u8>(PROCESS_LISTS_BUSY, 1);
        e.call(0x0096_d810, &args![PROCESS_LISTS]);
        e.call(0x0096_eb40, &args![PROCESS_LISTS]);
        e.set_global::<u8>(PROCESS_LISTS_BUSY, 0);
        let byte = e.mem.u8(state.addr() + 5);
        e.call(0x0087_8250, &args![byte as u32]);
        e.call(0x0087_8200, &args![state]);
    });
    if e.call(0x0065_2110, &args![]).u32() != 0 {
        let cache = e.call(0x0065_2110, &args![]).u32();
        e.call(0x0065_0a30, &args![cache, 0u32]);
    }
    e.call(0x00aa_7030, &args![]);
}

// Translated from 0093d660 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the player may fast travel now, printing a message with the sad
/// vault-boy icon and returning false at the first condition that fails: the
/// player is somewhere fast travel is not allowed (`009764a0` with the
/// interior flag `00575d10` gives), a certain list (`00971c30` kind `0x15`)
/// is not empty (it is also cleared), an active effect of the `MagicTarget`
/// base of the player has the property virtual `+0x3C` reports, the travel
/// flag (`0093db40`) is off, the entry point `0x33` leaves a delay below 1.0
/// (message without icon), health or the other actor value `0x16` are below 1,
/// `008849c0` says so, or the parent cell/world space refuses
/// (`00544520`/`00586210`).
pub fn fn_0093d660(e: &mut Engine, this: Ptr<PlayerCharacter>) -> bool {
    let player = e.global::<u32>(PLAYER_SINGLETON);
    let interior = e.call(0x0057_5d10, &args![player]).bool();
    if e.call(0x0097_64a0, &args![PROCESS_LISTS, interior as u32])
        .bool()
    {
        show_message(e, 0x011d_3dd8, FAST_TRAVEL_ICON);
        return false;
    }
    let player = e.global::<u32>(PLAYER_SINGLETON);
    let list = e
        .call(0x0097_1c30, &args![PROCESS_LISTS, player, 0x15u32, 0u32])
        .u32();
    if list != 0 && !e.call(LIST_IS_EMPTY, &args![list]).bool() {
        show_message(e, 0x011d_4114, FAST_TRAVEL_ICON);
        e.call(LIST_CLEAR, &args![list]);
        if list != 0 {
            e.call(LIST_DELETING_DESTRUCT, &args![list, 1u32]);
        }
        return false;
    }
    let mut effect = e.vcall(this.addr() + MAGIC_TARGET_BASE, 8, &args![]).u32();
    if e.call(0x008d_8520, &args![this]).u32() != 0 {
        while effect != 0 && !e.call(LIST_IS_EMPTY, &args![effect]).bool() {
            let next = e.call(LIST_NEXT, &args![effect]).u32();
            let item = e.call(LIST_ITEM_ADDRESS, &args![effect]).u32();
            let item = e.mem.u32(item);
            if item != 0 && e.vcall(item, 0x3c, &args![]).bool() {
                show_message(e, 0x011d_255c, FAST_TRAVEL_ICON);
                return false;
            }
            effect = next;
        }
    }
    let player = e.global::<u32>(PLAYER_SINGLETON);
    if !fn_0093db40(e, Ptr::new(player)) {
        show_message(e, 0x011d_3630, FAST_TRAVEL_ICON);
        return false;
    }
    let player = e.global::<u32>(PLAYER_SINGLETON);
    if e.vcall(player, 0x358, &args![]).bool() {
        let delay = e.with_stack(4, |e, slot| {
            e.mem.set_f32(slot.addr(), 0.0);
            e.call(0x005e_58f0, &args![0x33u32, this, slot]);
            e.mem.f32(slot.addr())
        });
        let one: f64 = e.global(0x0101_2070);
        if (delay as f64) < one {
            show_message(e, 0x011d_50c8, 0);
            return false;
        }
    }
    let player = e.global::<u32>(PLAYER_SINGLETON);
    let owner = player + ACTOR_VALUE_OWNER_BASE;
    if e.vcall(owner, 8, &args![0x16u32]).i32() < 1 {
        show_message(e, 0x011d_255c, FAST_TRAVEL_ICON);
        return false;
    }
    let player = e.global::<u32>(PLAYER_SINGLETON);
    if e.vcall(player + ACTOR_VALUE_OWNER_BASE, 8, &args![0x10u32])
        .i32()
        < 1
    {
        show_message(e, 0x011d_255c, FAST_TRAVEL_ICON);
        return false;
    }
    let player = e.global::<u32>(PLAYER_SINGLETON);
    if e.call(0x0088_49c0, &args![player]).bool() {
        show_message(e, 0x011d_4120, FAST_TRAVEL_ICON);
        return false;
    }
    let mut allowed = false;
    if e.call(0x008d_6f30, &args![this]).u32() != 0 {
        let parent = e.call(0x008d_6f30, &args![this]).u32();
        allowed = !e.call(0x0054_4520, &args![parent]).bool();
    } else {
        let world_space = e.call(0x0057_5d70, &args![this]).u32();
        if world_space != 0 {
            allowed = !e.call(0x0058_6210, &args![world_space]).bool();
        }
    }
    if !allowed {
        show_message(e, 0x011d_4738, FAST_TRAVEL_ICON);
    }
    allowed
}

/// The sad vault-boy icon path (a string in the exe).
const FAST_TRAVEL_ICON: u32 = 0x0102_08a0;
/// Byte global set while the process lists are being updated.
const PROCESS_LISTS_BUSY: u32 = 0x011e_0e34;

/// Prints the text of the setting object `setting` as a message with `icon`
/// (`007052f0`: text, sound 0, icon, 0, duration 2.0, 0).
fn show_message(e: &mut Engine, setting: u32, icon: u32) {
    e.with_stack(0x1f4, |e, text| {
        let format = e.call(0x0040_3df0, &args![setting]).u32();
        e.call(0x0040_6d00, &args![text, 0x1f4u32, format]);
        let duration: f32 = e.global(TWO_FLOAT);
        e.call(0x0070_52f0, &args![text, 0u32, icon, 0u32, duration, 0u32]);
    });
}

// Translated from 0093db40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit `0x1` of the byte at `+0x66D`.
pub fn fn_0093db40(e: &mut Engine, this: Ptr<PlayerCharacter>) -> bool {
    e.mem.u8(this.addr().wrapping_add(0x66d)) & 1 != 0
}

// Translated from 0093db60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::CenterOnCell` (Xbox PDB): places the player at the
/// centre of a cell: `cell`, or the one named `editor_id` (looked up in the
/// loaded cells, or else among the exterior cells in the data files, which are
/// loaded after clearing the grid). The position is the cell's
/// `COC` placement (`0054cfd0`); [`player_character_position_player`] puts the
/// player there and, in an exterior, the height is then set to the land height
/// at the position (0 if negative).
pub fn player_character_center_on_cell(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    editor_id: u32,
    cell: Ptr,
) {
    e.call(0x0040_fbe0, &args![this]);
    let handler = e.global::<u32>(DATA_HANDLER);
    let mut cell = cell.addr();
    e.with_stack(0x30, |e, frame| {
        let position = frame.addr();
        let rotation = position + 0x0c;
        let coordinates = position + 0x18;
        e.call(LIST_ITEM_ADDRESS, &args![position]);
        if cell == 0 {
            cell = e.call(0x0046_1ae0, &args![handler, editor_id]).u32();
        }
        let loader = e.global::<u32>(EXTERIOR_CELL_LOADER);
        if loader != 0 {
            e.call(0x0052_8540, &args![loader]);
        }
        if cell == 0 {
            e.mem.set_u32(coordinates, 0);
            e.mem.set_u32(coordinates + 4, 0);
            let world_space = e
                .call(
                    0x0046_1cf0,
                    &args![handler, editor_id, coordinates, coordinates + 4],
                )
                .u32();
            if world_space != 0 {
                player_character_clear_cells_if_target_not_loaded(
                    e,
                    this,
                    Ptr::NULL,
                    Ptr::new(world_space),
                );
                let x = e.mem.u32(coordinates);
                let y = e.mem.u32(coordinates + 4);
                cell = e.call(0x0058_5b30, &args![world_space, x, y]).u32();
            }
        } else if !e.call(CELL_IS_INTERIOR, &args![cell]).bool() {
            let world_space = e.call(0x0054_ddd0, &args![cell]).u32();
            if world_space != 0 {
                player_character_clear_cells_if_target_not_loaded(
                    e,
                    this,
                    Ptr::new(cell),
                    Ptr::new(world_space),
                );
                let y = e.call(0x0054_4c60, &args![cell]).u32();
                let x = e.call(0x0054_4c30, &args![cell]).u32();
                e.call(0x0058_5b30, &args![world_space, x, y]);
            }
        }
        if cell != 0 {
            let up = read_words(e, ZERO_VECTOR);
            for (i, word) in up.iter().enumerate() {
                e.mem.set_u32(rotation + 4 * i as u32, *word);
            }
            e.call(0x0054_cfd0, &args![cell, position, rotation]);
            let p = read_floats(e, position);
            let r = read_floats(e, rotation);
            player_character_position_player(
                e,
                this,
                p[0],
                p[1],
                p[2],
                r[0],
                r[1],
                r[2],
                Ptr::new(cell),
                true,
            );
            if !e.call(CELL_IS_INTERIOR, &args![cell]).bool() {
                let height = e.with_stack(4, |e, slot| {
                    let tes = e.global::<u32>(TES_SINGLETON);
                    e.call(0x0045_72e0, &args![tes, position, slot]);
                    e.mem.f32(slot.addr())
                });
                let z = if height >= 0.0 { height } else { 0.0 };
                e.mem.set_f32(position + 8, z);
                e.vcall(this.addr(), 0x2a8, &args![position]);
            }
        }
    });
}

// Translated from 0093dd20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the virtual at `+0xC` of the player's `ActorMover` (`pActorMover`)
/// with `(008846e0(this) & 0xCC00) | 0x400`.
pub fn fn_0093dd20(e: &mut Engine, this: Ptr<PlayerCharacter>) {
    let flags = e.call(0x0088_46e0, &args![this]).u16();
    let value = (flags & 0xcc00) | 0x400;
    let mover = e
        .get(
            this.cast::<crate::units::fallout_ai::actor::Actor>(),
            crate::units::fallout_ai::actor::Actor::pActorMover,
        )
        .addr();
    e.vcall(mover, 0xc, &args![value as u32]);
}

// Translated from 0093dd80 (decompiled, FalloutNV.exe 1.4.0.525)
/// For every perceived actor (`pListofPercievedActors`, entries of an actor and
/// a byte) whose byte is set, makes a temp effect object (`0081f580`, 0x6C
/// bytes, from the actor, the object `007b8f10` finds in the data handler and
/// -1.0) and adds it to the process lists when its virtual `+0xC4` accepts
/// it, deleting it otherwise. Nothing happens without the handler's object.
/// The SEH frame is not translated.
pub fn fn_0093dd80(e: &mut Engine, this: Ptr<PlayerCharacter>) {
    let mut node = e.get(this, PlayerCharacter::pListofPercievedActors).addr();
    let handler = e.global::<u32>(DATA_HANDLER);
    let target = e.call(0x007b_8f10, &args![handler]).u32();
    if target == 0 {
        return;
    }
    while node != 0 {
        let item = e.call(LIST_ITEM_ADDRESS, &args![node]).u32();
        if e.mem.u32(item) == 0 {
            break;
        }
        let item = e.call(LIST_ITEM_ADDRESS, &args![node]).u32();
        let entry = e.mem.u32(item);
        if e.mem.u8(entry + 4) != 0 {
            add_temp_effect(e, e.mem.u32(entry), target);
        }
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
}

/// Creates a temp effect (`0081f580`) for `actor` against `target` with the
/// -1.0 strength, adds it to the process lists when its virtual `+0xC4`
/// accepts it and deletes it otherwise.
fn add_temp_effect(e: &mut Engine, actor: u32, target: u32) {
    let block = e.call(0x00aa_13e0, &args![0x6cu32]).u32();
    let effect = if block != 0 {
        let strength: f32 = e.global(TIMER_RESET);
        e.call(0x0081_f580, &args![block, actor, target, strength])
            .u32()
    } else {
        0
    };
    if e.vcall(effect, 0xc4, &args![]).bool() {
        e.call(0x0097_3fd0, &args![PROCESS_LISTS, effect]);
    } else if effect != 0 {
        e.vcall(effect, 0, &args![1u32]);
    }
}

// Translated from 0093dee0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::IsTargetPerceivedAndHostile` (Xbox PDB): looks `actor` up
/// in the perceived actors (`pListofPercievedActors`) and returns the byte of
/// its entry (false when absent).
pub fn player_character_is_target_perceived_and_hostile(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    actor: u32,
) -> bool {
    let mut node = e.get(this, PlayerCharacter::pListofPercievedActors).addr();
    while node != 0 {
        let item = e.call(LIST_ITEM_ADDRESS, &args![node]).u32();
        if e.mem.u32(item) == 0 {
            break;
        }
        let item = e.call(LIST_ITEM_ADDRESS, &args![node]).u32();
        let entry = e.mem.u32(item);
        if e.mem.u32(entry) == actor {
            return e.mem.u8(entry + 4) != 0;
        }
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
    false
}

// Translated from 0093df50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Spawns temp effects (see [`add_temp_effect`]) for the actors four
/// iterators yield (`005271f0` with the kinds `0x28`, `0x29`, `0x2F` and `0x2E`
/// for `this`, walked with `005273c0`), against the object
/// `0093e4f0` finds in the data handler, with the strength of the setting
/// `011d0658`. Nothing is spawned without that object. The SEH frame is not
/// translated.
pub fn fn_0093df50(e: &mut Engine, this: Ptr<PlayerCharacter>) {
    e.with_stack(0x120, |e, frame| {
        let kinds = [0x28u32, 0x29, 0x2f, 0x2e];
        let iterators: Vec<u32> = (0..4).map(|i| frame.addr() + 0x40 * i).collect();
        for (iterator, kind) in iterators.iter().zip(kinds) {
            e.call(0x0052_71f0, &args![*iterator, kind, this]);
        }
        let strength = e.call(SETTING_FLOAT_GETTER, &args![0x011d_0658u32]).u32();
        let strength = e.mem.f32(strength);
        let handler = e.global::<u32>(DATA_HANDLER);
        let target = fn_0093e4f0(e, Ptr::new(handler));
        let cursor = frame.addr() + 0x100;
        let state = frame.addr() + 0x108;
        e.mem.set_u32(cursor, 0);
        e.mem.set_u32(state, 0);
        if target == 0 {
            for iterator in iterators.iter().rev() {
                fn_0093e510(e, Ptr::new(*iterator));
            }
            return;
        }
        let list = e.global::<u32>(0x011c_95c8);
        for iterator in &iterators {
            while e
                .call(0x0052_73c0, &args![list, *iterator, cursor, state, 3u32])
                .bool()
            {
                let actor = e.mem.u32(cursor);
                let block = e.call(0x00aa_13e0, &args![0x6cu32]).u32();
                let effect = if block != 0 {
                    e.call(0x0081_f580, &args![block, actor, target, strength])
                        .u32()
                } else {
                    0
                };
                if e.vcall(effect, 0xc4, &args![]).bool() {
                    e.call(0x0097_3fd0, &args![PROCESS_LISTS, effect]);
                } else if effect != 0 {
                    e.vcall(effect, 0, &args![1u32]);
                }
            }
        }
        for iterator in iterators.iter().rev() {
            fn_0093e510(e, Ptr::new(*iterator));
        }
    });
}

// Translated from 0093e4f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the word at `+0x634` of the object (for the player this is
/// `spGrabSpring`'s pointer; the data handler singleton is also passed).
pub fn fn_0093e4f0(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr().wrapping_add(0x634))
}

// Translated from 0093e510 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `005272b0` on the member at `+0x1C` of the object.
pub fn fn_0093e510(e: &mut Engine, this: Ptr) {
    e.call(0x0052_72b0, &args![this.addr().wrapping_add(0x1c)]);
}

// Translated from 0093e530 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::SetSlowMoCamera` (Xbox PDB): with a kill camera setting
/// (`eKillCameraSetting`), plays the "UIPopUpQuestNew" sound and, unless the
/// player's parent cell is the form `0x161E98` or `mode` is 0 or the VATS
/// object `0044ddc0` says so, starts the slow motion: `time` goes to
/// `fTimeInSlowMoCam` (only if the cool-down `fKillCamCooldown` is not
/// positive when `queued` is set, which also sets the cool-down from the
/// setting `011d0a54`; with `queued` set and a positive cool-down nothing
/// happens). For setting 2 the third-person flag is recorded in `011f21d1`
/// and the VATS object is told (`009ca2c0`) and the gun scope hidden when the
/// player is aiming; for other settings the time is scaled by 0.4. Unless the
/// player has the `ActorValueOwner` value `0x33`, `0093e750` runs. The SEH
/// frame is not translated.
pub fn player_character_set_slow_mo_camera(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    mode: u32,
    time: f32,
    queued: bool,
    extra: u32,
) {
    if e.get(this, PlayerCharacter::eKillCameraSetting) == 0 {
        return;
    }
    if e.call(0x008d_6f30, &args![this]).u32() != 0 {
        let parent = e.call(0x008d_6f30, &args![this]).u32();
        if e.call(0x0084_e3a0, &args![parent]).u32() == 0x0016_1e98 {
            return;
        }
    }
    e.with_stack(24, |e, frame| {
        let handle = frame.addr();
        let temporary = handle + 12;
        e.call(SOUND_HANDLE_CONSTRUCT, &args![handle]);
        let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
        let found = e
            .call(
                AUDIO_GET_SOUND_HANDLE_BY_NAME,
                &args![audio, temporary, 0x0106_f370u32, 0x121u32],
            )
            .u32();
        e.call(SOUND_HANDLE_ASSIGN, &args![handle, found]);
        e.call(SOUND_HANDLE_DESTRUCT, &args![temporary]);
        e.call(SOUND_HANDLE_PLAY, &args![handle, 0u32]);
        if mode == 0 || e.call(0x0044_ddc0, &args![VATS_OBJECT]).u32() != 0 {
            e.call(SOUND_HANDLE_DESTRUCT, &args![handle]);
            return;
        }
        let zero: f64 = e.global(ZERO_DOUBLE);
        let cooldown = e.get(this, PlayerCharacter::fKillCamCooldown);
        if queued && (cooldown as f64) <= zero {
            let setting = e.call(SETTING_FLOAT_GETTER, &args![0x011d_0a54u32]).u32();
            let value = e.mem.f32(setting);
            e.set(this, PlayerCharacter::fKillCamCooldown, value);
        } else if queued {
            e.call(SOUND_HANDLE_DESTRUCT, &args![handle]);
            return;
        }
        e.set(this, PlayerCharacter::fTimeInSlowMoCam, time);
        if e.get(this, PlayerCharacter::eKillCameraSetting) == 2 {
            let flag = if e.get(this, PlayerCharacter::b3rdPersonSaved) {
                !e.get(this, PlayerCharacter::bsave3rdPerson)
            } else {
                let player = e.global::<u32>(PLAYER_SINGLETON);
                !e.call(0x004e_af60, &args![player]).bool()
            };
            e.set_global::<u8>(0x011f_21d1, flag as u8);
            e.call(0x009c_a2c0, &args![VATS_OBJECT, mode, extra]);
            if e.call(0x008b_bc10, &args![this]).bool() {
                e.call(0x0070_9c40, &args![0u32]);
            }
        } else {
            let scale: f64 = e.global(0x0102_90c0);
            let current = e.get(this, PlayerCharacter::fTimeInSlowMoCam);
            e.set(
                this,
                PlayerCharacter::fTimeInSlowMoCam,
                (current as f64 * scale) as f32,
            );
        }
        let player = e.global::<u32>(PLAYER_SINGLETON);
        if e.vcall(player + ACTOR_VALUE_OWNER_BASE, 8, &args![0x33u32])
            .i32()
            == 0
        {
            fn_0093e750(e);
        }
        e.call(SOUND_HANDLE_DESTRUCT, &args![handle]);
    });
}

// Translated from 0093e750 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the float at `011ac3a4` to `011ac3a8`.
pub fn fn_0093e750(e: &mut Engine) {
    let value: f32 = e.global(0x011a_c3a4);
    e.set_global(0x011a_c3a8, value);
}

// Translated from 0093e770 (decompiled, FalloutNV.exe 1.4.0.525)
/// Ends the slow-motion camera: restores the global time multiplier
/// ([`fn_0093e840`] on the game timer), and resets the first-/third-person
/// animation (`GetAnimation` for the opposite of `004eaf60`, whose animation
/// is told `1.0` by `0088b030`). For `mode` 2 (unless `flag` is clear and
/// `004a4040` holds) the VATS playback is quit and its data cleared, the VATS
/// mode set, and `011f21d0` set. `fTimeInSlowMoCam` is reset to 0.
pub fn fn_0093e770(e: &mut Engine, this: Ptr<PlayerCharacter>, mode: i32, flag: bool) {
    fn_0093e840(e, Ptr::new(GAME_TIMER));
    let third_person = e.call(0x004e_af60, &args![this]).bool();
    if e.call(0x0095_0a60, &args![this, !third_person]).u32() != 0 {
        let third_person = e.call(0x004e_af60, &args![this]).bool();
        let animation = e.call(0x0095_0a60, &args![this, !third_person]).u32();
        e.call(0x0088_b030, &args![animation, 1.0f32]);
    }
    if mode == 2 {
        if !flag && e.call(0x004a_4040, &args![]).bool() {
            e.set(this, PlayerCharacter::fTimeInSlowMoCam, 0.0);
            return;
        }
        let player = e.global::<u32>(PLAYER_SINGLETON);
        e.call(0x0095_f530, &args![player, 0u32, 3u32]);
        e.call(0x009c_8950, &args![VATS_OBJECT, 0u32, 1u32]);
        e.call(0x009c_6ba0, &args![VATS_OBJECT]);
        e.call(0x009c_6c30, &args![VATS_OBJECT, 0u32, 1u32]);
        e.set_global::<u8>(0x011f_21d0, 1);
    }
    e.set(this, PlayerCharacter::fTimeInSlowMoCam, 0.0);
}

// Translated from 0093e840 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00aa4db0` on the object (the game timer) with the float at
/// `011ac3a8` and 0 (`BSTimer::SetGlobalTimeMultiplier`).
pub fn fn_0093e840(e: &mut Engine, this: Ptr) {
    let multiplier: f32 = e.global(0x011a_c3a8);
    e.call(0x00aa_4db0, &args![this, multiplier, 0u32]);
}

/// Size of the frame block `Update` keeps its address-taken locals in, and
/// the distance from its start to the game's frame base (locals are addressed
/// with negative offsets from that base).
const UPDATE_FRAME_SIZE: u32 = 0x600;
const UPDATE_FRAME_LOCALS: u32 = 0x560;

/// The address of the local at the (negative) frame offset `offset`.
fn local(frame: u32, offset: i32) -> u32 {
    frame
        .wrapping_add(UPDATE_FRAME_LOCALS)
        .wrapping_add(offset as u32)
}

const COUNTDOWN_TIMER: u32 = 0x011e_07e8;
const VATS_ENDED_FLAG: u32 = 0x011f_21d0;
const VATS_ENDED_THIRD_PERSON: u32 = 0x011f_21d1;
const UPDATE_GUARD_FILE: u32 = 0x0108_a8e0;
const FORM_ID_8CD8C: u32 = 0x0008_cd8c;
const EFFECT_PARAMETER: u32 = 0x0101_7868;
const STATE_FLAG_011E07AA: u32 = 0x011e_07aa;
const STATE_FLAG_011E07B8: u32 = 0x011e_07b8;
const STATE_FLAG_011E07B9: u32 = 0x011e_07b9;
const STATE_FLAG_011E07C3: u32 = 0x011e_07c3;
const MOVE_SPEED_011A3B3C: u32 = 0x011a_3b3c;
const LOOK_SPEED_011A3B40: u32 = 0x011a_3b40;
const CAMERA_VALUE_011E07BC: u32 = 0x011e_07bc;
const CAMERA_VALUE_011E07C4: u32 = 0x011e_07c4;
const CAMERA_VALUE_011E0B5C: u32 = 0x011e_0b5c;
const HELD_TIMER_011E07E0: u32 = 0x011e_07e0;
const HELD_FLAG_011E0BC1: u32 = 0x011e_0bc1;
const HOLD_VALUE_011E07AC: u32 = 0x011e_07ac;
const HOLD_VALUE_011E07B0: u32 = 0x011e_07b0;
const ORBIT_SCALE: u32 = 0x0102_3128;
const ORBIT_VALUE_011E08FC: u32 = 0x011e_08fc;
const ORBIT_VALUE_011E08F4: u32 = 0x011e_08f4;
const ORBIT_ANGLE_011E0BB8: u32 = 0x011e_0bb8;
const IDLE_OBJECT_011E09E0: u32 = 0x011e_09e0;
const MARKER_OBJECT_011E07EC: u32 = 0x011e_07ec;
const MARKER_COUNT_011E0BB4: u32 = 0x011e_0bb4;
const MARKER_OFFSET_XY: u32 = 0x0102_40c0;
const MARKER_OFFSET_Z: u32 = 0x0102_e430;

const ACTIVATE_INTERVAL_SETTING: u32 = 0x011c_d5c8;
const ACTIVATE_TIMER_011E0BC4: u32 = 0x011e_0bc4;
const ALERT_TIME_011E0BA8: u32 = 0x011e_0ba8;
const AMMO_SWAP_SETTING: u32 = 0x011c_d158;
const AUDIO_LEVEL_LIMIT: u32 = 0x0106_d508;
const AXIS_DIVISOR: u32 = 0x0101_7a40;
const CAMERA_FLAG_011A3B31: u32 = 0x011a_3b31;
const CAMERA_SETTING_011CD62C: u32 = 0x011c_d62c;
const CAMERA_SETTING_011CDE8C: u32 = 0x011c_de8c;
const CAMERA_TIMER_011E07C8: u32 = 0x011e_07c8;
const CAMERA_VALUE_011E0B58: u32 = 0x011e_0b58;
const CAMERA_VALUE_011E0B60: u32 = 0x011e_0b60;
const CAMERA_VALUE_011E0BBC: u32 = 0x011e_0bbc;
const CELL_CHANGE_STRING: u32 = 0x0108_b028;
const CELL_MANAGER_011C3B3C: u32 = 0x011c_3b3c;
const CHASE_SETTING: u32 = 0x011d_0b08;
const CLEARED_EACH_FRAME_011E077C: u32 = 0x011e_077c;
const COMBAT_SETTING_011CE3A8: u32 = 0x011c_e3a8;
const CONTROL_COUNTER_011E0798: u32 = 0x011e_0798;
const CONTROL_COUNTER_LIMIT: u32 = 0x0101_5a38;
const CROUCH_DOWN_SOUND_NAME: u32 = 0x0108_b050;
const CROUCH_UP_SOUND_NAME: u32 = 0x0108_b064;
const FACE_FACTOR_011E0BAC: u32 = 0x011e_0bac;
const FACE_RELAX_01016248: u32 = 0x0101_6248;
const FLAG_SETTING_011E0B64: u32 = 0x011e_0b64;
const HELD_SETTING_011CDFCC: u32 = 0x011c_dfcc;
const HELLO_SCALE: u32 = 0x0101_7b70;
const HELLO_SETTING_011D03A0: u32 = 0x011d_03a0;
const IDLE_SETTING_011CCF50: u32 = 0x011c_cf50;
const INTERACT_DISTANCE_01084D20: u32 = 0x0108_4d20;
const INT_SETTING_011CDDE0: u32 = 0x011c_dde0;
const ITEM_CHECK_INTERVAL_SETTING: u32 = 0x011d_0658;
const KEY_AXIS_LIMIT: u32 = 0x0106_6808;
const LOOK_SCALE: u32 = 0x0101_6ff0;
const MESSAGE_SETTING_011D20AC: u32 = 0x011d_20ac;
const MESSAGE_SETTING_011D28B0: u32 = 0x011d_28b0;
const MESSAGE_SETTING_011D2E50: u32 = 0x011d_2e50;
const MESSAGE_SETTING_011D3E20: u32 = 0x011d_3e20;
const MESSAGE_SETTING_011D400C: u32 = 0x011d_400c;
const MESSAGE_SETTING_011D4474: u32 = 0x011d_4474;
const MESSAGE_SETTING_011D4528: u32 = 0x011d_4528;
const MESSAGE_SETTING_011D47D4: u32 = 0x011d_47d4;
const MESSAGE_SETTING_011D4948: u32 = 0x011d_4948;
const MESSAGE_SETTING_011D4B10: u32 = 0x011d_4b10;
const MESSAGE_SETTING_011D4B40: u32 = 0x011d_4b40;
const MESSAGE_SHOWN_011E07C0: u32 = 0x011e_07c0;
const ONE_DOUBLE: u32 = 0x0101_2070;
const ORBIT_SETTING_011CD198: u32 = 0x011c_d198;
const ORBIT_SETTING_011CD4CC: u32 = 0x011c_d4cc;
const ORBIT_SETTING_011CD984: u32 = 0x011c_d984;
const PENDING_OBJECT_011E0788: u32 = 0x011e_0788;
const QUICK_MENU_SOUND_NAME: u32 = 0x0107_7d84;
const QUICK_STATE_011E0780: u32 = 0x011e_0780;
const RUN_WEIGHT_SETTING: u32 = 0x011d_097c;
const SAVED_VIEW_011E0BC0: u32 = 0x011e_0bc0;
const SLOW_MOTION_ANIMATION_SPEED: u32 = 0x0108_b084;
const SLOW_MOTION_TIME_SCALE: u32 = 0x0101_7870;
const SORT_ACTOR_DISTANCE_SETTING: u32 = 0x011c_d8f0;
const STATE_FLAG_011E07C1: u32 = 0x011e_07c1;
const STATIC_FLAGS_011E0BC8: u32 = 0x011e_0bc8;
const STICK_FACTOR: u32 = 0x0108_b044;
const STICK_REMAP_A: u32 = 0x0101_8238;
const STICK_REMAP_C: u32 = 0x0103_0ff0;
const STICK_REMAP_D: u32 = 0x0101_e2bc;
const STICK_REMAP_E: u32 = 0x0105_3154;
const STICK_SCALE_SETTING: u32 = 0x011e_0b94;
const STICK_THRESHOLD_HIGH: u32 = 0x0107_d1b8;
const STICK_THRESHOLD_LOW: u32 = 0x0102_17d8;
const TIMER_011A3B34: u32 = 0x011a_3b34;
const TRACKED_CELL_011E0BB0: u32 = 0x011e_0bb0;
const TURBO_MODIFIER_NAME: u32 = 0x0108_b078;
const TURBO_TIME_SCALE_SETTING: u32 = 0x011d_0fa8;
const VATS_AUDIO_SETTING: u32 = 0x011c_a7c0;
const VATS_SOUND_HANDLE: u32 = 0x011f_6df0;
const VATS_SOUND_NAME: u32 = 0x0108_b088;
const WALK_SPEED_SETTING: u32 = 0x011e_0b78;
const YIELD_SETTING_011CEDB8: u32 = 0x011c_edb8;
/// `Actor::pCurrentProcess` of the object.
fn process_of(e: &Engine, this: Ptr<PlayerCharacter>) -> u32 {
    e.get(this.cast::<Actor>(), Actor::pCurrentProcess).addr()
}

/// `pCurrentProcess` of the player singleton.
fn singleton_process(e: &Engine) -> u32 {
    let player = e.global::<u32>(PLAYER_SINGLETON);
    e.get(Ptr::<Actor>::new(player), Actor::pCurrentProcess)
        .addr()
}

/// Sets bits in the 16-bit word at `slot`.
fn or_word(e: &mut Engine, slot: u32, bits: u16) {
    let value = e.mem.u16(slot);
    e.mem.set_u16(slot, value | bits);
}

/// Keeps only the bits of `mask` in the 16-bit word at `slot`.
fn and_word(e: &mut Engine, slot: u32, mask: u16) {
    let value = e.mem.u16(slot);
    e.mem.set_u16(slot, value & mask);
}

/// Sets the animation speed `strength` on the animation selected by the
/// player's third-person flag (or, with `opposite`, by its negation).
fn refresh_animation_speed(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    strength: f32,
    opposite: bool,
) {
    let flag = e.call(0x004e_af60, &args![this]).bool();
    let animation = e
        .call(
            0x0095_0a60,
            &args![this, if opposite { !flag } else { flag }],
        )
        .u32();
    e.call(0x0088_b030, &args![animation, strength]);
}

/// Prints the text of the setting object `setting` as a message with `icon`.
fn show_text(e: &mut Engine, setting: u32, icon: u32) {
    let text = e.call(0x0040_3df0, &args![setting]).u32();
    let duration: f32 = 2.0;
    e.call(0x0070_52f0, &args![text, 0u32, icon, 0u32, duration, 0u32]);
}

// Translated from 0093e860 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::Update` (Xbox PDB): the per-frame update of the player
/// with the frame time `delta`. In order: the VATS and slow-motion
/// bookkeeping and sounds, cool-down timers, the queued weapon attach, the
/// warning timers, a forced activation (which ends the update), the process
/// flags, the view switch (which also ends the update), the controlled player
/// (which ends the update after the activation checks), the input handling
/// (crouch toggle, movement from keys or stick, activation, menus), the
/// camera orbit and idle timers, then the combat, ammo, item-alert and hello
/// timers. The scope guard and the sound handle the function keeps are
/// released at every exit. The SEH frame is not translated; the block at
/// `009411e9..0094134c`, which only runs without a gamepad while the axis
/// values are still 0, is left out.
pub fn player_character_update(e: &mut Engine, this: Ptr<PlayerCharacter>, delta: f32) {
    e.with_stack(UPDATE_FRAME_SIZE, |e, frame| {
        update_in_frame(e, this, delta, frame.addr());
    });
}

#[allow(clippy::cognitive_complexity, clippy::if_same_then_else)]
fn update_in_frame(e: &mut Engine, this: Ptr<PlayerCharacter>, delta: f32, frame: u32) {
    let guard = local(frame, -0x50);
    let handle = local(frame, -0x78);
    'update: {
        // ---- 0093e860..0093ef51: VATS/slow-motion bookkeeping, timers ----
        let main = e.global::<u32>(MAIN_SINGLETON);
        let main_state = e.call(0x0087_7720, &args![main]).u32(); // -0x1c
        let zero: f64 = e.global(ZERO_DOUBLE);
        let countdown: f32 = e.global(COUNTDOWN_TIMER);
        e.set_global(COUNTDOWN_TIMER, (countdown as f64 - delta as f64) as f32);
        if e.global::<u8>(VATS_ENDED_FLAG) != 0 {
            let player = e.global::<u32>(PLAYER_SINGLETON);
            let flag = e.global::<u8>(VATS_ENDED_THIRD_PERSON) != 0;
            e.call(0x0095_0110, &args![player, flag as u32]);
            e.set_global::<u8>(VATS_ENDED_FLAG, 0);
        }
        let in_vats = e.call(0x0044_ddc0, &args![VATS_OBJECT]).u32() != 0; // -0x20
        let level_limit: f64 = e.global(AUDIO_LEVEL_LIMIT);
        if !in_vats {
            e.call(AUDIO_INSTANCE, &args![]);
            if e.call(SOUND_HANDLE_IS_VALID, &args![VATS_SOUND_HANDLE])
                .bool()
            {
                e.call(AUDIO_INSTANCE, &args![]);
                e.call(
                    SOUND_HANDLE_FADE_OUT_AND_RELEASE,
                    &args![VATS_SOUND_HANDLE, 0x5dcu32],
                );
            }
            let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
            let level = e.call(0x00ad_7c80, &args![audio]).f64();
            if level < level_limit {
                let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
                e.call(0x00ad_7ec0, &args![audio]);
            }
        } else {
            let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
            let level = e.call(0x00ad_7c80, &args![audio]).f64();
            if level > level_limit && (setting_float(e, VATS_AUDIO_SETTING) as f64) < level_limit {
                let volume = setting_float(e, VATS_AUDIO_SETTING);
                let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
                e.call(0x00ad_7da0, &args![audio, volume]);
                if !e
                    .call(SOUND_HANDLE_IS_VALID, &args![VATS_SOUND_HANDLE])
                    .bool()
                {
                    e.with_stack(12, |e, found| {
                        let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
                        let result = e
                            .call(
                                AUDIO_GET_SOUND_HANDLE_BY_NAME,
                                &args![audio, found, VATS_SOUND_NAME, 0x31u32],
                            )
                            .u32();
                        e.call(SOUND_HANDLE_ASSIGN, &args![VATS_SOUND_HANDLE, result]);
                        e.call(SOUND_HANDLE_DESTRUCT, &args![found]);
                    });
                }
                e.call(SOUND_HANDLE_PLAY, &args![VATS_SOUND_HANDLE, 0u32]);
            }
        }
        let cooldown = e.get(this, PlayerCharacter::fKillCamCooldown);
        if cooldown as f64 > zero {
            e.set(
                this,
                PlayerCharacter::fKillCamCooldown,
                (cooldown as f64 - delta as f64) as f32,
            );
        }
        let mut slow_mo_running = false; // -0x35
        let slow_mo_time = e.get(this, PlayerCharacter::fTimeInSlowMoCam);
        if slow_mo_time as f64 > zero {
            if e.call(0x00a2_4660, &args![main_state, 5u32, 1u32]).u32() != 0 {
                e.set(this, PlayerCharacter::fTimeInSlowMoCam, 0.0);
            }
            let remaining = e.get(this, PlayerCharacter::fTimeInSlowMoCam);
            e.set(
                this,
                PlayerCharacter::fTimeInSlowMoCam,
                (remaining as f64 - delta as f64) as f32,
            );
            let slow_scale: f32 = e.global(SLOW_MOTION_TIME_SCALE);
            e.call(0x00aa_4db0, &args![GAME_TIMER, slow_scale, 0u32]);
            if e.get(this, PlayerCharacter::bIgnoresGTM) {
                let strength: f32 = e.global(SLOW_MOTION_ANIMATION_SPEED);
                refresh_animation_speed(e, this, strength, true);
                let strength: f32 = e.global(SLOW_MOTION_ANIMATION_SPEED);
                refresh_animation_speed(e, this, strength, false);
            }
            let remaining = e.get(this, PlayerCharacter::fTimeInSlowMoCam);
            if remaining as f64 > zero || remaining.is_nan() {
                slow_mo_running = true;
            } else {
                let setting = e.get(this, PlayerCharacter::eKillCameraSetting) as i32;
                fn_0093e770(e, this, setting, false);
            }
        } else {
            refresh_animation_speed(e, this, 1.0, true);
            refresh_animation_speed(e, this, 1.0, false);
        }
        let counter_timer = e.call(0x0094_43a0, &args![this]).f64();
        if counter_timer > zero {
            let counter_timer = e.call(0x0094_43a0, &args![this]).f64();
            e.call(
                0x0094_4380,
                &args![this, (counter_timer - delta as f64) as f32],
            );
        }
        let has_perk = e
            .vcall(this.addr() + ACTOR_VALUE_OWNER_BASE, 8, &args![0x33u32])
            .i32();
        if has_perk > 0 && !slow_mo_running {
            if !e.get(this, PlayerCharacter::bTurboISM) {
                fn_0093e750(e);
                let modifier = e.call(0x0048_3a00, &args![TURBO_MODIFIER_NAME]).u32();
                e.call(0x0052_99a0, &args![modifier, 1.0f32, 0u32]);
                e.set(this, PlayerCharacter::bTurboISM, true);
            }
            let scale = setting_float(e, TURBO_TIME_SCALE_SETTING);
            e.call(0x00aa_4db0, &args![GAME_TIMER, scale, 0u32]);
            let ratio = e.call(0x0071_6440, &args![]).f64();
            let speed = (1.0 / ratio) as f32;
            refresh_animation_speed(e, this, speed, true);
            refresh_animation_speed(e, this, speed, false);
        } else if !slow_mo_running {
            let modifier = e.call(0x0048_3a00, &args![TURBO_MODIFIER_NAME]).u32();
            e.call(0x0052_9c90, &args![modifier]);
            if e.get(this, PlayerCharacter::bTurboISM) {
                fn_0093e840(e, Ptr::new(GAME_TIMER));
            }
            e.set(this, PlayerCharacter::bTurboISM, false);
        }
        let attach = e.get(this, PlayerCharacter::pQueuedWeaponAttach);
        if !attach.is_null() {
            let process = e.get(this.cast::<Actor>(), Actor::pCurrentProcess).addr();
            if !e.vcall(process, 0x474, &args![1u32]).bool() {
                let attach = e.get(this, PlayerCharacter::pQueuedWeaponAttach);
                e.call(0x0057_1760, &args![this, attach]);
                e.set(this, PlayerCharacter::pQueuedWeaponAttach, Ptr::NULL);
            }
        }
        if e.get(this, PlayerCharacter::bBlockActivate) && !e.call(0x0094_43c0, &args![main]).bool()
        {
            let timer = e.get(this, PlayerCharacter::fBlockActivateTimer);
            let timer = (timer as f64 - delta as f64) as f32;
            e.set(this, PlayerCharacter::fBlockActivateTimer, timer);
            if (timer as f64) < zero {
                e.set(this, PlayerCharacter::fBlockActivateTimer, 0.0);
                e.set(this, PlayerCharacter::bBlockActivate, false);
            }
        }
        e.set_global(CLEARED_EACH_FRAME_011E077C, 0u32);
        let targeted = e
            .call(
                0x0055_9450,
                &args![member(this, PlayerCharacter::spTargeted3D)],
            )
            .u32();
        let target = e.call(0x0056_f930, &args![targeted]).u32(); // -0x4c
        let distant = e.get(this, PlayerCharacter::bTarget3DDistant);
        e.call(0x0077_2d10, &args![target, distant as u32]);
        e.call(
            NI_POINTER_ASSIGN,
            &args![member(this, PlayerCharacter::spTargeted3D), 0u32],
        );
        e.call(0x008b_bdb0, &args![this]);
        e.call(
            0x0040_4eb0,
            &args![guard, 0x34u32, 1u32, UPDATE_GUARD_FILE, 0xc41u32],
        );
        let root = e.call(0x0045_c670, &args![]).u32(); // -0x54
        let process = e.get(this.cast::<Actor>(), Actor::pCurrentProcess).addr();
        let combat_target = e.vcall(process, 0x148, &args![]).u32();
        let combat_target_state = if combat_target != 0 {
            e.call(0x0044_ddc0, &args![combat_target]).u32()
        } else {
            0
        }; // -0x5c
        let animation_third_person = e.call(0x0095_0a60, &args![this, 0u32]).u32(); // -0x60
        let animation_first_person = e.call(0x0095_0a60, &args![this, 1u32]).u32(); // -0x64
        let mut moved_flag = false; // -0x65
        e.mem.set_u16(local(frame, -0x6c), 0);
        e.call(SOUND_HANDLE_CONSTRUCT, &args![handle]);
        e.set(this, PlayerCharacter::fDropAngleMod, 0.0);
        e.set(this, PlayerCharacter::fLastDropAngleMod, 0.0);
        let actor_timer = e.get(this.cast::<Actor>(), Actor::fTimeronAction);
        if actor_timer as f64 > zero {
            let elapsed = e.call(0x0084_d030, &args![GAME_TIMER]).f64();
            let actor_timer = e.get(this.cast::<Actor>(), Actor::fTimeronAction);
            e.set(
                this.cast::<Actor>(),
                Actor::fTimeronAction,
                (actor_timer as f64 - elapsed) as f32,
            );
        } else {
            e.set(this.cast::<Actor>(), Actor::iActionValue, 0);
        }
        e.call(0x0096_2d00, &args![this]);
        let process = e.get(this.cast::<Actor>(), Actor::pCurrentProcess).addr();
        e.vcall(process, 0x464, &args![this]);
        if e.call(0x0096_7950, &args![this]).u32() != 0 {
            let item = e.call(0x0096_7950, &args![this]).u32();
            let duration: f32 = e.global(TWO_FLOAT);
            e.call(0x0096_3eb0, &args![this, 7u32, duration, item]);
        }
        if e.call(0x0096_7a00, &args![this]).u32() != 0 {
            let actor_timer = e.get(this.cast::<Actor>(), Actor::fTimeronAction);
            if actor_timer as f64 <= zero {
                let setting = e.call(0x0043_d4d0, &args![INT_SETTING_011CDDE0]).u32();
                let value = e.mem.u32(setting);
                e.call(0x008b_c240, &args![this, value]);
            }
        }
        let cached = e.call(0x008b_bc10, &args![this]).bool(); // -0x79
        let process = e.get(this.cast::<Actor>(), Actor::pCurrentProcess).addr();
        let combat_process_flag = cached && e.vcall(process, 0x454, &args![]).bool();
        if combat_process_flag {
            let tes = e.global::<u32>(TES_SINGLETON);
            let mut dark = false; // -0x7a
            let parent = e.call(0x008d_6f30, &args![this]).u32();
            if e.call(CELL_IS_INTERIOR, &args![parent]).bool() {
                dark = true;
            } else {
                let sky = e.call(0x008d_8520, &args![tes]).u32(); // -0x80
                let first = e.call(0x0096_6a20, &args![sky]).f32();
                let second = e.call(0x0059_5f50, &args![sky]).f32();
                let third = e.call(0x0059_5fc0, &args![sky]).f32();
                // `third <= first`, else `second >= first` (x87 compares).
                if (third as f64) <= first as f64 || (second as f64) >= first as f64 {
                    dark = true;
                }
            }
            if e.call(0x0044_ddc0, &args![VATS_OBJECT]).u32() != 0
                || e.get(this, PlayerCharacter::fTimeInSlowMoCam) as f64 > zero
            {
                dark = false;
            }
            let process = e.get(this.cast::<Actor>(), Actor::pCurrentProcess).addr();
            let combat_object = e.vcall(process, 0x148, &args![]).u32(); // -0x90
            let combat_state = if combat_object != 0 {
                e.call(0x0044_ddc0, &args![combat_object]).u32()
            } else {
                0
            }; // -0x98
            let mut applied = false;
            if combat_state != 0
                && e.call(0x0094_42c0, &args![combat_state]).bool()
                && !e.get(this, PlayerCharacter::bNightVisionOn)
                && dark
            {
                let ready = if e.call(0x004a_d030, &args![combat_state]).bool() {
                    e.call(0x004b_da70, &args![combat_object, 0x0eu32]).bool()
                } else {
                    true
                };
                if ready {
                    e.set(this, PlayerCharacter::bNightVisionOn, true);
                    let modifier = e.call(0x0048_39c0, &args![FORM_ID_8CD8C]).u32();
                    e.call(0x0052_99a0, &args![modifier, 1.0f32, 0u32]);
                    applied = true;
                }
            }
            if !applied && !dark {
                let modifier = e.call(0x0048_39c0, &args![FORM_ID_8CD8C]).u32();
                e.call(0x0052_9c90, &args![modifier]);
                e.call(0x0096_3e00, &args![this, 9u32, 0u32]);
                e.set(this, PlayerCharacter::bNightVisionOn, false);
            }
            if e.call(0x0096_4100, &args![this, 9u32]).u32() == 0 {
                let strength: f32 = e.global(EFFECT_PARAMETER);
                e.call(0x0096_3eb0, &args![this, 9u32, strength, 0u32]);
            }
        } else {
            if e.get(this, PlayerCharacter::bNightVisionOn) {
                let modifier = e.call(0x0048_39c0, &args![FORM_ID_8CD8C]).u32();
                e.call(0x0052_9c90, &args![modifier]);
                e.set(this, PlayerCharacter::bNightVisionOn, false);
            }
            e.call(0x0096_3e00, &args![this, 9u32, 0u32]);
        }
        if cached
            && !e.get(this, PlayerCharacter::bHasSpotterActive)
            && e.call(0x0044_ddc0, &args![VATS_OBJECT]).u32() != 4
        {
            let delay = e.with_stack(4, |e, slot| {
                e.mem.set_f32(slot.addr(), 0.0);
                let player = e.global::<u32>(PLAYER_SINGLETON);
                e.call(0x005e_58f0, &args![0x46u32, player, slot]);
                e.mem.f32(slot.addr())
            });
            if delay as f64 > zero {
                e.set(this, PlayerCharacter::bHasSpotterActive, true);
                fn_0093dd80(e, this);
            }
        } else if e.get(this, PlayerCharacter::bHasSpotterActive) && !cached {
            let handler = e.global::<u32>(DATA_HANDLER);
            let object = e.call(0x007b_8f10, &args![handler]).u32();
            e.call(0x0097_4af0, &args![PROCESS_LISTS, object]);
            e.set(this, PlayerCharacter::bHasSpotterActive, false);
        }
        let owner = this.addr() + ACTOR_VALUE_OWNER_BASE;
        if e.vcall(owner, 8, &args![0x32u32]).i32() > 0 {
            e.set(this, PlayerCharacter::bHasCateyeActive, true);
        } else if e.get(this, PlayerCharacter::bHasCateyeActive)
            && e.vcall(owner, 8, &args![0x32u32]).i32() <= 0
        {
            let handler = e.global::<u32>(DATA_HANDLER);
            let object = e.call(0x0089_f4e0, &args![handler]).u32();
            e.call(0x0097_4af0, &args![PROCESS_LISTS, object]);
            e.set(this, PlayerCharacter::bHasCateyeActive, false);
        }
        let player = e.global::<u32>(PLAYER_SINGLETON);
        let item_delay = e.with_stack(4, |e, slot| {
            e.mem.set_f32(slot.addr(), 0.0);
            e.call(0x005e_58f0, &args![0x47u32, player, slot]);
            e.mem.f32(slot.addr())
        });
        let check = e.get(this, PlayerCharacter::fCheckForItems);
        let check = (check as f64 - delta as f64) as f32;
        e.set(this, PlayerCharacter::fCheckForItems, check);
        if item_delay as f64 > zero && check as f64 <= zero && cached {
            let interval = setting_float(e, ITEM_CHECK_INTERVAL_SETTING);
            e.set(this, PlayerCharacter::fCheckForItems, interval);
            fn_0093df50(e, this);
        }
        e.call(0x0096_3bf0, &args![this]);
        if e.call(0x004d_1360, &args![this]).bool() {
            e.call(0x0096_9c30, &args![this]);
        }
        let sky_process = e.call(0x008d_8520, &args![this]).u32();
        e.vcall(sky_process, 0x500, &args![this, 0u32]);
        if e.call(0x0096_78a0, &args![this]).bool()
            && e.vcall(this.addr(), 0x214, &args![]).u32() == 0
        {
            if e.call(0x0096_4100, &args![this, 8u32]).u32() == 0 {
                let strength: f32 = e.global(EFFECT_PARAMETER);
                e.call(0x0096_3eb0, &args![this, 8u32, strength, 0u32]);
            }
        } else {
            e.call(0x0096_3e00, &args![this, 8u32, 0u32]);
        }
        let controller = e.call(0x0093_06d0, &args![this]).u32();
        if controller != 0 && e.call(0x0094_4430, &args![controller]).bool() {
            if !e.get(this, PlayerCharacter::bOnElevator) {
                e.call(0x0097_3de0, &args![PROCESS_LISTS]);
                e.set(this, PlayerCharacter::bOnElevator, true);
            }
        } else {
            e.set(this, PlayerCharacter::bOnElevator, false);
        }
        let mover_process = e.call(0x008d_8520, &args![this]).u32();
        let speed_factor = e.vcall(mover_process, 0x65c, &args![]).f64();
        if speed_factor > zero {
            let mover_process = e.call(0x008d_8520, &args![this]).u32();
            e.vcall(mover_process, 0x658, &args![]);
        } else {
            let mover_process = e.call(0x008d_8520, &args![this]).u32();
            e.vcall(mover_process, 0x4a4, &args![0.0f32]);
        }
        if e.get(this, PlayerCharacter::bBeingChased) {
            let value = e.vcall(this.addr(), 0x2b4, &args![]).f64();
            if value <= zero {
                e.call(0x0096_2190, &args![this]);
                let tes_object = e.call(0x008d_8520, &args![this]).u32();
                let amount = setting_float(e, CHASE_SETTING);
                e.vcall(tes_object, 0x2c8, &args![amount]);
            } else {
                e.vcall(this.addr(), 0x2b8, &args![]);
            }
        }
        // The warning timers count down; once they run out they are set to -1.
        let reset: f32 = e.global(TIMER_RESET);
        let steal = e.get(this, PlayerCharacter::fStealWarningTimer);
        if steal as f64 > zero {
            e.set(
                this,
                PlayerCharacter::fStealWarningTimer,
                (steal as f64 - delta as f64) as f32,
            );
        } else {
            e.set(this, PlayerCharacter::fStealWarningTimer, reset);
        }
        let pickpocket = e.get(this, PlayerCharacter::fPickPocketWarningTimer);
        if pickpocket as f64 > zero {
            e.set(
                this,
                PlayerCharacter::fPickPocketWarningTimer,
                (pickpocket as f64 - delta as f64) as f32,
            );
        } else {
            e.set(this, PlayerCharacter::fPickPocketWarningTimer, reset);
        }
        let sort_timer = e.get(this, PlayerCharacter::fSortActorDistanceTimer);
        if sort_timer as f64 > zero || sort_timer.is_nan() {
            e.set(
                this,
                PlayerCharacter::fSortActorDistanceTimer,
                (sort_timer as f64 - delta as f64) as f32,
            );
        } else {
            e.call(0x0096_e570, &args![PROCESS_LISTS]);
            let interval = setting_float(e, SORT_ACTOR_DISTANCE_SETTING);
            e.set(this, PlayerCharacter::fSortActorDistanceTimer, interval);
        }
        if e.get(this, PlayerCharacter::bReturnToLastKnownGoodPosition) {
            e.set(this, PlayerCharacter::bReturnToLastKnownGoodPosition, false);
            e.call(0x0094_dbe0, &args![this, 1u32]);
        }
        let force_activate = e.call(0x0094_4320, &args![this]).u32();
        if force_activate != 0 {
            let force_activate = e.call(0x0094_4320, &args![this]).u32();
            e.call(0x0057_3170, &args![force_activate, this, 0u32, 0u32, 1u32]);
            e.call(0x0051_9020, &args![this, 0u32]);
            break 'update;
        }
        let tes_object = e.call(0x008d_8520, &args![this]).u32();
        e.vcall(tes_object, 0xd4, &args![this]);
        let process = process_of(e, this);
        let process_flags = e.vcall(process, 0x618, &args![]).u32(); // -0xcc
        let process = process_of(e, this);
        e.vcall(process, 0x61c, &args![]);
        if process_flags & 4 != 0 {
            let process = process_of(e, this);
            if !e.vcall(process, 0x2b0, &args![this]).bool() {
                let process = process_of(e, this);
                e.vcall(process, 0x708, &args![0u32]);
            }
        }
        if process_flags & 8 != 0 {
            let process = process_of(e, this);
            if !e.vcall(process, 0x2b8, &args![this]).bool() {
                let process = process_of(e, this);
                e.vcall(process, 0x708, &args![0u32]);
            }
        }
        let state_flags = e.call(0x0088_46e0, &args![this]).u16(); // -0xd0
        let one: f64 = e.global(ONE_DOUBLE);
        if e.call(0x0049_97b0, &args![this]).bool() && e.call(0x0049_38e0, &args![this]).bool() {
            if e.call(0x0070_38a0, &args![]).i32() < 3
                && e.get(this, PlayerCharacter::bActorinSneakRange)
            {
                let sneaking = e.get(this, PlayerCharacter::fsecondSneaking);
                if sneaking as f64 > one {
                    e.vcall(this.addr(), 0x47c, &args![0x2au32, 0u32]);
                    e.set(this, PlayerCharacter::fsecondSneaking, 0.0);
                } else {
                    e.set(
                        this,
                        PlayerCharacter::fsecondSneaking,
                        (sneaking as f64 + delta as f64) as f32,
                    );
                }
            }
        } else if e.call(0x005a_2030, &args![this]).bool()
            && e.call(0x0049_38e0, &args![this]).bool()
        {
            let swimming = e.get(this, PlayerCharacter::fsecondSwimming);
            if swimming as f64 > one {
                e.set(this, PlayerCharacter::fsecondSwimming, 0.0);
            } else {
                e.set(
                    this,
                    PlayerCharacter::fsecondSwimming,
                    (swimming as f64 + delta as f64) as f32,
                );
            }
        } else if state_flags & 0x200 != 0 && e.call(0x0049_38e0, &args![this]).bool() {
            let running = e.get(this, PlayerCharacter::fsecondRunning);
            if running as f64 > one {
                e.set(this, PlayerCharacter::fsecondRunning, 0.0);
            } else {
                e.set(
                    this,
                    PlayerCharacter::fsecondRunning,
                    (running as f64 + delta as f64) as f32,
                );
            }
        }
        let word_slot = local(frame, -0x6c);
        e.mem.set_u16(word_slot, state_flags & 0xcc00);
        let penetration = e
            .get(this.cast::<Actor>(), Actor::pPenetrationDetection)
            .addr();
        e.call(0x00ca_1410, &args![penetration]);
        if e.call(0x0094_45b0, &args![this, delta, 0u32, word_slot, 0u32])
            .bool()
        {
            moved_flag = true;
        }
        let blocked_a =
            e.global::<u8>(STATE_FLAG_011E07B8) != 0 || e.global::<u8>(STATE_FLAG_011E07C1) != 0;
        let view_switch_allowed = if blocked_a {
            let process = process_of(e, this);
            e.vcall(process, 0x610, &args![]).u32() != 0
        } else {
            true
        };
        if view_switch_allowed && {
            let process = process_of(e, this);
            e.call(0x008f_ec10, &args![process, this]).bool()
        } {
            // The player's view is switched by the process; refresh both views.
            if e.call(
                0x0055_9450,
                &args![member(this, PlayerCharacter::spGrabSpring)],
            )
            .u32()
                != 0
            {
                e.call(0x0095_f6a0, &args![this]);
            }
            e.call(0x0094_81d0, &args![this]);
            for _ in 0..2 {
                let toggled = !e.get(this, PlayerCharacter::b3rdPerson);
                e.set(this, PlayerCharacter::b3rdPerson, toggled);
                e.call(0x008d_3550, &args![this, delta]);
                let first_person = !e.get(this, PlayerCharacter::b3rdPerson);
                let animation = e.call(0x0095_0a60, &args![this, first_person]).u32();
                e.call(0x0088_85e0, &args![this, animation, delta]);
            }
            e.call(0x0094_ae40, &args![this, 0u32, 0u32]);
            break 'update;
        }
        let tes_object = e.call(0x008d_8520, &args![this]).u32();
        let mut attack_ok = false; // -0xdd; only written
        if e.vcall(tes_object, 0x6dc, &args![]).bool()
            && !e.call(0x0052_5430, &args![VATS_OBJECT]).bool()
        {
            let process = process_of(e, this);
            if e.vcall(process, 0x148, &args![]).u32() != 0 {
                let kind = e.call(0x008a_7570, &args![this]).u32();
                if (2..=6).contains(&kind) {
                    let process = process_of(e, this);
                    let target = e.vcall(process, 0x148, &args![]).u32();
                    let target_state = e.call(0x0044_ddc0, &args![target]).u32();
                    if target_state != 0 {
                        attack_ok = true;
                        let player = e.global::<u32>(PLAYER_SINGLETON);
                        if e.call(0x0052_5980, &args![target_state, player]).u32() != 0 {
                            let process = process_of(e, this);
                            let list = e.vcall(process, 0x14c, &args![]).u32();
                            if list == 0 {
                                attack_ok = false;
                            } else {
                                let count = e.call(0x0072_6070, &args![list]).i32();
                                let limit = e.call(0x0052_4b60, &args![target_state]).u8() as i32;
                                if count < limit {
                                    attack_ok = false;
                                }
                            }
                        }
                    }
                }
            }
        }
        let _ = attack_ok;
        let process = process_of(e, this);
        if e.vcall(process, 0x40c, &args![]).u32() != 0
            || e.vcall(this.addr(), 0x234, &args![]).bool()
        {
            e.call(0x009c_8950, &args![VATS_OBJECT, 0u32, 0u32]);
            e.set(this, PlayerCharacter::bAiControlledActivate, false);
            e.set_global(PENDING_OBJECT_011E0788, 0u32);
            if !e.get(this, PlayerCharacter::b3rdPerson) {
                e.call(0x0095_0340, &args![this, 1u32]);
            }
        }
        let controlled = fn_0093a740(e, this) || e.vcall(this.addr(), 0x22c, &args![0u32]).bool();
        if controlled {
            if e.get(this, PlayerCharacter::bActually3rdPerson)
                && !e.get(this, PlayerCharacter::b3rdPerson)
                && e.call(0x005a_03f0, &args![this, 0x10u32]).bool()
            {
                e.call(0x0095_1a10, &args![this, 1u32]);
            }
            let mover = e.get(this.cast::<Actor>(), Actor::pActorMover).addr();
            let word_value = e.mem.u16(word_slot) as u32;
            e.vcall(mover, 0xc, &args![word_value]);
            let counter: f32 = e.global(CONTROL_COUNTER_011E0798);
            let counter_limit: f64 = e.global(CONTROL_COUNTER_LIMIT);
            let in_state = e.vcall(this.addr(), 0x214, &args![]).u32();
            if counter as f64 > counter_limit {
                let in_state_again = if in_state != 0 {
                    e.vcall(this.addr(), 0x214, &args![]).u32()
                } else {
                    0
                };
                if in_state == 0 || in_state_again == 4 {
                    if fn_0093a640(e, this) && e.vcall(this.addr(), 0x214, &args![]).u32() == 4 {
                        fn_0093a6f0(e, this, false);
                        let process = process_of(e, this);
                        e.vcall(process, 0x214, &args![]);
                    } else if e.vcall(this.addr(), 0x22c, &args![0u32]).bool()
                        || e.vcall(this.addr(), 0x214, &args![]).u32() == 0
                    {
                        let process = process_of(e, this);
                        e.vcall(process, 0x214, &args![]);
                        fn_0093a5f0(e, this, false);
                        e.set_global(CONTROL_COUNTER_011E0798, 0.0f32);
                    }
                    e.call(0x0088_d640, &args![this]);
                    e.set_global(CONTROL_COUNTER_011E0798, 0.0f32);
                    break 'update;
                }
            }
            if fn_0093a5d0(e, this) {
                let counter: f32 = e.global(CONTROL_COUNTER_011E0798);
                e.set_global(
                    CONTROL_COUNTER_011E0798,
                    (counter as f64 + delta as f64) as f32,
                );
                let process = process_of(e, this);
                let mut reset = e.vcall(process, 0x228, &args![this]).bool();
                if !reset {
                    reset = e.vcall(this.addr(), 0x214, &args![]).u32() == 4;
                }
                if !reset {
                    let process = process_of(e, this);
                    reset = e.vcall(process, 0x4bc, &args![]).u32() == 9;
                }
                if !reset {
                    let process = process_of(e, this);
                    reset = e.vcall(process, 0x27c, &args![]).u32() == 0;
                }
                if reset {
                    fn_0093a5f0(e, this, false);
                    let process = process_of(e, this);
                    e.vcall(process, 0x214, &args![]);
                    e.set_global(CONTROL_COUNTER_011E0798, 0.0f32);
                }
            } else if fn_0093a640(e, this) {
                let counter: f32 = e.global(CONTROL_COUNTER_011E0798);
                e.set_global(
                    CONTROL_COUNTER_011E0798,
                    (counter as f64 + delta as f64) as f32,
                );
                if e.vcall(this.addr(), 0x214, &args![]).u32() == 0 {
                    fn_0093a6f0(e, this, false);
                    let process = process_of(e, this);
                    e.vcall(process, 0x214, &args![]);
                    e.set_global(CONTROL_COUNTER_011E0798, 0.0f32);
                }
            }
            if e.mem.u8(this.addr() + 0x64c) == e.mem.u8(this.addr() + 0x64a) {
                e.call(0x0088_6360, &args![this, delta]);
            }
            let mover = e.get(this.cast::<Actor>(), Actor::pActorMover).addr();
            e.call(0x009e_a570, &args![mover, ZERO_VECTOR]);
            let mover = e.get(this.cast::<Actor>(), Actor::pActorMover).addr();
            e.vcall(mover, 0x14, &args![delta]);
            let chase_scale = setting_float(e, CAMERA_SETTING_011CD62C);
            if chase_scale as f64 > zero {
                let mode = e.call(0x004f_8960, &args![this]).u32();
                if mode == 2 || e.call(0x004f_8960, &args![this]).u32() == 1 {
                    let value: f32 = e.global(TIMER_011A3B34);
                    if (value as f64) < zero {
                        e.call(0x0070_9c40, &args![0u32]);
                        let setting = setting_float(e, CAMERA_SETTING_011CD62C);
                        e.set_global(TIMER_011A3B34, setting);
                    }
                    let value: f32 = e.global(TIMER_011A3B34);
                    let value = (value as f64 - delta as f64) as f32;
                    e.set_global(TIMER_011A3B34, value);
                    if (value as f64) < zero {
                        e.set_global(TIMER_011A3B34, e.global::<f32>(TIMER_RESET));
                        let reactor = e.global::<u32>(FAST_TRAVEL_REACTOR);
                        if !e.call(0x0085_12f0, &args![reactor]).bool() {
                            e.call(0x007d_0a70, &args![]);
                        }
                    }
                } else {
                    e.set_global(TIMER_011A3B34, e.global::<f32>(TIMER_RESET));
                }
            } else if e.call(0x004f_8960, &args![this]).u32() == 2
                && e.global::<u8>(MESSAGE_SHOWN_011E07C0) == 0
            {
                e.vcall(this.addr(), 0x2a0, &args![]);
                e.call(0x0070_9c40, &args![0u32]);
                let reactor = e.global::<u32>(FAST_TRAVEL_REACTOR);
                if e.call(0x0085_1230, &args![reactor]).bool() {
                    let text_a = e.call(0x0040_3df0, &args![MESSAGE_SETTING_011D4528]).u32();
                    let text_b = e.call(0x0040_3df0, &args![MESSAGE_SETTING_011D400C]).u32();
                    let text_c = e.call(0x0040_3df0, &args![MESSAGE_SETTING_011D4B40]).u32();
                    e.call(
                        0x0070_3e80,
                        &args![
                            text_c,
                            0u32,
                            0u32,
                            0x0096_1d50u32,
                            1u32,
                            0x17u32,
                            0.0f32,
                            0.0f32,
                            text_b,
                            text_a,
                            0u32
                        ],
                    );
                } else {
                    e.call(0x007d_0a70, &args![]);
                }
                e.set_global::<u8>(MESSAGE_SHOWN_011E07C0, 1);
            }
            e.vcall(this.addr(), 0x1e0, &args![]);
            let ahead: f32 = e.global(MOVE_SPEED_011A3B3C);
            let behind: f32 = e.global(LOOK_SPEED_011A3B40);
            if e.get(this, PlayerCharacter::b3rdPerson) {
                e.call(0x0089_5110, &args![this, ahead, behind]);
            }
            let toggled = !e.get(this, PlayerCharacter::b3rdPerson);
            e.set(this, PlayerCharacter::b3rdPerson, toggled);
            if e.get(this, PlayerCharacter::b3rdPerson) {
                e.call(0x0089_5110, &args![this, ahead, behind]);
            }
            e.call(0x008d_3550, &args![this, delta]);
            let first_person = !e.get(this, PlayerCharacter::b3rdPerson);
            let animation = e.call(0x0095_0a60, &args![this, first_person]).u32();
            e.call(0x0088_85e0, &args![this, animation, delta]);
            let toggled = !e.get(this, PlayerCharacter::b3rdPerson);
            e.set(this, PlayerCharacter::b3rdPerson, toggled);
            e.call(0x008d_3550, &args![this, delta]);
            let first_person = !e.get(this, PlayerCharacter::b3rdPerson);
            let animation = e.call(0x0095_0a60, &args![this, first_person]).u32();
            e.call(0x0088_85e0, &args![this, animation, delta]);
            e.call(0x0094_ae40, &args![this, 0u32, 0u32]);
            let parent = e.call(0x008d_6f30, &args![this]).u32();
            e.call(0x0054_a070, &args![parent, this, 1u32, 0u32]);
            let node = e.vcall(this.addr(), 0x1d0, &args![]).u32();
            let tes_object = e.call(0x0045_0b80, &args![0u32]).u32();
            e.call(0x00b5_d9f0, &args![tes_object, node, 1u32]);
            let parent = e.call(0x008d_6f30, &args![this]).u32();
            let music = e.call(0x0054_74b0, &args![parent, 0u32]).u32();
            e.set(this, PlayerCharacter::pLastKnownMusicType, Ptr::new(music));
            if e.get(this, PlayerCharacter::bAiControlledActivate) {
                if e.global::<u32>(PENDING_OBJECT_011E0788) != 0 {
                    if e.global::<u32>(STATIC_FLAGS_011E0BC8) & 1 == 0 {
                        let flags = e.global::<u32>(STATIC_FLAGS_011E0BC8);
                        e.set_global(STATIC_FLAGS_011E0BC8, flags | 1);
                        let interval = setting_float(e, ACTIVATE_INTERVAL_SETTING);
                        e.set_global(ACTIVATE_TIMER_011E0BC4, interval);
                    }
                    let wait_a = e.call(0x00a2_4660, &args![main_state, 5u32, 1u32]).u32() != 0
                        || e.call(0x00a2_4660, &args![main_state, 5u32, 0u32]).u32() != 0;
                    if wait_a && !e.call(0x005a_03f0, &args![this, 1u32]).bool() {
                        let node = e.vcall(this.addr(), 0x1e4, &args![]).u32();
                        e.call(0x0092_2650, &args![node, 7u32, 1u32]);
                        let timer: f32 = e.global(ACTIVATE_TIMER_011E0BC4);
                        let mut timer = (timer as f64 - delta as f64) as f32;
                        e.set_global(ACTIVATE_TIMER_011E0BC4, timer);
                        while (timer as f64) < zero {
                            let interval = setting_float(e, ACTIVATE_INTERVAL_SETTING);
                            timer = (timer as f64 + interval as f64) as f32;
                            e.set_global(ACTIVATE_TIMER_011E0BC4, timer);
                            let pending = e.global::<u32>(PENDING_OBJECT_011E0788);
                            if pending != 0 {
                                e.vcall(pending, 0x124, &args![0u32, this, 0u32, 0u32, 0u32]);
                            }
                            timer = e.global(ACTIVATE_TIMER_011E0BC4);
                        }
                    } else {
                        e.with_stack(12, |e, handle| {
                            e.call(SOUND_HANDLE_CONSTRUCT, &args![handle]);
                            let player = e.global::<u32>(PLAYER_SINGLETON);
                            let found = e.call(0x005d_43c0, &args![player]).u32();
                            e.call(0x0041_8a40, &args![found, handle]);
                            e.call(SOUND_HANDLE_STOP, &args![handle]);
                            let node = e.vcall(this.addr(), 0x1e4, &args![]).u32();
                            e.call(0x0092_2650, &args![node, 7u32, 0u32]);
                            let interval = setting_float(e, ACTIVATE_INTERVAL_SETTING);
                            e.set_global(ACTIVATE_TIMER_011E0BC4, interval);
                            e.set_global(PENDING_OBJECT_011E0788, 0u32);
                            e.call(SOUND_HANDLE_DESTRUCT, &args![handle]);
                        });
                    }
                }
                let node = e.vcall(this.addr(), 0x1e4, &args![]).u32();
                if e.call(0x0049_85f0, &args![node]).bool() {
                    let process = process_of(e, this);
                    if e.vcall(process, 0x3e4, &args![]).i32() == -1 {
                        let process = process_of(e, this);
                        if e.vcall(process, 0x40c, &args![]).u32() == 0 {
                            e.set(this, PlayerCharacter::bAiControlledActivate, false);
                            e.set_global(PENDING_OBJECT_011E0788, 0u32);
                        }
                    }
                }
            }
            if e.call(0x00a2_4660, &args![main_state, 0x1au32, 1u32]).u32() != 0 {
                let reactor = e.global::<u32>(FAST_TRAVEL_REACTOR);
                if e.call(0x0085_09f0, &args![reactor]).bool() {
                    e.set_global(TIMER_011A3B34, e.global::<f32>(TIMER_RESET));
                }
            }
            'activation: {
                if e.vcall(this.addr(), 0x22c, &args![0u32]).bool()
                    || e.vcall(this.addr(), 0x2e8, &args![]).bool()
                {
                    break 'activation;
                }
                e.call(0x008c_3c40, &args![this, 0u32, 0u32]);
                if e.call(0x00a2_4660, &args![main_state, 5u32, 1u32]).u32() == 0
                    || e.call(0x0096_7ae0, &args![this]).bool()
                    || e.call(0x0070_2360, &args![]).bool()
                    || e.vcall(this.addr(), 0x234, &args![]).bool()
                    || e.call(0x0057_21e0, &args![this]).bool()
                {
                    break 'activation;
                }
                let reference = e.call(0x0070_3350, &args![]).u32(); // -0x1f4
                let mut allow = true; // -0xf5
                let mut blocked = false; // -0xf6
                if reference != 0 && e.call(0x0056_8680, &args![reference]).bool() && cached {
                    blocked = true;
                }
                if e.call(0x005a_03f0, &args![this, 1u32]).bool() {
                    blocked = true;
                }
                if reference != 0 && e.vcall(reference, 0x224, &args![]).bool() {
                    let cast = e
                        .call(
                            0x00ec_43fb,
                            &args![reference, 0u32, 0x0118_41ccu32, 0x011a_28e0u32, 0u32],
                        )
                        .u32();
                    if cast != 0 && e.call(0x008c_e390, &args![cast]).u32() == 0 {
                        blocked = true;
                    }
                }
                if reference != 0 && e.vcall(reference, 0x100, &args![]).bool() {
                    let ref_process = e.call(0x008d_8520, &args![reference]).u32();
                    let kind = e.vcall(ref_process, 0x610, &args![]).u32();
                    if kind == 5 || e.vcall(ref_process, 0x610, &args![]).u32() == 6 {
                        blocked = true;
                    }
                }
                let state = e.vcall(this.addr(), 0x214, &args![]).u32();
                let index = state.wrapping_sub(1);
                if index <= 9 && index != 3 && index != 8 {
                    blocked = true;
                }
                let class = e.call(0x008a_7570, &args![this]).u32();
                if class != u32::MAX {
                    blocked = true;
                }
                if e.mem.u8(this.addr() + 0x64c) != e.mem.u8(this.addr() + 0x64a) {
                    blocked = true;
                }
                if e.call(0x0094_4340, &args![this]).bool() {
                    blocked = true;
                }
                let mut activated = false; // -0x10d
                if !blocked {
                    e.call(0x0070_62e0, &args![0u32, 0u32, 0u32]);
                    if reference != 0 {
                        e.with_stack(8, |e, list| {
                            e.call(LIST_CONSTRUCT, &args![list]);
                            e.call(
                                0x005e_58f0,
                                &args![0x1bu32, this, reference, list, reference],
                            );
                            if !e.call(LIST_IS_EMPTY, &args![list]).bool() {
                                if !e.call(0x007a_a720, &args![list, reference]).bool() {
                                    let activator = e.call(0x0077_8930, &args![reference]).u32();
                                    if e.call(
                                        0x0057_3170,
                                        &args![reference, this, 0u32, 0u32, activator],
                                    )
                                    .bool()
                                    {
                                        allow = false;
                                    }
                                }
                            } else {
                                let activator = e.call(0x0077_8930, &args![reference]).u32();
                                if e.call(
                                    0x0057_3170,
                                    &args![reference, this, 0u32, 0u32, activator],
                                )
                                .bool()
                                {
                                    allow = false;
                                }
                            }
                            e.call(LIST_DESTRUCT, &args![list]);
                        });
                    } else {
                        activated = true;
                    }
                    if allow {
                        let process = process_of(e, this);
                        if e.vcall(process, 0x4c8, &args![]).u32() != 0 {
                            let process = process_of(e, this);
                            let object = e.vcall(process, 0x4c8, &args![]).u32();
                            activated = !e
                                .call(0x0057_3170, &args![object, this, 0u32, 0u32, 1u32])
                                .bool();
                        } else {
                            activated = true;
                        }
                    }
                }
                if activated || blocked {
                    e.with_stack(24, |e, scratch| {
                        let found = e.call(0x005d_43c0, &args![this]).u32();
                        e.call(0x0041_8940, &args![found, handle]);
                        let sound_form = e.call(0x0082_ec10, &args![]).u32(); // -0x124
                        if !e.call(SOUND_HANDLE_IS_VALID, &args![handle]).bool() && sound_form != 0
                        {
                            let name = e.call(0x0051_1840, &args![sound_form]).u32();
                            let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
                            let result = e
                                .call(
                                    0x00ad_7480,
                                    &args![audio, scratch, name, 0x121u32, sound_form],
                                )
                                .u32();
                            e.call(SOUND_HANDLE_ASSIGN, &args![handle, result]);
                            e.call(SOUND_HANDLE_DESTRUCT, &args![scratch]);
                        }
                        if !e.call(0x00ad_8930, &args![handle]).bool() {
                            e.call(SOUND_HANDLE_PLAY, &args![handle, 0u32]);
                        }
                        let found = e.call(0x005d_43c0, &args![this]).u32();
                        e.call(0x0041_a090, &args![found, handle]);
                    });
                }
            }
            break 'update;
        }
        // ---- 009408e4: the player is not under control ----
        let player_process = singleton_process(e);
        if e.vcall(player_process, 0x22c, &args![]).u32() != 0 {
            let player_process = singleton_process(e);
            e.vcall(player_process, 0x234, &args![]);
        }
        let player_process = singleton_process(e);
        if e.vcall(player_process, 0x20c, &args![]).u32() != 0 {
            let player_process = singleton_process(e);
            e.vcall(player_process, 0x214, &args![]);
        }
        e.call(0x0095_3d40, &args![this]);
        e.call(0x0095_3d00, &args![this]);
        let mut looked_flag = false; // -0x139
        let ammo_timer = e.get(this, PlayerCharacter::fAmmoSwapButtonTimer);
        e.set(
            this,
            PlayerCharacter::fAmmoSwapButtonTimer,
            (ammo_timer as f64 + delta as f64) as f32,
        );
        if e.call(0x00a2_4660, &args![main_state, 0x12u32, 1u32]).u32() != 0 {
            let process = process_of(e, this);
            if e.vcall(process, 0x148, &args![]).u32() != 0
                && e.call(0x008a_16d0, &args![this]).bool()
            {
                let first = e.call(0x0071_7a40, &args![1u32, 0u32]).i32();
                let second = e.call(0x0071_7a40, &args![4u32, 0u32]).i32();
                let third = e.call(0x0071_7a40, &args![3u32, 0u32]).i32();
                let first_up = first == 2 || first == 0;
                let second_up = second == 2 || second == 0;
                let third_up = third == 2 || third == 0;
                if !first_up || (!second_up && !third_up) {
                    let interval = setting_float(e, AMMO_SWAP_SETTING);
                    let timer = e.get(this, PlayerCharacter::fAmmoSwapButtonTimer);
                    let long_enough = (interval as f64) < timer as f64;
                    e.call(0x0094_62c0, &args![this, long_enough, 0u32]);
                    if long_enough {
                        e.set(this, PlayerCharacter::fAmmoSwapButtonTimer, 0.0);
                    }
                }
            }
        }
        if e.call(0x00a2_4660, &args![main_state, 7u32, 1u32]).u32() != 0
            || e.call(0x00a2_4660, &args![main_state, 7u32, 0u32]).u32() != 0
        {
            if !e.call(0x005a_03f0, &args![this, 8u32]).bool() {
                let held: f32 = e.global(HELD_TIMER_011E07E0);
                e.set_global(HELD_TIMER_011E07E0, (held as f64 + delta as f64) as f32);
                if e.call(0x0088_43a0, &args![this]).bool()
                    && e.call(0x008a_7570, &args![this]).u32() == u32::MAX
                {
                    let mut handled = false;
                    if e.global::<u8>(HELD_FLAG_011E0BC1) == 0 {
                        let limit = setting_float(e, HELD_SETTING_011CDFCC);
                        let held: f32 = e.global(HELD_TIMER_011E07E0);
                        let mut run = (limit as f64) < held as f64;
                        if !run {
                            run = if combat_target_state == 0 {
                                true
                            } else {
                                let player = e.global::<u32>(PLAYER_SINGLETON);
                                if e.call(0x0052_5980, &args![combat_target_state, player])
                                    .u32()
                                    != 0
                                {
                                    false
                                } else {
                                    !e.call(0x0047_4a80, &args![combat_target_state + 0xa4])
                                        .bool()
                                }
                            };
                        }
                        if run && e.call(0x008a_6970, &args![this]).bool() {
                            looked_flag = true;
                            e.call(0x008a_6840, &args![this, 0u32]);
                            e.set_global::<u8>(HELD_FLAG_011E0BC1, 1);
                            handled = true;
                        }
                    }
                    if !handled
                        && e.global::<u8>(HELD_FLAG_011E0BC1) == 0
                        && !e.call(0x008a_6970, &args![this]).bool()
                    {
                        if e.call(0x0088_46e0, &args![this]).u32() & 0x800 == 0 {
                            e.call(0x008a_6840, &args![this, 1u32]);
                        }
                        e.set_global::<u8>(HELD_FLAG_011E0BC1, 1);
                    }
                }
            }
        } else if e.call(0x00a2_4660, &args![main_state, 7u32, 2u32]).u32() == 0 {
            e.set_global(HELD_TIMER_011E07E0, 0.0f32);
            e.set_global::<u8>(HELD_FLAG_011E0BC1, 0);
        }
        e.call(0x0096_73d0, &args![this, delta]);
        if e.call(0x00a2_4660, &args![main_state, 0x0au32, 1u32]).u32() != 0 {
            let toggled = !e.get(this, PlayerCharacter::bAlwaysRun);
            e.set(this, PlayerCharacter::bAlwaysRun, toggled);
        }
        if e.call(0x00a2_4660, &args![main_state, 0x0bu32, 1u32]).u32() != 0 {
            let toggled = !e.get(this, PlayerCharacter::bAutoMove);
            e.set(this, PlayerCharacter::bAutoMove, toggled);
        }
        if e.get(this, PlayerCharacter::bAutoMove)
            && !e.call(0x0070_9bc0, &args![]).bool()
            && !e.call(0x0070_2360, &args![]).bool()
        {
            if e.call(0x00a2_4660, &args![main_state, 0u32, 0u32]).u32() != 0
                || e.call(0x00a2_4660, &args![main_state, 1u32, 0u32]).u32() != 0
                || e.call(0x00a2_4660, &args![main_state, 3u32, 0u32]).u32() != 0
                || e.call(0x00a2_4660, &args![main_state, 2u32, 0u32]).u32() != 0
            {
                e.set(this, PlayerCharacter::bAutoMove, false);
            } else {
                e.call(0x00a2_4280, &args![main_state, 0u32]);
            }
        }
        let mut crouch_handled = false; // -0x160
        if e.call(0x00a2_4660, &args![main_state, 8u32, 1u32]).u32() != 0 {
            let busy = e.vcall(this.addr(), 0x234, &args![]).bool()
                || e.vcall(this.addr(), 0x22c, &args![0u32]).bool()
                || e.vcall(this.addr(), 0x230, &args![]).bool()
                || e.call(0x0043_7bf0, &args![this]).bool()
                || e.call(0x0043_7bd0, &args![this]).bool()
                || e.call(0x005a_03f0, &args![this, 0x40u32]).bool()
                || e.vcall(this.addr(), 0x214, &args![]).u32() != 0;
            if !busy {
                // The classes (8a7570 + 1) for which the crouch sounds play.
                const PLAYS_CROUCH_SOUND: [bool; 19] = [
                    true, false, false, true, true, true, true, true, true, false, true, false,
                    false, false, false, false, false, false, true,
                ];
                let class = e.call(0x008a_7570, &args![this]).u32().wrapping_add(1);
                if class <= 0x12 && PLAYS_CROUCH_SOUND[class as usize] {
                    let state = e.mem.u16(word_slot);
                    let (name, argument) = if state & 0x400 != 0 {
                        e.mem.set_u16(word_slot, state & 0xfbff);
                        (CROUCH_UP_SOUND_NAME, 0x0bu32)
                    } else {
                        e.mem.set_u16(word_slot, state | 0x400);
                        (CROUCH_DOWN_SOUND_NAME, 0x0au32)
                    };
                    e.with_stack(12, |e, found| {
                        let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
                        let result = e
                            .call(
                                AUDIO_GET_SOUND_HANDLE_BY_NAME,
                                &args![audio, found, name, 0x0004_0102u32],
                            )
                            .u32();
                        e.call(SOUND_HANDLE_ASSIGN, &args![handle, result]);
                        e.call(SOUND_HANDLE_DESTRUCT, &args![found]);
                    });
                    let position = e.vcall(this.addr(), 0x1f4, &args![]).u32();
                    e.call(0x0068_a7d0, &args![handle, position]);
                    e.call(SOUND_HANDLE_PLAY, &args![handle, 0u32]);
                    e.vcall(this.addr(), 0x340, &args![argument]);
                    if e.mem.u8(this.addr() + 0x64f) == 0 {
                        e.call(0x008b_b650, &args![this, 0u32, 0u32, 0u32]);
                        e.call(0x0089_4cc0, &args![this, 0u32]);
                    }
                }
            }
            crouch_handled = true;
        }
        // ---- 00940fb5: movement input ----
        let root_node = e.call(0x0055_8310, &args![root]).u32();
        let root_matrix = e.call(0x006a_9540, &args![root_node]).u32();
        let matrix = local(frame, -0x1b0);
        copy_matrix(e, matrix, root_matrix);
        let current = e.call(0x0043_6aa0, &args![this]).u32();
        let position_copy = local(frame, -0x1bc);
        let second_copy = local(frame, -0x1c8);
        let words = read_words(e, current);
        for (i, word) in words.iter().enumerate() {
            e.mem.set_u32(position_copy + 4 * i as u32, *word);
            e.mem.set_u32(second_copy + 4 * i as u32, *word);
        }
        let right = local(frame, -0x1d4);
        let forward = local(frame, -0x1e0);
        e.call(LIST_ITEM_ADDRESS, &args![right]);
        e.call(LIST_ITEM_ADDRESS, &args![forward]);
        let unit = e
            .call(
                0x0041_6870,
                &args![local(frame, -0x1ec), 1.0f32, 0.0f32, 0.0f32],
            )
            .u32();
        let words = read_words(e, unit);
        for (i, word) in words.iter().enumerate() {
            e.mem.set_u32(forward + 4 * i as u32, *word);
        }
        e.call(0x0043_9f50, &args![matrix, 1u32, right]);
        if !e.call(0x0050_d4a0, &args![this]).bool() {
            e.mem.set_f32(right + 8, 0.0);
            e.call(0x004a_0c10, &args![right]);
        } else {
            e.call(0x0043_9f50, &args![matrix, 0u32, forward]);
        }
        let mut grab_flag = false; // -0x1ed
        if e.get(this, PlayerCharacter::eGrabType) == 2 {
            grab_flag = e.call(0x0096_13c0, &args![this, delta]).bool();
        }
        e.call(0x0096_2350, &args![this]);
        let mut axis_a = 0i32; // -0x1f4
        let mut axis_b = 0i32; // -0x1f8
        let mut axis_c = 0i32; // -0x1fc
        let gamepad = e.call(0x004b_71d0, &args![]).bool();
        if gamepad {
            axis_a = e.call(0x00a2_3390, &args![main_state, 0u32, 7u32]).i32();
            axis_c = e.call(0x00a2_3390, &args![main_state, 0u32, 9u32]).i32();
            axis_b = e.call(0x00a2_3390, &args![main_state, 0u32, 8u32]).i32();
            if e.call(0x00ec_7d40, &args![axis_a]).i32() < 0x1ea9 {
                axis_a = 0;
            }
            if e.call(0x00ec_7d40, &args![axis_b]).i32() < 0x1ea9 {
                axis_b = 0;
            }
            if e.call(0x00ec_7d40, &args![axis_c]).i32() < 0x21f1 {
                axis_c = 0;
            }
            axis_a = axis_a.wrapping_mul(100) / 0x7fff;
            axis_b = axis_b.wrapping_mul(100) / 0x7fff;
            axis_c /= 0x7fff;
        }
        // (The block at 009411e9..0094134c runs only without a gamepad, where the
        // three axis values are still 0, so it has no effect and is left out.)
        let process = process_of(e, this);
        let aim_object = e.vcall(process, 0x148, &args![]).u32();
        let _aim_state = if aim_object != 0 {
            e.call(0x0044_ddc0, &args![aim_object]).u32()
        } else {
            0
        };
        let one: f64 = e.global(ONE_DOUBLE);
        let move_speed: f32 = e.global(MOVE_SPEED_011A3B3C);
        if move_speed as f64 > one {
            e.set_global(MOVE_SPEED_011A3B3C, 1.0f32);
        }
        if e.call(0x004b_71d0, &args![]).bool() {
            let divisor: f64 = e.global(AXIS_DIVISOR);
            let mut stick_x = (axis_a as f64 / divisor) as f32; // -0x224
            let mut stick_y = (axis_b as f64 / divisor) as f32; // -0x228
            let mut stick_z = (axis_c as f64 / divisor) as f32; // -0x22c
            if e.call(0x008a_7570, &args![this]).u32() == 0xb {
                stick_x = 0.0;
                stick_y = 0.0;
                stick_z = 0.0;
            }
            let stick = local(frame, -0x238);
            e.call(LIST_ITEM_ADDRESS, &args![stick]);
            let abs_x = e.call(0x0040_8840, &args![stick_x]).f64();
            let scale = setting_float(e, STICK_SCALE_SETTING);
            let weighted = (scale as f64 * stick_y as f64) as f32;
            let abs_weighted = e.call(0x0040_8840, &args![weighted]).f64();
            let y_dominant = abs_weighted > abs_x;
            if !y_dominant {
                if (stick_x as f64) < zero {
                    or_word(e, word_slot, 0x4);
                } else if stick_x as f64 > zero {
                    or_word(e, word_slot, 0x8);
                }
            }
            if y_dominant {
                if (stick_y as f64) < zero {
                    or_word(e, word_slot, 0x2);
                } else if stick_y as f64 > zero {
                    or_word(e, word_slot, 0x1);
                }
            }
            e.mem.set_f32(stick, stick_x);
            e.mem.set_f32(stick + 4, stick_y);
            e.mem.set_f32(stick + 8, 0.0);
            e.set_global(MOVE_SPEED_011A3B3C, 0.0f32);
            let mut full_speed = false; // -0x24a
            let lengthed = local(frame, -0x258);
            e.call(LIST_ITEM_ADDRESS, &args![lengthed]);
            e.mem.set_f32(lengthed, stick_x);
            e.mem.set_f32(lengthed + 4, stick_y);
            e.mem.set_f32(lengthed + 8, 0.0);
            let length = e.call(0x0045_7990, &args![lengthed]).f64();
            let clamped = if length > one {
                1.0f32
            } else {
                e.call(0x0045_7990, &args![lengthed]).f32()
            };
            let speed: f32 = e.global(MOVE_SPEED_011A3B3C);
            if (speed as f64) < one {
                let high: f64 = e.global(STICK_THRESHOLD_HIGH);
                let low: f64 = e.global(STICK_THRESHOLD_LOW);
                if clamped as f64 > high {
                    e.set_global(MOVE_SPEED_011A3B3C, 1.0f32);
                    full_speed = true;
                } else if clamped as f64 > low {
                    let a: f32 = e.global(STICK_REMAP_A);
                    let c: f32 = e.global(STICK_REMAP_C);
                    let mapped = e
                        .call(0x004b_3ab0, &args![a, 1.0f32, c, 1.0f32, clamped])
                        .f32();
                    e.set_global(MOVE_SPEED_011A3B3C, mapped);
                    full_speed = true;
                } else {
                    let a: f32 = e.global(STICK_REMAP_D);
                    let b: f32 = e.global(STICK_REMAP_E);
                    let c: f32 = e.global(STICK_REMAP_C);
                    let mapped = e.call(0x004b_3ab0, &args![a, b, 0.0f32, c, clamped]).f32();
                    e.set_global(MOVE_SPEED_011A3B3C, mapped);
                }
            }
            let stick_length = e.call(0x0045_7990, &args![stick]).f64();
            if stick_length > zero {
                if e.call(0x00a2_4660, &args![main_state, 9u32, 1u32]).u32() != 0
                    || e.call(0x00a2_4660, &args![main_state, 9u32, 0u32]).u32() != 0
                    || full_speed
                {
                    let process = process_of(e, this);
                    let mut run = e.vcall(process, 0x3e4, &args![]).u32() != 7
                        && !e.vcall(this.addr(), 0x358, &args![]).bool()
                        && !e.call(0x008b_bc10, &args![this]).bool();
                    if run {
                        let limit = setting_float(e, RUN_WEIGHT_SETTING);
                        let weight = e.get(this, PlayerCharacter::fGrabObjectWeight);
                        run = (limit as f64) >= weight as f64;
                    }
                    if run {
                        or_word(e, word_slot, 0x200);
                    } else {
                        let walk = setting_float(e, WALK_SPEED_SETTING);
                        e.set_global(MOVE_SPEED_011A3B3C, walk);
                        or_word(e, word_slot, 0x100);
                    }
                } else {
                    or_word(e, word_slot, 0x100);
                }
            }
            let speed: f32 = e.global(MOVE_SPEED_011A3B3C);
            if speed as f64 > one && e.vcall(this.addr(), 0x358, &args![]).bool() {
                e.set_global(MOVE_SPEED_011A3B3C, 1.0f32);
            }
            let speed: f32 = e.global(MOVE_SPEED_011A3B3C);
            if speed as f64 > one && e.call(0x008b_bc10, &args![this]).bool() {
                e.set_global(MOVE_SPEED_011A3B3C, 1.0f32);
            }
            let look_scale: f64 = e.global(LOOK_SCALE);
            let abs_z = e.call(0x0040_8840, &args![stick_z]).f64();
            let abs_x2 = e.call(0x0040_8840, &args![stick_x]).f64();
            let look_axis = if abs_x2 < abs_z {
                let abs_z2 = e.call(0x0040_8840, &args![stick_z]).f64();
                let abs_y = e.call(0x0040_8840, &args![stick_y]).f64();
                if abs_y < abs_z2 {
                    e.call(0x0040_8840, &args![stick_z]).f64()
                } else {
                    e.call(0x0040_8840, &args![stick_x]).f64()
                }
            } else {
                e.call(0x0040_8840, &args![stick_x]).f64()
            };
            e.set_global(LOOK_SPEED_011A3B40, (look_axis * look_scale) as f32);
            let speed = e.call(0x008a_0b10, &args![this]).f64();
            let move_speed: f32 = e.global(MOVE_SPEED_011A3B3C);
            let step = (speed * move_speed as f64) as f32;
            e.call(0x0043_9180, &args![stick, step]);
            if e.call(0x0050_d4a0, &args![this]).bool() {
                let turn = local(frame, -0x298);
                e.call(LIST_ITEM_ADDRESS, &args![turn]);
                let heading = e.call(0x0093_1d70, &args![this]).f32();
                e.call(0x0052_4ac0, &args![turn, -heading]);
                let turned = e
                    .call(0x004b_3ae0, &args![local(frame, -0x2a4), stick, turn])
                    .u32();
                let words = read_words(e, turned);
                for (i, word) in words.iter().enumerate() {
                    e.mem.set_u32(stick + 4 * i as u32, *word);
                }
            }
            let factor: f32 = e.global(STICK_FACTOR);
            e.call(0x0043_9180, &args![stick, factor]);
            e.call(0x0063_c8a0, &args![position_copy, stick]);
            let words = read_words(e, stick);
            for (i, word) in words.iter().enumerate() {
                e.mem.set_u32(forward + 4 * i as u32, *word);
            }
        } else {
            e.set_global(MOVE_SPEED_011A3B3C, 1.0f32);
            or_word(e, word_slot, 0x40);
            let keys = local(frame, -0x2ac);
            e.call(LIST_ITEM_ADDRESS, &args![keys]);
            e.mem.set_f32(keys, axis_a as f32);
            e.mem.set_f32(keys + 4, axis_b as f32);
            let key_length = e.call(0x0058_9850, &args![keys]).f64();
            let key_limit: f64 = e.global(KEY_AXIS_LIMIT);
            if key_length > key_limit {
                e.call(0x00a2_4280, &args![main_state, 9u32]);
                let process = process_of(e, this);
                if e.vcall(process, 0x3e4, &args![]).u32() != 7
                    && !e.vcall(this.addr(), 0x358, &args![]).bool()
                    && !e.call(0x008b_bc10, &args![this]).bool()
                {
                    or_word(e, word_slot, 0x200);
                }
            }
            let run_held = |e: &mut Engine| {
                e.call(0x00a2_4660, &args![main_state, 9u32, 1u32]).u32() != 0
                    || e.call(0x00a2_4660, &args![main_state, 9u32, 0u32]).u32() != 0
            };
            if !e.call(0x004b_71d0, &args![]).bool() && e.get(this, PlayerCharacter::bAlwaysRun) {
                if !run_held(e) {
                    let process = process_of(e, this);
                    if e.vcall(process, 0x3e4, &args![]).u32() != 7
                        && !e.vcall(this.addr(), 0x358, &args![]).bool()
                        && !e.call(0x008b_bc10, &args![this]).bool()
                    {
                        e.call(0x00a2_4280, &args![main_state, 9u32]);
                        or_word(e, word_slot, 0x200);
                    }
                }
            } else if run_held(e) {
                let process = process_of(e, this);
                if e.vcall(process, 0x3e4, &args![]).u32() != 7
                    && !e.vcall(this.addr(), 0x358, &args![]).bool()
                    && !e.call(0x008b_bc10, &args![this]).bool()
                {
                    or_word(e, word_slot, 0x200);
                }
            }
            let limit = setting_float(e, RUN_WEIGHT_SETTING);
            let weight = e.get(this, PlayerCharacter::fGrabObjectWeight);
            if (limit as f64) < weight as f64 {
                and_word(e, word_slot, 0xfdff);
            }
            let speed = e.call(0x008a_0b10, &args![this]).f32();
            let move_speed: f32 = e.global(MOVE_SPEED_011A3B3C);
            let step = (speed as f64 * move_speed as f64) as f32;
            e.call(0x0043_9180, &args![forward, step]);
            let step = (speed as f64 * move_speed as f64) as f32;
            e.call(0x0043_9180, &args![right, step]);
            let key_pressed = |e: &mut Engine, key: u32| {
                e.call(0x00a2_4660, &args![main_state, key, 1u32]).u32() != 0
                    || e.call(0x00a2_4660, &args![main_state, key, 0u32]).u32() != 0
            };
            if key_pressed(e, 0) {
                e.call(0x0063_c8a0, &args![position_copy, right]);
                if e.mem.u16(word_slot) & 0x200 == 0 {
                    or_word(e, word_slot, 0x100);
                }
                or_word(e, word_slot, 0x1);
            }
            if key_pressed(e, 1) {
                e.call(0x0045_78c0, &args![position_copy, right]);
                if e.mem.u16(word_slot) & 0x200 == 0 {
                    or_word(e, word_slot, 0x100);
                }
                or_word(e, word_slot, 0x2);
            }
            if key_pressed(e, 2) {
                e.call(0x0045_78c0, &args![position_copy, forward]);
                if e.mem.u16(word_slot) & 0x200 == 0 {
                    or_word(e, word_slot, 0x100);
                }
                or_word(e, word_slot, 0x4);
            }
            if key_pressed(e, 3) {
                e.call(0x0063_c8a0, &args![position_copy, forward]);
                if e.mem.u16(word_slot) & 0x200 == 0 {
                    or_word(e, word_slot, 0x100);
                }
                or_word(e, word_slot, 0x8);
            }
        }
        let _ = grab_flag;
        // ---- 00941d13: apply the input ----
        if e.call(0x0044_ddc0, &args![VATS_OBJECT]).u32() == 4 {
            and_word(e, word_slot, 0xffc0);
        }
        let mover = e.get(this.cast::<Actor>(), Actor::pActorMover).addr();
        let word_value = e.mem.u16(word_slot) as u32;
        e.vcall(mover, 0xc, &args![word_value]);
        if crouch_handled {
            e.call(0x0088_4f80, &args![this]);
        }
        let mover = e.get(this.cast::<Actor>(), Actor::pActorMover).addr();
        e.vcall(mover, 0x18, &args![delta]);
        e.call(0x0095_03d0, &args![this]);
        e.call(0x0095_0530, &args![this]);
        let third_person = e.mem.u8(this.addr() + 0x64a);
        e.mem.set_u8(this.addr() + 0x648, third_person);
        e.set(this, PlayerCharacter::b3rdPersonSaved, true);
        e.mem.set_u8(this.addr() + 0x64a, 1);
        let process = process_of(e, this);
        let skip_cancel = e.call(0x00a2_4660, &args![main_state, 4u32, 0u32]).u32() != 0
            && e.global::<u8>(STATE_FLAG_011E07AA) != 0
            && e.vcall(process, 0x14c, &args![]).u32() != 0
            && e.call(0x008a_6970, &args![this]).bool();
        if !skip_cancel {
            let process = process_of(e, this);
            let weight = e.vcall(process, 0x444, &args![]).f64();
            if weight != zero || weight.is_nan() {
                let in_vats_playback = e.call(0x0044_ddc0, &args![VATS_OBJECT]).u32() == 4
                    && e.call(0x009c_71c0, &args![VATS_OBJECT]).u32() != 0
                    && {
                        let record = e.call(0x009c_71c0, &args![VATS_OBJECT]).u32();
                        e.mem.u8(record + 8) != 0
                    };
                if !in_vats_playback {
                    let process = process_of(e, this);
                    e.vcall(process, 0x434, &args![0u32]);
                }
            }
        }
        let mut busy = false; // -0x2b9
        {
            let process = process_of(e, this);
            if e.vcall(this.addr(), 0x230, &args![]).bool()
                || e.vcall(this.addr(), 0x234, &args![]).bool()
                || e.call(0x004f_8960, &args![this]).u32() != 0
                || e.call(0x008a_7570, &args![this]).u32() == 0x0a
                || e.vcall(process, 0x4bc, &args![]).u32() != 0
            {
                busy = true;
            }
        }
        if busy || e.call(0x005a_03f0, &args![this, 1u32]).bool() {
            let mover = e.get(this.cast::<Actor>(), Actor::pActorMover).addr();
            e.call(0x009e_a360, &args![mover, 0x33fu32, 0u32]);
        }
        if !busy && !e.call(0x005a_03f0, &args![this, 8u32]).bool() {
            let grab_type = e.get(this, PlayerCharacter::eGrabType);
            if grab_type == 2 || grab_type == 3 {
                e.set_global(HOLD_VALUE_011E07AC, 0u32);
                e.set_global(HOLD_VALUE_011E07B0, 0.0f32);
            } else {
                if e.call(0x00a2_4660, &args![main_state, 6u32, 1u32]).u32() != 0
                    || e.call(0x00a2_4660, &args![main_state, 6u32, 0u32]).u32() != 0
                {
                    let process = process_of(e, this);
                    if e.vcall(process, 0x3e4, &args![]).u32() != 7
                        && !e.call(0x008b_bc10, &args![this]).bool()
                        && e.mem.u8(this.addr() + 0x64c) == e.mem.u8(this.addr() + 0x64b)
                        && !e.call(0x0096_7ae0, &args![this]).bool()
                        && {
                            let process = process_of(e, this);
                            e.vcall(process, 0x3f8, &args![]).bool()
                        }
                    {
                        let aiming = if combat_target_state != 0 {
                            e.call(0x0064_50c0, &args![combat_target_state]).bool()
                                && e.call(0x008a_16d0, &args![this]).bool()
                        } else {
                            !e.call(0x008a_16d0, &args![this]).bool()
                        };
                        if aiming {
                            e.call(0x008b_b650, &args![this, 1u32, 0u32, 0u32]);
                        } else {
                            e.call(0x0089_4cc0, &args![this, 1u32]);
                        }
                        if e.global::<u8>(STATE_FLAG_011E07B8) == 0
                            || e.global::<u8>(STATE_FLAG_011E07B9) != 0
                        {
                            moved_flag = true;
                        }
                    }
                } else if e.call(0x00a2_4660, &args![main_state, 6u32, 2u32]).u32() != 0
                    || e.call(0x00a2_4660, &args![main_state, 6u32, 0u32]).u32() == 0
                    || e.call(0x0096_7ae0, &args![this]).bool()
                {
                    let mut release = e.call(0x008b_bc10, &args![this]).bool();
                    if !release {
                        let process = process_of(e, this);
                        release = e.vcall(process, 0x3e4, &args![]).u32() == 7;
                    }
                    if release {
                        e.call(0x008b_b650, &args![this, 0u32, 0u32, 0u32]);
                        e.call(0x0089_4cc0, &args![this, 0u32]);
                        moved_flag = true;
                    }
                }
                let process = process_of(e, this);
                let word_value = e.mem.u16(word_slot);
                if e.vcall(process, 0x3f8, &args![]).bool() && word_value & 0x800 == 0 {
                    if e.call(0x0094_8310, &args![this]).bool() {
                        moved_flag = true;
                    }
                } else {
                    e.set_global(HOLD_VALUE_011E07AC, 0u32);
                    e.set_global(HOLD_VALUE_011E07B0, 0.0f32);
                }
            }
        }
        if !busy
            && !e.call(0x005a_03f0, &args![this, 1u32]).bool()
            && e.call(0x00a2_4660, &args![main_state, 0x0cu32, 1u32]).u32() != 0
            && !e.vcall(this.addr(), 0x358, &args![]).bool()
        {
            let node = e.vcall(this.addr(), 0x1e4, &args![]).u32();
            let kind = e.call(0x0043_01b0, &args![node, 4u32]).u16();
            if !e.call(0x005f_2670, &args![kind as u32]).bool()
                && e.call(0x008a_7570, &args![this]).u32() != 0x0b
            {
                let node = e.vcall(this.addr(), 0x1e4, &args![]).u32();
                if e.call(0x008a_7570, &args![this]).u32() == 0x0c
                    && e.call(0x0049_1040, &args![node, 1u32]).u32() != 0
                {
                    let held = e.call(0x0049_1040, &args![node, 1u32]).u32();
                    let owner = e.call(0x0048_f7f0, &args![held]).u32();
                    if e.call(0x005f_4db0, &args![owner]).bool() {
                        e.call(0x008a_73e0, &args![this, u32::MAX, 0u32]);
                    }
                }
                if e.call(0x008a_7570, &args![this]).u32() != 0x0c {
                    let free_hands = !(e.call(0x0088_49c0, &args![this]).bool()
                        || e.call(0x005a_2030, &args![this]).bool());
                    let held_6 = e.call(0x00a2_4660, &args![main_state, 6u32, 0u32]).u32() != 0;
                    let blocking = e.call(0x0089_4d60, &args![this]).bool();
                    if held_6 {
                        if e.call(0x0070_3350, &args![]).u32() == 0 {
                            let controller = e.call(0x0093_06d0, &args![this]).u32();
                            if e.call(0x005c_0880, &args![controller]).u32() == 0 {
                                let low = e.mem.u8(word_slot) as u32;
                                e.call(0x0089_4f90, &args![this, low]);
                            }
                        } else if !blocking {
                            let controller = e.call(0x0093_06d0, &args![this]).u32();
                            if e.call(0x0094_4400, &args![controller]).bool() {
                                e.vcall(this.addr(), 0x254, &args![]);
                            }
                        }
                    } else {
                        let controller = e.call(0x0093_06d0, &args![this]).u32();
                        if e.call(0x0094_4400, &args![controller]).bool() {
                            e.vcall(this.addr(), 0x254, &args![]);
                        }
                    }
                    let _ = free_hands;
                    e.call(0x0095_f6a0, &args![this]);
                    moved_flag = true;
                }
            }
        }
        if !e.vcall(this.addr(), 0x448, &args![]).bool() {
            let holder = e.call(0x005d_43c0, &args![this]).u32();
            if e.call(0x0041_cd70, &args![holder]).u32() != 0 {
                let holder = e.call(0x005d_43c0, &args![this]).u32();
                e.call(0x0041_cda0, &args![holder]);
            }
        }
        let mut wait_menu = false; // -0x2c8
        if e.call(0x00a2_4660, &args![main_state, 0x0fu32, 1u32]).u32() != 0
            && !e.call(0x005a_03d0, &args![this]).bool()
        {
            let _ = &mut wait_menu;
            let icon = FAST_TRAVEL_ICON;
            if e.vcall(this.addr(), 0x448, &args![]).bool() {
                show_text(e, MESSAGE_SETTING_011D4474, icon);
            } else if e.call(0x0095_3c80, &args![this]).bool() {
                show_text(e, MESSAGE_SETTING_011D28B0, icon);
            } else {
                let distance: f32 = e.global(INTERACT_DISTANCE_01084D20);
                let parent = e.call(0x008d_6f30, &args![this]).u32();
                let position = e.vcall(this.addr(), 0x1f4, &args![]).u32();
                if e.call(0x0088_5520, &args![this, position, parent, distance])
                    .bool()
                {
                    show_text(e, MESSAGE_SETTING_011D4948, icon);
                } else {
                    let player = e.global::<u32>(PLAYER_SINGLETON);
                    let interior = e.call(0x0057_5d10, &args![player]).bool();
                    if e.call(0x0097_64a0, &args![PROCESS_LISTS, interior as u32])
                        .bool()
                    {
                        show_text(e, MESSAGE_SETTING_011D2E50, icon);
                    } else {
                        let player = e.global::<u32>(PLAYER_SINGLETON);
                        let controller = e.call(0x0093_06d0, &args![player]).u32();
                        let first = e.call(0x005c_0880, &args![controller]).u32();
                        let second = if first == 1 {
                            1
                        } else {
                            let player = e.global::<u32>(PLAYER_SINGLETON);
                            let controller = e.call(0x0093_06d0, &args![player]).u32();
                            e.call(0x005c_0880, &args![controller]).u32()
                        };
                        if first == 1 || second == 2 {
                            show_text(e, MESSAGE_SETTING_011D3E20, icon);
                        } else {
                            let player = e.global::<u32>(PLAYER_SINGLETON);
                            let parent = e.call(0x008d_6f30, &args![player]).u32();
                            let cannot_wait = e.call(0x0054_44c0, &args![parent]).bool() || {
                                let player = e.global::<u32>(PLAYER_SINGLETON);
                                !e.call(0x0094_4360, &args![player]).bool()
                            };
                            if cannot_wait {
                                show_text(e, MESSAGE_SETTING_011D20AC, icon);
                            } else if e.call(0x0050_98e0, &args![PROCESS_LISTS]).bool() {
                                show_text(e, MESSAGE_SETTING_011D4B10, icon);
                            } else {
                                let waiting = e.call(0x008d_8520, &args![this]).u32() != 0 && {
                                    let process_like = e.call(0x008d_8520, &args![this]).u32();
                                    e.vcall(process_like, 0x764, &args![]).f64() != zero
                                };
                                if waiting {
                                    show_text(e, MESSAGE_SETTING_011D4B10, icon);
                                } else {
                                    let player = e.global::<u32>(PLAYER_SINGLETON);
                                    if e.call(0x0082_2e00, &args![player + MAGIC_TARGET_BASE])
                                        .bool()
                                    {
                                        show_text(e, MESSAGE_SETTING_011D47D4, icon);
                                    } else {
                                        e.call(0x0070_54f0, &args![0u32]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        e.mem.set_u8(this.addr() + 0x64a, 1);
        e.vcall(this.addr(), 0x1e0, &args![]);
        let ahead: f32 = e.global(MOVE_SPEED_011A3B3C);
        let behind: f32 = e.global(LOOK_SPEED_011A3B40);
        e.call(0x0089_5110, &args![this, ahead, behind]);
        let process = process_of(e, this);
        e.vcall(process, 0x41c, &args![this]);
        let saved_third_person = e.mem.u8(this.addr() + 0x648);
        e.mem.set_u8(this.addr() + 0x64a, saved_third_person);
        e.set(this, PlayerCharacter::b3rdPersonSaved, false);
        if e.call(0x005a_1e50, &args![this]).bool() {
            e.call(0x0094_df80, &args![this]);
        }
        if e.call(0x0045_6c70, &args![]).bool() || {
            let setting = e.call(0x0040_8d60, &args![FLAG_SETTING_011E0B64]).u32();
            e.mem.u8(setting) != 0
        } {
            if !e.call(0x0050_d4a0, &args![this]).bool() {
                e.call(LIST_ITEM_ADDRESS, &args![local(frame, -0x2e0)]);
                let controller = e.call(0x0093_06d0, &args![this]).u32();
                if controller != 0 {
                    e.call(0x0056_20e0, &args![controller, position_copy]);
                    let value = e.call(0x0045_8b20, &args![]).u32();
                    e.call(0x0080_fb00, &args![controller, value]);
                }
                e.call(0x0069_3ef0, &args![this, 1u32]);
            }
            let moved_by = local(frame, -0x2f0);
            e.call(0x0043_9ef0, &args![position_copy, moved_by, second_copy]);
            e.call(0x0043_9180, &args![moved_by, delta]);
            let lift = e.mem.f32(moved_by + 8);
            let pushed = e
                .call(
                    0x0041_6870,
                    &args![local(frame, -0x2fc), 0.0f32, 0.0f32, lift],
                )
                .u32();
            let sum = e
                .call(
                    0x0043_9e90,
                    &args![second_copy, local(frame, -0x308), pushed],
                )
                .u32();
            e.vcall(this.addr(), 0x2a8, &args![sum]);
        } else {
            e.call(0x0069_3ef0, &args![this, 0u32]);
        }
        let mover = e.get(this.cast::<Actor>(), Actor::pActorMover).addr();
        e.call(0x009e_a570, &args![mover, forward]);
        let mover = e.get(this.cast::<Actor>(), Actor::pActorMover).addr();
        e.vcall(mover, 0x14, &args![delta]);
        e.call(0x0094_7b10, &args![this]);
        if e.mem.u16(word_slot) & 0xf != 0 {
            moved_flag = true;
        }
        if e.call(0x00a2_4660, &args![main_state, 0x19u32, 1u32]).u32() != 0 {
            let reactor = e.global::<u32>(FAST_TRAVEL_REACTOR);
            e.call(0x0085_09a0, &args![reactor]);
        }
        if e.call(0x00a2_4660, &args![main_state, 0x1au32, 1u32]).u32() != 0 {
            let reactor = e.global::<u32>(FAST_TRAVEL_REACTOR);
            e.call(0x0085_09f0, &args![reactor]);
        }
        let quick_slot = e.call(0x00a2_4660, &args![main_state, 0x10u32, 1u32]).i32() > 0; // -0x309
        let mut run_body = e.global::<u8>(QUICK_STATE_011E0780) != 0;
        if !run_body && quick_slot {
            run_body = !(e.call(0x005a_03f0, &args![this, 8u32]).bool()
                || e.vcall(this.addr(), 0x214, &args![]).u32() != 0
                || e.call(0x0044_ddc0, &args![VATS_OBJECT]).u32() == 4
                || e.call(0x0096_7ae0, &args![this]).bool()
                || e.call(0x0070_5a00, &args![]).bool()
                || e.call(0x0044_ddc0, &args![VATS_OBJECT]).u32() != 0
                || e.call(0x0088_46e0, &args![this]).u32() & 0x800 != 0);
        }
        if run_body {
            if e.call(0x007f_52c0, &args![]).bool() {
                if quick_slot {
                    let player = e.global::<u32>(PLAYER_SINGLETON);
                    let flag = !e.call(0x004e_af60, &args![player]).bool();
                    e.set_global::<u8>(VATS_ENDED_THIRD_PERSON, flag as u8);
                }
                let mut pipboy_ok = false; // -0x30a
                let class = e.call(0x008a_7570, &args![this]).u32();
                if class == u32::MAX || class == 7 {
                    let a = e
                        .call(0x0049_1040, &args![animation_third_person, 4u32])
                        .u32();
                    let b = e
                        .call(0x0049_1040, &args![animation_third_person, 4u32])
                        .u32();
                    let x = if a != 0 {
                        e.call(0x0048_f7f0, &args![a]).u32()
                    } else {
                        0
                    };
                    let id = if x != 0 {
                        e.call(0x005f_2420, &args![x]).i32()
                    } else {
                        0xff
                    };
                    if !(0xc4..0xc7).contains(&id) {
                        pipboy_ok = true;
                    } else if a != 0 {
                        e.call(0x0049_74a0, &args![animation_third_person]);
                        e.call(0x0049_94f0, &args![animation_third_person, 4u32, 0u32]);
                        if b != 0 {
                            e.call(0x0049_74a0, &args![animation_first_person]);
                            e.call(0x0049_94f0, &args![animation_first_person, 4u32, 0u32]);
                        }
                    }
                }
                let mut opened = false;
                if pipboy_ok && !e.get(this, PlayerCharacter::bTemp1stPerson) {
                    let first = e.call(0x0071_6440, &args![]).f64();
                    let second = e.call(0x0094_43f0, &args![]).f64();
                    if first == second
                        && !e.get(this, PlayerCharacter::bTemp3rdPerson)
                        && e.call(0x008a_16d0, &args![this]).bool()
                        && !e.call(0x0095_0090, &args![this]).bool()
                        && !e.get(this, PlayerCharacter::b3rdPerson)
                        && e.mem.u8(this.addr() + 0x64c) == e.mem.u8(this.addr() + 0x64a)
                    {
                        opened = true;
                    }
                }
                if opened {
                    let form = e.call(0x005d_2860, &args![]).u32();
                    e.call(0x0052_9c90, &args![form]);
                    e.call(0x0086_fd90, &args![main, 1u32]);
                    e.call(0x0070_5640, &args![]);
                    e.set_global::<u8>(QUICK_STATE_011E0780, 0);
                } else {
                    e.set_global::<u8>(QUICK_STATE_011E0780, 1);
                    e.call(0x008a_6840, &args![this, 1u32]);
                    if e.call(0x0095_0090, &args![this]).bool()
                        || e.get(this, PlayerCharacter::b3rdPerson)
                    {
                        e.call(0x0095_00a0, &args![this]);
                        e.call(0x0095_1a10, &args![this, 1u32]);
                        e.set(this, PlayerCharacter::bWant3rdPerson, false);
                        e.set(this, PlayerCharacter::b3rdPerson, false);
                    }
                }
            } else {
                e.with_stack(12, |e, found| {
                    let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
                    let result = e
                        .call(
                            AUDIO_GET_SOUND_HANDLE_BY_NAME,
                            &args![audio, found, QUICK_MENU_SOUND_NAME, 0x121u32],
                        )
                        .u32();
                    e.call(SOUND_HANDLE_PLAY, &args![result, 0u32]);
                    e.call(SOUND_HANDLE_DESTRUCT, &args![found]);
                });
                e.set_global::<u8>(QUICK_STATE_011E0780, 0);
            }
            moved_flag = true;
        }
        if !e.call(0x0096_7ae0, &args![this]).bool()
            && e.global::<u32>(PENDING_OBJECT_011E0788) == 0
            && !e.get(this, PlayerCharacter::bTemp1stPerson)
            && (!e.get(this, PlayerCharacter::bTemp3rdPerson)
                || e.global::<u8>(STATE_FLAG_011E07B8) != 0)
            && e.global::<u8>(QUICK_STATE_011E0780) == 0
        {
            if (e.call(0x00a2_4660, &args![main_state, 0x0du32, 1u32]).u32() != 0
                || e.call(0x00a2_4660, &args![main_state, 0x0du32, 0u32]).u32() != 0)
                && !e.call(0x005a_03f0, &args![this, 0x10u32]).bool()
            {
                {
                    if !e.call(0x0070_2360, &args![]).bool()
                        && e.global::<u8>(CAMERA_FLAG_011A3B31) != 0
                    {
                        moved_flag = true;
                        e.set_global::<u8>(STATE_FLAG_011E07C1, 1);
                        if e.global::<u8>(STATE_FLAG_011E07B8) == 0 {
                            let actually = e.mem.u8(this.addr() + 0x64b);
                            e.set_global::<u8>(SAVED_VIEW_011E0BC0, actually);
                            e.call(0x0095_0110, &args![this, 0u32]);
                            e.set_global(CAMERA_VALUE_011E07C4, 0.0f32);
                            e.set_global::<u8>(STATE_FLAG_011E07B9, 0);
                            e.set_global::<u8>(STATE_FLAG_011E07B8, 1);
                            e.call(0x0077_1700, &args![0x14u32]);
                            e.set_global(CAMERA_VALUE_011E0B60, 0.0f32);
                            let value: f32 = e.global(CAMERA_VALUE_011E0B5C);
                            e.set_global(CAMERA_VALUE_011E0BBC, value);
                        }
                        let timer: f32 = e.global(CAMERA_TIMER_011E07C8);
                        e.set_global(CAMERA_TIMER_011E07C8, (timer as f64 + delta as f64) as f32);
                    }
                }
            } else if e.call(0x00a2_4660, &args![main_state, 0x0du32, 2u32]).u32() != 0
                || (e.global::<u8>(STATE_FLAG_011E07B8) != 0
                    && e.global::<u8>(STATE_FLAG_011E07B9) == 0)
            {
                e.set_global(CAMERA_VALUE_011E07C4, 0.0f32);
                e.set_global::<u8>(STATE_FLAG_011E07B9, 0);
                e.set_global::<u8>(STATE_FLAG_011E07B8, 0);
                e.call(0x0077_1700, &args![1u32]);
                let limit = setting_float(e, CAMERA_SETTING_011CDE8C);
                let timer: f32 = e.global(CAMERA_TIMER_011E07C8);
                if limit as f64 > timer as f64 {
                    let same = if e.call(0x004b_71d0, &args![]).bool() {
                        true
                    } else {
                        let a: f32 = e.global(CAMERA_VALUE_011E0B5C);
                        let b: f32 = e.global(CAMERA_VALUE_011E0BBC);
                        a == b
                    };
                    if same && !e.call(0x005a_03f0, &args![this, 0x10u32]).bool() {
                        if e.global::<u8>(SAVED_VIEW_011E0BC0) != 0 {
                            let flag = e.mem.u8(this.addr() + 0x64e) == 0;
                            e.call(0x0095_0110, &args![this, flag as u32]);
                        }
                        e.set(this, PlayerCharacter::bTemp3rdPerson, false);
                        e.set(this, PlayerCharacter::bTemp3rdPersonSwitchBack, false);
                    }
                }
                e.set_global(CAMERA_TIMER_011E07C8, 0.0f32);
                moved_flag = true;
                e.set_global::<u8>(STATE_FLAG_011E07C1, 0);
                e.set_global(CAMERA_VALUE_011E0B58, 0.0f32);
                e.set_global(CAMERA_VALUE_011E0B60, 0.0f32);
            }
        }
        let mut weapon_ready = false; // -0x345
        let process = process_of(e, this);
        if e.vcall(process, 0x4c8, &args![]).u32() != 0 && e.mem.u16(word_slot) & 0xf != 0 {
            weapon_ready = true;
        }
        if e.call(0x00a2_4660, &args![main_state, 5u32, 1u32]).u32() != 0 || weapon_ready {
            'second_activation: {
                if e.global::<u8>(QUICK_STATE_011E0780) != 0
                    || e.call(0x0096_7ae0, &args![this]).bool()
                    || e.call(0x0070_2360, &args![]).bool()
                    || e.vcall(this.addr(), 0x234, &args![]).bool()
                    || grab_flag
                    || e.call(0x0057_21e0, &args![this]).bool()
                {
                    break 'second_activation;
                }
                let reference = e.call(0x0070_3350, &args![]).u32(); // -0x34c
                let mut allow = true; // -0x34d
                let mut blocked = false; // -0x34e
                if reference != 0 && e.call(0x0056_8680, &args![reference]).bool() && cached {
                    blocked = true;
                }
                if e.call(0x005a_03f0, &args![this, 1u32]).bool() {
                    blocked = true;
                }
                if reference != 0 && e.vcall(reference, 0x224, &args![]).bool() {
                    let cast = e
                        .call(
                            0x00ec_43fb,
                            &args![reference, 0u32, 0x0118_41ccu32, 0x011a_28e0u32, 0u32],
                        )
                        .u32();
                    if cast != 0 && e.call(0x008c_e390, &args![cast]).u32() == 0 {
                        blocked = true;
                    }
                }
                if reference != 0 && e.vcall(reference, 0x100, &args![]).bool() {
                    let ref_process = e.call(0x008d_8520, &args![reference]).u32();
                    let distance: f32 = e.global(INTERACT_DISTANCE_01084D20);
                    if !e.vcall(reference, 0x22c, &args![0u32]).bool() {
                        let position = e.vcall(reference, 0x1f4, &args![]).u32();
                        let cell = e.call(0x008d_6f30, &args![reference]).u32();
                        if e.call(0x0088_5520, &args![reference, position, cell, distance])
                            .bool()
                        {
                            blocked = true;
                        } else {
                            let cell = e.call(0x008d_6f30, &args![this]).u32();
                            let player = e.global::<u32>(PLAYER_SINGLETON);
                            let position = e.vcall(player, 0x1f4, &args![]).u32();
                            if e.call(0x0088_5520, &args![player, position, cell, distance])
                                .bool()
                            {
                                blocked = true;
                            } else if ref_process != 0
                                && e.vcall(ref_process, 0x610, &args![]).u32() == 5
                            {
                                blocked = true;
                            } else if ref_process != 0
                                && e.vcall(ref_process, 0x610, &args![]).u32() == 6
                            {
                                blocked = true;
                            }
                        }
                    }
                }
                let state = e.vcall(this.addr(), 0x214, &args![]).u32();
                let index = state.wrapping_sub(1);
                if index <= 9 && index != 3 && index != 8 {
                    blocked = true;
                }
                if e.call(0x008a_7570, &args![this]).u32() != u32::MAX {
                    blocked = true;
                }
                if e.mem.u8(this.addr() + 0x64c) != e.mem.u8(this.addr() + 0x64a) {
                    blocked = true;
                }
                if e.call(0x0094_4340, &args![this]).bool() {
                    blocked = true;
                }
                let mut activated = false; // -0x365
                if !blocked {
                    if reference == 0 || !e.vcall(reference, 0x100, &args![]).bool() {
                        e.call(0x0070_62e0, &args![0u32, 0u32, 0u32]);
                    }
                    if reference != 0 && !weapon_ready {
                        e.with_stack(8, |e, list| {
                            e.call(LIST_CONSTRUCT, &args![list]);
                            e.call(
                                0x005e_58f0,
                                &args![0x1bu32, this, reference, list, reference],
                            );
                            if !e.call(LIST_IS_EMPTY, &args![list]).bool() {
                                if !e.call(0x007a_a720, &args![list, reference]).bool() {
                                    let activator = e.call(0x0077_8930, &args![reference]).u32();
                                    if e.call(
                                        0x0057_3170,
                                        &args![reference, this, 0u32, 0u32, activator],
                                    )
                                    .bool()
                                    {
                                        allow = false;
                                    }
                                }
                            } else {
                                let activator = e.call(0x0077_8930, &args![reference]).u32();
                                if e.call(
                                    0x0057_3170,
                                    &args![reference, this, 0u32, 0u32, activator],
                                )
                                .bool()
                                {
                                    allow = false;
                                } else {
                                    let owner = e.call(0x0056_9160, &args![reference]).u32();
                                    if owner != 0 {
                                        let owner = e.call(0x0056_9160, &args![reference]).u32();
                                        let level =
                                            e.call(0x0043_09e0, &args![owner, reference]).u32();
                                        if level != 5
                                            && !e.call(0x0057_b460, &args![reference]).bool()
                                        {
                                            allow = false;
                                        }
                                    }
                                }
                            }
                            e.call(LIST_DESTRUCT, &args![list]);
                        });
                    } else {
                        activated = true;
                    }
                    if allow {
                        let process = process_of(e, this);
                        if e.vcall(process, 0x4c8, &args![]).u32() != 0 {
                            let process = process_of(e, this);
                            let object = e.vcall(process, 0x4c8, &args![]).u32();
                            activated = !e
                                .call(0x0057_3170, &args![object, this, 0u32, 0u32, 1u32])
                                .bool();
                        } else {
                            activated = true;
                        }
                    }
                }
                if activated || blocked {
                    e.with_stack(0x30, |e, scratch| {
                        let sound = scratch.addr();
                        let temporary = scratch.addr() + 0x14;
                        e.call(SOUND_HANDLE_CONSTRUCT, &args![sound]);
                        let found = e.call(0x005d_43c0, &args![this]).u32();
                        e.call(0x0041_8940, &args![found, sound]);
                        let sound_form = e.call(0x0082_ec10, &args![]).u32();
                        if !e.call(SOUND_HANDLE_IS_VALID, &args![sound]).bool() && sound_form != 0 {
                            let name = e.call(0x0051_1840, &args![sound_form]).u32();
                            let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
                            let result = e
                                .call(
                                    0x00ad_7480,
                                    &args![audio, temporary, name, 0x121u32, sound_form],
                                )
                                .u32();
                            e.call(SOUND_HANDLE_ASSIGN, &args![sound, result]);
                            e.call(SOUND_HANDLE_DESTRUCT, &args![temporary]);
                        }
                        if !e.call(0x00ad_8930, &args![sound]).bool() {
                            e.call(SOUND_HANDLE_PLAY, &args![sound, 0u32]);
                        }
                        let found = e.call(0x005d_43c0, &args![this]).u32();
                        e.call(0x0041_a090, &args![found, sound]);
                        e.call(SOUND_HANDLE_DESTRUCT, &args![sound]);
                    });
                }
                moved_flag = true;
            }
        }
        // ---- 00943487: camera orbit, effects and animation refresh ----
        if e.global::<u8>(STATE_FLAG_011E07B9) != 0 {
            if moved_flag || e.call(0x005a_03f0, &args![this, 0x10u32]).bool() {
                e.call(0x0095_00a0, &args![this]);
            } else {
                let orbit_scale: f64 = e.global(ORBIT_SCALE);
                let speed_a = setting_float(e, ORBIT_SETTING_011CD4CC);
                let value: f32 = e.global(ORBIT_VALUE_011E08FC);
                let step = speed_a as f64 * orbit_scale * delta as f64;
                e.set_global(ORBIT_VALUE_011E08FC, (value as f64 - step) as f32);
                let speed_b = setting_float(e, ORBIT_SETTING_011CD198);
                let angle: f32 = e.global(ORBIT_ANGLE_011E0BB8);
                let step = speed_b as f64 * orbit_scale * delta as f64;
                let angle = (angle as f64 + step) as f32;
                e.set_global(ORBIT_ANGLE_011E0BB8, angle);
                let wave = e.call(0x004e_44b0, &args![angle]).f64();
                let amplitude = setting_float(e, ORBIT_SETTING_011CD984);
                e.set_global(
                    ORBIT_VALUE_011E08F4,
                    ((amplitude as f64 * wave) * orbit_scale) as f32,
                );
                let angle: f32 = e.global(ORBIT_ANGLE_011E0BB8);
                let two_pi: f64 = e.global(TWO_PI_DOUBLE);
                if angle as f64 > two_pi {
                    e.set_global(ORBIT_ANGLE_011E0BB8, (angle as f64 - two_pi) as f32);
                }
            }
        } else if e.global::<u8>(STATE_FLAG_011E07B8) == 0 {
            if moved_flag {
                e.set_global(CAMERA_VALUE_011E07C4, 0.0f32);
            } else {
                let idle: f32 = e.global(CAMERA_VALUE_011E07C4);
                e.set_global(CAMERA_VALUE_011E07C4, (idle as f64 + delta as f64) as f32);
            }
            if e.call(0x004b_9930, &args![IDLE_OBJECT_011E09E0]).bool() {
                let limit = setting_float(e, IDLE_SETTING_011CCF50);
                let idle: f32 = e.global(CAMERA_VALUE_011E07C4);
                if (limit as f64) < idle as f64
                    && !e.call(0x005a_03f0, &args![this, 0x10u32]).bool()
                {
                    let value: f32 = e.global(CAMERA_VALUE_011E0B5C);
                    e.set_global(CAMERA_VALUE_011E07BC, value);
                    e.set_global(ORBIT_VALUE_011E08FC, 0.0f32);
                    if e.get(this, PlayerCharacter::b3rdPerson) {
                        let heading = e.call(0x0093_1d70, &args![this]).f32();
                        e.set_global(ORBIT_VALUE_011E08F4, heading);
                    } else {
                        e.set_global(ORBIT_VALUE_011E08F4, 0.0f32);
                    }
                    e.call(0x0095_0340, &args![this, 1u32]);
                    e.set_global::<u8>(STATE_FLAG_011E07B9, 1);
                    e.set_global::<u8>(STATE_FLAG_011E07B8, 1);
                    e.call(0x0077_1700, &args![0x14u32]);
                    e.set_global::<u8>(STATE_FLAG_011E07C3, 1);
                }
            }
        }
        e.call(0x0095_f6c0, &args![this, delta]);
        let marker = e.global::<u32>(MARKER_OBJECT_011E07EC);
        if marker != 0 {
            let count = e.global::<u32>(MARKER_COUNT_011E0BB4);
            e.set_global(MARKER_COUNT_011E0BB4, count.wrapping_add(1));
            if count < 0x14 {
                let mut at = read_floats(e, position_copy);
                let z: f64 = e.global(MARKER_OFFSET_Z);
                let xy: f64 = e.global(MARKER_OFFSET_XY);
                at[2] = (at[2] as f64 + z) as f32;
                at[1] = (at[1] as f64 + xy) as f32;
                at[0] = (at[0] as f64 + xy) as f32;
                let marker_position = local(frame, -0x3b4);
                for (i, value) in at.iter().enumerate() {
                    e.mem.set_f32(marker_position + 4 * i as u32, *value);
                }
                e.call(0x0056_10f0, &args![marker, marker_position]);
                e.call(0x0056_15d0, &args![marker, ZERO_VECTOR]);
            }
        }
        if !e.vcall(this.addr(), 0x22c, &args![0u32]).bool()
            && !e.vcall(this.addr(), 0x2e8, &args![]).bool()
        {
            e.call(0x008c_3c40, &args![this, 0u32, 0u32]);
        }
        let value = e.call(0x0094_42a0, &args![animation_third_person]).f32();
        e.call(0x0089_50f0, &args![animation_first_person, value]);
        let value = e.call(0x0050_8070, &args![animation_third_person]).f32();
        e.call(0x004c_0c90, &args![animation_first_person, value]);
        if !e.call(0x0070_50d0, &args![]).bool() {
            e.call(0x0095_de30, &args![this, delta]);
        }
        for _ in 0..2 {
            let toggled = !e.get(this, PlayerCharacter::b3rdPerson);
            e.set(this, PlayerCharacter::b3rdPerson, toggled);
            e.call(0x008d_3550, &args![this, delta]);
            let first_person = !e.get(this, PlayerCharacter::b3rdPerson);
            let animation = e.call(0x0095_0a60, &args![this, first_person]).u32();
            e.call(0x0088_85e0, &args![this, animation, delta]);
        }
        e.call(0x008b_a600, &args![this]);
        if e.global::<u8>(VATS_ENDED_FLAG) == 0 {
            e.call(0x0094_ae40, &args![this, 0u32, 0u32]);
        }
        let parent = e.call(0x008d_6f30, &args![this]).u32();
        e.call(0x0054_a070, &args![parent, this, 1u32, 0u32]);
        let node = e.vcall(this.addr(), 0x1d0, &args![]).u32();
        let tes_object = e.call(0x0045_0b80, &args![0u32]).u32();
        e.call(0x00b5_d9f0, &args![tes_object, node, 1u32]);
        let process = process_of(e, this);
        let tracked = e.vcall(process, 0x6b8, &args![]).u32();
        if tracked != 0 {
            let process = process_of(e, this);
            let tracked = e.vcall(process, 0x6b8, &args![]).u32();
            e.call(0x009b_b080, &args![tracked, delta, this]);
        }
        if e.vcall(this.addr(), 0x22c, &args![0u32]).bool() {
            let record_is_end = e.call(0x0044_ddc0, &args![VATS_OBJECT]).u32() == 4 && {
                let record = e.call(0x009c_71c0, &args![VATS_OBJECT]).u32();
                record != 0 && {
                    let record = e.call(0x009c_71c0, &args![VATS_OBJECT]).u32();
                    e.mem.u32(record) == 0x0f
                }
            };
            if !record_is_end {
                e.call(0x009c_a1f0, &args![VATS_OBJECT]);
            }
        }
        e.call(0x0055_5c20, &args![]);
        // ---- 009438fb: tracked cell, level up, combat updates ----
        let tracked_cell = e.global::<u32>(TRACKED_CELL_011E0BB0);
        if tracked_cell == 0 || tracked_cell != e.call(0x008d_6f30, &args![this]).u32() {
            let cell = e.call(0x008d_6f30, &args![this]).u32();
            e.set_global(TRACKED_CELL_011E0BB0, cell);
            let manager = e.global::<u32>(CELL_MANAGER_011C3B3C);
            if e.call(0x0044_7950, &args![manager, cell]).bool() {
                let tes = e.global::<u32>(TES_SINGLETON);
                e.call(0x0045_81e0, &args![tes, CELL_CHANGE_STRING]);
                let cell = e.global::<u32>(TRACKED_CELL_011E0BB0);
                let manager = e.global::<u32>(CELL_MANAGER_011C3B3C);
                e.call(0x0044_6b50, &args![manager, cell]);
                let io = e.global::<u32>(IO_MANAGER);
                e.call(0x00c3_dfa0, &args![io, 0u32]);
            }
        }
        let status_slot = local(frame, -0x2c8);
        e.mem.set_u8(status_slot, 0);
        let progression = e.call(0x0046_4e10, &args![this]).u32();
        if e.call(0x008d_51f0, &args![progression]).bool()
            && !e.call(0x0095_3c50, &args![this, status_slot]).bool()
            && !e.call(0x0052_5430, &args![VATS_OBJECT]).bool()
            && e.call(0x0094_44a0, &args![]).bool()
            && e.call(0x0070_9be0, &args![]).bool()
            && e.call(0x0094_4270, &args![]).u8() == 0
            && !e.call(0x005c_7870, &args![this]).bool()
        {
            let progression = e.call(0x0046_4e10, &args![this]).u32();
            e.call(0x008d_5210, &args![progression]);
        }
        let reputation = e.get(this, PlayerCharacter::pReputationUpdate);
        if !reputation.is_null()
            && !e.call(0x0095_3c50, &args![this, status_slot]).bool()
            && !e.call(0x0052_5430, &args![VATS_OBJECT]).bool()
            && !e.call(0x0070_50d0, &args![]).bool()
            && !e.call(0x004a_4040, &args![]).bool()
        {
            let reputation = e.get(this, PlayerCharacter::pReputationUpdate);
            e.call(0x0061_55f0, &args![reputation]);
            e.set(this, PlayerCharacter::pReputationUpdate, Ptr::NULL);
        }
        if e.get(this.cast::<Actor>(), Actor::pCurrentProcess).addr() != 0 {
            let process = process_of(e, this);
            e.vcall(process, 0x104, &args![]);
        }
        e.call(0x0094_44d0, &args![this, delta]);
        let player = e.global::<u32>(PLAYER_SINGLETON);
        if e.call(0x0089_4900, &args![player]).bool() {
            let factor: f32 = e.global(FACE_FACTOR_011E0BAC);
            if (factor as f64) < one {
                e.set_global(FACE_FACTOR_011E0BAC, 1.0f32);
                let face = e.call(0x008a_dcb0, &args![this]).u32();
                if face != 0 {
                    e.vcall(face, 0x114, &args![0x0eu32, 1.0f32]);
                }
            }
        } else {
            let factor: f32 = e.global(FACE_FACTOR_011E0BAC);
            if factor as f64 > zero {
                e.set_global(FACE_FACTOR_011E0BAC, 0.0f32);
                let face = e.call(0x008a_dcb0, &args![this]).u32();
                if face != 0 {
                    let relax: f32 = e.global(FACE_RELAX_01016248);
                    e.vcall(face, 0xb4, &args![relax, 1u32, 0u32, 0u32, 0u32, 0u32]);
                }
            }
        }
        if looked_flag {
            let combat_limit = setting_float(e, COMBAT_SETTING_011CE3A8);
            let combat_timer = e.get(this, PlayerCharacter::fCombatTimer);
            let yield_timer = e.get(this, PlayerCharacter::fYieldTimer);
            if combat_limit as f64 > combat_timer as f64
                && yield_timer as f64 <= zero
                && e.call(0x0096_7da0, &args![this]).u32() != 0
            {
                let interval = setting_float(e, YIELD_SETTING_011CEDB8);
                e.set(this, PlayerCharacter::fYieldTimer, interval);
            }
        }
        let yield_timer = e.get(this, PlayerCharacter::fYieldTimer);
        if yield_timer as f64 > zero {
            let reduced = (yield_timer as f64 - delta as f64) as f32;
            e.set(this, PlayerCharacter::fYieldTimer, reduced);
            let floor = e.call(0x0040_4010, &args![reduced, 0.0f32]).f32();
            e.set(this, PlayerCharacter::fYieldTimer, floor);
        }
        let process = process_of(e, this);
        let target = e.vcall(process, 0x148, &args![]).u32();
        if target != 0 {
            let process = process_of(e, this);
            let target = e.vcall(process, 0x148, &args![]).u32();
            let target_state = e.call(0x0044_ddc0, &args![target]).u32(); // -0x3c0
            let process = process_of(e, this);
            let weapon = e.vcall(process, 0x148, &args![]).u32();
            let flag = e.call(0x004b_da70, &args![weapon, 6u32]).bool();
            let rate = e.call(0x0070_9430, &args![target_state, flag as u32]).f64();
            if rate > zero {
                let regen = e.call(0x0094_4300, &args![this]).f64();
                let elapsed = e.call(0x0084_d030, &args![GAME_TIMER]).f64();
                e.call(0x0094_42e0, &args![this, (regen - elapsed) as f32]);
                let process = process_of(e, this);
                let list = e.vcall(process, 0x14c, &args![]).u32();
                if list != 0 {
                    let regen = e.call(0x0094_4300, &args![this]).f64();
                    if regen <= zero {
                        let process = process_of(e, this);
                        let weapon = e.vcall(process, 0x148, &args![]).u32();
                        let flag = e.call(0x004b_da70, &args![weapon, 6u32]).bool();
                        let rate = e.call(0x0070_9430, &args![target_state, flag as u32]).f32();
                        e.call(0x0094_42e0, &args![this, rate]);
                        let process = process_of(e, this);
                        let list = e.vcall(process, 0x14c, &args![]).u32();
                        let count = e.call(0x0072_6070, &args![list]).i32();
                        let process = process_of(e, this);
                        let weapon = e.vcall(process, 0x148, &args![]).u32();
                        let flag = e.call(0x004b_da70, &args![weapon, 2u32]).bool();
                        let limit = e.call(0x004f_e160, &args![target_state, flag as u32]).i32();
                        if count < limit {
                            let next = count + 1;
                            let process = process_of(e, this);
                            let list = e.vcall(process, 0x14c, &args![]).u32();
                            e.call(0x006e_cd40, &args![list, next]);
                        }
                    }
                } else {
                    let regen = e.call(0x0094_4300, &args![this]).f64();
                    if regen <= zero {
                        e.with_stack(24, |e, object| {
                            e.call(0x0048_1610, &args![object]);
                            let player = e.global::<u32>(PLAYER_SINGLETON);
                            let first = e
                                .call(0x0052_5980, &args![combat_target_state, player])
                                .u32();
                            e.call(0x0048_18e0, &args![object, first, 1u32, 0u32]);
                            let player = e.global::<u32>(PLAYER_SINGLETON);
                            e.call(0x0048_21a0, &args![object, player, 1u32]);
                            let player = e.global::<u32>(PLAYER_SINGLETON);
                            let second = e
                                .call(0x0052_5980, &args![combat_target_state, player])
                                .u32();
                            let player = e.global::<u32>(PLAYER_SINGLETON);
                            e.call(
                                0x0088_c830,
                                &args![player, second, 1u32, 0u32, 1u32, 0u32, 0u32],
                            );
                            e.call(0x0048_1680, &args![object]);
                        });
                    }
                }
            }
        }
        if e.call(0x0049_38e0, &args![this]).bool() {
            let cell = e.call(0x008d_6f30, &args![this]).u32();
            if cell != 0 {
                let mut list = e.call(0x0096_04f0, &args![cell]).u32();
                let now = e.call(0x0045_7fe0, &args![]).u32();
                if e.global::<u32>(ALERT_TIME_011E0BA8) < now {
                    while list != 0 {
                        let item = e.call(LIST_ITEM_ADDRESS, &args![list]).u32();
                        if e.mem.u32(item) == 0 {
                            break;
                        }
                        let item = e.call(LIST_ITEM_ADDRESS, &args![list]).u32();
                        let reference = e.mem.u32(item);
                        if e.call(0x0040_1170, &args![reference]).u32() == 0x3a {
                            let item = e.call(LIST_ITEM_ADDRESS, &args![list]).u32();
                            let base = e.mem.u32(item);
                            let form = e.call(0x007a_f430, &args![base]).u32();
                            let cast = e
                                .call(
                                    0x00ec_43fb,
                                    &args![form, 0u32, 0x0118_3108u32, 0x0118_6568u32, 0u32],
                                )
                                .u32();
                            if cast != 0 && e.mem.i8(cast + 0x50) != -1 {
                                let item = e.call(LIST_ITEM_ADDRESS, &args![list]).u32();
                                let first = e.mem.u32(item);
                                if e.vcall(first, 0x1d0, &args![]).u32() != 0 {
                                    let item = e.call(LIST_ITEM_ADDRESS, &args![list]).u32();
                                    let second = e.mem.u32(item);
                                    let node = e.vcall(second, 0x1d0, &args![]).u32();
                                    let radius = e.call(0x0087_ce50, &args![node]).f32();
                                    let radius_squared = (radius as f64 * radius as f64) as f32;
                                    let item = e.call(LIST_ITEM_ADDRESS, &args![list]).u32();
                                    let third = e.mem.u32(item);
                                    let own = e.vcall(this.addr(), 0x1f4, &args![]).u32();
                                    let other = e.vcall(third, 0x1f4, &args![]).u32();
                                    let offset = local(frame, -0x40c);
                                    let difference =
                                        e.call(0x0043_9ef0, &args![other, offset, own]).u32();
                                    let distance = e.call(0x004a_7290, &args![difference]).f64();
                                    if distance < radius_squared as f64 {
                                        let kind = e.mem.i8(cast + 0x50) as i32;
                                        let sound_form = e.call(0x005e_2a70, &args![kind]).u32();
                                        if sound_form != 0 {
                                            e.with_stack(12, |e, found| {
                                                let id =
                                                    e.call(0x0084_e3a0, &args![sound_form]).u32();
                                                let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
                                                let result = e
                                                    .call(
                                                        0x00ad_73b0,
                                                        &args![audio, found, id, 0x101u32],
                                                    )
                                                    .u32();
                                                e.call(SOUND_HANDLE_PLAY, &args![result, 0u32]);
                                                e.call(SOUND_HANDLE_DESTRUCT, &args![found]);
                                            });
                                            let now = e.call(0x0045_7fe0, &args![]).u32();
                                            let wait = e
                                                .call(0x0094_4460, &args![0x5dcu32, 0x7d0u32])
                                                .u32();
                                            e.set_global(
                                                ALERT_TIME_011E0BA8,
                                                now.wrapping_add(wait),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        list = e.call(LIST_NEXT, &args![list]).u32();
                    }
                }
            }
        }
        e.call(0x0096_4260, &args![this]);
        let beam = e
            .get(this.cast::<Actor>(), Actor::pContinuousBeamPersistant)
            .addr();
        if beam != 0 {
            let mut keep_beam = true; // -0x425
            let process = process_of(e, this);
            let list = e.vcall(process, 0x14c, &args![]).u32();
            if list == 0 {
                keep_beam = false;
            } else {
                let count = e.call(0x0072_6070, &args![list]).i32();
                let limit = e.call(0x0052_4b60, &args![combat_target_state]).u8() as i32;
                if count < limit {
                    keep_beam = false;
                }
            }
            if !keep_beam
                || e.call(0x00a2_4660, &args![main_state, 4u32, 2u32]).u32() != 0
                || e.call(0x00a2_4660, &args![main_state, 4u32, 0u32]).u32() == 0
            {
                e.call(0x009a_b9a0, &args![beam, this]);
            }
            let beam = e
                .get(this.cast::<Actor>(), Actor::pContinuousBeamPersistant)
                .addr();
            e.vcall(beam, 0x310, &args![0.0f32]);
        }
        let last_hello = e.get(this, PlayerCharacter::fLastHelloTime);
        if last_hello as f64 > zero {
            let now = e.call(0x0045_7fe0, &args![]).u32();
            let elapsed = now as f64 - last_hello as f64;
            let limit = setting_float(e, HELLO_SETTING_011D03A0);
            let scale: f64 = e.global(HELLO_SCALE);
            if (limit as f64 * scale) < elapsed {
                let reset: f32 = e.global(TIMER_RESET);
                e.set(this, PlayerCharacter::fLastHelloTime, reset);
            }
        }
    }
    // Every exit of the function (the early ones have already done the same) ends
    // the sound handle and the scope guard.
    e.call(SOUND_HANDLE_DESTRUCT, &args![handle]);
    e.call(0x0040_4ee0, &args![guard]);
}
// Translated from 00944270 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the byte at `+0x4CD` of the object the global `011d8a80` points to
/// (0 when it is null).
pub fn fn_00944270(e: &mut Engine) -> u8 {
    let object = e.global::<u32>(0x011d_8a80);
    if object == 0 {
        0
    } else {
        e.mem.u8(object + 0x4cd)
    }
}

// Translated from 009442a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the float at `+0x10C` of the object.
pub fn fn_009442a0(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr().wrapping_add(0x10c))
}

// Translated from 009442c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x1000` of the word at `+0x12C` of the object is set.
pub fn fn_009442c0(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr().wrapping_add(0x12c)) & 0x1000 != 0
}

// Translated from 009442e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the float at `+0x1F4` (for the player `fTimeSinceLastAmmoRegenTick`).
pub fn fn_009442e0(e: &mut Engine, this: Ptr<PlayerCharacter>, value: f32) {
    e.set(this, PlayerCharacter::fTimeSinceLastAmmoRegenTick, value);
}

/// Allocates 8 bytes and builds a `BSSimpleList` in them with `0096a2d0`,
/// the way the constructor creates each heap list (null when the allocation
/// fails). Returns what the list constructor returns.
fn new_list(e: &mut Engine) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![8u32]).u32();
    if block != 0 {
        e.call(LIST_CONSTRUCT, &args![block]).u32()
    } else {
        0
    }
}

// Translated from 00938180 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter::PlayerCharacter` (Xbox PDB): constructs the player in
/// the 0xE50 bytes at `this` and returns it. Runs the `Actor` base
/// constructor (`008d1d30`), installs the six vtables, constructs every
/// embedded member (lists, smart pointers, sound handles, the per-perk lists)
/// and gives every field its initial value; it also creates the player's
/// process (a 0x46C-byte object), the `CameraCaster`, the active-effect and
/// other heap lists, and the two 0x1A0-byte objects the globals
/// `011e0774` and `011e0760` point to.
pub fn player_character_construct(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
) -> Ptr<PlayerCharacter> {
    let t = this.addr();
    e.call(0x008d_1d30, &args![this]);
    e.mem.set_u32(t, 0x0108_aa3c);
    e.mem.set_u32(t + 0x18, 0x0108_aa30);
    e.mem.set_u32(t + 0x88, 0x0108_a9dc);
    e.mem.set_u32(t + 0x94, 0x0108_a9a4);
    e.mem.set_u32(t + 0xa4, 0x0108_a974);
    e.mem.set_u32(t + 0xa8, 0x0108_a92c);

    // Embedded members, in declaration order.
    e.call(
        LIST_CONSTRUCT,
        &args![member(this, PlayerCharacter::listNotes)],
    );
    e.call(
        LIST_CONSTRUCT,
        &args![member(this, PlayerCharacter::pListOfTeammates)],
    );
    for field in [
        PlayerCharacter::spGrabSpring,
        PlayerCharacter::sp1stPerson3D,
        PlayerCharacter::spInventoryPC,
    ] {
        e.call(NI_POINTER_CONSTRUCT, &args![member(this, field), 0u32]);
    }
    for field in [
        PlayerCharacter::listTopics,
        PlayerCharacter::listQuestLog,
        PlayerCharacter::listObjectives,
        PlayerCharacter::listQuestTargets,
    ] {
        e.call(LIST_CONSTRUCT, &args![member(this, field)]);
    }
    e.call(
        0x006f_48b0,
        &args![member(this, PlayerCharacter::PlayerMarkerPath)],
    );
    e.call(
        0x004f_6320,
        &args![member(this, PlayerCharacter::m_AllOccupiedRegions), 1u32],
    );
    e.call(
        LIST_CONSTRUCT,
        &args![member(this, PlayerCharacter::CurrentRegionSoundList)],
    );
    e.call(
        SOUND_HANDLE_CONSTRUCT,
        &args![member(this, PlayerCharacter::StatusSoundHandle)],
    );
    e.call(
        LIST_ITEM_ADDRESS,
        &args![member(this, PlayerCharacter::LastKnownGoodPosition)],
    );
    e.call(
        LIST_CONSTRUCT,
        &args![member(this, PlayerCharacter::MapMarkerList)],
    );
    e.call(
        LIST_CONSTRUCT,
        &args![member(this, PlayerCharacter::AudioMarkerList)],
    );
    e.call(
        LIST_ITEM_ADDRESS,
        &args![member(this, PlayerCharacter::UFOCameraPos)],
    );
    e.call(
        SOUND_HANDLE_CONSTRUCT,
        &args![member(this, PlayerCharacter::SelectedSpellCastSound)],
    );
    e.call(
        VECTOR_CONSTRUCTOR_ITERATOR,
        &args![
            member(this, PlayerCharacter::MagicFailureSounds),
            0xcu32,
            6u32,
            SOUND_HANDLE_CONSTRUCT,
            SOUND_HANDLE_DESTRUCT
        ],
    );
    e.call(
        LIST_CONSTRUCT,
        &args![member(this, PlayerCharacter::DroppedRefList)],
    );
    e.call(
        0x0096_a280,
        &args![member(this, PlayerCharacter::RandomDoorSpaceMap), 0x25u32],
    );
    e.call(
        NI_POINTER_CONSTRUCT,
        &args![member(this, PlayerCharacter::sp1stPersonLight), 0u32],
    );
    e.call(
        NI_POINTER_CONSTRUCT,
        &args![member(this, PlayerCharacter::sp3rdPersonLight), 0u32],
    );
    e.call(
        0x008d_4360,
        &args![member(this, PlayerCharacter::CharacterProgressionInfo)],
    );
    e.call(LIST_CONSTRUCT, &args![member(this, PlayerCharacter::Perks)]);
    e.call(
        VECTOR_CONSTRUCTOR_ITERATOR,
        &args![
            member(this, PlayerCharacter::PerkEntryLists),
            8u32,
            0x4au32,
            LIST_CONSTRUCT,
            LIST_DESTRUCT
        ],
    );
    e.call(
        LIST_CONSTRUCT,
        &args![member(this, PlayerCharacter::CompanionPerks)],
    );
    e.call(
        VECTOR_CONSTRUCTOR_ITERATOR,
        &args![
            member(this, PlayerCharacter::CompanionPerkEntryLists),
            8u32,
            0x4au32,
            LIST_CONSTRUCT,
            LIST_DESTRUCT
        ],
    );
    e.call(
        LIST_ITEM_ADDRESS,
        &args![member(this, PlayerCharacter::BulletAutoAim)],
    );
    e.call(
        NI_POINTER_CONSTRUCT,
        &args![member(this, PlayerCharacter::spTargeted3D), 0u32],
    );
    for field in [
        PlayerCharacter::kCamera3rdPersonShoulderOffset,
        PlayerCharacter::Cached1stPersonCameraPos,
        PlayerCharacter::CachedWorldCameraPos,
    ] {
        e.call(LIST_ITEM_ADDRESS, &args![member(this, field)]);
    }
    e.call(
        NI_POINTER_CONSTRUCT,
        &args![member(this, PlayerCharacter::spCameraRigidBody), 0u32],
    );
    e.call(
        0x0096_a570,
        &args![member(this, PlayerCharacter::RockItLauncherAmmoList)],
    );
    e.call(
        0x0096_a5c0,
        &args![member(this, PlayerCharacter::HotKeyLastAmmo)],
    );

    // A scope guard the game keeps on its stack for the rest of the body
    // (constructed with `00404eb0`, destroyed with `00404ee0`).
    e.with_stack(4, |e, guard| {
        e.call(
            0x0040_4eb0,
            &args![guard, 0x34u32, 1u32, 0x0108_a8e0u32, 0x2c7u32],
        );
        e.call(0x0048_4ab0, &args![this, 0u32]);
        set_zero_vector(e, member(this, PlayerCharacter::Cached1stPersonCameraPos));
        set_zero_vector(e, member(this, PlayerCharacter::CachedWorldCameraPos));
        e.call(0x008d_66b0, &args![]);
        e.call(0x008d_1820, &args![]);
        let setting: u32 = e.call(SETTING_FLOAT_GETTER, &args![0x011c_cf2cu32]).u32();
        let value = e.mem.f32(setting);
        e.set_global(0x011e_0b5c, value);
        e.set(this, PlayerCharacter::bAiControlledActivate, false);

        for i in 0..0x4d {
            e.mem.set_f32(
                member(this, PlayerCharacter::TemporaryActorValueModifiers) + 4 * i,
                0.0,
            );
            e.mem.set_f32(
                member(this, PlayerCharacter::ScriptActorValueModifiers) + 4 * i,
                0.0,
            );
            e.mem.set_f32(
                member(this, PlayerCharacter::DamageActorValueModifiers) + 4 * i,
                0.0,
            );
        }
        e.set(this, PlayerCharacter::fTimeSinceLastAmmoRegenTick, 0.0);
        e.set(this, PlayerCharacter::cShotsSinceLastAmmoRegen, 0);
        e.set(this, PlayerCharacter::fHealthModifier, 0.0);
        e.set(this, PlayerCharacter::iSleepTime, 0);
        e.set(this, PlayerCharacter::fsecondRunning, 0.0);
        e.set(this, PlayerCharacter::fsecondSwimming, 0.0);
        e.set(this, PlayerCharacter::fsecondSneaking, 0.0);
        e.set(this, PlayerCharacter::iAmountStolenSold, 0);
        e.set(this, PlayerCharacter::iNumAdvance, 0);
        e.set(this, PlayerCharacter::btransporting, false);
        e.set(this, PlayerCharacter::bCanFastTravel, false);
        e.call(0x005d_14d0, &args![this, 1u32]);
        e.set(this, PlayerCharacter::bCanWait, true);
        e.set(this, PlayerCharacter::bChargen, false);
        e.set(this, PlayerCharacter::bAllowEGMCacheClear, true);
        e.set(this, PlayerCharacter::eskilladvance, 0x4d);
        e.set(this, PlayerCharacter::pDefaultClass, Ptr::NULL);
        e.set(this, PlayerCharacter::pListofActions, Ptr::NULL);
        e.set(this, PlayerCharacter::pAutoAimActor, Ptr::NULL);
        set_zero_vector(e, member(this, PlayerCharacter::BulletAutoAim));
        e.set(this, PlayerCharacter::pPlayersTargetActor, Ptr::NULL);
        e.set(this, PlayerCharacter::bAiControlledPackage, false);
        e.set(this, PlayerCharacter::pClassBasedOn, Ptr::NULL);
        e.set(this, PlayerCharacter::bTravelUseDoor, false);
        e.call(
            NI_POINTER_ASSIGN,
            &args![member(this, PlayerCharacter::sp1stPersonLight), 0u32],
        );
        e.call(
            NI_POINTER_ASSIGN,
            &args![member(this, PlayerCharacter::sp3rdPersonLight), 0u32],
        );
        e.set(this, PlayerCharacter::bOnElevator, true);

        // The player's process: a 0x46C-byte object built here, told about the
        // process it replaces (virtual +4), registered with the process lists
        // before and after the swap, while the old one is destroyed.
        let block = e.call(OPERATOR_NEW, &args![0x46cu32]).u32();
        let process = if block != 0 {
            e.call(0x008d_7510, &args![block]).u32()
        } else {
            0
        };
        let old_process = e.mem.u32(at(this, ACTOR_CURRENT_PROCESS));
        e.vcall(process, 4, &args![old_process]);
        let key = e.call(0x0093_1850, &args![this]).u32();
        e.call(0x0096_d470, &args![PROCESS_LISTS, this, key]);
        let old_process = e.mem.u32(at(this, ACTOR_CURRENT_PROCESS));
        if old_process != 0 {
            e.vcall(old_process, 0, &args![1u32]);
        }
        e.mem.set_u32(at(this, ACTOR_CURRENT_PROCESS), process);
        let key = e.call(0x0093_1850, &args![this]).u32();
        e.call(0x0096_d470, &args![PROCESS_LISTS, this, key]);

        let caster_base: Ptr<MagicCasterBase> = Ptr::new(t + MAGIC_CASTER_BASE);
        fn_00939800(e, caster_base, 0);
        fn_00939840(e, caster_base, 0);

        let effects = new_list(e);
        e.set(this, PlayerCharacter::pActiveEffectList, Ptr::new(effects));

        e.call(0x0093_0c70, &args![this]);
        let camera_owner: Ptr = e.call(0x0093_06d0, &args![this]).ptr();
        if !camera_owner.is_null() {
            fn_00939860(e, camera_owner, 0x3e8);
            e.call(0x0045_34f0, &args![this, 1u32]);
            let index = 9u32;
            e.call(0x00c6_d770, &args![camera_owner, index]);
            e.with_stack(4, |e, out| {
                e.call(0x0070_c440, &args![camera_owner, out]);
            });
            let block = e.call(OPERATOR_NEW, &args![8u32]).u32();
            let caster = if block != 0 {
                let setting = e.call(SETTING_FLOAT_GETTER, &args![0x011e_0934u32]).u32();
                let value = e.mem.f32(setting);
                e.call(0x0062_0850, &args![block, value, index]).u32()
            } else {
                0
            };
            e.set(this, PlayerCharacter::pCameraCaster, Ptr::new(caster));
        } else {
            e.set(this, PlayerCharacter::pCameraCaster, Ptr::NULL);
        }
        e.set(this, PlayerCharacter::b3rdPersonSaved, false);
        e.set(this, PlayerCharacter::bsave3rdPerson, false);
        e.set(this, PlayerCharacter::b3rdPerson, false);
        e.set(this, PlayerCharacter::bActually3rdPerson, false);
        e.set(this, PlayerCharacter::bWant3rdPerson, false);
        e.set(this, PlayerCharacter::bTemp3rdPerson, false);
        e.set(this, PlayerCharacter::bTemp3rdPersonSwitchBack, false);
        e.set(this, PlayerCharacter::bTemp1stPerson, false);
        e.set(this, PlayerCharacter::bTemp1stPersonSwitchBack, false);
        let setting = e.call(SETTING_BYTE_GETTER, &args![0x011e_0b4cu32]).u32();
        let always_run = e.mem.u8(setting);
        e.set(this, PlayerCharacter::bAlwaysRun, always_run != 0);
        e.set(this, PlayerCharacter::bAutoMove, false);
        let setting = e.call(SETTING_FLOAT_GETTER, &args![0x0120_3150u32]).u32();
        let value = e.mem.f32(setting);
        e.call(0x0095_0610, &args![this, value]);
        e.set(this, PlayerCharacter::p1stPersonBipedAnim, Ptr::NULL);
        e.set(this, PlayerCharacter::p1stPersonAnimation, Ptr::NULL);
        e.call(
            NI_POINTER_ASSIGN,
            &args![member(this, PlayerCharacter::sp1stPerson3D), 0u32],
        );
        let eye_height = e.global::<f32>(EYE_HEIGHT_DEFAULT);
        e.set(this, PlayerCharacter::fEyeHeight, eye_height);
        e.call(
            NI_POINTER_ASSIGN,
            &args![member(this, PlayerCharacter::spInventoryPC), 0u32],
        );
        e.set(this, PlayerCharacter::pInventoryAnimation, Ptr::NULL);
        e.set(this, PlayerCharacter::pInventoryAnimation, Ptr::NULL);
        set_zero_vector_global(e);
        e.set_global(0x011e_0768, 0.0f32);
        e.set_global(0x011e_07c3, 0u8);
        e.set(this, PlayerCharacter::bGreetingPlayer, false);
        e.set(this, PlayerCharacter::ucControlsDisabled, 0);
        e.set(this, PlayerCharacter::pClosestConversation, Ptr::NULL);
        e.set(this, PlayerCharacter::bBlockActivate, false);
        let timer = e.global::<f32>(BLOCK_ACTIVATE_TIMER_RESET);
        e.set(this, PlayerCharacter::fBlockActivateTimer, timer);
        e.set(this, PlayerCharacter::ihourstosleep, 0);
        e.set(this, PlayerCharacter::fSitHeadingDelta, 0.0);
        e.set(this, PlayerCharacter::cMurder, 0);
        e.set(this, PlayerCharacter::bBeenAttacked, false);
        e.set(this, PlayerCharacter::pSelectedSpell, Ptr::NULL);
        e.set(this, PlayerCharacter::pSelectedScroll, Ptr::NULL);
        e.set(this, PlayerCharacter::pPlayerMapMarker, Ptr::NULL);
        e.set(this, PlayerCharacter::pActiveQuest, Ptr::NULL);

        let eat_drink = new_list(e);
        e.set(this, PlayerCharacter::EatDrinkItems, Ptr::new(eat_drink));
        e.set(this, PlayerCharacter::QueuedWornEnchantments, Ptr::NULL);
        e.set(this, PlayerCharacter::iNumberTraining, 0);
        e.call(0x0051_9020, &args![this, 0u32]);
        e.set(this, PlayerCharacter::bIsSleeping, false);
        e.set(this, PlayerCharacter::pPendingPoison, Ptr::NULL);
        e.set(this, PlayerCharacter::bTelekinesisSelected, false);
        e.set(this, PlayerCharacter::pOccupiedRegion, Ptr::NULL);
        e.call(
            NI_POINTER_ASSIGN,
            &args![member(this, PlayerCharacter::spGrabSpring), 0u32],
        );
        e.set(this, PlayerCharacter::pGrabbedObject, Ptr::NULL);
        e.set(this, PlayerCharacter::eGrabType, 0);
        e.set(this, PlayerCharacter::fGrabObjectWeight, 0.0);
        e.set(this, PlayerCharacter::fGrabDistance, 0.0);
        e.set(this, PlayerCharacter::pInitialStateBuffer, Ptr::NULL);
        e.set(this, PlayerCharacter::iTotalPlayingTime, 0);
        e.call(0x0085_1d10, &args![this]);
        for i in 0..5 {
            e.mem
                .set_u32(member(this, PlayerCharacter::pCrimeCounts) + 4 * i, 0);
        }
        let seed = e.call(0x0061_b9d0, &args![0u32]).u32();
        e.set(this, PlayerCharacter::iCharacterSeed, seed);
        fn_0093a5f0(e, this, false);
        fn_0093a6f0(e, this, false);
        e.set(this, PlayerCharacter::bInBorderContainedCell, false);
        e.set(this, PlayerCharacter::bReturnToLastKnownGoodPosition, false);
        set_zero_vector(e, member(this, PlayerCharacter::LastKnownGoodPosition));
        e.set(this, PlayerCharacter::pLastKnownGoodLocation, Ptr::NULL);
        e.set(this, PlayerCharacter::pLastKnownMusicType, Ptr::NULL);
        e.set(this, PlayerCharacter::pBorderRegions, Ptr::NULL);

        let casino = new_list(e);
        e.set(this, PlayerCharacter::pListofCasinoData, Ptr::new(casino));
        let inactive = new_list(e);
        e.set(
            this,
            PlayerCharacter::pInactiveListofCaravanCards,
            Ptr::new(inactive),
        );
        let active = new_list(e);
        e.set(
            this,
            PlayerCharacter::pActiveListofCaravanCards,
            Ptr::new(active),
        );
        e.set(this, PlayerCharacter::iCaravanCapWinnings, 0);
        e.set(this, PlayerCharacter::iCaravanCapLosses, 0);
        e.set(this, PlayerCharacter::iCaravanWinnings, 0);
        e.set(this, PlayerCharacter::iCaravanLosses, 0);
        e.set(this, PlayerCharacter::iCaravanLargestWinning, 0);
        e.set(this, PlayerCharacter::iCasinoCheatLevel, 0);
        e.call(0x0096_9e90, &args![this, 0u32, 0u32]);
        e.set(this, PlayerCharacter::bAlwaysHardcore, true);
        let setting = e.call(SETTING_INT_GETTER, &args![0x011e_0940u32]).u32();
        let difficulty = e.mem.u32(setting);
        e.set(this, PlayerCharacter::eDifficultyLevel, difficulty);
        let setting = e.call(SETTING_INT_GETTER, &args![0x011e_087cu32]).u32();
        let kill_camera = e.mem.u32(setting);
        e.set(this, PlayerCharacter::eKillCameraSetting, kill_camera);
        e.set(this, PlayerCharacter::pInventoryWeaponEffect, Ptr::NULL);
        e.set(this, PlayerCharacter::bBeingChased, false);
        e.set(this, PlayerCharacter::bIsYoung, false);
        e.set(this, PlayerCharacter::bIsToddler, false);
        e.set(this, PlayerCharacter::bCanUsePowerArmor, false);
        e.set(this, PlayerCharacter::iCombatPersue, 0);
        e.set(this, PlayerCharacter::pMapWorld, Ptr::NULL);
        e.set(this, PlayerCharacter::fUFOCameraHeading, 0.0);
        e.set(this, PlayerCharacter::fUFOCameraPitch, 0.0);
        set_zero_vector(e, member(this, PlayerCharacter::UFOCameraPos));
        e.set(this, PlayerCharacter::bShowQuestItemsInInventory, true);
        e.set(this, PlayerCharacter::iSelectedSpellCastSoundID, 0);

        // The selected-spell sound and the six failure sounds are replaced by
        // freshly constructed (empty) handles.
        e.with_stack(0xc, |e, handle| {
            let fresh = e.call(SOUND_HANDLE_CONSTRUCT, &args![handle]).u32();
            e.call(
                SOUND_HANDLE_ASSIGN,
                &args![member(this, PlayerCharacter::SelectedSpellCastSound), fresh],
            );
            e.call(SOUND_HANDLE_DESTRUCT, &args![handle]);
        });
        for i in 0..6u32 {
            e.with_stack(0xc, |e, handle| {
                let fresh = e.call(SOUND_HANDLE_CONSTRUCT, &args![handle]).u32();
                e.call(
                    SOUND_HANDLE_ASSIGN,
                    &args![
                        member(this, PlayerCharacter::MagicFailureSounds) + i * 0xc,
                        fresh
                    ],
                );
                e.call(SOUND_HANDLE_DESTRUCT, &args![handle]);
            });
        }
        e.set(this, PlayerCharacter::pLastExtDoorActivated, Ptr::NULL);
        e.call(
            LIST_CLEAR,
            &args![member(this, PlayerCharacter::DroppedRefList)],
        );
        e.call(0x0096_27f0, &args![this]);
        e.call(0x004e_d780, &args![0x011c_d8f0u32, 0.0f32]);
        e.set(this, PlayerCharacter::fDropAngleMod, 0.0);
        e.set(this, PlayerCharacter::fLastDropAngleMod, 0.0);
        let flags = e.call(0x0044_ddc0, &args![this]).u32() | 0x400;
        e.call(0x0040_3550, &args![this, flags]);
        let setting = e.call(SETTING_FLOAT_GETTER, &args![0x0120_315cu32]).u32();
        let value = e.mem.f32(setting);
        e.set(this, PlayerCharacter::fWorldFOV, value);
        let setting = e.call(SETTING_FLOAT_GETTER, &args![0x011c_da58u32]).u32();
        let value = e.mem.f32(setting);
        e.set(this, PlayerCharacter::f3rdPersonFOV, value);
        let setting = e.call(SETTING_FLOAT_GETTER, &args![0x0120_3168u32]).u32();
        let value = e.mem.f32(setting);
        e.set(this, PlayerCharacter::f1stPersonFOV, value);
        let process = e.mem.u32(at(this, ACTOR_CURRENT_PROCESS));
        e.vcall(process, 0x6e0, &args![0u32]);
        e.call(0x0096_4190, &args![this]);
        e.set(this, PlayerCharacter::fProjectileReleaseTimer, 0.0);
        e.set(this, PlayerCharacter::iNumberofStealWarnings, 0);
        let timer = e.global::<f32>(TIMER_RESET);
        e.set(this, PlayerCharacter::fStealWarningTimer, timer);
        e.set(this, PlayerCharacter::iNumberofPickpocketWarnings, 0);
        e.set(this, PlayerCharacter::fPickPocketWarningTimer, timer);
        e.set(this, PlayerCharacter::pListofPercievedActors, Ptr::NULL);
        e.set(this, PlayerCharacter::bMenuModeButtonClicked, false);
        e.set(this, PlayerCharacter::fMenuModeButtonTimer, 0.0);
        e.set(this, PlayerCharacter::fAmmoSwapButtonTimer, 0.0);
        e.set(this, PlayerCharacter::pIronsightsDOFInstance, Ptr::NULL);
        e.set(this, PlayerCharacter::pVatsDOFInstance, Ptr::NULL);
        e.set(this, PlayerCharacter::pVatsDRBInstance, Ptr::NULL);
        set_zero_vector(
            e,
            member(this, PlayerCharacter::kCamera3rdPersonShoulderOffset),
        );
        e.set(this, PlayerCharacter::bSpeaking, false);
        e.set(this, PlayerCharacter::pQueuedTargetLoc, Ptr::NULL);
        e.set(this, PlayerCharacter::pQueuedWeaponAttach, Ptr::NULL);
        e.set(this, PlayerCharacter::pCombatGroup, Ptr::NULL);
        e.set(this, PlayerCharacter::iTeammateCount, 0);
        e.set(this, PlayerCharacter::bPlayerInCombat, false);
        e.set(this, PlayerCharacter::bAllCombatTargetsSearching, false);
        e.set(this, PlayerCharacter::fCombatTimer, 0.0);
        e.set(this, PlayerCharacter::fYieldTimer, 0.0);
        e.call(
            NI_POINTER_ASSIGN,
            &args![member(this, PlayerCharacter::spCameraRigidBody), 0u32],
        );
        let value = e.call(0x0054_6a90, &args![]).u32();
        e.call(0x0056_7dd0, &args![this, value]);
        e.call(0x005d_add0, &args![this, 0u32]);

        // Two 0x1A0-byte objects, held by globals.
        for global in [0x011e_0774u32, 0x011e_0760] {
            let block = e.call(OPERATOR_NEW, &args![0x1a0u32]).u32();
            let object = if block != 0 {
                fn_00939880(e, Ptr::new(block)).addr()
            } else {
                0
            };
            e.set_global(global, object);
        }

        e.call(
            MEMSET,
            &args![member(this, PlayerCharacter::pWobbleNodes), 0u32, 0x60u32],
        );
        e.call(0x0096_9110, &args![this, 0u32]);
        e.set(this, PlayerCharacter::pAIConversationRunning, Ptr::NULL);
        e.set(this, PlayerCharacter::bHostileDetection, false);
        e.set(this, PlayerCharacter::iSandmanDetection, 0);
        e.call(
            NI_POINTER_ASSIGN,
            &args![member(this, PlayerCharacter::spTargeted3D), 0u32],
        );
        e.set(this, PlayerCharacter::bTarget3DDistant, false);
        e.set(this, PlayerCharacter::bNightVisionOn, false);
        e.set(this, PlayerCharacter::bTurboISM, false);
        e.set(this, PlayerCharacter::pReputationUpdate, Ptr::NULL);
        e.set(this, PlayerCharacter::fTimeInSlowMoCam, 0.0);
        e.set(this, PlayerCharacter::fKillCamCooldown, 0.0);
        e.set(this, PlayerCharacter::bIgnoresGTM, false);
        e.mem.set_u32(at(this, ACTOR_CONTINUOUS_BEAM_PERSISTANT), 0);
        e.set(this, PlayerCharacter::bIsAcousticSpaceTransition, false);
        let hello = e.global::<f32>(TIMER_RESET);
        e.set(this, PlayerCharacter::fLastHelloTime, hello);
        for i in 0..3u32 {
            let entry = member(this, PlayerCharacter::PlayerAchievements) + i * 0xc;
            e.mem.set_u8(entry, 0);
            e.mem.set_u32(entry + 4, 0);
            let flags = e.mem.i8(ACHIEVEMENT_TABLE + i * 0x28) as i32;
            e.mem.set_u8(entry + 8, (!(flags & 2) != 0) as u8);
        }
        e.set(this, PlayerCharacter::pClosestAudioMarkerInfo, Ptr::NULL);
        e.set(this, PlayerCharacter::fCounterAttackTimer, 0.0);
        e.set(this, PlayerCharacter::bCounterAttackCamera, false);
        e.set(this, PlayerCharacter::bHasCateyeActive, false);
        e.set(this, PlayerCharacter::bHasSpotterActive, false);
        e.set(this, PlayerCharacter::fCheckForItems, 0.0);
        e.call(0x008b_bbf0, &args![this, 0u32]);
        for _ in 0..8 {
            e.with_stack(4, |e, null| {
                e.call(
                    0x007c_b2e0,
                    &args![member(this, PlayerCharacter::HotKeyLastAmmo), null],
                );
            });
        }
        e.set(this, PlayerCharacter::pcOnlyWord, 0);
        e.call(0x0040_4ee0, &args![guard]);
    });
    this
}

/// How the destructor releases each item of one of the player's heap lists.
#[derive(Clone, Copy)]
enum ItemRelease {
    /// `operator delete` on the item (the loop stops at a null item).
    Delete,
    /// The item's scalar deleting destructor (virtual `+0`, argument 1),
    /// skipped for a null item.
    VirtualDestructor,
    /// The item's virtual `+0x10` with argument 1; the loop stops at a null
    /// item.
    VirtualSlot10,
}

/// Releases the items of one of the player's heap lists (see
/// [`ItemRelease`]): takes the head node's item, releases it and removes the
/// head node (`0063f7b0`) until the list is gone or finished, then deletes the
/// list object (`004702f0`). `check_empty_first` skips the whole step when
/// `008256d0` says the list is empty; `check_empty_each_pass` ends the loop
/// when it says so.
fn destroy_heap_list(
    e: &mut Engine,
    this: Ptr<PlayerCharacter>,
    slot: Field<PlayerCharacter, Ptr>,
    release: ItemRelease,
    check_empty_each_pass: bool,
    check_empty_first: bool,
) {
    if e.get(this, slot).is_null() {
        return;
    }
    if check_empty_first {
        let list = e.get(this, slot);
        if e.call(LIST_IS_EMPTY, &args![list]).bool() {
            return;
        }
    }
    loop {
        let list = e.get(this, slot);
        if list.is_null() {
            break;
        }
        if check_empty_each_pass && e.call(LIST_IS_EMPTY, &args![list]).bool() {
            break;
        }
        let node = e.call(LIST_ITEM_ADDRESS, &args![list]).u32();
        let item = e.mem.u32(node);
        match release {
            ItemRelease::Delete => {
                if item == 0 {
                    break;
                }
                let list = e.get(this, slot);
                let node = e.call(LIST_ITEM_ADDRESS, &args![list]).u32();
                let item = e.mem.u32(node);
                e.call(OPERATOR_DELETE, &args![item]);
            }
            ItemRelease::VirtualDestructor => {
                if item != 0 {
                    e.vcall(item, 0, &args![1u32]);
                }
            }
            ItemRelease::VirtualSlot10 => {
                if item == 0 {
                    break;
                }
                let list = e.get(this, slot);
                let node = e.call(LIST_ITEM_ADDRESS, &args![list]).u32();
                let item = e.mem.u32(node);
                if item != 0 {
                    e.vcall(item, 0x10, &args![1u32]);
                }
            }
        }
        let list = e.get(this, slot);
        e.call(LIST_REMOVE_HEAD, &args![list]);
    }
    let list = e.get(this, slot);
    if !list.is_null() {
        e.call(LIST_DELETING_DESTRUCT, &args![list, 1u32]);
    }
}

/// Frees the item of every node of the embedded list at `list` with
/// `operator delete`, walking with `00726070`.
fn free_embedded_list_items(e: &mut Engine, list: u32) {
    let mut node = list;
    while node != 0 {
        let item_address = e.call(LIST_ITEM_ADDRESS, &args![node]).u32();
        let item = e.mem.u32(item_address);
        e.call(OPERATOR_DELETE, &args![item]);
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
}

// Translated from 009398d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The body of `PlayerCharacter::~PlayerCharacter` (Xbox PDB name of the
/// function the engine map does not name): resets the six vtables to this
/// class's, releases the player's process, the camera caster, every heap list
/// and its items, the sounds, the per-perk lists and the two 0x1A0-byte
/// objects the constructor created, destroys the embedded members in reverse
/// order and runs the `Actor` base destructor (`008d2060`). The compiler's
/// exception-unwinding state writes are not translated.
pub fn fn_009398d0(e: &mut Engine, this: Ptr<PlayerCharacter>) {
    let t = this.addr();
    e.mem.set_u32(t, 0x0108_aa3c);
    e.mem.set_u32(t + 0x18, 0x0108_aa30);
    e.mem.set_u32(t + 0x88, 0x0108_a9dc);
    e.mem.set_u32(t + 0x94, 0x0108_a9a4);
    e.mem.set_u32(t + 0xa4, 0x0108_a974);
    e.mem.set_u32(t + 0xa8, 0x0108_a92c);

    e.call(0x0094_7d10, &args![this]);
    e.call(0x0094_8050, &args![this]);
    e.call(LIST_CLEAR, &args![t + 0x768]);
    e.call(0x008d_68d0, &args![]);
    e.call(0x008d_1920, &args![]);
    e.call(0x0095_0c20, &args![this, 0u32]);

    // The process.
    let process = e.mem.u32(t + ACTOR_CURRENT_PROCESS);
    if process != 0 {
        e.vcall(process, 0, &args![1u32]);
    }
    e.mem.set_u32(t + ACTOR_CURRENT_PROCESS, 0);

    // Lists of the eat/drink items and queued worn enchantments.
    for slot in [
        PlayerCharacter::EatDrinkItems,
        PlayerCharacter::QueuedWornEnchantments,
    ] {
        let list = e.get(this, slot);
        if !list.is_null() {
            e.call(LIST_CLEAR, &args![list]);
            let list = e.get(this, slot);
            if !list.is_null() {
                e.call(LIST_DELETING_DESTRUCT, &args![list, 1u32]);
            }
        }
    }

    // The camera caster.
    let caster = e.get(this, PlayerCharacter::pCameraCaster);
    if !caster.is_null() {
        camera_caster_vector_deleting_destructor(e, caster, 1);
    }

    e.call(
        NI_POINTER_ASSIGN,
        &args![member(this, PlayerCharacter::spGrabSpring), 0u32],
    );
    let selected_spell = e.get(this, PlayerCharacter::pSelectedSpell);
    if !selected_spell.is_null() {
        e.call(0x0040_b800, &args![selected_spell, 1u32]);
    }

    destroy_heap_list(
        e,
        this,
        PlayerCharacter::pActiveEffectList,
        ItemRelease::VirtualDestructor,
        true,
        false,
    );
    destroy_heap_list(
        e,
        this,
        PlayerCharacter::pListofActions,
        ItemRelease::Delete,
        false,
        false,
    );
    destroy_heap_list(
        e,
        this,
        PlayerCharacter::pListofCasinoData,
        ItemRelease::Delete,
        false,
        true,
    );
    destroy_heap_list(
        e,
        this,
        PlayerCharacter::pInactiveListofCaravanCards,
        ItemRelease::VirtualSlot10,
        false,
        true,
    );
    destroy_heap_list(
        e,
        this,
        PlayerCharacter::pActiveListofCaravanCards,
        ItemRelease::VirtualSlot10,
        false,
        true,
    );

    e.call(0x008b_3180, &args![this]);
    for field in [
        PlayerCharacter::listTopics,
        PlayerCharacter::listQuestLog,
        PlayerCharacter::listQuestTargets,
        PlayerCharacter::listObjectives,
    ] {
        e.call(LIST_CLEAR, &args![member(this, field)]);
    }
    e.call(0x0096_7290, &args![this]);
    e.call(0x0096_90a0, &args![this]);

    let weapon_effect = e.get(this, PlayerCharacter::pInventoryWeaponEffect);
    if !weapon_effect.is_null() {
        e.vcall(weapon_effect.addr(), 0, &args![1u32]);
    }

    e.call(0x0094_eb40, &args![this, 0u32, 1u32]);
    let initial_state = e.get(this, PlayerCharacter::pInitialStateBuffer);
    if !initial_state.is_null() {
        let object = e.global::<u32>(0x011d_e45c);
        e.call(0x0085_8700, &args![object, initial_state]);
    }
    let border_regions = e.get(this, PlayerCharacter::pBorderRegions);
    if !border_regions.is_null() {
        e.call(0x005e_03d0, &args![border_regions]);
        let border_regions = e.get(this, PlayerCharacter::pBorderRegions);
        if !border_regions.is_null() {
            e.vcall(border_regions.addr(), 0, &args![1u32]);
        }
        e.set(this, PlayerCharacter::pBorderRegions, Ptr::NULL);
    }
    e.call(0x0096_1f90, &args![this]);

    // Stop and clear the six failure sounds.
    for i in 0..6u32 {
        let handle = member(this, PlayerCharacter::MagicFailureSounds) + i * 0xc;
        if e.call(SOUND_HANDLE_IS_VALID, &args![handle]).bool() {
            e.call(SOUND_HANDLE_STOP, &args![handle]);
            e.with_stack(0xc, |e, empty| {
                let fresh = e.call(SOUND_HANDLE_CONSTRUCT, &args![empty]).u32();
                e.call(SOUND_HANDLE_ASSIGN, &args![handle, fresh]);
                e.call(SOUND_HANDLE_DESTRUCT, &args![empty]);
            });
        }
    }

    e.call(
        LIST_CLEAR,
        &args![member(this, PlayerCharacter::DroppedRefList)],
    );
    e.call(0x0096_2490, &args![this, 0u32, 1u32]);
    e.call(0x0096_2490, &args![this, 0u32, 0u32]);
    e.call(0x0070_5fc0, &args![0x011e_0aa8u32, 0u32]);
    e.call(
        0x00e9_8e20,
        &args![0x011e_0aa8u32, ZERO_VECTOR, ZERO_VECTOR, 0u32],
    );

    // The perk lists.
    free_embedded_list_items(e, member(this, PlayerCharacter::Perks));
    e.call(LIST_CLEAR, &args![member(this, PlayerCharacter::Perks)]);
    for i in 0..0x4au32 {
        e.call(
            LIST_CLEAR,
            &args![member(this, PlayerCharacter::PerkEntryLists) + i * 8],
        );
    }
    free_embedded_list_items(e, member(this, PlayerCharacter::CompanionPerks));
    e.call(
        LIST_CLEAR,
        &args![member(this, PlayerCharacter::CompanionPerks)],
    );
    for i in 0..0x4au32 {
        e.call(
            LIST_CLEAR,
            &args![member(this, PlayerCharacter::CompanionPerkEntryLists) + i * 8],
        );
    }

    e.call(LIST_CLEAR, &args![member(this, PlayerCharacter::listNotes)]);
    e.call(0x0096_55a0, &args![this]);
    e.call(0x0096_55e0, &args![this]);
    e.call(0x0095_2f90, &args![this]);
    let object = e.global::<u32>(0x011e_0774);
    e.call(OPERATOR_DELETE, &args![object]);
    let object = e.global::<u32>(0x011e_0760);
    e.call(OPERATOR_DELETE, &args![object]);

    // Members, in reverse order of construction.
    e.call(
        0x0096_a5f0,
        &args![member(this, PlayerCharacter::HotKeyLastAmmo)],
    );
    e.call(
        0x0096_a5a0,
        &args![member(this, PlayerCharacter::RockItLauncherAmmoList)],
    );
    e.call(
        NI_POINTER_DESTRUCT,
        &args![member(this, PlayerCharacter::spCameraRigidBody)],
    );
    e.call(
        NI_POINTER_DESTRUCT,
        &args![member(this, PlayerCharacter::spTargeted3D)],
    );
    e.call(
        VECTOR_DESTRUCTOR_ITERATOR,
        &args![
            member(this, PlayerCharacter::CompanionPerkEntryLists),
            8u32,
            0x4au32,
            LIST_DESTRUCT
        ],
    );
    e.call(
        LIST_DESTRUCT,
        &args![member(this, PlayerCharacter::CompanionPerks)],
    );
    e.call(
        VECTOR_DESTRUCTOR_ITERATOR,
        &args![
            member(this, PlayerCharacter::PerkEntryLists),
            8u32,
            0x4au32,
            LIST_DESTRUCT
        ],
    );
    e.call(LIST_DESTRUCT, &args![member(this, PlayerCharacter::Perks)]);
    e.call(
        0x008d_4380,
        &args![member(this, PlayerCharacter::CharacterProgressionInfo)],
    );
    e.call(
        NI_POINTER_DESTRUCT,
        &args![member(this, PlayerCharacter::sp3rdPersonLight)],
    );
    e.call(
        NI_POINTER_DESTRUCT,
        &args![member(this, PlayerCharacter::sp1stPersonLight)],
    );
    e.call(
        0x0096_a4b0,
        &args![member(this, PlayerCharacter::RandomDoorSpaceMap)],
    );
    e.call(
        LIST_DESTRUCT,
        &args![member(this, PlayerCharacter::DroppedRefList)],
    );
    e.call(
        VECTOR_DESTRUCTOR_ITERATOR,
        &args![
            member(this, PlayerCharacter::MagicFailureSounds),
            0xcu32,
            6u32,
            SOUND_HANDLE_DESTRUCT
        ],
    );
    e.call(
        SOUND_HANDLE_DESTRUCT,
        &args![member(this, PlayerCharacter::SelectedSpellCastSound)],
    );
    e.call(
        LIST_DESTRUCT,
        &args![member(this, PlayerCharacter::AudioMarkerList)],
    );
    e.call(
        LIST_DESTRUCT,
        &args![member(this, PlayerCharacter::MapMarkerList)],
    );
    e.call(
        SOUND_HANDLE_DESTRUCT,
        &args![member(this, PlayerCharacter::StatusSoundHandle)],
    );
    e.call(
        LIST_DESTRUCT,
        &args![member(this, PlayerCharacter::CurrentRegionSoundList)],
    );
    e.call(
        0x004f_64b0,
        &args![member(this, PlayerCharacter::m_AllOccupiedRegions)],
    );
    e.call(
        0x006f_4930,
        &args![member(this, PlayerCharacter::PlayerMarkerPath)],
    );
    for field in [
        PlayerCharacter::listQuestTargets,
        PlayerCharacter::listObjectives,
        PlayerCharacter::listQuestLog,
        PlayerCharacter::listTopics,
    ] {
        e.call(LIST_DESTRUCT, &args![member(this, field)]);
    }
    for field in [
        PlayerCharacter::spInventoryPC,
        PlayerCharacter::sp1stPerson3D,
        PlayerCharacter::spGrabSpring,
    ] {
        e.call(NI_POINTER_DESTRUCT, &args![member(this, field)]);
    }
    e.call(
        LIST_DESTRUCT,
        &args![member(this, PlayerCharacter::pListOfTeammates)],
    );
    e.call(
        LIST_DESTRUCT,
        &args![member(this, PlayerCharacter::listNotes)],
    );

    e.call(0x008d_2060, &args![this]);
}

/// The address `offset` bytes into the player.
fn at(this: Ptr<PlayerCharacter>, offset: u32) -> u32 {
    this.addr().wrapping_add(offset)
}

/// Copies the zero vector into the three words at `011e0808`.
fn set_zero_vector_global(e: &mut Engine) {
    set_zero_vector(e, 0x011e_0808);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x004a9950,
            ni_tmap_base_set_value(Ptr, Ptr<NiTMapItem>, u32, u8)
        ),
        entry!(0x005f73f0, fn_005f73f0(Ptr, u32) -> i32),
        entry!(
            0x006bf8f0,
            bs_simple_array_item_change_remove(Ptr<BSSimpleArray>, u32, bool)
        ),
        entry!(0x00815340, fn_00815340(Ptr)),
        entry!(0x00815360, fn_00815360(Ptr)),
        entry!(
            0x00938180,
            player_character_construct(Ptr<PlayerCharacter>) -> Ptr<PlayerCharacter>
        ),
        entry!(0x00939680, fn_00939680(Ptr)),
        entry!(0x009396a0, fn_009396a0(Ptr<PlayerCharacter>) -> Ptr),
        entry!(0x009396c0, fn_009396c0(Ptr, u32, u32, u32, u32, u32, u32, u32) -> bool),
        entry!(0x009396d0, fn_009396d0(Ptr<PlayerCharacter>) -> Ptr),
        entry!(0x009396f0, fn_009396f0(Ptr<PlayerCharacter>, Ptr)),
        entry!(0x00939710, fn_00939710(Ptr<MagicTargetBase>) -> Ptr),
        entry!(0x00939730, fn_00939730(Ptr<PlayerCharacter>, u32)),
        entry!(0x00939750, fn_00939750(Ptr<PlayerCharacter>) -> u32),
        entry!(0x00939770, fn_00939770(Ptr<PlayerCharacter>, u32)),
        entry!(0x00939790, fn_00939790(Ptr<PlayerCharacter>) -> u32),
        entry!(0x009397b0, fn_009397b0(Ptr<PlayerCharacter>, u8) -> Ptr),
        entry!(0x009397e0, fn_009397e0(Ptr)),
        entry!(0x00939800, fn_00939800(Ptr<MagicCasterBase>, u32)),
        entry!(0x00939820, fn_00939820(Ptr<MagicCasterBase>) -> u32),
        entry!(0x00939840, fn_00939840(Ptr<MagicCasterBase>, u32)),
        entry!(0x00939860, fn_00939860(Ptr, u32)),
        entry!(0x00939880, fn_00939880(Ptr) -> Ptr),
        entry!(
            0x009398a0,
            player_character_vector_deleting_destructor(
                Ptr<PlayerCharacter>,
                u32,
            ) -> Ptr<PlayerCharacter>
        ),
        entry!(0x009398d0, fn_009398d0(Ptr<PlayerCharacter>)),
        entry!(0x0093a5a0, camera_caster_vector_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0093a5d0, fn_0093a5d0(Ptr<PlayerCharacter>) -> bool),
        entry!(0x0093a5f0, fn_0093a5f0(Ptr<PlayerCharacter>, bool)),
        entry!(0x0093a640, fn_0093a640(Ptr<PlayerCharacter>) -> bool),
        entry!(
            0x0093a660,
            player_character_remove_actor_from_playercombat_list(Ptr<PlayerCharacter>, Ptr)
        ),
        entry!(
            0x0093a690,
            player_character_add_actor_to_player_combat_list(Ptr<PlayerCharacter>, Ptr)
        ),
        entry!(0x0093a6f0, fn_0093a6f0(Ptr<PlayerCharacter>, bool)),
        entry!(0x0093a740, fn_0093a740(Ptr<PlayerCharacter>) -> bool),
        entry!(0x0093a7a0, fn_0093a7a0(Ptr<PlayerCharacter>, Ptr)),
        entry!(
            0x0093a7c0,
            player_character_set_actor_base_value(Ptr<PlayerCharacter>, u32, i32)
        ),
        entry!(0x0093a850, fn_0093a850(Ptr<PlayerCharacter>, u32, f32)),
        entry!(0x0093a8f0, fn_0093a8f0(Ptr<PlayerCharacter>, u32, i32)),
        entry!(0x0093aa10, fn_0093aa10(Ptr<PlayerCharacter>, u32, f32)),
        entry!(0x0093ab30, fn_0093ab30(Ptr<PlayerCharacter>, u32)),
        entry!(0x0093ab80, fn_0093ab80(Ptr<PlayerCharacter>, f32)),
        entry!(0x0093abe0, fn_0093abe0(Ptr<PlayerCharacter>, Ptr, Ptr)),
        entry!(0x0093ac20, fn_0093ac20(Ptr, u32) -> i32),
        entry!(0x0093acb0, fn_0093acb0(Ptr, u32) -> f32),
        entry!(0x0093ad30, fn_0093ad30(Ptr, u32) -> i32),
        entry!(
            0x0093ad60,
            player_character_get_permanent_actor_float_value(Ptr, u32) -> f32
        ),
        entry!(0x0093ae40, fn_0093ae40(Ptr<PlayerCharacter>, u32, f32)),
        entry!(0x0093ae80, fn_0093ae80(Ptr<PlayerCharacter>, u32, i32, Ptr)),
        entry!(0x0093afb0, fn_0093afb0(Ptr<PlayerCharacter>, u32, f32, Ptr)),
        entry!(0x0093b0f0, fn_0093b0f0(Ptr<PlayerCharacter>, u32, i32, Ptr)),
        entry!(0x0093b240, fn_0093b240(Ptr<PlayerCharacter>, u32, f32, Ptr)),
        entry!(0x0093b3a0, fn_0093b3a0(Ptr<PlayerCharacter>, u32, i32, Ptr)),
        entry!(0x0093b7a0, fn_0093b7a0(Ptr<PlayerCharacter>, u32, f32, Ptr)),
        entry!(
            0x0093bba0,
            player_character_get_water_cell(Ptr<PlayerCharacter>, f32) -> u32
        ),
        entry!(
            0x0093be30,
            player_character_request_position_player(
                Ptr<PlayerCharacter>,
                Ptr<PositionPlayerRequest>,
            )
        ),
        entry!(
            0x0093bea0,
            player_character_handle_position_player_request(Ptr<PlayerCharacter>) -> bool
        ),
        entry!(0x0093c1e0, fn_0093c1e0(Ptr<PlayerCharacter>) -> bool),
        entry!(
            0x0093c200,
            player_character_position_player(
                Ptr<PlayerCharacter>,
                f32,
                f32,
                f32,
                f32,
                f32,
                f32,
                Ptr,
                bool,
            )
        ),
        entry!(0x0093ccd0, fn_0093ccd0() -> u32),
        entry!(
            0x0093cce0,
            player_character_position_player_exterior(
                Ptr<PlayerCharacter>,
                f32,
                f32,
                f32,
                f32,
                f32,
                f32,
                Ptr,
                bool,
            )
        ),
        entry!(
            0x0093cdf0,
            player_character_fast_travel(Ptr<PlayerCharacter>, Ptr)
        ),
        entry!(0x0093d4f0, fn_0093d4f0() -> bool),
        entry!(
            0x0093d500,
            player_character_clear_cells_if_target_not_loaded(Ptr<PlayerCharacter>, Ptr, Ptr)
        ),
        entry!(0x0093d660, fn_0093d660(Ptr<PlayerCharacter>) -> bool),
        entry!(0x0093db40, fn_0093db40(Ptr<PlayerCharacter>) -> bool),
        entry!(
            0x0093db60,
            player_character_center_on_cell(Ptr<PlayerCharacter>, u32, Ptr)
        ),
        entry!(0x0093dd20, fn_0093dd20(Ptr<PlayerCharacter>)),
        entry!(0x0093dd80, fn_0093dd80(Ptr<PlayerCharacter>)),
        entry!(
            0x0093dee0,
            player_character_is_target_perceived_and_hostile(Ptr<PlayerCharacter>, u32) -> bool
        ),
        entry!(0x0093df50, fn_0093df50(Ptr<PlayerCharacter>)),
        entry!(0x0093e4f0, fn_0093e4f0(Ptr) -> u32),
        entry!(0x0093e510, fn_0093e510(Ptr)),
        entry!(
            0x0093e530,
            player_character_set_slow_mo_camera(Ptr<PlayerCharacter>, u32, f32, bool, u32)
        ),
        entry!(0x0093e750, fn_0093e750()),
        entry!(0x0093e770, fn_0093e770(Ptr<PlayerCharacter>, i32, bool)),
        entry!(0x0093e840, fn_0093e840(Ptr)),
        entry!(
            0x0093e860,
            player_character_update(Ptr<PlayerCharacter>, f32)
        ),
        entry!(0x00944270, fn_00944270() -> u8),
        entry!(0x009442a0, fn_009442a0(Ptr) -> f32),
        entry!(0x009442c0, fn_009442c0(Ptr) -> bool),
        entry!(0x009442e0, fn_009442e0(Ptr<PlayerCharacter>, f32)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    fn int(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn float(value: f64) -> Ret {
        Ret {
            st0: value,
            ..Ret::default()
        }
    }

    fn f(words: &[u32], i: usize) -> f32 {
        f32::from_bits(words[i])
    }

    /// An engine that records every call.
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.call_log = Some(vec![]);
        e
    }

    fn player(e: &mut Engine) -> Ptr<PlayerCharacter> {
        e.new_object()
    }

    /// The argument lists of the calls made to `addr`.
    fn called(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, words)| words.clone())
            .collect()
    }

    fn call_addresses(e: &Engine) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(a, _)| *a)
            .collect()
    }

    /// Writes a vtable at `base` whose slots (byte offsets) point at the given
    /// targets, and makes `object` use it.
    fn put_vtable(e: &mut Engine, object: u32, base: u32, slots: &[(u32, u32)]) {
        vtable_only(e, base, slots);
        e.mem.set_u32(object, base);
    }

    /// Writes a vtable at `base` whose slots (byte offsets) point at the given
    /// targets.
    fn vtable_only(e: &mut Engine, base: u32, slots: &[(u32, u32)]) {
        e.map(base, 0x800);
        for (slot, target) in slots {
            e.mem.set_u32(base + slot, *target);
        }
    }

    /// A zeroed 0x40-byte object whose vtable has the given slots.
    fn object(e: &mut Engine, base: u32, slots: &[(u32, u32)]) -> Ptr {
        let object = Ptr::new(e.mem.alloc(0x40));
        put_vtable(e, object.addr(), base, slots);
        object
    }

    fn stub(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
    }

    /// Doubles the destructor's and constructor's callees that return their
    /// first argument (constructors) or nothing.
    fn stub_returning_this(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            e.register(*addr, |_, a| int(a[0]));
        }
    }

    #[test]
    fn ni_tmap_base_set_value_stores_the_key_and_value_in_the_item() {
        let mut e = engine();
        let item = e.new_object::<NiTMapItem>();
        e.call(0x004a_9950, &args![0u32, item, 0x1234u32, 0x56u8]);
        assert_eq!(e.get(item, NiTMapItem::m_key), 0x1234);
        assert_eq!(e.get(item, NiTMapItem::m_val), 0x56);
        assert_eq!(e.get(item, NiTMapItem::m_pkNext), Ptr::NULL);
    }

    fn actor_value_owner(e: &mut Engine, value: f32) -> Ptr {
        let target = 0x00b0_0020;
        e.register_double(target, move |_, _| float(value as f64));
        object(e, 0x0200_0000, &[(0x20, target)])
    }

    #[test]
    fn fn_005f73f0_converts_the_virtual_result_and_truncates_it() {
        for (value, expected) in [(6.75f32, 6), (-6.75, -6)] {
            let mut e = engine();
            let owner = actor_value_owner(&mut e, value);
            e.register(0x0040_6ce0, |_, a| float(f32::from_bits(a[0]) as f64));
            e.register(FTOL, |_, a| {
                let bits = a[0] as u64 | (a[1] as u64) << 32;
                int(f64::from_bits(bits) as i32 as u32)
            });
            let result = e.call(0x005f_73f0, &args![owner, 0x2au32]).i32();
            assert_eq!(result, expected);
            assert_eq!(called(&e, 0x00b0_0020), vec![vec![owner.addr(), 0x2a]]);
            assert_eq!(called(&e, 0x0040_6ce0), vec![vec![value.to_bits()]]);
        }
    }

    /// A `BSSimpleArray` of `size` pointers at a fresh buffer, with a vtable
    /// whose slot 4 hands out `new_buffer`.
    fn array(e: &mut Engine, size: u32, new_buffer: u32) -> (Ptr<BSSimpleArray>, u32) {
        let array = e.new_object::<BSSimpleArray>();
        let buffer = e.mem.alloc(64);
        e.set(array, BSSimpleArray::pBuffer, buffer);
        e.set(array, BSSimpleArray::iSize, size);
        e.set(array, BSSimpleArray::iReservedSize, 16);
        e.register_double(0x00b0_0004, move |_, _| int(new_buffer));
        put_vtable(e, array.addr(), 0x0200_0000, &[(4, 0x00b0_0004)]);
        (array, buffer)
    }

    #[test]
    fn bs_simple_array_remove_without_shrinking_shifts_the_tail_down() {
        let mut e = engine();
        let (array, buffer) = array(&mut e, 5, 0);
        stub(&mut e, &[0x0072_ba80, 0x0042_fb60]);
        e.call(0x006b_f8f0, &args![array, 1u32, false]);
        assert_eq!(
            called(&e, 0x0072_ba80),
            vec![vec![array.addr(), buffer + 4, 1]]
        );
        assert_eq!(
            called(&e, 0x0042_fb60),
            vec![vec![array.addr(), buffer + 4, buffer + 8, 3]]
        );
        assert_eq!(e.get(array, BSSimpleArray::iSize), 4);
        assert_eq!(e.get(array, BSSimpleArray::pBuffer), buffer);
    }

    #[test]
    fn bs_simple_array_remove_with_shrinking_moves_to_the_new_buffer() {
        let mut e = engine();
        let (array, buffer) = array(&mut e, 5, 0x0300_0000);
        stub(&mut e, &[0x0072_ba80, 0x0042_fb60, 0x006a_8500]);
        e.register(0x006f_3170, |_, _| int(1));
        e.register(0x0086_9600, |_, _| int(8));
        e.call(0x006b_f8f0, &args![array, 2u32, true]);
        assert_eq!(called(&e, 0x00b0_0004), vec![vec![array.addr(), 8]]);
        assert_eq!(
            called(&e, 0x0042_fb60),
            vec![
                vec![array.addr(), 0x0300_0000, buffer, 2],
                // The count of the second copy is the old size minus one, as
                // the code has it.
                vec![array.addr(), 0x0300_0008, buffer + 12, 4],
            ]
        );
        assert_eq!(
            called(&e, 0x0072_ba80),
            vec![vec![array.addr(), buffer + 8, 1]]
        );
        assert_eq!(called(&e, 0x006a_8500), vec![vec![array.addr()]]);
        assert_eq!(e.get(array, BSSimpleArray::pBuffer), 0x0300_0000);
        assert_eq!(e.get(array, BSSimpleArray::iReservedSize), 8);
        assert_eq!(e.get(array, BSSimpleArray::iSize), 4);
    }

    #[test]
    fn bs_simple_array_remove_without_the_shrink_test_passing_does_not_shrink() {
        let mut e = engine();
        let (array, buffer) = array(&mut e, 3, 0x0300_0000);
        stub(&mut e, &[0x0072_ba80, 0x0042_fb60]);
        e.register(0x006f_3170, |_, _| int(0));
        e.call(0x006b_f8f0, &args![array, 0u32, true]);
        assert!(called(&e, 0x0086_9600).is_empty());
        assert_eq!(
            called(&e, 0x0042_fb60),
            vec![vec![array.addr(), buffer, buffer + 4, 2]]
        );
        assert_eq!(e.get(array, BSSimpleArray::iSize), 2);
    }

    #[test]
    fn fn_00815360_calls_the_virtual_at_0x20() {
        let mut e = engine();
        let target = 0x00b0_0020;
        stub(&mut e, &[target]);
        let this = object(&mut e, 0x0200_0000, &[(0x20, target)]);
        e.call(0x0081_5360, &args![this]);
        assert_eq!(called(&e, target), vec![vec![this.addr()]]);
    }

    #[test]
    fn fn_00815340_calls_fn_00815360_which_calls_the_virtual_at_0x20() {
        let mut e = engine();
        let target = 0x00b0_0020;
        stub(&mut e, &[target]);
        let this = object(&mut e, 0x0200_0000, &[(0x20, target)]);
        e.call(0x0081_5340, &args![this]);
        assert_eq!(called(&e, target), vec![vec![this.addr()]]);
    }

    #[test]
    fn fn_00939680_calls_the_virtual_at_0x20() {
        let mut e = engine();
        let target = 0x00b0_0020;
        stub(&mut e, &[target]);
        let this = object(&mut e, 0x0200_0000, &[(0x20, target)]);
        e.call(0x0093_9680, &args![this]);
        assert_eq!(called(&e, target), vec![vec![this.addr()]]);
    }

    #[test]
    fn fn_009396a0_returns_the_players_target_actor() {
        let mut e = engine();
        let this = player(&mut e);
        e.set(this, PlayerCharacter::pPlayersTargetActor, Ptr::new(0x1234));
        assert_eq!(e.call(0x0093_96a0, &args![this]).u32(), 0x1234);
    }

    #[test]
    fn fn_009396c0_returns_false_whatever_the_seven_words() {
        let mut e = engine();
        let ret = e.call(
            0x0093_96c0,
            &args![0u32, 1u32, 2u32, 3u32, 4u32, 5u32, 6u32, 7u32],
        );
        assert!(!ret.bool());
    }

    #[test]
    fn fn_009396d0_returns_the_combat_group() {
        let mut e = engine();
        let this = player(&mut e);
        e.set(this, PlayerCharacter::pCombatGroup, Ptr::new(0x4321));
        assert_eq!(e.call(0x0093_96d0, &args![this]).u32(), 0x4321);
    }

    #[test]
    fn fn_009396f0_stores_the_combat_group() {
        let mut e = engine();
        let this = player(&mut e);
        e.call(0x0093_96f0, &args![this, 0x4321u32]);
        assert_eq!(e.mem.u32(this.addr() + 0xd64), 0x4321);
    }

    #[test]
    fn fn_00939710_returns_the_word_at_0x17c_of_the_magic_target_base() {
        let mut e = engine();
        let this = player(&mut e);
        e.mem.set_u32(this.addr() + 0x94 + 0x17c, 0x7777);
        assert_eq!(
            e.get(this, PlayerCharacter::pActiveEffectList).addr(),
            0x7777
        );
        assert_eq!(
            e.call(0x0093_9710, &args![this.addr() + 0x94]).u32(),
            0x7777
        );
    }

    #[test]
    fn fn_00939730_stores_the_advance_number() {
        let mut e = engine();
        let this = player(&mut e);
        e.call(0x0093_9730, &args![this, 3u32]);
        assert_eq!(e.mem.u32(this.addr() + 0x734), 3);
    }

    #[test]
    fn fn_00939750_returns_the_advance_number() {
        let mut e = engine();
        let this = player(&mut e);
        e.mem.set_u32(this.addr() + 0x734, 9);
        assert_eq!(e.call(0x0093_9750, &args![this]).u32(), 9);
    }

    #[test]
    fn fn_00939770_stores_the_skill_advance() {
        let mut e = engine();
        let this = player(&mut e);
        e.call(0x0093_9770, &args![this, 5u32]);
        assert_eq!(e.mem.u32(this.addr() + 0x738), 5);
    }

    #[test]
    fn fn_00939790_returns_the_skill_advance() {
        let mut e = engine();
        let this = player(&mut e);
        e.mem.set_u32(this.addr() + 0x738, 6);
        assert_eq!(e.call(0x0093_9790, &args![this]).u32(), 6);
    }

    #[test]
    fn fn_009397b0_picks_the_perk_list_by_the_flag() {
        let mut e = engine();
        let this = player(&mut e);
        assert_eq!(
            e.call(0x0093_97b0, &args![this, 0u8]).u32(),
            this.addr() + 0x87c
        );
        assert_eq!(
            e.call(0x0093_97b0, &args![this, 1u8]).u32(),
            this.addr() + 0xad4
        );
        assert_eq!(
            e.call(0x0093_97b0, &args![this, 7u8]).u32(),
            this.addr() + 0xad4
        );
    }

    #[test]
    fn fn_009397e0_calls_005a03f0_with_one() {
        let mut e = engine();
        stub(&mut e, &[0x005a_03f0]);
        e.call(0x0093_97e0, &args![0x5000u32]);
        assert_eq!(called(&e, 0x005a_03f0), vec![vec![0x5000, 1]]);
    }

    #[test]
    fn fn_00939800_stores_the_current_spell_through_the_magic_caster_base() {
        let mut e = engine();
        let this = player(&mut e);
        e.call(0x0093_9800, &args![this.addr() + 0x88, 0x99u32]);
        assert_eq!(e.get(this, PlayerCharacter::pCurrentSpell).addr(), 0x99);
    }

    #[test]
    fn fn_00939820_returns_the_desired_target() {
        let mut e = engine();
        let this = player(&mut e);
        e.set(this, PlayerCharacter::pDesiredTarget, Ptr::new(0x55));
        assert_eq!(e.call(0x0093_9820, &args![this.addr() + 0x88]).u32(), 0x55);
    }

    #[test]
    fn fn_00939840_stores_the_desired_target() {
        let mut e = engine();
        let this = player(&mut e);
        e.call(0x0093_9840, &args![this.addr() + 0x88, 0x66u32]);
        assert_eq!(e.get(this, PlayerCharacter::pDesiredTarget).addr(), 0x66);
    }

    #[test]
    fn fn_00939860_stores_the_word_at_0x604() {
        let mut e = engine();
        let block: Ptr = Ptr::new(e.mem.alloc(0x610));
        e.call(0x0093_9860, &args![block, 0x3e8u32]);
        assert_eq!(e.mem.u32(block.addr() + 0x604), 0x3e8);
    }

    #[test]
    fn fn_00939880_clears_the_float_and_returns_the_object() {
        let mut e = engine();
        let block: Ptr = Ptr::new(e.mem.alloc(0x1a0));
        e.mem.set_f32(block.addr() + 0x198, 2.5);
        assert_eq!(e.call(0x0093_9880, &args![block]).u32(), block.addr());
        assert_eq!(e.mem.f32(block.addr() + 0x198), 0.0);
    }

    #[test]
    fn fn_0093a5d0_returns_the_to_pos_flag() {
        let mut e = engine();
        let this = player(&mut e);
        assert!(!e.call(0x0093_a5d0, &args![this]).bool());
        e.mem.set_u8(this.addr() + 0x798, 1);
        assert!(e.call(0x0093_a5d0, &args![this]).bool());
    }

    #[test]
    fn fn_0093a640_returns_the_from_pos_flag() {
        let mut e = engine();
        let this = player(&mut e);
        assert!(!e.call(0x0093_a640, &args![this]).bool());
        e.mem.set_u8(this.addr() + 0x799, 1);
        assert!(e.call(0x0093_a640, &args![this]).bool());
    }

    #[test]
    fn fn_0093a740_is_true_when_any_ai_control_flag_is_set() {
        for (offset, expected) in [
            (0x798, true),
            (0x799, true),
            (0x79a, true),
            (0x79b, true),
            (0x79c, false),
        ] {
            let mut e = engine();
            let this = player(&mut e);
            e.mem.set_u8(this.addr() + offset, 1);
            assert_eq!(
                e.call(0x0093_a740, &args![this]).bool(),
                expected,
                "{offset:x}"
            );
        }
        let mut e = engine();
        let this = player(&mut e);
        assert!(!e.call(0x0093_a740, &args![this]).bool());
    }

    /// A player whose virtual at `+0x258` is recorded.
    fn player_with_process_virtual(e: &mut Engine) -> Ptr<PlayerCharacter> {
        let this = player(e);
        stub(e, &[0x00b0_0258]);
        put_vtable(e, this.addr(), 0x0200_0000, &[(0x258, 0x00b0_0258)]);
        this
    }

    #[test]
    fn fn_0093a5f0_stores_the_flag_and_passes_the_inverse_of_any_to_the_virtual() {
        let mut e = engine();
        let this = player_with_process_virtual(&mut e);
        e.call(0x0093_a5f0, &args![this, true]);
        assert_eq!(e.mem.u8(this.addr() + 0x798), 1);
        assert_eq!(called(&e, 0x00b0_0258), vec![vec![this.addr(), 0]]);
        e.call(0x0093_a5f0, &args![this, false]);
        assert_eq!(e.mem.u8(this.addr() + 0x798), 0);
        assert_eq!(called(&e, 0x00b0_0258)[1], vec![this.addr(), 1]);
        // Another flag keeps the result at 0.
        e.mem.set_u8(this.addr() + 0x79b, 1);
        e.call(0x0093_a5f0, &args![this, false]);
        assert_eq!(called(&e, 0x00b0_0258)[2], vec![this.addr(), 0]);
    }

    #[test]
    fn fn_0093a6f0_stores_the_flag_and_passes_the_inverse_of_any_to_the_virtual() {
        let mut e = engine();
        let this = player_with_process_virtual(&mut e);
        e.call(0x0093_a6f0, &args![this, true]);
        assert_eq!(e.mem.u8(this.addr() + 0x799), 1);
        assert_eq!(called(&e, 0x00b0_0258), vec![vec![this.addr(), 0]]);
        e.call(0x0093_a6f0, &args![this, false]);
        assert_eq!(e.mem.u8(this.addr() + 0x799), 0);
        assert_eq!(called(&e, 0x00b0_0258)[1], vec![this.addr(), 1]);
    }

    #[test]
    fn camera_caster_destructor_runs_the_body_and_frees_only_with_the_flag() {
        let mut e = engine();
        stub(&mut e, &[0x0062_08d0]);
        let caster: Ptr = Ptr::new(e.mem.alloc(8));
        assert_eq!(
            e.call(0x0093_a5a0, &args![caster, 0u32]).u32(),
            caster.addr()
        );
        assert_eq!(called(&e, 0x0062_08d0), vec![vec![caster.addr()]]);
        assert!(e.mem.block_size(caster.addr()).is_some());
        e.call(0x0093_a5a0, &args![caster, 1u32]);
        assert!(e.mem.block_size(caster.addr()).is_none());
    }

    #[test]
    fn remove_actor_from_playercombat_list_needs_a_group() {
        let mut e = engine();
        stub(&mut e, &[0x0098_6500]);
        let this = player(&mut e);
        e.call(0x0093_a660, &args![this, 0xaau32]);
        assert!(called(&e, 0x0098_6500).is_empty());
        e.set(this, PlayerCharacter::pCombatGroup, Ptr::new(0x6000));
        e.call(0x0093_a660, &args![this, 0xaau32]);
        assert_eq!(called(&e, 0x0098_6500), vec![vec![0x6000, 0xaa]]);
    }

    #[test]
    fn add_actor_to_player_combat_list_creates_the_group_once() {
        let mut e = engine();
        stub(&mut e, &[0x0098_67d0, 0x0098_6410]);
        e.map(0x011f_1000, 0x1000);
        e.set_global(COMBAT_MANAGER, 0x7100u32);
        e.register(0x0099_1e80, |_, _| int(0x6000));
        let this = player(&mut e);
        e.call(0x0093_a690, &args![this, 0xaau32]);
        assert_eq!(called(&e, 0x0099_1e80), vec![vec![0x7100]]);
        assert_eq!(called(&e, 0x0098_67d0), vec![vec![0x6000, this.addr()]]);
        assert_eq!(called(&e, 0x0098_6410), vec![vec![0x6000, 0xaa]]);
        assert_eq!(e.mem.u32(this.addr() + 0xd64), 0x6000);
        // With a group in place only the target is added.
        e.call(0x0093_a690, &args![this, 0xbbu32]);
        assert_eq!(called(&e, 0x0099_1e80).len(), 1);
        assert_eq!(called(&e, 0x0098_6410)[1], vec![0x6000, 0xbb]);
    }

    #[test]
    fn fn_0093a7a0_stores_the_occupied_region() {
        let mut e = engine();
        let this = player(&mut e);
        e.call(0x0093_a7a0, &args![this, 0x8800u32]);
        assert_eq!(e.mem.u32(this.addr() + 0x760), 0x8800);
    }

    /// A player whose virtuals `+0x48C` (override query: writes `has_override`
    /// to its second argument), `+0x490`, and the settings object `004181e0`
    /// returns (virtuals `+0x190`, `+0x194`, `+0x198`, `+0x19C`) are recorded.
    fn setter_world(e: &mut Engine, has_override: bool) -> (Ptr<PlayerCharacter>, Ptr) {
        let this = player(e);
        e.register_double(0x00b0_048c, move |e, a| {
            e.mem.set_u8(a[2], has_override as u8);
            float(1.0)
        });
        stub(e, &[0x00b0_0490]);
        put_vtable(
            e,
            this.addr(),
            0x0200_0000,
            &[(0x48c, 0x00b0_048c), (0x490, 0x00b0_0490)],
        );
        let base = object(
            e,
            0x0201_0000,
            &[
                (0x190, 0x00b0_0190),
                (0x194, 0x00b0_0194),
                (0x198, 0x00b0_0198),
                (0x19c, 0x00b0_019c),
            ],
        );
        stub(e, &[0x00b0_0190, 0x00b0_0194, 0x00b0_0198, 0x00b0_019c]);
        e.register_double(0x0041_81e0, move |_, _| int(base.addr()));
        stub(e, &[0x0070_4e10, 0x0088_08f0]);
        (this, base)
    }

    #[test]
    fn set_actor_base_value_without_an_override_uses_the_base_object() {
        let mut e = engine();
        let (this, base) = setter_world(&mut e, false);
        e.call(0x0093_a7c0, &args![this, 7u32, -5i32]);
        assert_eq!(
            called(&e, 0x00b0_0194),
            vec![vec![base.addr(), 7, -5i32 as u32]]
        );
        assert!(called(&e, 0x00b0_0490).is_empty());
        assert_eq!(called(&e, 0x0070_4e10), vec![vec![7]]);
        assert_eq!(called(&e, 0x0088_08f0), vec![vec![this.addr(), 7, 1]]);
    }

    #[test]
    fn set_actor_base_value_with_an_override_sets_it_as_a_float() {
        let mut e = engine();
        let (this, _) = setter_world(&mut e, true);
        e.call(0x0093_a7c0, &args![this, 7u32, -5i32]);
        assert_eq!(called(&e, 0x00b0_0490).len(), 1);
        let call = &called(&e, 0x00b0_0490)[0];
        assert_eq!((call[0], call[1], f(call, 2)), (this.addr(), 7, -5.0));
        assert!(called(&e, 0x00b0_0194).is_empty());
        assert_eq!(called(&e, 0x0070_4e10), vec![vec![7]]);
    }

    #[test]
    fn fn_0093a850_sets_a_float_base_value() {
        let mut e = engine();
        let (this, base) = setter_world(&mut e, false);
        e.call(0x0093_a850, &args![this, 7u32, 2.5f32]);
        let call = &called(&e, 0x00b0_0190)[0];
        assert_eq!((call[0], call[1], f(call, 2)), (base.addr(), 7, 2.5));
        assert_eq!(called(&e, 0x0088_08f0), vec![vec![this.addr(), 7, 1]]);
    }

    #[test]
    fn fn_0093a850_with_an_override_sets_the_override() {
        let mut e = engine();
        let (this, _) = setter_world(&mut e, true);
        e.call(0x0093_a850, &args![this, 7u32, 2.5f32]);
        let call = &called(&e, 0x00b0_0490)[0];
        assert_eq!((call[0], call[1], f(call, 2)), (this.addr(), 7, 2.5));
        assert!(called(&e, 0x00b0_0190).is_empty());
    }

    /// The additions' extra doubles: `0066ee10` (does the value apply) and
    /// `0066ee50`, and the owner's virtual `+0xC` returning `current`.
    fn delta_world(
        e: &mut Engine,
        has_override: bool,
        applies: bool,
        current: f32,
    ) -> (Ptr<PlayerCharacter>, Ptr) {
        let (this, base) = setter_world(e, has_override);
        e.register_double(0x0066_ee10, move |_, _| int(applies as u32));
        stub(e, &[0x0066_ee50]);
        e.register_double(0x00b0_000c, move |_, _| float(current as f64));
        e.mem.set_u32(this.addr() + 0xa4, 0x0203_0000);
        e.map(0x0203_0000, 0x100);
        e.mem.set_u32(0x0203_000c, 0x00b0_000c);
        (this, base)
    }

    #[test]
    fn fn_0093a8f0_without_an_override_adds_through_the_base_object() {
        let mut e = engine();
        let (this, base) = delta_world(&mut e, false, true, 10.0);
        e.call(0x0093_a8f0, &args![this, 7u32, 3i32]);
        assert_eq!(called(&e, 0x00b0_019c), vec![vec![base.addr(), 7, 3]]);
        assert_eq!(called(&e, 0x00b0_000c), vec![vec![this.addr() + 0xa4, 7]]);
        let report = &called(&e, 0x0066_ee50)[0];
        assert_eq!(
            (report[0], report[1], f(report, 2), f(report, 3), report[4]),
            (this.addr() + 0xa4, 7, 10.0, 3.0, 0)
        );
    }

    #[test]
    fn fn_0093a8f0_with_an_override_sets_the_sum() {
        let mut e = engine();
        let (this, _) = delta_world(&mut e, true, true, 10.5);
        e.call(0x0093_a8f0, &args![this, 7u32, 3i32]);
        let call = &called(&e, 0x00b0_0490)[0];
        assert_eq!((call[0], call[1], f(call, 2)), (this.addr(), 7, 13.5));
        assert!(called(&e, 0x00b0_019c).is_empty());
    }

    #[test]
    fn fn_0093a8f0_treats_a_value_that_does_not_apply_as_zero() {
        let mut e = engine();
        let (this, _) = delta_world(&mut e, true, false, 10.5);
        e.call(0x0093_a8f0, &args![this, 7u32, 3i32]);
        assert!(called(&e, 0x00b0_000c).is_empty());
        let call = &called(&e, 0x00b0_0490)[0];
        assert_eq!(f(call, 2), 3.0);
        let report = &called(&e, 0x0066_ee50)[0];
        assert_eq!((f(report, 2), f(report, 3)), (0.0, 3.0));
    }

    #[test]
    fn fn_0093aa10_without_an_override_adds_a_float_through_the_base_object() {
        let mut e = engine();
        let (this, base) = delta_world(&mut e, false, true, 10.0);
        e.call(0x0093_aa10, &args![this, 7u32, 0.25f32]);
        let call = &called(&e, 0x00b0_0198)[0];
        assert_eq!((call[0], call[1], f(call, 2)), (base.addr(), 7, 0.25));
        let report = &called(&e, 0x0066_ee50)[0];
        assert_eq!((f(report, 2), f(report, 3)), (10.0, 0.25));
    }

    #[test]
    fn fn_0093aa10_with_an_override_sets_the_sum() {
        let mut e = engine();
        let (this, _) = delta_world(&mut e, true, true, 10.0);
        e.call(0x0093_aa10, &args![this, 7u32, 0.25f32]);
        let call = &called(&e, 0x00b0_0490)[0];
        assert_eq!((call[0], call[1], f(call, 2)), (this.addr(), 7, 10.25));
        assert_eq!(called(&e, 0x0088_08f0), vec![vec![this.addr(), 7, 1]]);
    }

    #[test]
    fn fn_0093ab30_passes_the_action_point_cost_to_the_virtual() {
        let mut e = engine();
        let this = player(&mut e);
        stub(&mut e, &[0x00b0_033c]);
        put_vtable(&mut e, this.addr(), 0x0200_0000, &[(0x33c, 0x00b0_033c)]);
        e.register(0x0052_5430, |_, _| int(1));
        e.register(0x0066_dca0, |_, a| float(a[0] as f64 + a[1] as f64 * 0.5));
        e.call(0x0093_ab30, &args![this, 10u32]);
        assert_eq!(called(&e, 0x0052_5430), vec![vec![0x011f_2250, 0, 0]]);
        assert_eq!(called(&e, 0x0066_dca0), vec![vec![10, 1]]);
        let call = &called(&e, 0x00b0_033c)[0];
        assert_eq!((call[0], f(call, 1)), (this.addr(), 10.5));
    }

    /// The action-point spender's world: god mode (`009526b0`), the other
    /// test (`0042ce10`) on the global object, and the virtual `+0x3AC`.
    fn spender(e: &mut Engine, god: bool, blocked: bool) -> Ptr<PlayerCharacter> {
        let this = player(e);
        stub(e, &[0x00b0_03ac]);
        put_vtable(e, this.addr(), 0x0200_0000, &[(0x3ac, 0x00b0_03ac)]);
        e.register_double(0x0095_26b0, move |_, _| int(god as u32));
        e.register_double(0x0042_ce10, move |_, _| int(blocked as u32));
        e.map(0x011d_d000, 0x1000);
        e.map(0x0101_2000, 0x1000);
        e.set_global(GLOBAL_OBJECT_0042CE10, 0x9900u32);
        e.set_global(ZERO_DOUBLE, 0.0f64);
        this
    }

    #[test]
    fn fn_0093ab80_spends_positive_amounts_as_a_negative_change() {
        let mut e = engine();
        let this = spender(&mut e, false, false);
        e.call(0x0093_ab80, &args![this, 4.5f32]);
        let call = &called(&e, 0x00b0_03ac)[0];
        assert_eq!(
            (call[0], call[1], f(call, 2), call[3]),
            (this.addr(), 0xc, -4.5, 0)
        );
        assert_eq!(called(&e, 0x0042_ce10), vec![vec![0x9900]]);
    }

    #[test]
    fn fn_0093ab80_does_nothing_for_zero_or_negative_amounts() {
        for amount in [0.0f32, -1.0] {
            let mut e = engine();
            let this = spender(&mut e, false, false);
            e.call(0x0093_ab80, &args![this, amount]);
            assert!(called(&e, 0x00b0_03ac).is_empty());
        }
    }

    #[test]
    fn fn_0093ab80_does_nothing_in_god_mode_or_when_the_other_test_says_so() {
        let mut e = engine();
        let this = spender(&mut e, true, false);
        e.call(0x0093_ab80, &args![this, 4.5f32]);
        assert!(called(&e, 0x00b0_03ac).is_empty());
        assert!(called(&e, 0x0042_ce10).is_empty());

        let mut e = engine();
        let this = spender(&mut e, false, true);
        e.call(0x0093_ab80, &args![this, 4.5f32]);
        assert!(called(&e, 0x00b0_03ac).is_empty());
    }

    // ---- the constructor and the destructor ----

    const PROCESS_VTABLE: u32 = 0x0204_0000;

    /// Every address the constructor calls other than `operator new`.
    const CONSTRUCTOR_CALLEES: &[u32] = &[
        0x008d_1d30,
        0x0063_3c90,
        0x006f_48b0,
        0x004f_6320,
        0x006a_8500,
        0x0096_a280,
        0x008d_4360,
        0x0096_a570,
        0x0096_a5c0,
        0x0040_4eb0,
        0x0048_4ab0,
        0x008d_66b0,
        0x008d_1820,
        0x005d_14d0,
        0x0066_b0d0,
        0x0096_d470,
        0x0093_0c70,
        0x0045_34f0,
        0x00c6_d770,
        0x0070_c440,
        0x0095_0610,
        0x0051_9020,
        0x0085_1d10,
        0x0096_9e90,
        0x0047_0470,
        0x0096_27f0,
        0x004e_d780,
        0x0040_3550,
        0x0096_4190,
        0x0056_7dd0,
        0x005d_add0,
        0x0040_3d30,
        0x0096_9110,
        0x008b_bbf0,
        0x007c_b2e0,
        0x0040_4ee0,
        0x0048_3710,
        0x0041_8900,
        0x00ec_782f,
    ];

    /// Doubles for everything the constructor calls. `camera_owner` is what
    /// `009306d0` returns.
    fn constructor_world(e: &mut Engine, camera_owner: u32) -> (Ptr<PlayerCharacter>, u32) {
        stub(e, CONSTRUCTOR_CALLEES);
        // Constructors return their object.
        stub_returning_this(
            e,
            &[
                0x0096_a2d0, // list
                0x0041_a250, // sound handle
                0x0068_15c0, // NiPoint3 default
                0x0062_0850, // CameraCaster
                0x0044_ddc0, // flags getter
                0x0056_7dd0,
            ],
        );
        e.register(0x008d_7510, |e, a| {
            e.mem.set_u32(a[0], PROCESS_VTABLE);
            int(a[0])
        });
        for slot in [0, 4, 0x6e0] {
            e.register(0x00b1_0000 + slot, |_, _| Ret::default());
        }
        vtable_only(
            e,
            PROCESS_VTABLE,
            &[(0, 0x00b1_0000), (4, 0x00b1_0004), (0x6e0, 0x00b1_06e0)],
        );
        e.register(0x0093_1850, |_, a| int(a[0] ^ 0x1000));
        e.register_double(0x0093_06d0, move |_, _| int(camera_owner));
        e.register(0x0054_6a90, |_, _| int(0x77));
        e.register(0x0061_b9d0, |_, _| int(0xfeed));
        // The settings getters return a pointer to a cell holding a value
        // that depends on the setting's address.
        for getter in [
            SETTING_FLOAT_GETTER,
            SETTING_BYTE_GETTER,
            SETTING_INT_GETTER,
        ] {
            e.register(getter, |e, a| {
                let cell = e.mem.alloc(8);
                e.mem.set_u32(cell, a[0] & 0xffff);
                if [
                    0x011c_cf2c,
                    0x011e_0934,
                    0x0120_315c,
                    0x011c_da58,
                    0x0120_3168,
                    0x0120_3150,
                ]
                .contains(&a[0])
                {
                    e.mem.set_f32(cell, (a[0] & 0xffff) as f32);
                }
                int(cell)
            });
        }
        for (addr, len) in [
            (0x011e_0000, 0x1000),
            (0x011f_4000, 0x1000),
            (0x0101_2000, 0x1000),
            (0x0101_7000, 0x1000),
            (0x0108_a000, 0x1000),
            (0x011a_3000, 0x1000),
            (0x011d_e000, 0x1000),
        ] {
            e.map(addr, len);
        }
        // The virtual at +0x258 that the AI-control setters call through the
        // player's own vtable.
        e.mem.set_u32(0x0108_aa3c + 0x258, 0x00b3_0258);
        e.register(0x00b3_0258, |_, _| Ret::default());
        e.set_global(TIMER_RESET, 5.5f32);
        e.set_global(BLOCK_ACTIVATE_TIMER_RESET, 1.25f32);
        e.set_global(EYE_HEIGHT_DEFAULT, 64.0f32);
        let this = Ptr::<PlayerCharacter>::new(e.mem.alloc(0xe50));
        // The old process the constructor replaces.
        let old: Ptr = Ptr::new(e.mem.alloc(0x40));
        put_vtable(e, old.addr(), 0x0205_0000, &[(0, 0x00b2_0000)]);
        e.register(0x00b2_0000, |_, _| Ret::default());
        e.mem.set_u32(this.addr() + 0x68, old.addr());
        (this, old.addr())
    }

    #[test]
    fn construct_initializes_the_player_and_replaces_the_process() {
        let mut e = engine();
        let (this, old_process) = constructor_world(&mut e, 0x5000);
        e.map(0x0000_5000, 0x1000);
        let result = e.call(0x0093_8180, &args![this]);
        assert_eq!(result.u32(), this.addr());
        // Calls begin with the base constructor and end with the guard.
        let order = call_addresses(&e);
        assert_eq!(order[0], 0x0093_8180);
        assert_eq!(order[1], 0x008d_1d30);
        assert_eq!(*order.last().unwrap(), 0x0040_4ee0);
        // The six vtables.
        for (offset, table) in [
            (0, 0x0108_aa3c),
            (0x18, 0x0108_aa30),
            (0x88, 0x0108_a9dc),
            (0x94, 0x0108_a9a4),
            (0xa4, 0x0108_a974),
            (0xa8, 0x0108_a92c),
        ] {
            assert_eq!(e.mem.u32(this.addr() + offset), table);
        }
        // The process: a fresh 0x46C-byte object, told about the old one, and
        // the old one destroyed.
        let process = e.mem.u32(this.addr() + 0x68);
        assert_ne!(process, old_process);
        assert_eq!(e.mem.block_size(process), Some(0x470));
        assert_eq!(called(&e, 0x00b1_0004), vec![vec![process, old_process]]);
        assert_eq!(called(&e, 0x00b2_0000), vec![vec![old_process, 1]]);
        assert_eq!(called(&e, 0x00b1_06e0), vec![vec![process, 0]]);
        assert_eq!(
            called(&e, 0x0096_d470),
            vec![
                vec![0x011e_0e80, this.addr(), this.addr() ^ 0x1000],
                vec![0x011e_0e80, this.addr(), this.addr() ^ 0x1000]
            ]
        );
        // Embedded members, in order.
        assert_eq!(
            called(&e, 0x0096_a2d0).first(),
            Some(&vec![this.addr() + 0x5e4])
        );
        assert_eq!(
            called(&e, 0x00ec_782f),
            vec![
                vec![this.addr() + 0x804, 0xc, 6, 0x0041_a250, 0x0048_3710],
                vec![this.addr() + 0x884, 8, 0x4a, 0x0096_a2d0, 0x0046_ffb0],
                vec![this.addr() + 0xadc, 8, 0x4a, 0x0096_a2d0, 0x0046_ffb0],
            ]
        );
        // Field values.
        assert!(e.get(this, PlayerCharacter::bCanWait));
        assert!(!e.get(this, PlayerCharacter::bCanFastTravel));
        assert!(e.get(this, PlayerCharacter::bOnElevator));
        assert!(e.get(this, PlayerCharacter::bAllowEGMCacheClear));
        assert!(e.get(this, PlayerCharacter::bAlwaysHardcore));
        assert!(e.get(this, PlayerCharacter::bShowQuestItemsInInventory));
        assert_eq!(e.get(this, PlayerCharacter::eskilladvance), 0x4d);
        assert_eq!(e.get(this, PlayerCharacter::iCharacterSeed), 0xfeed);
        assert_eq!(e.get(this, PlayerCharacter::fStealWarningTimer), 5.5);
        assert_eq!(e.get(this, PlayerCharacter::fPickPocketWarningTimer), 5.5);
        assert_eq!(e.get(this, PlayerCharacter::fLastHelloTime), 5.5);
        assert_eq!(e.get(this, PlayerCharacter::fBlockActivateTimer), 1.25);
        assert_eq!(e.get(this, PlayerCharacter::fEyeHeight), 64.0);
        assert_eq!(e.get(this, PlayerCharacter::fWorldFOV), 0x315c as f32);
        assert_eq!(e.get(this, PlayerCharacter::eDifficultyLevel), 0x0940);
        assert_eq!(e.get(this, PlayerCharacter::eKillCameraSetting), 0x087c);
        assert!(e.get(this, PlayerCharacter::bAlwaysRun));
        // The camera caster was built with the setting and the index 9.
        let caster = e.get(this, PlayerCharacter::pCameraCaster).addr();
        assert_eq!(
            called(&e, 0x0062_0850),
            vec![vec![caster, ((0x0934u32) as f32).to_bits(), 9]]
        );
        assert_eq!(called(&e, 0x00c6_d770), vec![vec![0x5000, 9]]);
        assert_eq!(called(&e, 0x0070_c440).len(), 1);
        assert_eq!(e.mem.u32(0x5000 + 0x604), 0x3e8);
        assert_eq!(called(&e, 0x0045_34f0), vec![vec![this.addr(), 1]]);
        // The heap lists exist.
        for offset in [0x210, 0x238, 0x610, 0x614, 0x618] {
            assert_ne!(e.mem.u32(this.addr() + offset), 0, "{offset:x}");
        }
        assert_eq!(e.mem.u32(this.addr() + 0x23c), 0);
        // The two 0x1A0-byte objects.
        for global in [0x011e_0774, 0x011e_0760] {
            let object = e.mem.u32(global);
            assert_eq!(e.mem.block_size(object), Some(0x1a0));
            assert_eq!(e.mem.f32(object + 0x198), 0.0);
        }
        // The achievement entries: the flag quirk makes the last byte 1.
        for i in 0..3 {
            assert_eq!(e.mem.u8(this.addr() + 0x1c8 + i * 0xc + 8), 1);
        }
        // Eight nulls pushed to the hot-key array; memset of the wobble nodes.
        assert_eq!(called(&e, 0x007c_b2e0).len(), 8);
        assert_eq!(
            called(&e, 0x0040_3d30),
            vec![vec![this.addr() + 0xd74, 0, 0x60]]
        );
        // The failure-sound handles were re-assigned: seven times.
        assert_eq!(called(&e, 0x0041_8900).len(), 7);
        // The flags getter result got the 0x400 bit.
        assert_eq!(
            called(&e, 0x0040_3550),
            vec![vec![this.addr(), this.addr() | 0x400]]
        );
        // The hostile-AI flags were set through the virtual-calling setters
        // (no process virtual here: the world has none at +0x258).
    }

    #[test]
    fn construct_without_a_camera_owner_leaves_the_camera_caster_null() {
        let mut e = engine();
        let (this, _) = constructor_world(&mut e, 0);
        e.call(0x0093_8180, &args![this]);
        assert_eq!(e.mem.u32(this.addr() + 0x21c), 0);
        assert!(called(&e, 0x0062_0850).is_empty());
        assert!(called(&e, 0x0045_34f0).is_empty());
    }

    /// Every address the destructor calls apart from the list helpers it
    /// loops on.
    const DESTRUCTOR_CALLEES: &[u32] = &[
        0x0094_7d10,
        0x0094_8050,
        0x0047_0470,
        0x008d_68d0,
        0x008d_1920,
        0x0095_0c20,
        0x0047_02f0,
        0x0062_08d0,
        0x0066_b0d0,
        0x0040_b800,
        0x008b_3180,
        0x0096_7290,
        0x0096_90a0,
        0x0094_eb40,
        0x0085_8700,
        0x005e_03d0,
        0x0096_1f90,
        0x00ad_88f0,
        0x0048_3710,
        0x0041_8900,
        0x0096_2490,
        0x0070_5fc0,
        0x00e9_8e20,
        0x0096_55a0,
        0x0096_55e0,
        0x0095_2f90,
        0x0096_a5f0,
        0x0096_a5a0,
        0x0045_cec0,
        0x00ec_5fce,
        0x0046_ffb0,
        0x008d_4380,
        0x0096_a4b0,
        0x004f_64b0,
        0x006f_4930,
        0x008d_2060,
    ];

    /// The shared state of the list doubles: the items still in each list.
    type Lists = Rc<RefCell<HashMap<u32, Vec<u32>>>>;

    /// The destructor's world: every callee doubled, `operator delete`
    /// recording its blocks, and `BSSimpleList` helpers that work on cells
    /// (a list is the address of a word holding its head item). Returns the
    /// player, the lists' items, and the recorded deletes.
    fn destructor_world(e: &mut Engine) -> (Ptr<PlayerCharacter>, Lists, Rc<RefCell<Vec<u32>>>) {
        stub(e, DESTRUCTOR_CALLEES);
        stub_returning_this(e, &[0x0041_a250]);
        let lists: Lists = Rc::default();
        let deleted: Rc<RefCell<Vec<u32>>> = Rc::default();
        // Item address of a list's head node: the cell itself.
        e.register(LIST_ITEM_ADDRESS, |_, a| int(a[0]));
        e.register(LIST_NEXT, |_, _| int(0));
        e.register(SOUND_HANDLE_IS_VALID, |_, _| int(0));
        let state = lists.clone();
        e.register_double(LIST_REMOVE_HEAD, move |e, a| {
            let mut state = state.borrow_mut();
            let items = state.entry(a[0]).or_default();
            if !items.is_empty() {
                items.remove(0);
            }
            e.mem.set_u32(a[0], items.first().copied().unwrap_or(0));
            Ret::default()
        });
        let state = lists.clone();
        e.register_double(LIST_IS_EMPTY, move |_, a| {
            int(state.borrow().get(&a[0]).is_none_or(|v| v.is_empty()) as u32)
        });
        let record = deleted.clone();
        e.register_double(OPERATOR_DELETE, move |_, a| {
            record.borrow_mut().push(a[0]);
            Ret::default()
        });
        for (addr, len) in [(0x011d_e000, 0x1000), (0x011e_0000, 0x1000)] {
            e.map(addr, len);
        }
        let this = Ptr::<PlayerCharacter>::new(e.mem.alloc(0xe50));
        (this, lists, deleted)
    }

    /// A heap list cell holding `items` (head item in the cell).
    fn list_of(e: &mut Engine, lists: &Lists, items: &[u32]) -> u32 {
        let cell = e.mem.alloc(8);
        e.mem.set_u32(cell, items.first().copied().unwrap_or(0));
        lists.borrow_mut().insert(cell, items.to_vec());
        cell
    }

    #[test]
    fn destruct_of_an_empty_player_destroys_the_members_and_the_base() {
        let mut e = engine();
        let (this, _, deleted) = destructor_world(&mut e);
        e.mem.set_u32(0x011e_0774, 0x1111);
        e.mem.set_u32(0x011e_0760, 0x2222);
        e.call(0x0093_98d0, &args![this]);
        for (offset, table) in [
            (0, 0x0108_aa3c),
            (0x18, 0x0108_aa30),
            (0x88, 0x0108_a9dc),
            (0x94, 0x0108_a9a4),
            (0xa4, 0x0108_a974),
            (0xa8, 0x0108_a92c),
        ] {
            assert_eq!(e.mem.u32(this.addr() + offset), table);
        }
        let order = call_addresses(&e);
        assert_eq!(order[0], 0x0093_98d0);
        assert_eq!(order[1], 0x0094_7d10);
        assert_eq!(order[2], 0x0094_8050);
        assert_eq!(*order.last().unwrap(), 0x008d_2060);
        // Only the two global objects are deleted when no list has items.
        assert_eq!(*deleted.borrow(), vec![0, 0, 0x1111, 0x2222]);
        // The members are destroyed in reverse order of construction: the
        // last-declared (hot-key array) first, the notes list last.
        assert_eq!(called(&e, 0x0096_a5f0), vec![vec![this.addr() + 0xe3c]]);
        let destroyed: Vec<u32> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| [0x0045_cec0, 0x0046_ffb0].contains(a))
            .map(|(_, w)| w[0] - this.addr())
            .collect();
        assert_eq!(
            destroyed,
            vec![
                0xdec, 0xd3c, 0xad4, 0x87c, 0x868, 0x864, 0x84c, 0x7d4, 0x7c8, 0x774, 0x6c4, 0x6bc,
                0x6b0, 0x6a8, 0x69c, 0x694, 0x634, 0x5fc, 0x5e4
            ]
        );
        assert_eq!(
            called(&e, 0x00ec_5fce),
            vec![
                vec![this.addr() + 0xadc, 8, 0x4a, 0x0046_ffb0],
                vec![this.addr() + 0x884, 8, 0x4a, 0x0046_ffb0],
                vec![this.addr() + 0x804, 0xc, 6, 0x0048_3710],
            ]
        );
        // Nothing to release: no list deleting destructor runs.
        assert!(called(&e, 0x0047_02f0).is_empty());
        assert!(called(&e, 0x0040_b800).is_empty());
        assert!(called(&e, 0x0085_8700).is_empty());
        assert!(called(&e, 0x005e_03d0).is_empty());
        // Every perk list is cleared.
        let cleared: Vec<u32> = called(&e, 0x0047_0470)
            .iter()
            .map(|w| w[0] - this.addr())
            .collect();
        assert_eq!(cleared.len(), 1 + 4 + 1 + 1 + 74 + 1 + 74 + 1);
        assert_eq!(cleared[..6], [0x768, 0x6a8, 0x6b0, 0x6c4, 0x6bc, 0x84c]);
    }

    #[test]
    fn destruct_releases_the_processes_lists_and_items() {
        let mut e = engine();
        let (this, lists, deleted) = destructor_world(&mut e);
        let t = this.addr();
        for target in [0x00b4_0000u32, 0x00b4_0010] {
            e.register_double(target, |_, _| Ret::default());
        }
        // The process and the first-person effect have destructors at virtual +0.
        let process = object(&mut e, 0x0200_0000, &[(0, 0x00b4_0000)]);
        e.mem.set_u32(t + 0x68, process.addr());
        let weapon_effect = object(&mut e, 0x0201_0000, &[(0, 0x00b4_0000)]);
        e.mem.set_u32(t + 0x6a4, weapon_effect.addr());
        let border_regions = object(&mut e, 0x0202_0000, &[(0, 0x00b4_0000)]);
        e.mem.set_u32(t + 0x7b0, border_regions.addr());
        // Eat/drink list present, worn enchantments absent.
        let eat_drink = list_of(&mut e, &lists, &[]);
        e.mem.set_u32(t + 0x238, eat_drink);
        let caster = e.mem.alloc(8);
        e.mem.set_u32(t + 0x21c, caster);
        e.mem.set_u32(t + 0x6ec, 0x1234);
        e.mem.set_u32(t + 0x788, 0x2345);
        e.mem.set_u32(0x011d_e45c, 0x8800);
        // Active effects: two items with virtual destructors.
        let effect_1 = object(&mut e, 0x0203_0000, &[(0, 0x00b4_0000)]);
        let effect_2 = object(&mut e, 0x0204_0000, &[(0, 0x00b4_0000)]);
        let effects = list_of(&mut e, &lists, &[effect_1.addr(), effect_2.addr()]);
        e.mem.set_u32(t + 0x210, effects);
        // Actions: plain blocks deleted until the first null item.
        let actions = list_of(&mut e, &lists, &[0x111, 0x222]);
        e.mem.set_u32(t + 0x60c, actions);
        let casino = list_of(&mut e, &lists, &[0x333]);
        e.mem.set_u32(t + 0x610, casino);
        // Caravan cards use the item's virtual +0x10; the active list is
        // empty and is left alone.
        let card = object(&mut e, 0x0205_0000, &[(0x10, 0x00b4_0010)]);
        let inactive = list_of(&mut e, &lists, &[card.addr()]);
        e.mem.set_u32(t + 0x614, inactive);
        let active = list_of(&mut e, &lists, &[]);
        e.mem.set_u32(t + 0x618, active);
        // Perks: an embedded list of two nodes; companion perks of one.
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, 0x555);
        e.mem.set_u32(t + 0x87c, 0x444);
        e.mem.set_u32(t + 0xad4, 0x666);
        let chain: HashMap<u32, u32> =
            HashMap::from([(t + 0x87c, node), (node, 0), (t + 0xad4, 0)]);
        e.register_double(LIST_NEXT, move |_, a| int(chain[&a[0]]));
        // Sound 2 is valid.
        let valid_sound = t + 0x804 + 2 * 0xc;
        e.register_double(SOUND_HANDLE_IS_VALID, move |_, a| {
            int((a[0] == valid_sound) as u32)
        });
        e.mem.set_u32(0x011e_0774, 0x7771);
        e.mem.set_u32(0x011e_0760, 0x7772);
        e.call(0x0093_98d0, &args![this]);

        // The process is destroyed and its pointer cleared.
        assert_eq!(called(&e, 0x00b4_0000)[0], vec![process.addr(), 1]);
        assert_eq!(e.mem.u32(t + 0x68), 0);
        // The weapon effect and the border regions have virtual destructors.
        assert!(called(&e, 0x00b4_0000).contains(&vec![weapon_effect.addr(), 1]));
        assert!(called(&e, 0x00b4_0000).contains(&vec![border_regions.addr(), 1]));
        assert_eq!(called(&e, 0x005e_03d0), vec![vec![border_regions.addr()]]);
        assert_eq!(e.mem.u32(t + 0x7b0), 0);
        // The effects list: both items destroyed, then the list itself.
        assert!(called(&e, 0x00b4_0000).contains(&vec![effect_1.addr(), 1]));
        assert!(called(&e, 0x00b4_0000).contains(&vec![effect_2.addr(), 1]));
        // Deleted blocks: the camera caster, actions (all), casino data, perks,
        // companion perks, then the two global objects.
        assert_eq!(
            *deleted.borrow(),
            vec![caster, 0x111, 0x222, 0x333, 0x444, 0x555, 0x666, 0x7771, 0x7772]
        );
        // The caravan card's virtual +0x10 runs with 1.
        assert_eq!(called(&e, 0x00b4_0010), vec![vec![card.addr(), 1]]);
        // List objects are deleted with their deleting destructors, in order:
        // eat/drink, effects, actions, casino, inactive caravan cards. The
        // active (empty) list is skipped.
        let deleted_lists: Vec<u32> = called(&e, 0x0047_02f0).iter().map(|w| w[0]).collect();
        assert_eq!(
            deleted_lists,
            vec![eat_drink, effects, actions, casino, inactive]
        );
        assert!(called(&e, 0x0047_02f0).iter().all(|w| w[1] == 1));
        // The camera caster, spell, state buffer and the single valid sound.
        assert_eq!(called(&e, 0x0062_08d0), vec![vec![caster]]);
        assert_eq!(called(&e, 0x0040_b800), vec![vec![0x1234, 1]]);
        assert_eq!(called(&e, 0x0085_8700), vec![vec![0x8800, 0x2345]]);
        assert_eq!(called(&e, 0x00ad_88f0).len(), 1);
        assert_eq!(called(&e, 0x0041_8900).len(), 1);
        assert_eq!(called(&e, 0x0048_3710).len(), 3);
    }

    #[test]
    fn player_vector_deleting_destructor_frees_only_with_the_flag() {
        let mut e = engine();
        let (this, _, deleted) = destructor_world(&mut e);
        let result = e.call(0x0093_98a0, &args![this, 0u32]);
        assert_eq!(result.u32(), this.addr());
        assert_eq!(*deleted.borrow(), vec![0; 4]);
        e.call(0x0093_98a0, &args![this, 1u32]);
        assert_eq!(*deleted.borrow(), [vec![0; 8], vec![this.addr()]].concat());
        assert_eq!(called(&e, 0x008d_2060).len(), 2);
    }

    /// Gives the player an `ActorValueOwner` base (at +0xA4) whose vtable at
    /// `base` has the given slots.
    fn give_owner(e: &mut Engine, this: Ptr<PlayerCharacter>, base: u32, slots: &[(u32, u32)]) {
        put_vtable(e, this.addr() + 0xa4, base, slots);
    }

    /// A double for `_ftol2`: truncates the leading `f64` argument.
    fn register_ftol(e: &mut Engine) {
        e.register(FTOL, |_, a| {
            int(f64::from_bits(a[0] as u64 | (a[1] as u64) << 32) as i32 as u32)
        });
    }

    #[test]
    fn fn_0093abe0_writes_both_floats_through_the_pointers() {
        let mut e = engine();
        let this = player(&mut e);
        e.register(0x008a_0c20, |_, _| float(2.5));
        e.register_double(0x00b0_0008, |_, _| int(7));
        give_owner(&mut e, this, 0x0200_0000, &[(8, 0x00b0_0008)]);
        let out = e.mem.alloc(8);
        e.call(0x0093_abe0, &args![this, out, out + 4]);
        assert_eq!(e.mem.f32(out), 7.0);
        assert_eq!(e.mem.f32(out + 4), 2.5);
        assert_eq!(called(&e, 0x008a_0c20), vec![vec![this.addr()]]);
        assert_eq!(
            called(&e, 0x00b0_0008),
            vec![vec![this.addr() + 0xa4, 0x2e]]
        );
    }

    /// An `ActorValueOwner` (the player + 0xA4) with the virtuals at +0x0,
    /// +0x4 and +0x18 returning the given values.
    fn base_owner(
        e: &mut Engine,
        slot0: u32,
        slot4: f32,
        slot18: f32,
    ) -> (Ptr<PlayerCharacter>, Ptr) {
        let this = player(e);
        e.register_double(0x00b0_0000, move |_, _| int(slot0));
        e.register_double(0x00b0_0004, move |_, _| float(slot4 as f64));
        e.register_double(0x00b0_0018, move |_, _| float(slot18 as f64));
        give_owner(
            e,
            this,
            0x0200_0000,
            &[(0, 0x00b0_0000), (4, 0x00b0_0004), (0x18, 0x00b0_0018)],
        );
        (this, Ptr::new(this.addr() + 0xa4))
    }

    #[test]
    fn fn_0093ac20_sums_the_base_value_and_the_three_truncated_parts() {
        let mut e = engine();
        let (this, owner) = base_owner(&mut e, 10, 0.0, 0.0);
        e.register(0x0041_81e0, |_, _| int(1));
        e.register(0x0094_c3d0, |_, a| float([1.9, 2.5, -0.5][a[1] as usize]));
        register_ftol(&mut e);
        let result = e.call(0x0093_ac20, &args![owner, 5u32]).i32();
        assert_eq!(result, 10 + 1 + 2);
        assert_eq!(called(&e, 0x004_181e0), vec![vec![this.addr()]]);
        assert_eq!(
            called(&e, 0x0094_c3d0),
            vec![
                vec![this.addr(), 0, 5],
                vec![this.addr(), 1, 5],
                vec![this.addr(), 2, 5]
            ]
        );
    }

    #[test]
    fn fn_0093ac20_is_zero_without_the_object() {
        let mut e = engine();
        let (_, owner) = base_owner(&mut e, 10, 0.0, 0.0);
        e.register(0x0041_81e0, |_, _| int(0));
        assert_eq!(e.call(0x0093_ac20, &args![owner, 5u32]).i32(), 0);
        assert!(called(&e, 0x00b0_0000).is_empty());
    }

    #[test]
    fn fn_0093acb0_sums_the_float_base_value_and_the_three_parts() {
        let mut e = engine();
        let (this, owner) = base_owner(&mut e, 0, 1.5, 0.0);
        e.register(0x0094_c3d0, |_, a| float([0.25, 0.5, 0.125][a[1] as usize]));
        let result = e.call(0x0093_acb0, &args![owner, 5u32]).f32();
        assert_eq!(result, 2.375);
        assert_eq!(called(&e, 0x00b0_0004), vec![vec![owner.addr(), 5]]);
        assert_eq!(called(&e, 0x0094_c3d0)[2], vec![this.addr(), 2, 5]);
    }

    #[test]
    fn fn_0093ad30_converts_the_virtual_result_and_truncates_it() {
        let mut e = engine();
        let this = player(&mut e);
        e.register_double(0x00b0_0020, |_, _| float(4.75));
        put_vtable(&mut e, this.addr(), 0x0200_0000, &[(0x20, 0x00b0_0020)]);
        e.register(0x0040_6ce0, |_, a| float(f32::from_bits(a[0]) as f64 * 2.0));
        register_ftol(&mut e);
        let result = e.call(0x0093_ad30, &args![this, 3u32]).i32();
        assert_eq!(result, 9);
        assert_eq!(called(&e, 0x00b0_0020), vec![vec![this.addr(), 3]]);
    }

    /// The world of `GetPermanentActorFloatValue`: the owner's virtuals +0x4
    /// and +0x18, the player's virtual +0x48C, the skill derivation and the
    /// clamp (which doubles its argument so the test sees it is returned).
    fn permanent_world(e: &mut Engine, skill: bool) -> Ptr {
        let (this, owner) = base_owner(e, 0, 3.0, 10.0);
        put_vtable(e, this.addr(), 0x0210_0000, &[(0x48c, 0x00b0_048c)]);
        e.register_double(0x00b0_048c, |_, _| float(1.5));
        e.register_double(0x0047_f060, move |_, _| int(skill as u32));
        e.register(0x0064_3c90, |_, _| float(2.0));
        e.register(0x0066_f190, |_, a| float(f32::from_bits(a[1]) as f64 * 2.0));
        e.map(0x011d_e000, 0x1000);
        e.set_global(PLAYER_SINGLETON, 0u32);
        owner
    }

    #[test]
    fn permanent_float_value_of_a_skill_adds_the_modifier_and_the_derived_skill() {
        let mut e = engine();
        let owner = permanent_world(&mut e, true);
        let result = e.call(0x0093_ad60, &args![owner, 5u32]).f32();
        // (10 + (1.5 + 2.0)) clamped (doubled by the double).
        assert_eq!(result, 27.0);
        assert_eq!(called(&e, 0x0064_3c90), vec![vec![0, 5, 1]]);
        let clamp = &called(&e, 0x0066_f190)[0];
        assert_eq!((clamp[0], f(clamp, 1)), (5, 13.5));
        let modifier = &called(&e, 0x00b0_048c)[0];
        assert_eq!((modifier[0], modifier[1]), (owner.addr() - 0xa4, 5));
    }

    #[test]
    fn permanent_float_value_derives_from_the_player_singleton_owner() {
        let mut e = engine();
        let owner = permanent_world(&mut e, true);
        e.set_global(PLAYER_SINGLETON, 0x7000u32);
        e.call(0x0093_ad60, &args![owner, 5u32]);
        assert_eq!(called(&e, 0x0064_3c90), vec![vec![0x7000 + 0xa4, 5, 1]]);
    }

    #[test]
    fn permanent_float_value_of_other_values_adds_the_two_virtuals() {
        let mut e = engine();
        let owner = permanent_world(&mut e, false);
        let result = e.call(0x0093_ad60, &args![owner, 5u32]).f32();
        assert_eq!(result, 26.0);
        assert!(called(&e, 0x0064_3c90).is_empty());
        assert!(called(&e, 0x00b0_048c).is_empty());
    }

    #[test]
    fn fn_0093ae40_sets_the_value_and_notifies() {
        let mut e = engine();
        let this = player(&mut e);
        stub(&mut e, &[0x0088_0700, 0x0070_4e10, 0x0088_08f0]);
        e.call(0x0093_ae40, &args![this, 7u32, 1.5f32]);
        let call = &called(&e, 0x0088_0700)[0];
        assert_eq!((call[0], call[1], f(call, 2)), (this.addr(), 7, 1.5));
        assert_eq!(called(&e, 0x0070_4e10), vec![vec![7]]);
        assert_eq!(called(&e, 0x0088_08f0), vec![vec![this.addr(), 7, 1]]);
    }

    /// The world of the damage-style setters: `0066ee10` says the value does
    /// not apply (the old value is 0), the flag test `00406d70` answers per
    /// flag, `00880850`/`00880890` give the delta, `00952730` the permission.
    fn damage_world(
        e: &mut Engine,
        flag_100: bool,
        flag_200: bool,
        allowed: bool,
    ) -> Ptr<PlayerCharacter> {
        let this = player(e);
        e.register_double(0x0040_6d70, move |_, a| {
            int(if a[1] == 0x100 { flag_100 } else { flag_200 } as u32)
        });
        e.register_double(0x0095_2730, move |_, _| int(allowed as u32));
        e.register(0x0066_ee10, |_, _| int(0));
        e.register(0x0088_0850, |_, _| int(-4i32 as u32));
        e.register(0x0088_0890, |_, _| float(-2.5));
        stub(
            e,
            &[
                0x0094_c4f0,
                0x0070_4e10,
                0x0088_08f0,
                0x0066_ee50,
                0x00b0_04b8,
            ],
        );
        put_vtable(e, this.addr(), 0x0200_0000, &[(0x4b8, 0x00b0_04b8)]);
        e.map(0x0101_2000, 0x1000);
        e.set_global(ZERO_DOUBLE, 0.0f64);
        this
    }

    #[test]
    fn fn_0093ae80_applies_the_integer_change_and_reports_it() {
        let mut e = engine();
        let this = damage_world(&mut e, false, false, true);
        e.call(0x0093_ae80, &args![this, 7u32, 5i32, 0x6000u32]);
        assert_eq!(called(&e, 0x0040_6d70), vec![vec![7, 0x100]]);
        assert_eq!(
            called(&e, 0x0088_0850),
            vec![vec![this.addr(), 7, 5, 0x6000]]
        );
        let record = &called(&e, 0x0094_c4f0)[0];
        assert_eq!(
            (record[0], record[1], record[2], f(record, 3), record[4]),
            (this.addr(), 0, 7, -4.0, 2)
        );
        assert_eq!(called(&e, 0x0070_4e10), vec![vec![7]]);
        assert_eq!(called(&e, 0x0088_08f0), vec![vec![this.addr(), 7, 0]]);
        let report = &called(&e, 0x0066_ee50)[0];
        assert_eq!(
            (report[0], report[1], f(report, 2), f(report, 3), report[4]),
            (this.addr() + 0xa4, 7, 0.0, -4.0, 0x6000 + 0xa4)
        );
        assert!(called(&e, 0x00b0_04b8).is_empty());
    }

    #[test]
    fn fn_0093ae80_tells_the_player_when_health_is_lost() {
        let mut e = engine();
        let this = damage_world(&mut e, false, false, true);
        e.call(0x0093_ae80, &args![this, 0x10u32, 5i32, 0x6000u32]);
        let call = &called(&e, 0x00b0_04b8)[0];
        assert_eq!((call[0], call[1], f(call, 2)), (this.addr(), 0x6000, -4.0));
    }

    #[test]
    fn fn_0093ae80_does_nothing_for_a_flagged_value() {
        let mut e = engine();
        let this = damage_world(&mut e, true, false, true);
        e.call(0x0093_ae80, &args![this, 7u32, 5i32, 0u32]);
        assert_eq!(call_addresses(&e), vec![0x0093_ae80, 0x0040_6d70]);
    }

    #[test]
    fn fn_0093ae80_reports_a_null_source_as_null() {
        let mut e = engine();
        let this = damage_world(&mut e, false, false, true);
        e.call(0x0093_ae80, &args![this, 7u32, 5i32, 0u32]);
        assert_eq!(called(&e, 0x0066_ee50)[0][4], 0);
    }

    #[test]
    fn fn_0093afb0_applies_the_float_change() {
        let mut e = engine();
        let this = damage_world(&mut e, false, false, true);
        e.call(0x0093_afb0, &args![this, 0x10u32, 1.25f32, 0x6000u32]);
        let delta = &called(&e, 0x0088_0890)[0];
        assert_eq!(
            (delta[0], delta[1], f(delta, 2), delta[3]),
            (this.addr(), 0x10, 1.25, 0x6000)
        );
        let record = &called(&e, 0x0094_c4f0)[0];
        assert_eq!((record[1], f(record, 3), record[4]), (0, -2.5, 2));
        assert_eq!(called(&e, 0x00b0_04b8).len(), 1);
    }

    #[test]
    fn fn_0093afb0_keeps_positive_changes_from_the_health_virtual() {
        let mut e = engine();
        let this = damage_world(&mut e, false, false, true);
        e.register(0x0088_0890, |_, _| float(2.5));
        e.call(0x0093_afb0, &args![this, 0x10u32, 1.25f32, 0u32]);
        assert!(called(&e, 0x00b0_04b8).is_empty());
        let report = &called(&e, 0x0066_ee50)[0];
        assert_eq!(f(report, 3), 2.5);
    }

    #[test]
    fn fn_0093afb0_does_nothing_for_a_flagged_value() {
        let mut e = engine();
        let this = damage_world(&mut e, true, false, true);
        e.call(0x0093_afb0, &args![this, 7u32, 1.0f32, 0u32]);
        assert_eq!(call_addresses(&e), vec![0x0093_afb0, 0x0040_6d70]);
    }

    #[test]
    fn fn_0093b0f0_checks_permission_and_uses_kind_one() {
        let mut e = engine();
        let this = damage_world(&mut e, false, false, true);
        e.call(0x0093_b0f0, &args![this, 7u32, 3i32, 0x6000u32]);
        let check = &called(&e, 0x0095_2730)[0];
        assert_eq!((check[0], check[1], f(check, 2)), (this.addr(), 7, 3.0));
        let record = &called(&e, 0x0094_c4f0)[0];
        assert_eq!((record[1], f(record, 3)), (1, -4.0));
    }

    #[test]
    fn fn_0093b0f0_stops_when_not_permitted_or_flagged() {
        let mut e = engine();
        let this = damage_world(&mut e, false, false, false);
        e.call(0x0093_b0f0, &args![this, 7u32, 3i32, 0u32]);
        assert_eq!(call_addresses(&e), vec![0x0093_b0f0, 0x0095_2730]);
        let mut e = engine();
        let this = damage_world(&mut e, true, false, true);
        e.call(0x0093_b0f0, &args![this, 7u32, 3i32, 0u32]);
        assert!(called(&e, 0x0088_0850).is_empty());
    }

    #[test]
    fn fn_0093b240_applies_the_float_change_with_kind_one() {
        let mut e = engine();
        let this = damage_world(&mut e, false, false, true);
        e.call(0x0093_b240, &args![this, 7u32, 0.5f32, 0x6000u32]);
        let check = &called(&e, 0x0095_2730)[0];
        assert_eq!(f(check, 2), 0.5);
        let record = &called(&e, 0x0094_c4f0)[0];
        assert_eq!((record[1], f(record, 3)), (1, -2.5));
    }

    #[test]
    fn fn_0093b240_stops_when_not_permitted_or_flagged() {
        let mut e = engine();
        let this = damage_world(&mut e, false, false, false);
        e.call(0x0093_b240, &args![this, 7u32, 0.5f32, 0u32]);
        assert_eq!(call_addresses(&e), vec![0x0093_b240, 0x0095_2730]);
        let mut e = engine();
        let this = damage_world(&mut e, true, false, true);
        e.call(0x0093_b240, &args![this, 7u32, 0.5f32, 0u32]);
        assert!(called(&e, 0x0088_0890).is_empty());
    }

    /// The world of the health setters `0093b3a0` and `0093b7a0`: the owner's
    /// virtual +0xC gives the health `before` (first call) and `after`
    /// (second call) as fractions of a maximum of 100, the two heartbeat
    /// settings are `low` and `high`, and the sound functions are recorded.
    fn health_world(
        e: &mut Engine,
        before: f32,
        after: f32,
        low: f32,
        high: f32,
    ) -> Ptr<PlayerCharacter> {
        let this = damage_world(e, false, false, true);
        let calls = Rc::new(RefCell::new(0u32));
        e.register_double(0x00b0_000c, move |_, _| {
            let mut n = calls.borrow_mut();
            *n += 1;
            float(if *n == 1 { before } else { after } as f64 * 100.0)
        });
        e.register_double(0x00b0_0020, |_, _| float(100.0));
        give_owner(
            e,
            this,
            0x0201_0000,
            &[(0xc, 0x00b0_000c), (0x20, 0x00b0_0020)],
        );
        e.map(0x0300_0000, 0x100);
        e.mem.set_f32(0x0300_0000, low);
        e.mem.set_f32(0x0300_0004, high);
        e.register(0x0040_3e20, |_, a| {
            int(if a[0] == HEARTBEAT_SETTING_LOW {
                0x0300_0000
            } else {
                0x0300_0004
            })
        });
        e.register(0x0045_3a70, |_, _| int(0x0400_0000));
        e.register(0x00ad_7550, |_, a| int(a[1]));
        stub(
            e,
            &[
                0x00ad_88f0,
                0x00ad_8d10,
                0x0041_8900,
                0x0048_3710,
                0x00ad_8830,
                0x00ad_8da0,
            ],
        );
        this
    }

    /// The sound the heartbeat switched to (the name passed to `00ad7550`).
    fn heartbeat_names(e: &Engine) -> Vec<u32> {
        called(e, 0x00ad_7550).iter().map(|w| w[2]).collect()
    }

    #[test]
    fn health_loss_to_nothing_fades_the_status_sound_out() {
        let mut e = engine();
        let this = health_world(&mut e, 0.5, 0.0, 0.8, 0.4);
        e.call(0x0093_b3a0, &args![this, 0x10u32, 5i32, 0x6000u32]);
        assert_eq!(
            called(&e, 0x00ad_8da0),
            vec![vec![this.addr() + 0x77c, 1000]]
        );
        assert!(heartbeat_names(&e).is_empty());
        // The loss itself: the interface, the player's virtual and the report.
        assert_eq!(called(&e, 0x00b0_04b8).len(), 1);
        assert_eq!(called(&e, 0x0088_08f0), vec![vec![this.addr(), 0x10, 0]]);
        let report = &called(&e, 0x0066_ee50)[0];
        assert_eq!((f(report, 2), f(report, 3)), (0.0, -4.0));
    }

    #[test]
    fn health_above_the_upper_threshold_fades_the_status_sound_out() {
        let mut e = engine();
        let this = health_world(&mut e, 0.5, 0.9, 0.8, 0.4);
        e.call(0x0093_b3a0, &args![this, 0x10u32, 5i32, 0u32]);
        assert_eq!(called(&e, 0x00ad_8da0).len(), 1);
        assert!(heartbeat_names(&e).is_empty());
    }

    #[test]
    fn falling_below_the_lower_threshold_starts_the_blp_heartbeat() {
        let mut e = engine();
        let this = health_world(&mut e, 0.9, 0.3, 0.8, 0.4);
        e.call(0x0093_b3a0, &args![this, 0x10u32, 5i32, 0u32]);
        assert_eq!(heartbeat_names(&e), vec![HEARTBEAT_SOUND_BLP]);
        let lookup = &called(&e, 0x00ad_7550)[0];
        assert_eq!((lookup[0], lookup[3]), (0x0400_0000, 0x31));
        let handle = this.addr() + 0x77c;
        assert_eq!(called(&e, 0x00ad_88f0), vec![vec![handle]]);
        assert_eq!(called(&e, 0x00ad_8d10), vec![vec![handle]]);
        assert_eq!(called(&e, 0x0041_8900), vec![vec![handle, lookup[1]]]);
        assert_eq!(called(&e, 0x0048_3710), vec![vec![lookup[1]]]);
        assert_eq!(called(&e, 0x00ad_8830), vec![vec![handle, 1]]);
        assert!(called(&e, 0x00ad_8da0).is_empty());
    }

    #[test]
    fn rising_above_the_lower_threshold_starts_the_alp_heartbeat() {
        let mut e = engine();
        let this = health_world(&mut e, 0.3, 0.6, 0.8, 0.4);
        e.call(0x0093_b3a0, &args![this, 0x10u32, 5i32, 0u32]);
        assert_eq!(heartbeat_names(&e), vec![HEARTBEAT_SOUND_ALP]);
    }

    #[test]
    fn rising_from_between_the_thresholds_changes_no_sound() {
        let mut e = engine();
        let this = health_world(&mut e, 0.5, 0.6, 0.8, 0.4);
        e.call(0x0093_b3a0, &args![this, 0x10u32, 5i32, 0u32]);
        assert!(heartbeat_names(&e).is_empty());
        assert!(called(&e, 0x00ad_8da0).is_empty());
    }

    #[test]
    fn rising_from_above_the_upper_threshold_while_inside_starts_the_alp_heartbeat() {
        let mut e = engine();
        let this = health_world(&mut e, 0.9, 0.6, 0.8, 0.4);
        e.call(0x0093_b3a0, &args![this, 0x10u32, 5i32, 0u32]);
        assert_eq!(heartbeat_names(&e), vec![HEARTBEAT_SOUND_ALP]);
    }

    #[test]
    fn other_values_leave_the_heartbeat_alone() {
        let mut e = engine();
        let this = health_world(&mut e, 0.5, 0.0, 0.8, 0.4);
        e.call(0x0093_b3a0, &args![this, 7u32, 5i32, 0x6000u32]);
        assert!(called(&e, 0x00ad_8da0).is_empty());
        assert!(called(&e, 0x00b0_04b8).is_empty());
        assert!(called(&e, 0x00b0_000c).is_empty());
        assert_eq!(called(&e, 0x0070_4e10), vec![vec![7]]);
    }

    #[test]
    fn fn_0093b3a0_records_the_flag_of_the_value() {
        let mut e = engine();
        let this = damage_world(&mut e, false, true, true);
        e.call(0x0093_b3a0, &args![this, 7u32, 5i32, 0u32]);
        let record = &called(&e, 0x0094_c4f0)[0];
        assert_eq!((record[1], record[2], record[4]), (2, 7, 1));
    }

    #[test]
    fn fn_0093b3a0_stops_when_not_permitted_or_flagged() {
        let mut e = engine();
        let this = damage_world(&mut e, false, false, false);
        e.call(0x0093_b3a0, &args![this, 0x10u32, 5i32, 0u32]);
        assert_eq!(call_addresses(&e), vec![0x0093_b3a0, 0x0095_2730]);
        let mut e = engine();
        let this = damage_world(&mut e, true, false, true);
        e.call(0x0093_b3a0, &args![this, 0x10u32, 5i32, 0u32]);
        assert!(called(&e, 0x0088_0850).is_empty());
    }

    #[test]
    fn fn_0093b7a0_follows_the_health_of_a_float_change() {
        let mut e = engine();
        let this = health_world(&mut e, 0.9, 0.3, 0.8, 0.4);
        e.call(0x0093_b7a0, &args![this, 0x10u32, 4.0f32, 0x6000u32]);
        let delta = &called(&e, 0x0088_0890)[0];
        assert_eq!((delta[1], f(delta, 2), delta[3]), (0x10, 4.0, 0x6000));
        let call = &called(&e, 0x00b0_04b8)[0];
        assert_eq!((call[1], f(call, 2)), (0x6000, -2.5));
        assert_eq!(heartbeat_names(&e), vec![HEARTBEAT_SOUND_BLP]);
        let record = &called(&e, 0x0094_c4f0)[0];
        assert_eq!((record[1], f(record, 3)), (2, -2.5));
    }

    #[test]
    fn fn_0093b7a0_stops_when_not_permitted_or_flagged() {
        let mut e = engine();
        let this = damage_world(&mut e, false, false, false);
        e.call(0x0093_b7a0, &args![this, 0x10u32, 4.0f32, 0u32]);
        assert_eq!(call_addresses(&e), vec![0x0093_b7a0, 0x0095_2730]);
        let mut e = engine();
        let this = damage_world(&mut e, true, false, true);
        e.call(0x0093_b7a0, &args![this, 0x10u32, 4.0f32, 0u32]);
        assert!(called(&e, 0x0088_0890).is_empty());
    }

    /// The globals of the cell-grid functions: the `TES` singleton pointer
    /// (pointing at 0x5000), the water search constants and the cell size.
    fn grid_globals(e: &mut Engine) {
        e.map(0x011d_e000, 0x1000);
        e.set_global(TES_SINGLETON, 0x5000u32);
        e.map(0x0101_6000, 0x3000);
        e.set_global(WATER_RADIUS_LIMIT, 2048.0f64);
        e.set_global(WATER_RADIUS_CLAMPED, 2048.0f32);
        e.set_global(CELL_SIZE, 4096.0f64);
    }

    /// The water-cell search's world: the player has no parent cell (or
    /// `parent`), stands at (`x`, `y`); `451900` returns a cell number from
    /// the grid coordinates (`1000 + 10x + y`) and `004518e0` accepts only
    /// `accepted`.
    fn water_world(
        e: &mut Engine,
        parent: u32,
        x: f32,
        y: f32,
        accepted: u32,
    ) -> Ptr<PlayerCharacter> {
        let this = player(e);
        grid_globals(e);
        e.register_double(0x008d_6f30, move |_, _| int(parent));
        e.register(0x0042_5fd0, |_, _| int(0));
        e.register_double(0x0045_18e0, move |_, a| int((a[0] == accepted) as u32));
        e.register(0x004f_d3e0, |_, _| int(1));
        let block = e.mem.alloc(0x40);
        e.mem.set_f32(block + 0x10, x);
        e.mem.set_f32(block + 0x14, y);
        e.register_double(0x0089_1170, move |_, _| int(block));
        e.register(0x0040_6d90, |_, a| int(f32::from_bits(a[0]) as i32 as u32));
        e.register(0x0045_2dc0, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            e.mem.set_u32(a[0] + 4, a[2]);
            int(a[0])
        });
        e.register(0x0045_1900, |_, a| int(1000 + 10 * a[1] + a[2]));
        e.register(0x0045_16d0, |_, _| int(0));
        this
    }

    #[test]
    fn water_cell_of_the_current_cell() {
        let mut e = engine();
        let this = water_world(&mut e, 0x3000, 4200.0, 4200.0, 0x3000);
        let cell = e.call(0x0093_bba0, &args![this, 500.0f32]).u32();
        assert_eq!(cell, 0x3000);
        assert!(called(&e, 0x0045_1900).is_empty());
    }

    #[test]
    fn water_cell_of_an_unloaded_current_cell_is_none() {
        let mut e = engine();
        let this = water_world(&mut e, 0x3000, 4200.0, 4200.0, 0);
        e.register(0x004f_d3e0, |_, _| int(0));
        let cell = e.call(0x0093_bba0, &args![this, 500.0f32]).u32();
        assert_eq!(cell, 0);
    }

    #[test]
    fn water_cell_searches_the_neighbours_toward_the_edges() {
        let mut e = engine();
        // Grid cell (1, 1); 4200 is within 500 of the cell's low edges.
        let this = water_world(&mut e, 0, 4200.0, 4200.0, 1000);
        let cell = e.call(0x0093_bba0, &args![this, 500.0f32]).u32();
        assert_eq!(cell, 1000);
        assert_eq!(
            called(&e, 0x0045_1900),
            vec![vec![0x5000, 0, 1], vec![0x5000, 1, 0], vec![0x5000, 0, 0]]
        );
        // The bounds of grid cell (1, 1) were built with the cell size.
        let bounds = called(&e, 0x0045_2dc0);
        assert_eq!(bounds[0][1..], [4096.0f32.to_bits(), 4096.0f32.to_bits()]);
        assert_eq!(bounds[1][1..], [8192.0f32.to_bits(), 8192.0f32.to_bits()]);
    }

    #[test]
    fn water_cell_stays_in_the_middle_of_a_cell() {
        let mut e = engine();
        let this = water_world(&mut e, 0, 6000.0, 6000.0, 1011);
        let cell = e.call(0x0093_bba0, &args![this, 500.0f32]).u32();
        // No neighbour is needed: nothing is looked up and nothing is accepted.
        assert_eq!(cell, 0);
        assert!(called(&e, 0x0045_1900).is_empty());
    }

    #[test]
    fn water_cell_toward_the_high_edge_steps_one_cell_up() {
        let mut e = engine();
        // 8000 is within 500 of the high edge (8192) of grid cell (1, 1).
        let this = water_world(&mut e, 0, 6000.0, 8000.0, 1012);
        let cell = e.call(0x0093_bba0, &args![this, 500.0f32]).u32();
        assert_eq!(cell, 1012);
        assert_eq!(called(&e, 0x0045_1900), vec![vec![0x5000, 1, 2]]);
    }

    #[test]
    fn water_cell_clamps_a_large_radius_and_falls_back_to_the_first_water_cell() {
        let mut e = engine();
        let this = water_world(&mut e, 0, 4200.0, 4200.0, 0x7777);
        let found = e.mem.alloc(4);
        e.mem.set_u32(found, 0x7777);
        e.register_double(0x0045_16d0, move |_, _| int(found));
        let cell = e.call(0x0093_bba0, &args![this, 3000.0f32]).u32();
        assert_eq!(cell, 0x7777);
        assert_eq!(called(&e, 0x0045_16d0), vec![vec![0x5000]]);
        // An unclamped search does not use the fallback.
        let mut e = engine();
        let this = water_world(&mut e, 0, 4200.0, 4200.0, 0x7777);
        e.register(0x0045_16d0, |_, _| int(0x1234));
        e.call(0x0093_bba0, &args![this, 500.0f32]);
        assert!(called(&e, 0x0045_16d0).is_empty());
    }

    #[test]
    fn water_cell_is_none_when_the_final_cell_is_not_accepted() {
        let mut e = engine();
        let this = water_world(&mut e, 0, 6000.0, 6000.0, 1);
        e.register(0x0045_1900, |_, _| int(0x4444));
        let cell = e.call(0x0093_bba0, &args![this, 500.0f32]).u32();
        assert_eq!(cell, 0);
    }

    /// Every function `PositionPlayer` calls that is outside this file.
    const POSITION_PLAYER_CALLEES: &[u32] = &[
        0x0040_3df0,
        0x0040_8840,
        0x0040_8da0,
        0x0041_6870,
        0x0042_5fd0,
        0x0042_ce10,
        0x0043_d410,
        0x0043_f8d0,
        0x0043_fa80,
        0x0044_0460,
        0x0044_ddc0,
        0x0045_0b80,
        0x0045_1530,
        0x0045_2480,
        0x0045_3550,
        0x0045_3a70,
        0x0045_3dc0,
        0x0045_4450,
        0x0045_6520,
        0x0045_7d70,
        0x0045_8200,
        0x0045_8400,
        0x0045_a750,
        0x0045_c670,
        0x0046_dd00,
        0x0048_3710,
        0x004a_0c90,
        0x004a_3e00,
        0x004b_aec0,
        0x004f_d3e0,
        0x0052_4ac0,
        0x0052_4c90,
        0x0052_6070,
        0x0052_60a0,
        0x0054_6c20,
        0x0054_74b0,
        0x0054_8210,
        0x0054_8230,
        0x0054_ca90,
        0x0054_ddd0,
        0x0055_47c0,
        0x0055_8310,
        0x0055_9450,
        0x0056_10f0,
        0x0057_5bb0,
        0x0057_5d70,
        0x0058_6170,
        0x005b_9e80,
        0x005c_53d0,
        0x005f_36f0,
        0x0063_d060,
        0x0066_52e0,
        0x0066_5610,
        0x0066_5860,
        0x0068_15c0,
        0x006f_ca90,
        0x006f_cdb0,
        0x0070_0960,
        0x0070_5990,
        0x0070_5e30,
        0x0070_ede0,
        0x0070_edf0,
        0x0077_1700,
        0x0077_2c30,
        0x0078_cfc0,
        0x007f_a200,
        0x007f_a310,
        0x0082_2b90,
        0x0082_4400,
        0x0083_04a0,
        0x0084_d030,
        0x0084_e3a0,
        0x0086_7e30,
        0x0086_d490,
        0x0087_1dc0,
        0x0088_85e0,
        0x008d_3550,
        0x008d_6f30,
        0x0092_f260,
        0x0094_ae40,
        0x0095_0a60,
        0x0095_2c30,
        0x0096_1f90,
        0x0097_3de0,
        0x0097_85d0,
        0x00a0_3c90,
        0x00a0_9030,
        0x00a5_9c60,
        0x00a5_a040,
        0x00ad_78b0,
        0x00ad_7940,
        0x00ad_8780,
        0x00b4_f5c0,
        0x00b5_fd60,
        0x00b6_55b0,
        0x00ec_17c0,
        0x00ec_1800,
        0x00ec_2320,
        0x00ec_408c,
    ];

    /// Maps the exe regions the position code reads (constants and globals)
    /// and stores the constants it compares with.
    fn map_position_globals(e: &mut Engine) {
        e.map(0x0101_0000, 0x2_0000);
        e.map(0x0108_0000, 0x1_0000);
        e.map(0x011a_0000, 0x6_0000);
        e.map(0x0120_0000, 0x1_0000);
        e.map(0x0126_f000, 0x1000);
        e.set_global(PI_DOUBLE, std::f64::consts::PI);
        e.set_global(TWO_PI_DOUBLE, 2.0 * std::f64::consts::PI);
        e.set_global(NEGATIVE_PI_DOUBLE, -std::f64::consts::PI);
        e.set_global(LAND_CLEARANCE, 32.0f64);
        e.set_global(SETTLE_LIMIT, 0.1f64);
        e.set_global(SETTLE_TIME_STEP, 0.16f32);
        e.set_global(ZERO_DOUBLE, 0.0f64);
        e.set_global(IDENTITY_MATRIX, 0x3f80_0000u32);
        e.set_global(TES_SINGLETON, 0x5000u32);
        e.set_global(GLOBAL_OBJECT_0042CE10, 0x9900u32);
    }

    /// Registers a double over `addr` that records, at each call, the `count`
    /// floats the pointer argument number `index` points to (the frame
    /// blocks are gone after the call).
    fn watch_floats(
        e: &mut Engine,
        addr: u32,
        index: usize,
        count: u32,
    ) -> Rc<RefCell<Vec<Vec<f32>>>> {
        let seen = Rc::new(RefCell::new(vec![]));
        let log = seen.clone();
        e.register_double(addr, move |e, a| {
            let floats = (0..count).map(|i| e.mem.f32(a[index] + 4 * i)).collect();
            log.borrow_mut().push(floats);
            Ret::default()
        });
        seen
    }

    /// The player and the position block its virtual `+0x1F4` returns.
    struct PositionWorld {
        this: Ptr<PlayerCharacter>,
        /// The position the player reports.
        position: u32,
    }

    /// The world of `PositionPlayer`: every callee is a stub except those
    /// that must produce something usable (`NiMatrix3` products and the
    /// vector constructor return their output block).
    fn position_world(e: &mut Engine) -> PositionWorld {
        map_position_globals(e);
        let this = player(e);
        stub(e, POSITION_PLAYER_CALLEES);
        e.register(0x0043_f8d0, |_, a| int(a[1]));
        e.register(0x0041_6870, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            e.mem.set_u32(a[0] + 4, a[2]);
            e.mem.set_u32(a[0] + 8, a[3]);
            int(a[0])
        });
        e.set_global(PLAYER_SINGLETON, this.addr());
        let position = e.mem.alloc(16);
        e.mem.set_f32(position, 1.0);
        e.mem.set_f32(position + 4, 2.0);
        e.mem.set_f32(position + 8, 100.0);
        for slot in [0x1d0u32, 0x2a8, 0x3f4] {
            stub(e, &[0x00b0_0000 + slot]);
        }
        e.register_double(0x00b0_01f4, move |_, _| int(position));
        e.register(0x00b0_02bc, |_, _| float(0.5));
        put_vtable(
            e,
            this.addr(),
            0x0200_0000,
            &[
                (0x1d0, 0x00b0_01d0),
                (0x1f4, 0x00b0_01f4),
                (0x2a8, 0x00b0_02a8),
                (0x2bc, 0x00b0_02bc),
                (0x3f4, 0x00b0_03f4),
            ],
        );
        PositionWorld { this, position }
    }

    /// Calls `PositionPlayer` with position (10, 20, 30), rotation
    /// (`x`, `y`, `z`) and the given cell.
    fn position_player(
        e: &mut Engine,
        world: &PositionWorld,
        rotation: [f32; 3],
        cell: u32,
        weather: bool,
    ) {
        e.call(
            0x0093_c200,
            &args![
                world.this,
                10.0f32,
                20.0f32,
                30.0f32,
                rotation[0],
                rotation[1],
                rotation[2],
                cell,
                weather
            ],
        );
    }

    #[test]
    fn position_player_into_an_interior_cell() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0042_5fd0, |_, _| int(1));
        e.register(0x0054_74b0, |_, _| int(0x7700));
        let grid_position = watch_floats(&mut e, 0x0045_3dc0, 2, 3);
        let placed_at = watch_floats(&mut e, 0x00b0_02a8, 1, 3);
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        let this = world.this;
        assert_eq!(
            e.get(this, PlayerCharacter::pLastKnownMusicType).addr(),
            0x7700
        );
        // Placed into the cell with the position and rotation in the frame.
        let placed = called(&e, 0x0057_5bb0);
        assert_eq!(placed[0][1], 0x6000);
        assert_eq!(placed[1], vec![this.addr(), 0]);
        let grid = &called(&e, 0x0045_3dc0)[0];
        assert_eq!((grid[0], grid[1]), (0x5000, 0x6000));
        assert_eq!(grid_position.borrow()[0], vec![10.0, 20.0, 30.0]);
        // The player was moved to the position first.
        assert_eq!(placed_at.borrow()[0], vec![10.0, 20.0, 30.0]);
        assert_eq!(called(&e, 0x0054_8230), vec![vec![0x6000, this.addr(), 0]]);
        assert_eq!(called(&e, 0x00ad_8780).len(), 1);
        assert_eq!(called(&e, 0x00ad_8780)[0][1], 0x1000);
        // The part for cells that are not interior is skipped.
        assert!(called(&e, 0x0045_4450).is_empty());
        assert!(called(&e, 0x0045_8200).is_empty());
        assert!(called(&e, 0x0046_dd00).is_empty());
    }

    #[test]
    fn position_player_into_an_exterior_cell_clears_the_canopy_shadows() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0054_ddd0, |_, _| int(0x8800));
        e.register(0x0044_ddc0, |_, _| int(0x9100));
        let tail_position = watch_floats(&mut e, 0x0045_4450, 1, 3);
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        assert_eq!(called(&e, 0x0066_5860).len(), 1);
        assert_eq!(called(&e, 0x004b_aec0), vec![vec![0x9100]]);
        assert_eq!(called(&e, 0x0045_8200), vec![vec![0x5000, 0x8800]]);
        assert_eq!(called(&e, 0x0045_4450)[0][0], 0x5000);
        assert_eq!(tail_position.borrow()[0], vec![10.0, 20.0, 30.0]);
        assert!(called(&e, 0x0045_3dc0).is_empty());
    }

    #[test]
    fn position_player_keeps_the_canopy_shadows_of_a_world_with_them() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0045_2480, |_, _| int(1));
        e.register(0x0054_ddd0, |_, _| int(0x8800));
        let flag = e.mem.alloc(4);
        e.mem.set_u8(flag, 1);
        e.register_double(0x0054_8210, move |_, _| int(flag));
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        assert_eq!(called(&e, 0x0054_8210), vec![vec![0x8800]]);
        assert_eq!(called(&e, 0x0066_5610).len(), 1);
        assert!(called(&e, 0x0066_5860).is_empty());
        assert_eq!(called(&e, 0x0045_8200), vec![vec![0x5000, 0x8800]]);
    }

    #[test]
    fn position_player_without_a_cell_only_places_the_player() {
        let mut e = engine();
        let world = position_world(&mut e);
        position_player(&mut e, &world, [0.0; 3], 0, false);
        assert!(called(&e, 0x0054_74b0).is_empty());
        assert!(called(&e, 0x0045_8200).is_empty());
        assert_eq!(called(&e, 0x0045_4450).len(), 1);
        assert_eq!(called(&e, 0x0057_5bb0)[0], vec![world.this.addr(), 0]);
    }

    #[test]
    fn position_player_resets_the_weather_when_asked() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0046_dd00, |_, _| int(0xaa00));
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        assert!(called(&e, 0x0063_d060).is_empty());
        position_player(&mut e, &world, [0.0; 3], 0x6000, true);
        assert_eq!(called(&e, 0x0063_d060), vec![vec![0xaa00]]);
    }

    #[test]
    fn position_player_in_move_mode_dispels_a_spell_target() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x005f_36f0, |_, _| int(1));
        e.register(0x0082_2b90, |_, _| int(1));
        e.register(0x0070_5990, |_, _| int(0xbb00));
        e.mem.set_u32(GLOBAL_RETURNED_BY_0093CCD0, 0x4000);
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        let magic = world.this.addr() + 0x94;
        assert_eq!(called(&e, 0x0082_2b90), vec![vec![magic, 0x4018, 1]]);
        assert_eq!(
            called(&e, 0x007f_a310),
            vec![vec![0xbb00, 1, 0, 1], vec![0xbb00, 0, 0, 1]]
        );
        assert_eq!(called(&e, 0x0082_4400), vec![vec![magic, 0x4018, 0, 0]]);
    }

    #[test]
    fn position_player_in_move_mode_leaves_other_targets_alone() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x005f_36f0, |_, _| int(1));
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        let magic = world.this.addr() + 0x94;
        // No object from `0093ccd0`: the null target is passed.
        assert_eq!(called(&e, 0x0082_2b90), vec![vec![magic, 0, 1]]);
        assert!(called(&e, 0x0082_4400).is_empty());
    }

    #[test]
    fn position_player_switches_the_encounter_zones() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0054_6c20, |_, a| int(a[0] + 1));
        e.register(0x008d_6f30, |_, _| int(0x3300));
        e.register(0x0086_7e30, |_, _| int(0x1111));
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        assert_eq!(called(&e, 0x0052_6070), vec![vec![0x3301, 0x1111]]);
        assert_eq!(called(&e, 0x0052_60a0), vec![vec![0x6001, 0x1111]]);
        assert_eq!(called(&e, 0x0086_7e30), vec![vec![CALENDAR_OBJECT]]);
        // The player leaves its old parent cell.
        assert_eq!(
            called(&e, 0x0054_ca90),
            vec![vec![0x3300, world.this.addr()]]
        );
    }

    #[test]
    fn position_player_keeps_equal_encounter_zones() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0054_6c20, |_, _| int(5));
        e.register(0x008d_6f30, |_, _| int(0x3300));
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        assert!(called(&e, 0x0086_7e30).is_empty());
        assert!(called(&e, 0x0052_6070).is_empty());
    }

    #[test]
    fn position_player_normalizes_the_heading() {
        let tau = 2.0 * std::f64::consts::PI;
        for (heading, expected) in [
            (7.0f32, (7.0f32 as f64 - tau) as f32),
            (-7.0, (-7.0f32 as f64 + tau) as f32),
            (1.0, 1.0),
        ] {
            let mut e = engine();
            let world = position_world(&mut e);
            position_player(&mut e, &world, [0.0, 0.0, heading], 0x6000, false);
            let call = &called(&e, 0x004a_0c90)[0];
            assert_eq!(f(call, 1), expected);
        }
    }

    #[test]
    fn position_player_tilts_by_pitch_and_roll() {
        for (heading, expected) in [(1.0f32, 0.375f32), (-1.0, 0.625)] {
            let mut e = engine();
            let world = position_world(&mut e);
            position_player(&mut e, &world, [0.5, 0.125, heading], 0x6000, false);
            let call = &called(&e, 0x0052_4ac0)[0];
            assert_eq!(f(call, 1), expected);
        }
    }

    #[test]
    fn position_player_builds_the_root_transform_and_listener() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0045_3a70, |_, _| int(0xaa00));
        e.register(0x005b_9e80, |_, a| {
            float(f32::from_bits(a[0]) as f64 + 10.0)
        });
        e.register(0x005c_53d0, |_, a| {
            float(f32::from_bits(a[0]) as f64 + 20.0)
        });
        e.register(0x0045_c670, |_, _| int(0x1230));
        e.register(0x0055_8310, |_, a| int(a[0] + 1));
        let translated = watch_floats(&mut e, 0x0044_0460, 1, 3);
        e.set_global(LISTENER_UP_VECTOR, 1.0f32);
        e.set_global(LISTENER_UP_VECTOR + 4, 2.0f32);
        e.set_global(LISTENER_UP_VECTOR + 8, 3.0f32);
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        // The matrix products: player matrix times the turn, then the turn
        // times the result.
        assert_eq!(called(&e, 0x0043_f8d0).len(), 2);
        // The root node gets the matrix and the position, then an update.
        let rotate = &called(&e, 0x0043_fa80);
        assert_eq!(rotate.last().unwrap()[0], 0x1231);
        assert_eq!(called(&e, 0x0044_0460)[0][0], 0x1231);
        assert_eq!(translated.borrow()[0], vec![10.0, 20.0, 30.0]);
        let update = &called(&e, 0x0043_d410)[0];
        assert_eq!((f(update, 1), update[2], update[3]), (0.0, 0, 0));
        assert_eq!(called(&e, 0x00a5_9c60)[0][0], 0x1231);
        // The listener: the position, then the forward vector (sin and cos
        // of the heading from the player's virtual) and the up constant.
        assert_eq!(
            called(&e, 0x00ad_78b0),
            vec![vec![
                0xaa00,
                10.0f32.to_bits(),
                20.0f32.to_bits(),
                30.0f32.to_bits()
            ]]
        );
        let orient = &called(&e, 0x00ad_7940)[0];
        assert_eq!(orient[0], 0xaa00);
        assert_eq!(f(orient, 1), 20.5);
        assert_eq!(f(orient, 2), 10.5);
        assert_eq!(f(orient, 3), 0.0);
        assert_eq!((f(orient, 4), f(orient, 5), f(orient, 6)), (1.0, 2.0, 3.0));
    }

    #[test]
    fn position_player_updates_the_3d_node_of_the_player() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register_double(0x00b0_01d0, |_, _| int(0x4400));
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        // The player's own node gets the turn matrix, once, and the
        // fader is not started.
        assert_eq!(called(&e, 0x0043_fa80)[0][0], 0x4400);
        assert!(called(&e, 0x0070_0960).is_empty());
    }

    #[test]
    fn position_player_starts_the_fader_for_a_player_without_3d() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0045_6520, |_, _| int(0));
        e.set_global(FADER_MANAGER, 0x6600u32);
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        let fader = &called(&e, 0x0070_0960)[0];
        assert_eq!(
            (fader[0], fader[1], f(fader, 2), fader[3]),
            (0x6600, 1, 0.0, 0)
        );
        // Not when the other test says no.
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0042_ce10, |_, _| int(1));
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        assert!(called(&e, 0x0070_0960).is_empty());
    }

    #[test]
    fn position_player_closes_the_loading_menu_and_deletes_the_menu() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0070_edf0, |_, _| int(1));
        e.register(0x0070_ede0, |_, _| int(0x1500));
        e.register(0x00a0_9030, |_, a| int(a[0] + 1));
        e.register_double(0x00b0_0000, |_, _| int(0));
        let menu = e.mem.alloc(16);
        put_vtable(&mut e, menu, 0x0220_0000, &[(0, 0x00b0_0000)]);
        e.register_double(0x00a0_3c90, move |_, _| int(menu));
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        assert_eq!(called(&e, 0x0070_5e30).len(), 1);
        assert_eq!(called(&e, 0x00a0_9030), vec![vec![0x1500]]);
        assert_eq!(called(&e, 0x00a0_3c90), vec![vec![0x1501]]);
        assert_eq!(called(&e, 0x00b0_0000), vec![vec![menu, 1]]);
    }

    #[test]
    fn position_player_updates_terrain_and_the_hud_with_the_world_space() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0057_5d70, |_, _| int(0x3100));
        e.register(0x0058_6170, |_, a| int(a[0] + 1));
        e.register(0x0040_8da0, |_, a| int(a[0] + 2));
        let update_position = watch_floats(&mut e, 0x006f_ca90, 1, 3);
        let morph_position = watch_floats(&mut e, 0x006f_cdb0, 1, 3);
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        let update = &called(&e, 0x006f_ca90)[0];
        assert_eq!((update[0], update[2]), (0x3101, 0x0f));
        assert_eq!(update_position.borrow()[0], vec![1.0, 2.0, 100.0]);
        assert_eq!(called(&e, 0x006f_cdb0)[0][0], 0x3101);
        assert_eq!(morph_position.borrow()[0], vec![1.0, 2.0, 100.0]);
        // The HUD text is built from the singleton's world space + 0x18.
        assert_eq!(called(&e, 0x0040_8da0), vec![vec![0x3118]]);
        assert_eq!(called(&e, 0x0077_2c30), vec![vec![0x311a]]);
        assert_eq!(called(&e, 0x0097_3de0), vec![vec![PROCESS_LISTS]]);
    }

    #[test]
    fn position_player_without_a_world_space_has_no_terrain_or_text() {
        let mut e = engine();
        let world = position_world(&mut e);
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        assert!(called(&e, 0x006f_ca90).is_empty());
        assert!(called(&e, 0x0040_8da0).is_empty());
        assert_eq!(called(&e, 0x0077_2c30), vec![vec![0]]);
    }

    #[test]
    fn position_player_plays_the_movie_when_the_loading_screen_flag_is_set() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.mem.set_u8(LOADING_SCREEN_FLAG, 1);
        e.register(0x0040_3df0, |_, _| int(0x7e00));
        e.set_global(MOVIE_PLAYER, 0x6300u32);
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        assert_eq!(called(&e, 0x0045_7d70), vec![vec![0x5000, 0, 0, 0]]);
        assert_eq!(called(&e, 0x0077_1700), vec![vec![0x18], vec![0x0c]]);
        assert_eq!(called(&e, 0x0083_04a0).len(), 1);
        assert_eq!(called(&e, 0x0040_3df0), vec![vec![MOVIE_OBJECT]]);
        assert_eq!(
            called(&e, 0x00ec_2320),
            vec![vec![0x6300, 0x7e00, 1, 1, 0, 1, 1, 0, 1, 1]]
        );
    }

    #[test]
    fn position_player_resumes_the_controller_after_a_sequence() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x00ec_17c0, |_, _| int(1));
        e.set_global(MOVIE_PLAYER, 0x6300u32);
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        assert_eq!(called(&e, 0x00ec_1800), vec![vec![0x6300]]);
        let mut e = engine();
        let world = position_world(&mut e);
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        assert!(called(&e, 0x00ec_1800).is_empty());
    }

    /// The first-person settling world: the player has a first-person 3D
    /// (the `NiPointer` content getter answers for the member), the other
    /// test (`0042ce10`) is false and `Move` changes the height reported by
    /// the player by `fall` each call.
    fn settling_world(e: &mut Engine, fall: f32) -> PositionWorld {
        let world = position_world(e);
        let member_3d = world.this.addr() + 0x694;
        e.register_double(0x0055_9450, move |_, a| int((a[0] == member_3d) as u32));
        e.register(0x0040_8840, |_, a| float(f32::from_bits(a[0]).abs() as f64));
        let position = world.position;
        e.register_double(0x0092_f260, move |e, _| {
            let z = e.mem.f32(position + 8);
            e.mem.set_f32(position + 8, z + fall);
            Ret::default()
        });
        world
    }

    #[test]
    fn position_player_settles_a_player_that_stops_falling() {
        let mut e = engine();
        let world = settling_world(&mut e, -0.01);
        let vectors = Rc::new(RefCell::new(vec![]));
        let log = vectors.clone();
        let position = world.position;
        e.register_double(0x0092_f260, move |e, a| {
            log.borrow_mut().push(vec![
                e.mem.f32(a[2]),
                e.mem.f32(a[2] + 4),
                e.mem.f32(a[2] + 8),
            ]);
            let z = e.mem.f32(position + 8);
            e.mem.set_f32(position + 8, z - 0.01);
            Ret::default()
        });
        let placed_at = watch_floats(&mut e, 0x00b0_02a8, 1, 3);
        position_player(&mut e, &world, [0.0; 3], 0, false);
        // One Move: the first fall is already below 0.1.
        let moves = called(&e, 0x0092_f260);
        assert_eq!(moves.len(), 1);
        assert_eq!((moves[0][0], f(&moves[0], 1)), (world.this.addr(), 0.16));
        assert_eq!(moves[0][3], 0);
        // The move vector is the zero vector.
        assert_eq!(vectors.borrow()[0], vec![0.0, 0.0, 0.0]);
        // Settled: the player is not put back (only the first placement).
        assert_eq!(placed_at.borrow().len(), 1);
    }

    #[test]
    fn position_player_puts_a_player_that_keeps_falling_back_after_100_moves() {
        let mut e = engine();
        let world = settling_world(&mut e, -1.0);
        let placed_at = watch_floats(&mut e, 0x00b0_02a8, 1, 3);
        position_player(&mut e, &world, [0.0; 3], 0, false);
        assert_eq!(called(&e, 0x0092_f260).len(), 100);
        let sets = placed_at.borrow();
        assert_eq!(sets.len(), 2);
        // The start position (z 100 before the moves) is restored.
        assert_eq!(sets[1], vec![1.0, 2.0, 100.0]);
    }

    #[test]
    fn position_player_lifts_a_player_below_the_land() {
        let mut e = engine();
        let world = settling_world(&mut e, 0.0);
        e.register(0x0055_47c0, |e, a| {
            e.mem.set_f32(a[2], 90.0);
            int(1)
        });
        let placed_at = watch_floats(&mut e, 0x00b0_02a8, 1, 3);
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        // The land is at 90 and the clearance 32: the player (at 100) is
        // below 122 and is lifted to it.
        let sets = placed_at.borrow();
        assert_eq!(sets.len(), 2);
        assert_eq!(sets[1], vec![1.0, 2.0, 122.0]);
        let land = &called(&e, 0x0055_47c0)[0];
        assert_eq!(land[0], 0x6000);
    }

    #[test]
    fn position_player_leaves_a_player_above_the_land() {
        let mut e = engine();
        let world = settling_world(&mut e, 0.0);
        e.register(0x0055_47c0, |e, a| {
            e.mem.set_f32(a[2], 10.0);
            int(1)
        });
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        assert_eq!(called(&e, 0x00b0_02a8).len(), 1);
    }

    #[test]
    fn position_player_does_not_look_for_land_in_an_interior_cell() {
        let mut e = engine();
        let world = settling_world(&mut e, 0.0);
        e.register(0x0042_5fd0, |_, _| int(1));
        position_player(&mut e, &world, [0.0; 3], 0x6000, false);
        assert!(called(&e, 0x0055_47c0).is_empty());
    }

    #[test]
    fn position_player_refreshes_the_animations_and_the_camera() {
        let mut e = engine();
        let world = settling_world(&mut e, 0.0);
        e.register(0x0095_0a60, |_, a| int(0x1000 + a[1]));
        e.register(0x0042_ce10, |_, _| int(1));
        e.mem.set_u8(world.this.addr() + 0x64a, 0);
        position_player(&mut e, &world, [0.0; 3], 0, false);
        // The third-person flag is toggled around each refresh and ends as
        // it started.
        assert_eq!(e.mem.u8(world.this.addr() + 0x64a), 0);
        assert_eq!(
            called(&e, 0x0095_0a60),
            vec![vec![world.this.addr(), 0], vec![world.this.addr(), 1]]
        );
        let updates = called(&e, 0x0088_85e0);
        assert_eq!(updates[0][..2], [world.this.addr(), 0x1000]);
        assert_eq!(updates[1][..2], [world.this.addr(), 0x1001]);
        assert_eq!(called(&e, 0x008d_3550).len(), 2);
        assert_eq!(called(&e, 0x0094_ae40), vec![vec![world.this.addr(), 0, 0]]);
        // No settling when the other test says so.
        assert!(called(&e, 0x0092_f260).is_empty());
    }

    #[test]
    fn position_player_updates_the_camera_rigid_body() {
        let mut e = engine();
        let world = position_world(&mut e);
        let member = world.this.addr() + 0xdec;
        e.register_double(0x0055_9450, move |_, a| {
            int(if a[0] == member { 0x7a00 } else { 0 })
        });
        let camera_position = watch_floats(&mut e, 0x004a_3e00, 1, 3);
        position_player(&mut e, &world, [0.0; 3], 0, false);
        let matrix = &called(&e, 0x004a_3e00)[0];
        let body = &called(&e, 0x0056_10f0)[0];
        assert_eq!(body[0], 0x7a00);
        assert_eq!(body[1], matrix[1]);
        assert_eq!(camera_position.borrow()[0], vec![1.0, 2.0, 100.0]);
    }

    #[test]
    fn position_player_updates_the_transfer_state() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0052_4c90, |_, _| int(0x5100));
        e.register(0x0084_d030, |_, _| float(2.5));
        e.register(0x00b4_f5c0, |_, _| int(0x6200));
        position_player(&mut e, &world, [0.0; 3], 0, false);
        assert_eq!(called(&e, 0x0066_52e0), vec![vec![0x5100, 0]]);
        assert_eq!(called(&e, 0x0084_d030), vec![vec![GAME_TIMER]]);
        let fade = &called(&e, 0x0045_3550)[0];
        assert_eq!((fade[0], f(fade, 1)), (0x5000, 2.5));
        assert_eq!(called(&e, 0x00b6_55b0), vec![vec![0x6200]]);
        assert_eq!(called(&e, 0x0078_cfc0).len(), 1);
        assert_eq!(
            called(&e, 0x0095_2c30),
            vec![vec![world.this.addr(), world.this.addr()]]
        );
        assert_eq!(called(&e, 0x0096_1f90), vec![vec![world.this.addr()]]);
        assert_eq!(called(&e, 0x0045_a750), vec![vec![0x5000]]);
    }

    #[test]
    fn position_player_does_not_suspend_loading_when_the_other_test_says_so() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0042_ce10, |_, _| int(1));
        position_player(&mut e, &world, [0.0; 3], 0, false);
        assert!(called(&e, 0x0078_cfc0).is_empty());
    }

    #[test]
    fn position_player_renders_the_menu_background_when_the_grid_cannot_take_it() {
        let mut e = engine();
        let world = position_world(&mut e);
        e.set_global(MAIN_SINGLETON, 0x8100u32);
        position_player(&mut e, &world, [0.0; 3], 0, false);
        assert_eq!(called(&e, 0x0087_1dc0), vec![vec![0x8100]]);
        assert_eq!(called(&e, 0x0097_85d0), vec![vec![PROCESS_LISTS]]);
        let mut e = engine();
        let world = position_world(&mut e);
        e.register(0x0045_1530, |_, _| int(1));
        position_player(&mut e, &world, [0.0; 3], 0, false);
        assert!(called(&e, 0x0087_1dc0).is_empty());
    }

    /// Every function `FastTravel` calls that is outside this file.
    const FAST_TRAVEL_CALLEES: &[u32] = &[
        0x0040_37b0,
        0x0040_37d0,
        0x0040_6d00,
        0x0040_6d50,
        0x0040_8d60,
        0x0040_fba0,
        0x0040_fbf0,
        0x0043_0830,
        0x0045_39a0,
        0x0045_7d70,
        0x0045_9870,
        0x0045_aee0,
        0x0046_dd00,
        0x0048_3710,
        0x0048_39c0,
        0x004f_f7e0,
        0x0050_2670,
        0x0052_6ac0,
        0x0055_9450,
        0x0057_5d70,
        0x005b_5e40,
        0x005e_58f0,
        0x0063_e8f0,
        0x0065_0a30,
        0x0065_2110,
        0x0068_15c0,
        0x006d_4eb0,
        0x006d_cd70,
        0x0082_5c00,
        0x0085_0a40,
        0x0086_7950,
        0x0086_7a40,
        0x0087_8160,
        0x0087_8200,
        0x0087_8250,
        0x0088_4eb0,
        0x0088_b0a0,
        0x0088_b510,
        0x0088_d640,
        0x008d_0500,
        0x008d_6f30,
        0x0093_06d0,
        0x0096_1f90,
        0x0096_b050,
        0x0096_b470,
        0x0096_b810,
        0x0096_bcd0,
        0x0096_d490,
        0x0096_d4b0,
        0x0096_db30,
        0x0096_df40,
        0x0096_eb40,
        0x0097_2d30,
        0x0097_3ee0,
        0x0097_5d10,
        0x009e_a3b0,
        0x00aa_7030,
    ];

    /// The functions `PositionPlayerExterior` and `ClearCellsIfTargetNotLoaded`
    /// call outside this file (besides those in the lists above).
    const EXTERIOR_PLACEMENT_CALLEES: &[u32] = &[
        0x0045_11e0,
        0x0045_4af0,
        0x0046_1330,
        0x0052_8540,
        0x0054_ca90,
        0x0058_5b30,
        0x0058_75a0,
        0x0095_0bb0,
        0x0096_11e0,
        0x0096_d810,
    ];

    /// The world of `FastTravel`: the position world (the final placement
    /// runs `PositionPlayer`), the callees above as stubs, the player's
    /// virtual `+0x358` false and the destination object reporting the
    /// position (7, 8, 9).
    fn fast_travel_world(e: &mut Engine) -> (PositionWorld, u32) {
        let world = exterior_world(e);
        e.set_global(SECONDS_PER_HOUR_DOUBLE, 3600.0f64);
        e.set_global(SECONDS_PER_HOUR_FLOAT, 3600.0f32);
        e.set_global(TWO_FLOAT, 2.0f32);
        e.set_global(NO_PATH_LENGTH, f32::MAX as f64);
        register_ftol(e);
        e.register_double(0x00b0_0358, |_, _| int(0));
        e.mem.set_u32(0x0200_0000 + 0x358, 0x00b0_0358);
        let setting = e.mem.alloc(8);
        e.register_double(0x0040_8d60, move |_, _| int(setting));
        // The destination: an object with a virtual +0x1F4 giving its position.
        let destination = e.mem.alloc(16);
        let spot = e.mem.alloc(16);
        for (i, v) in [7.0f32, 8.0, 9.0].into_iter().enumerate() {
            e.mem.set_f32(spot + 4 * i as u32, v);
        }
        e.register_double(0x00b4_01f4, move |_, _| int(spot));
        put_vtable(e, destination, 0x0230_0000, &[(0x1f4, 0x00b4_01f4)]);
        // The destination's rotation (4, 5, 6).
        let rotation = e.mem.alloc(16);
        for (i, v) in [4.0f32, 5.0, 6.0].into_iter().enumerate() {
            e.mem.set_f32(rotation + 4 * i as u32, v);
        }
        e.register_double(0x0043_0830, move |_, _| int(rotation));
        (world, destination)
    }

    #[test]
    fn fast_travel_does_nothing_when_the_entry_point_leaves_no_delay() {
        let mut e = engine();
        let this = player(&mut e);
        e.map(0x0101_0000, 0x2_0000);
        e.set_global(ZERO_DOUBLE, 0.0f64);
        stub(&mut e, &[0x005e_58f0]);
        e.register_double(0x00b0_0358, |_, _| int(1));
        put_vtable(&mut e, this.addr(), 0x0200_0000, &[(0x358, 0x00b0_0358)]);
        e.call(0x0093_cdf0, &args![this, 0x7000u32]);
        assert_eq!(
            call_addresses(&e),
            vec![0x0093_cdf0, 0x005e_58f0, 0x00b0_0358]
        );
        let entry = &called(&e, 0x005e_58f0)[0];
        assert_eq!((entry[0], entry[1]), (0x33, this.addr()));
    }

    /// Runs `FastTravel` in the fast-travel world with the given path
    /// length and run speed; the multi-bound radius is 2.0, the calendar
    /// scale 20 and the clock 1000.
    fn run_fast_travel(e: &mut Engine, length: f32, speed: f32) -> (PositionWorld, u32) {
        run_fast_travel_with(e, length, speed, |_| {})
    }

    /// [`run_fast_travel`] with `setup` run after the world is built (so it
    /// can replace the stubs).
    fn run_fast_travel_with(
        e: &mut Engine,
        length: f32,
        speed: f32,
        setup: impl FnOnce(&mut Engine),
    ) -> (PositionWorld, u32) {
        let (world, destination) = fast_travel_world(e);
        e.register_double(0x006d_4eb0, move |_, _| float(length as f64));
        e.register_double(0x0088_4eb0, move |_, _| float(speed as f64));
        e.register(0x0052_6ac0, |_, _| float(2.0));
        e.register(0x0086_7950, |_, _| float(20.0));
        e.register(0x0096_d490, |_, _| float(1000.0));
        e.register(0x0057_5d70, |_, _| int(0x7300));
        e.register(0x0058_75a0, |_, _| int(0x7400));
        e.register(0x0045_11e0, |_, _| int(1));
        e.set_global(PLAYER_SINGLETON, world.this.addr());
        setup(e);
        e.call(0x0093_cdf0, &args![world.this, destination]);
        (world, destination)
    }

    #[test]
    fn fast_travel_advances_the_clock_hour_by_hour() {
        let mut e = engine();
        // 36000 units at speed 10 is 3600 seconds, an hour, times the
        // radius 2.0: two hours.
        let (world, destination) = run_fast_travel(&mut e, 36000.0, 10.0);
        let this = world.this;
        // Two hours of 3600 / 20 = 180 time units.
        let clock = called(&e, 0x0096_d4b0);
        assert_eq!(clock.len(), 2);
        assert_eq!((clock[0][0], f(&clock[0], 1)), (PROCESS_LISTS, 1180.0));
        let calendar = called(&e, 0x0086_7a40);
        assert_eq!(calendar.len(), 2);
        assert_eq!(
            (calendar[0][0], f(&calendar[0], 1)),
            (CALENDAR_OBJECT, 180.0)
        );
        assert_eq!(called(&e, 0x0040_fbf0), vec![vec![0x011f_11a0, 0]; 2]);
        assert_eq!(called(&e, 0x0040_fba0), vec![vec![0x011f_11a0]; 2]);
        for address in [0x0096_bcd0, 0x0096_b810, 0x0096_b470, 0x0096_b050] {
            let calls = called(&e, address);
            assert_eq!(calls.len(), 2);
            assert_eq!(
                (calls[0][0], f(&calls[0], 1), calls[0][2]),
                (PROCESS_LISTS, 3600.0, 1)
            );
        }
        let simulate = &called(&e, 0x0096_db30)[0];
        assert_eq!((f(simulate, 1), simulate[2], simulate[3]), (3600.0, 1, 0));
        assert_eq!(called(&e, 0x0096_eb40).len(), 2);
        // The player is healed each hour (with 2.0).
        let heal = called(&e, 0x0088_b510);
        assert_eq!(heal.len(), 2);
        assert_eq!((heal[0][0], f(&heal[0], 1)), (this.addr(), 2.0));
        assert_eq!(e.get(this, PlayerCharacter::iSleepTime), 0);
        assert!(!e.get(this, PlayerCharacter::bIsSleeping));
        let _ = destination;
    }

    #[test]
    fn fast_travel_advances_a_fractional_trip_at_once() {
        let mut e = engine();
        // 9000 units at speed 10 is 900 seconds: a quarter hour times the
        // radius 2.0 is half an hour: no whole hours.
        let (world, _) = run_fast_travel(&mut e, 9000.0, 10.0);
        assert!(called(&e, 0x0040_fbf0).is_empty());
        assert!(called(&e, 0x0088_b510).is_empty());
        let calendar = called(&e, 0x0086_7a40);
        assert_eq!(calendar.len(), 1);
        // 0.5 hours of 3600 seconds over the scale 20: 90 time units.
        assert_eq!(f(&calendar[0], 1), 90.0);
        assert_eq!(f(&called(&e, 0x0096_d4b0)[0], 1), 1090.0);
        assert_eq!(e.get(world.this, PlayerCharacter::iSleepTime), 0);
    }

    #[test]
    fn fast_travel_reports_a_missing_path_and_takes_no_time() {
        let mut e = engine();
        e.register(0x008d_6f30, |_, _| int(0));
        let (_world, _) = run_fast_travel(&mut e, f32::MAX, 10.0);
        // The message is built with the format (no cell name without a
        // parent cell) and logged; the trip takes no time.
        let format = &called(&e, 0x0040_6d00)[0];
        assert_eq!((format[1], format[2]), (0x400, 0x0108_aff4));
        assert_eq!(called(&e, 0x005b_5e40), vec![vec![format[0]]]);
        assert!(called(&e, 0x0040_37b0).is_empty());
        assert!(called(&e, 0x0040_fbf0).is_empty());
    }

    #[test]
    fn fast_travel_names_the_destination_cell_in_the_missing_path_message() {
        let mut e = engine();
        let cell = e.mem.alloc(16);
        e.register_double(0x00b4_0090, |_, _| Ret::default());
        put_vtable(&mut e, cell, 0x0231_0000, &[(0x90, 0x00b4_0090)]);
        let (_world, destination) = run_fast_travel_with(&mut e, f32::MAX, 10.0, |e| {
            e.register_double(0x008d_6f30, move |_, a| {
                int(if a[0] != 0 { cell } else { 0 })
            });
            e.register(0x0055_9450, |_, _| int(0x7e00));
        });
        let name = called(&e, 0x0040_37b0)[0][0];
        assert_eq!(called(&e, 0x00b4_0090), vec![vec![cell, name]]);
        let format = &called(&e, 0x0040_6d50)[0];
        assert_eq!((format[1], format[2]), (0x400, 0x7e00));
        assert_eq!(called(&e, 0x0040_37d0), vec![vec![name]]);
        let _ = destination;
    }

    #[test]
    fn fast_travel_places_the_player_at_the_destination() {
        let mut e = engine();
        let (world, destination) = run_fast_travel(&mut e, 36000.0, 10.0);
        let this = world.this;
        // PositionPlayerExterior found the cell of the destination's world
        // space and put the player there.
        assert_eq!(called(&e, 0x0058_75a0)[0][0], 0x7300);
        assert_eq!(called(&e, 0x0057_5bb0)[0], vec![this.addr(), 0x7400]);
        let listener = &called(&e, 0x00ad_78b0)[0];
        assert_eq!(
            listener[1..],
            [7.0f32.to_bits(), 8.0f32.to_bits(), 9.0f32.to_bits()]
        );
        // Around it: the travel flag, the sky and the beds/chairs set up.
        assert_eq!(e.mem.u8(FAST_TRAVELLING), 0);
        assert_eq!(called(&e, 0x0045_9870), vec![vec![0x5000]]);
        assert_eq!(called(&e, 0x0097_2d30), vec![vec![PROCESS_LISTS]]);
        assert_eq!(called(&e, 0x0096_df40), vec![vec![PROCESS_LISTS]]);
        assert_eq!(called(&e, 0x0097_5d10), vec![vec![PROCESS_LISTS]]);
        assert_eq!(
            called(&e, 0x0097_3ee0),
            vec![vec![PROCESS_LISTS, destination]]
        );
        assert_eq!(called(&e, 0x0096_1f90)[0], vec![this.addr()]);
        assert_eq!(e.mem.u8(this.addr() + 0x20c), 0);
        assert_eq!(e.mem.u8(this.addr() + 0x204), 0);
        assert_eq!(called(&e, 0x0045_7d70)[0], vec![0x5000, 1, destination, 0]);
    }

    #[test]
    fn fast_travel_sets_the_sky_and_the_travel_flag_around_the_placement() {
        // The sky is told "fast travelling" (1) before the placement and
        // "ended" (0) after it.
        let mut e = engine();
        let flags = Rc::new(RefCell::new(vec![]));
        let sky_calls = Rc::new(RefCell::new(vec![]));
        let (world, destination) = fast_travel_world(&mut e);
        e.register(0x0046_dd00, |_, _| int(0xaa00));
        let log = sky_calls.clone();
        e.register_double(0x0063_e8f0, move |e, a| {
            log.borrow_mut()
                .push((a[0], a[1], e.mem.u8(FAST_TRAVELLING)));
            Ret::default()
        });
        let log = flags.clone();
        e.register_double(0x0057_5bb0, move |e, a| {
            log.borrow_mut().push((a[1], e.mem.u8(FAST_TRAVELLING)));
            Ret::default()
        });
        e.register(0x0057_5d70, |_, _| int(0x7300));
        e.register(0x0058_75a0, |_, _| int(0x7400));
        e.register(0x0045_11e0, |_, _| int(1));
        e.register(0x006d_4eb0, |_, _| float(0.0));
        e.register(0x0088_4eb0, |_, _| float(1.0));
        e.register(0x0086_7950, |_, _| float(1.0));
        let rotation = e.mem.alloc(16);
        e.register_double(0x0043_0830, move |_, _| int(rotation));
        e.set_global(PLAYER_SINGLETON, world.this.addr());
        e.call(0x0093_cdf0, &args![world.this, destination]);
        assert_eq!(*sky_calls.borrow(), vec![(0xaa00, 1, 0), (0xaa00, 0, 0)]);
        // The travel flag is set while the player is placed (twice: into the
        // cell, then out of it) and cleared afterwards.
        assert_eq!(*flags.borrow(), vec![(0x7400, 1), (0, 1)]);
    }

    #[test]
    fn fast_travel_flushes_the_cells_when_the_pause_is_long() {
        let mut e = engine();
        let (world, destination) = fast_travel_world(&mut e);
        let setting = e.mem.alloc(8);
        e.mem.set_u8(setting, 1);
        e.register_double(0x0040_8d60, move |_, _| int(setting));
        e.set_global(FAST_TRAVEL_REACTOR, 0x6a00u32);
        let times = Rc::new(RefCell::new(vec![5000u32, 5000, 30000, 31000]));
        let queue = times.clone();
        e.register_double(0x0082_5c00, move |_, _| int(queue.borrow_mut().remove(0)));
        e.register(0x006d_4eb0, |_, _| float(0.0));
        e.register(0x0088_4eb0, |_, _| float(1.0));
        e.register(0x0086_7950, |_, _| float(1.0));
        e.register(0x0058_75a0, |_, _| int(0));
        e.register(0x0058_5b30, |_, _| int(0));
        e.register(0x0046_1330, |_, _| int(0));
        e.set_global(PLAYER_SINGLETON, world.this.addr());
        // Run it twice; the static timestamp is initialised the first time.
        e.call(0x0093_cdf0, &args![world.this, destination]);
        // First call: init stamp 5000, now 5000: elapsed 0, so it reacts
        // and re-stamps with 30000.
        assert_eq!(called(&e, 0x0085_0a40), vec![vec![0x6a00]]);
        assert_eq!(e.global::<u32>(FAST_TRAVEL_LAST_TIME), 30000);
    }

    #[test]
    fn fast_travel_leaves_the_pause_alone_when_the_setting_is_off() {
        let mut e = engine();
        let (world, destination) = fast_travel_world(&mut e);
        e.register(0x006d_4eb0, |_, _| float(0.0));
        e.register(0x0088_4eb0, |_, _| float(1.0));
        e.register(0x0086_7950, |_, _| float(1.0));
        e.register(0x0058_75a0, |_, _| int(0));
        e.register(0x0058_5b30, |_, _| int(0));
        e.register(0x0046_1330, |_, _| int(0));
        e.set_global(PLAYER_SINGLETON, world.this.addr());
        e.call(0x0093_cdf0, &args![world.this, destination]);
        assert!(called(&e, 0x0085_0a40).is_empty());
        assert_eq!(called(&e, 0x0082_5c00).len(), 2);
    }

    #[test]
    fn fast_travel_cache_cleanup_runs_when_the_face_cache_exists() {
        let mut e = engine();
        let (world, destination) = fast_travel_world(&mut e);
        e.register(0x006d_4eb0, |_, _| float(0.0));
        e.register(0x0088_4eb0, |_, _| float(1.0));
        e.register(0x0086_7950, |_, _| float(1.0));
        e.register(0x0058_75a0, |_, _| int(0));
        e.register(0x0058_5b30, |_, _| int(0));
        e.register(0x0046_1330, |_, _| int(0));
        e.register(0x0065_2110, |_, _| int(0x6b00));
        e.register(0x0045_4af0, |_, _| int(1));
        e.set_global(PLAYER_SINGLETON, world.this.addr());
        e.call(0x0093_cdf0, &args![world.this, destination]);
        assert_eq!(called(&e, 0x0065_0a30)[0], vec![0x6b00, 0]);
        // The object at 011c3c10 is asked twice (before and after) and the
        // grid told (0, then 1).
        assert_eq!(
            called(&e, 0x0045_aee0),
            vec![vec![0x5000, 0], vec![0x5000, 1]]
        );
    }

    #[test]
    fn the_cell_grid_object_query_returns_the_callee_result() {
        let mut e = engine();
        e.register(0x0045_4af0, |_, _| int(1));
        assert!(e.call(0x0093_d4f0, &args![]).bool());
        assert_eq!(called(&e, 0x0045_4af0), vec![vec![0x011c_3c10]]);
        e.register(0x0045_4af0, |_, _| int(0));
        assert!(!e.call(0x0093_d4f0, &args![]).bool());
    }

    /// The world of `ClearCellsIfTargetNotLoaded`: every callee a stub, the
    /// TES singleton pointing at 0x5000 and the player singleton at the
    /// returned player.
    fn clear_cells_world(e: &mut Engine) -> Ptr<PlayerCharacter> {
        let this = player(e);
        e.map(0x011a_0000, 0x6_0000);
        e.set_global(TES_SINGLETON, 0x5000u32);
        e.set_global(PLAYER_SINGLETON, this.addr());
        stub(
            e,
            &[
                0x0045_11e0,
                0x0045_39a0,
                0x0045_7d70,
                0x0054_ca90,
                0x0065_0a30,
                0x0065_2110,
                0x0087_8160,
                0x0087_8200,
                0x0087_8250,
                0x008d_6f30,
                0x0095_0bb0,
                0x0096_11e0,
                0x0096_d810,
                0x0096_eb40,
                0x00aa_7030,
            ],
        );
        this
    }

    #[test]
    fn clear_cells_does_nothing_for_a_loaded_cell() {
        let mut e = engine();
        let this = clear_cells_world(&mut e);
        e.register(0x0045_11e0, |_, _| int(1));
        e.call(0x0093_d500, &args![this, 0x6000u32, 0x7000u32]);
        assert_eq!(call_addresses(&e), vec![0x0093_d500, 0x0045_11e0]);
        assert_eq!(called(&e, 0x0045_11e0), vec![vec![0x5000, 0x6000, 0]]);
    }

    #[test]
    fn clear_cells_flushes_the_grid_for_the_cell() {
        let mut e = engine();
        let this = clear_cells_world(&mut e);
        e.register(0x008d_6f30, |_, _| int(0x3300));
        e.register(0x0087_8160, |e, a| {
            e.mem.set_u8(a[0] + 5, 0x55);
            Ret::default()
        });
        e.register(0x0065_2110, |_, _| int(0x6b00));
        let busy = Rc::new(RefCell::new(vec![]));
        let seen = busy.clone();
        e.register_double(0x0096_d810, move |e, _| {
            seen.borrow_mut().push(e.mem.u8(PROCESS_LISTS_BUSY));
            Ret::default()
        });
        // The found object has a virtual +0xC returning a handler holder.
        let found = e.mem.alloc(16);
        let holder = e.mem.alloc(16);
        let handler = e.mem.alloc(16);
        e.register_double(0x00b0_000c, move |_, _| int(holder));
        e.register_double(0x00b0_00e8, |_, _| Ret::default());
        put_vtable(&mut e, found, 0x0240_0000, &[(0xc, 0x00b0_000c)]);
        put_vtable(&mut e, handler, 0x0241_0000, &[(0xe8, 0x00b0_00e8)]);
        e.register_double(0x0095_0bb0, move |_, _| int(found));
        e.register_double(0x0096_11e0, move |_, _| int(handler));
        e.call(0x0093_d500, &args![this, 0x6000u32, 0x7000u32]);
        assert_eq!(called(&e, 0x0045_7d70), vec![vec![0x5000, 1, 0x6000, 0]]);
        // The player leaves its parent cell.
        assert_eq!(called(&e, 0x0054_ca90), vec![vec![0x3300, this.addr()]]);
        assert_eq!(called(&e, 0x0095_0bb0), vec![vec![this.addr(), 1]]);
        assert_eq!(called(&e, 0x0096_11e0)[0], vec![holder]);
        assert_eq!(called(&e, 0x00b0_00e8), vec![vec![handler, found]]);
        assert_eq!(called(&e, 0x0087_8160)[0][1..], [0, 1, 0]);
        assert_eq!(called(&e, 0x0045_39a0), vec![vec![0x5000, 0, 1]]);
        assert_eq!(*busy.borrow(), vec![1]);
        assert_eq!(e.mem.u8(PROCESS_LISTS_BUSY), 0);
        assert_eq!(called(&e, 0x0096_eb40), vec![vec![PROCESS_LISTS]]);
        assert_eq!(called(&e, 0x0087_8250), vec![vec![0x55]]);
        assert_eq!(called(&e, 0x0087_8200).len(), 1);
        assert_eq!(called(&e, 0x0065_0a30), vec![vec![0x6b00, 0]]);
        assert_eq!(called(&e, 0x00aa_7030).len(), 1);
    }

    #[test]
    fn clear_cells_uses_the_world_space_without_a_cell() {
        let mut e = engine();
        let this = clear_cells_world(&mut e);
        e.call(0x0093_d500, &args![this, 0u32, 0x7000u32]);
        assert!(called(&e, 0x0045_11e0).is_empty());
        assert_eq!(called(&e, 0x0045_7d70), vec![vec![0x5000, 1, 0x7000, 0]]);
        // Nothing found, no parent cell, no face cache.
        assert!(called(&e, 0x0054_ca90).is_empty());
        assert!(called(&e, 0x0096_11e0).is_empty());
        assert!(called(&e, 0x0065_0a30).is_empty());
    }

    /// The world of the queued-request handler: the position world with
    /// the handler's own callees stubbed, the player's virtual `+0x358`
    /// true (so a fast travel returns at once) and the deletions recorded.
    fn handler_world(e: &mut Engine) -> (PositionWorld, Rc<RefCell<Vec<u32>>>) {
        let world = exterior_world(e);
        stub(
            e,
            &[
                0x0045_2eb0,
                NI_POINTER_ASSIGN,
                0x0056_9b80,
                0x0057_3f20,
                0x0086_8d70,
                0x005d_14d0,
                0x0056_8680,
                0x008d_8520,
                0x007a_f430,
                0x0050_9420,
                0x0056_82c0,
                0x0056_8500,
                0x0088_d2f0,
                0x0093_06d0,
            ],
        );
        e.register_double(0x00b0_0358, |_, _| int(1));
        e.mem.set_u32(0x0200_0000 + 0x358, 0x00b0_0358);
        let deleted = Rc::new(RefCell::new(vec![]));
        let log = deleted.clone();
        e.register_double(OPERATOR_DELETE, move |_, a| {
            log.borrow_mut().push(a[0]);
            Ret::default()
        });
        (world, deleted)
    }

    /// Queues a request with the given fields on the player.
    fn queue_request(
        e: &mut Engine,
        this: Ptr<PlayerCharacter>,
        fill: impl FnOnce(&mut Engine, Ptr<PositionPlayerRequest>),
    ) -> Ptr<PositionPlayerRequest> {
        let request = e.new_object::<PositionPlayerRequest>();
        fill(e, request);
        e.set(this, PlayerCharacter::pQueuedTargetLoc, request.cast());
        request
    }

    #[test]
    fn handling_an_empty_queue_does_nothing() {
        let mut e = engine();
        let (world, deleted) = handler_world(&mut e);
        assert!(!e.call(0x0093_bea0, &args![world.this]).bool());
        assert_eq!(call_addresses(&e), vec![0x0093_bea0]);
        assert!(deleted.borrow().is_empty());
    }

    #[test]
    fn handling_an_invalid_request_reports_it_and_frees_it() {
        let mut e = engine();
        let (world, deleted) = handler_world(&mut e);
        let this = world.this;
        let request = queue_request(&mut e, this, |_, _| {});
        let moved = e.call(0x0093_bea0, &args![this]).bool();
        assert!(!moved);
        assert_eq!(called(&e, 0x005b_5e40), vec![vec![0x0108_afa0]]);
        assert_eq!(called(&e, 0x0045_2eb0), vec![vec![0x5000]]);
        assert_eq!(
            called(&e, NI_POINTER_ASSIGN),
            vec![vec![this.addr() + 0xd3c, 0]]
        );
        assert_eq!(called(&e, 0x0086_8d70), vec![vec![1]]);
        assert_eq!(*deleted.borrow(), vec![request.addr()]);
        assert!(e.get(this, PlayerCharacter::pQueuedTargetLoc).is_null());
        // Nothing moved: no follow-up on the player.
        assert!(called(&e, 0x005d_14d0).is_empty());
        assert!(called(&e, 0x0057_3f20).is_empty());
    }

    #[test]
    fn handling_a_fast_travel_request_travels_and_follows_up() {
        let mut e = engine();
        let (world, deleted) = handler_world(&mut e);
        let this = world.this;
        e.register(0x0056_9b80, |_, _| int(0));
        let request = queue_request(&mut e, this, |e, r| {
            e.set(
                r,
                PositionPlayerRequest::fast_travel_target,
                Ptr::new(0x7000),
            );
            // Everything else is ignored by a fast travel.
            e.set(r, PositionPlayerRequest::furniture, Ptr::new(0x8800));
        });
        assert!(e.call(0x0093_bea0, &args![this]).bool());
        assert_eq!(called(&e, 0x0056_9b80), vec![vec![0x7000]]);
        // The travel ran (and stopped at once): the entry point was asked.
        let entry = &called(&e, 0x005e_58f0)[0];
        assert_eq!((entry[0], entry[1]), (0x33, this.addr()));
        assert!(called(&e, 0x0056_8680).is_empty());
        assert_eq!(*deleted.borrow(), vec![request.addr()]);
        // The follow-up on the player.
        assert_eq!(called(&e, 0x005d_14d0), vec![vec![this.addr(), 1]]);
    }

    #[test]
    fn the_follow_up_is_skipped_when_forbidden() {
        let mut e = engine();
        let (world, _) = handler_world(&mut e);
        let this = world.this;
        e.register(0x0042_ce10, |_, _| int(1));
        queue_request(&mut e, this, |e, r| {
            e.set(
                r,
                PositionPlayerRequest::fast_travel_target,
                Ptr::new(0x7000),
            );
        });
        assert!(e.call(0x0093_bea0, &args![this]).bool());
        assert!(called(&e, 0x005d_14d0).is_empty());
        // Or by the fast-travel flag.
        let mut e = engine();
        let (world, _) = handler_world(&mut e);
        let this = world.this;
        e.mem.set_u8(this.addr() + 0x66d, 2);
        queue_request(&mut e, this, |e, r| {
            e.set(
                r,
                PositionPlayerRequest::fast_travel_target,
                Ptr::new(0x7000),
            );
        });
        assert!(e.call(0x0093_bea0, &args![this]).bool());
        assert!(called(&e, 0x005d_14d0).is_empty());
    }

    #[test]
    fn handling_a_cell_request_positions_the_player_and_runs_the_callback() {
        let mut e = engine();
        let (world, _) = handler_world(&mut e);
        let this = world.this;
        e.register(0x0093_06d0, |_, _| int(0x4400));
        e.register_double(0x00b0_0cb0, |_, _| Ret::default());
        queue_request(&mut e, this, |e, r| {
            e.set(r, PositionPlayerRequest::cell, Ptr::new(0x6100));
            for (i, v) in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0].into_iter().enumerate() {
                e.mem.set_f32(r.addr() + 8 + 4 * i as u32, v);
            }
            e.set(r, PositionPlayerRequest::callback, 0x00b0_0cb0);
            e.set(r, PositionPlayerRequest::callback_argument, 0x1234);
        });
        let placed_at = watch_floats(&mut e, 0x00b0_02a8, 1, 3);
        let turned = watch_floats(&mut e, 0x0086_d490, 1, 3);
        assert!(e.call(0x0093_bea0, &args![this]).bool());
        // PositionPlayer ran with the request's position and rotation.
        assert_eq!(called(&e, 0x0057_5bb0)[0], vec![this.addr(), 0x6100]);
        assert_eq!(placed_at.borrow()[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(turned.borrow()[0], vec![4.0, 5.0, 6.0]);
        // The controller gets the request's height (z), then the callback.
        let height = &called(&e, 0x0057_3f20)[0];
        assert_eq!((height[0], f(height, 1)), (0x4400, 3.0));
        assert_eq!(called(&e, 0x00b0_0cb0), vec![vec![0x1234]]);
    }

    #[test]
    fn handling_a_world_space_request_positions_the_player_in_that_world() {
        let mut e = engine();
        let (world, _) = handler_world(&mut e);
        let this = world.this;
        e.register(0x0058_75a0, |_, _| int(0x6200));
        e.register(0x0045_11e0, |_, _| int(1));
        queue_request(&mut e, this, |e, r| {
            e.set(r, PositionPlayerRequest::worldspace, Ptr::new(0x7200));
            // A cell is ignored when there is a world space.
            e.set(r, PositionPlayerRequest::cell, Ptr::new(0x6100));
            e.mem.set_f32(r.addr() + 8, 5000.0);
            e.mem.set_f32(r.addr() + 12, 9000.0);
            e.set(r, PositionPlayerRequest::reset_weather, 1);
        });
        e.register(0x0046_dd00, |_, _| int(0xaa00));
        assert!(e.call(0x0093_bea0, &args![this]).bool());
        assert_eq!(called(&e, 0x0058_75a0), vec![vec![0x7200, 1, 2]]);
        assert_eq!(called(&e, 0x0057_5bb0)[0], vec![this.addr(), 0x6200]);
        // The flag byte reset the weather.
        assert_eq!(called(&e, 0x0063_d060), vec![vec![0xaa00]]);
    }

    #[test]
    fn handling_a_request_with_furniture_puts_the_player_in_it() {
        let mut e = engine();
        let (world, _) = handler_world(&mut e);
        let this = world.this;
        e.register(0x0056_8680, |_, _| int(1));
        e.register(0x008d_8520, |_, _| int(0x4500));
        let process = e.mem.alloc(16);
        e.register_double(0x00b0_04d4, |_, _| int(0x77));
        put_vtable(&mut e, process, 0x0250_0000, &[(0x4d4, 0x00b0_04d4)]);
        e.register_double(0x008d_8520, move |_, _| int(process));
        e.register(0x007a_f430, |_, _| int(0x4600));
        e.register(0x0050_9420, |_, _| int(1));
        e.register(0x0056_82c0, |_, _| int(3));
        e.register(0x0056_8500, |_, _| int(1));
        queue_request(&mut e, this, |e, r| {
            e.set(r, PositionPlayerRequest::furniture, Ptr::new(0x8800));
        });
        e.call(0x0093_bea0, &args![this]);
        assert_eq!(called(&e, 0x0056_8680), vec![vec![0x8800]]);
        assert_eq!(called(&e, 0x0056_82c0), vec![vec![0x8800, 1]]);
        assert_eq!(called(&e, 0x0056_8500), vec![vec![0x8800, 3, 0x77]]);
        assert_eq!(
            called(&e, 0x0088_d2f0),
            vec![vec![this.addr(), 0x8800, 0x77, 3, 1]]
        );
    }

    #[test]
    fn furniture_without_a_free_marker_is_left_alone() {
        let mut e = engine();
        let (world, _) = handler_world(&mut e);
        let this = world.this;
        e.register(0x0056_8680, |_, _| int(1));
        let process = e.mem.alloc(16);
        e.register_double(0x00b0_04d4, |_, _| int(0x77));
        put_vtable(&mut e, process, 0x0250_0000, &[(0x4d4, 0x00b0_04d4)]);
        e.register_double(0x008d_8520, move |_, _| int(process));
        e.register(0x0056_82c0, |_, _| int(-1i32 as u32));
        queue_request(&mut e, this, |e, r| {
            e.set(r, PositionPlayerRequest::furniture, Ptr::new(0x8800));
        });
        e.call(0x0093_bea0, &args![this]);
        assert!(called(&e, 0x0056_8500).is_empty());
        assert!(called(&e, 0x0088_d2f0).is_empty());
        // A marker the furniture does not accept is not used either.
        let mut e = engine();
        let (world, _) = handler_world(&mut e);
        let this = world.this;
        e.register(0x0056_8680, |_, _| int(1));
        let process = e.mem.alloc(16);
        e.register_double(0x00b0_04d4, |_, _| int(0x77));
        put_vtable(&mut e, process, 0x0250_0000, &[(0x4d4, 0x00b0_04d4)]);
        e.register_double(0x008d_8520, move |_, _| int(process));
        e.register(0x0056_82c0, |_, _| int(2));
        e.register(0x0056_8500, |_, _| int(0));
        queue_request(&mut e, this, |e, r| {
            e.set(r, PositionPlayerRequest::furniture, Ptr::new(0x8800));
        });
        e.call(0x0093_bea0, &args![this]);
        assert_eq!(called(&e, 0x0056_8500).len(), 1);
        assert!(called(&e, 0x0088_d2f0).is_empty());
    }

    #[test]
    fn a_furniture_reference_that_is_not_furniture_is_ignored() {
        let mut e = engine();
        let (world, _) = handler_world(&mut e);
        let this = world.this;
        queue_request(&mut e, this, |e, r| {
            e.set(r, PositionPlayerRequest::furniture, Ptr::new(0x8800));
        });
        e.call(0x0093_bea0, &args![this]);
        assert_eq!(called(&e, 0x0056_8680), vec![vec![0x8800]]);
        assert!(called(&e, 0x008d_8520).is_empty());
    }

    /// The world of `RequestPositionPlayer`: the grid can take requests
    /// when `ready`; deletions are recorded.
    fn request_world(e: &mut Engine, ready: bool) -> (PositionWorld, Rc<RefCell<Vec<u32>>>) {
        let (world, deleted) = handler_world(e);
        e.register_double(0x0045_1530, move |_, _| int(ready as u32));
        (world, deleted)
    }

    #[test]
    fn requesting_a_position_stores_the_request() {
        let mut e = engine();
        let (world, deleted) = request_world(&mut e, false);
        let this = world.this;
        let request = e.new_object::<PositionPlayerRequest>();
        e.call(0x0093_be30, &args![this, request]);
        assert_eq!(
            e.get(this, PlayerCharacter::pQueuedTargetLoc).addr(),
            request.addr()
        );
        assert!(deleted.borrow().is_empty());
        assert!(called(&e, 0x005b_5e40).is_empty());
        assert_eq!(called(&e, 0x0045_1530), vec![vec![0x5000]]);
        assert!(called(&e, 0x0045_2eb0).is_empty());
    }

    #[test]
    fn requesting_a_position_replaces_an_earlier_request_with_a_warning() {
        let mut e = engine();
        let (world, deleted) = request_world(&mut e, false);
        let this = world.this;
        let earlier = queue_request(&mut e, this, |_, _| {});
        let request = e.new_object::<PositionPlayerRequest>();
        e.call(0x0093_be30, &args![this, request]);
        assert_eq!(called(&e, 0x005b_5e40), vec![vec![0x0108_af58]]);
        assert_eq!(*deleted.borrow(), vec![earlier.addr()]);
        assert_eq!(
            e.get(this, PlayerCharacter::pQueuedTargetLoc).addr(),
            request.addr()
        );
    }

    #[test]
    fn cancelling_a_request_frees_it_silently() {
        let mut e = engine();
        let (world, deleted) = request_world(&mut e, false);
        let this = world.this;
        let earlier = queue_request(&mut e, this, |_, _| {});
        e.call(0x0093_be30, &args![this, 0u32]);
        assert!(called(&e, 0x005b_5e40).is_empty());
        assert_eq!(*deleted.borrow(), vec![earlier.addr()]);
        assert!(e.get(this, PlayerCharacter::pQueuedTargetLoc).is_null());
    }

    #[test]
    fn a_ready_grid_handles_the_request_at_once() {
        let mut e = engine();
        let (world, deleted) = request_world(&mut e, true);
        let this = world.this;
        let request = e.new_object::<PositionPlayerRequest>();
        e.call(0x0093_be30, &args![this, request]);
        // The (invalid) request was handled and freed.
        assert_eq!(called(&e, 0x0045_2eb0).len(), 1);
        assert_eq!(*deleted.borrow(), vec![request.addr()]);
        assert!(e.get(this, PlayerCharacter::pQueuedTargetLoc).is_null());
    }

    /// The exterior-placement world: the position world, with the callees
    /// of the fast travel and of the cell clearing stubbed.
    fn exterior_world(e: &mut Engine) -> PositionWorld {
        let world = position_world(e);
        stub(e, FAST_TRAVEL_CALLEES);
        stub(e, EXTERIOR_PLACEMENT_CALLEES);
        e.register(0x0040_6d90, |_, a| int(f32::from_bits(a[0]) as i32 as u32));
        world
    }

    #[test]
    fn exterior_placement_without_a_world_space_does_nothing() {
        let mut e = engine();
        let world = exterior_world(&mut e);
        e.call(
            0x0093_cce0,
            &args![world.this, 1.0f32, 2.0f32, 3.0f32, 4.0f32, 5.0f32, 6.0f32, 0u32, true],
        );
        assert_eq!(call_addresses(&e), vec![0x0093_cce0]);
    }

    #[test]
    fn exterior_placement_cancels_loads_and_uses_the_cell_found() {
        let mut e = engine();
        let world = exterior_world(&mut e);
        e.set_global(EXTERIOR_CELL_LOADER, 0x5900u32);
        e.register(0x0058_75a0, |_, _| int(0x6200));
        e.register(0x0045_11e0, |_, _| int(1));
        e.call(
            0x0093_cce0,
            &args![
                world.this, 5000.0f32, 9000.0f32, 3.0f32, 4.0f32, 5.0f32, 6.0f32, 0x7200u32, false
            ],
        );
        assert_eq!(called(&e, 0x0052_8540), vec![vec![0x5900]]);
        // 5000 >> 12 = 1 and 9000 >> 12 = 2.
        assert_eq!(called(&e, 0x0058_75a0), vec![vec![0x7200, 1, 2]]);
        // The cell is already loaded: nothing is cleared, nothing created.
        assert_eq!(called(&e, 0x0045_11e0), vec![vec![0x5000, 0x6200, 0]]);
        assert!(called(&e, 0x0058_5b30).is_empty());
        assert!(called(&e, 0x0046_1330).is_empty());
        // The player singleton is placed in it.
        assert_eq!(called(&e, 0x0057_5bb0)[0], vec![world.this.addr(), 0x6200]);
    }

    #[test]
    fn exterior_placement_loads_and_then_creates_the_cell() {
        let mut e = engine();
        let world = exterior_world(&mut e);
        e.set_global(DATA_HANDLER, 0x5800u32);
        e.register(0x0058_75a0, |_, _| int(0));
        e.register(0x0058_5b30, |_, _| int(0));
        e.register(0x0046_1330, |_, _| int(0x6300));
        e.call(
            0x0093_cce0,
            &args![
                world.this, 5000.0f32, 9000.0f32, 3.0f32, 4.0f32, 5.0f32, 6.0f32, 0x7200u32, false
            ],
        );
        // Nothing found: the cells are cleared for the world space, the cell
        // is loaded, then created.
        assert_eq!(called(&e, 0x0045_7d70)[0], vec![0x5000, 1, 0x7200, 0]);
        assert_eq!(called(&e, 0x0058_5b30), vec![vec![0x7200, 1, 2]]);
        assert_eq!(called(&e, 0x0046_1330), vec![vec![0x5800, 0, 1, 2, 0x7200]]);
        assert_eq!(called(&e, 0x0057_5bb0)[0], vec![world.this.addr(), 0x6300]);
    }

    #[test]
    fn exterior_placement_gives_up_when_no_cell_can_be_made() {
        let mut e = engine();
        let world = exterior_world(&mut e);
        e.call(
            0x0093_cce0,
            &args![
                world.this, 5000.0f32, 9000.0f32, 3.0f32, 4.0f32, 5.0f32, 6.0f32, 0x7200u32, false
            ],
        );
        assert!(called(&e, 0x0057_5bb0).is_empty());
    }

    #[test]
    fn small_flag_getters() {
        let mut e = engine();
        let this = player(&mut e);
        for (byte, can_travel, travel_flag) in [
            (0u8, false, false),
            (1, false, true),
            (2, true, false),
            (3, true, true),
        ] {
            e.mem.set_u8(this.addr() + 0x66d, byte);
            assert_eq!(e.call(0x0093_c1e0, &args![this]).bool(), can_travel);
            assert_eq!(e.call(0x0093_db40, &args![this]).bool(), travel_flag);
        }
    }

    #[test]
    fn fn_0093ccd0_reads_its_global() {
        let mut e = engine();
        e.map(0x011c_3000, 0x1000);
        e.set_global(GLOBAL_RETURNED_BY_0093CCD0, 0x1234u32);
        assert_eq!(e.call(0x0093_ccd0, &args![]).u32(), 0x1234);
    }

    #[test]
    fn small_field_accessors() {
        let mut e = engine();
        let this = player(&mut e);
        e.call(0x0094_42e0, &args![this, 2.5f32]);
        assert_eq!(
            e.get(this, PlayerCharacter::fTimeSinceLastAmmoRegenTick),
            2.5
        );
        // The generic getters read their offsets.
        e.mem.set_f32(this.addr() + 0x10c, 7.5);
        assert_eq!(e.call(0x0094_42a0, &args![this]).f32(), 7.5);
        e.mem.set_u32(this.addr() + 0x12c, 0x1000);
        assert!(e.call(0x0094_42c0, &args![this]).bool());
        e.mem.set_u32(this.addr() + 0x12c, 0xefff);
        assert!(!e.call(0x0094_42c0, &args![this]).bool());
        e.mem.set_u32(this.addr() + 0x634, 0x9900);
        assert_eq!(e.call(0x0093_e4f0, &args![this]).u32(), 0x9900);
    }

    #[test]
    fn fn_00944270_reads_a_byte_of_the_global_object() {
        let mut e = engine();
        e.map(0x011d_8000, 0x1000);
        assert_eq!(e.call(0x0094_4270, &args![]).u8(), 0);
        let object = e.mem.alloc(0x500);
        e.mem.set_u8(object + 0x4cd, 9);
        e.set_global(0x011d_8a80, object);
        assert_eq!(e.call(0x0094_4270, &args![]).u8(), 9);
    }

    #[test]
    fn fn_0093e510_calls_through_the_member_at_0x1c() {
        let mut e = engine();
        stub(&mut e, &[0x0052_72b0]);
        e.call(0x0093_e510, &args![0x3000u32]);
        assert_eq!(called(&e, 0x0052_72b0), vec![vec![0x301c]]);
    }

    #[test]
    fn fn_0093dd20_passes_the_masked_flags_to_the_actor_mover() {
        let mut e = engine();
        let this = player(&mut e);
        let mover = e.mem.alloc(16);
        e.register_double(0x00b0_000c, |_, _| Ret::default());
        put_vtable(&mut e, mover, 0x0260_0000, &[(0xc, 0x00b0_000c)]);
        e.mem.set_u32(this.addr() + 0x190, mover);
        e.register(0x0088_46e0, |_, _| int(0xffff_ffff));
        e.call(0x0093_dd20, &args![this]);
        assert_eq!(called(&e, 0x00b0_000c), vec![vec![mover, 0xcc00 | 0x400]]);
        // Only the low word of the result counts, and the mask applies.
        e.register(0x0088_46e0, |_, _| int(0x1_0011));
        e.call(0x0093_dd20, &args![this]);
        assert_eq!(called(&e, 0x00b0_000c)[1], vec![mover, 0x400]);
    }

    /// The perceived-actors list: nodes of {item, next}, items {actor,
    /// hostile byte}. `LIST_ITEM_ADDRESS` returns the node itself.
    fn perceived_world(e: &mut Engine, entries: &[(u32, u8)]) -> Ptr<PlayerCharacter> {
        let this = player(e);
        let mut next = 0u32;
        for (actor, hostile) in entries.iter().rev() {
            let entry = e.mem.alloc(8);
            e.mem.set_u32(entry, *actor);
            e.mem.set_u8(entry + 4, *hostile);
            let node = e.mem.alloc(8);
            e.mem.set_u32(node, entry);
            e.mem.set_u32(node + 4, next);
            next = node;
        }
        e.set(
            this,
            PlayerCharacter::pListofPercievedActors,
            Ptr::new(next),
        );
        e.register(LIST_ITEM_ADDRESS, |_, a| int(a[0]));
        e.register(LIST_NEXT, |e, a| int(e.mem.u32(a[0] + 4)));
        this
    }

    #[test]
    fn the_hostile_flag_of_a_perceived_actor_is_found_by_actor() {
        let mut e = engine();
        let this = perceived_world(&mut e, &[(0x11, 0), (0x22, 1), (0x33, 0)]);
        assert!(!e.call(0x0093_dee0, &args![this, 0x11u32]).bool());
        assert!(e.call(0x0093_dee0, &args![this, 0x22u32]).bool());
        assert!(!e.call(0x0093_dee0, &args![this, 0x33u32]).bool());
        assert!(!e.call(0x0093_dee0, &args![this, 0x44u32]).bool());
    }

    #[test]
    fn nobody_is_hostile_when_nobody_is_perceived() {
        let mut e = engine();
        let this = perceived_world(&mut e, &[]);
        assert!(!e.call(0x0093_dee0, &args![this, 0x11u32]).bool());
    }

    #[test]
    fn hostile_perceived_actors_get_temp_effects() {
        let mut e = engine();
        let this = perceived_world(&mut e, &[(0x11, 0), (0x22, 1), (0x33, 1)]);
        e.map(0x0101_0000, 0x2_0000);
        e.map(0x011a_0000, 0x6_0000);
        e.set_global(DATA_HANDLER, 0x5800u32);
        e.set_global(TIMER_RESET, -1.0f32);
        e.register(0x007b_8f10, |_, _| int(0x9a00));
        let blocks = Rc::new(RefCell::new(vec![0x6001u32, 0x6002]));
        let queue = blocks.clone();
        e.register_double(0x00aa_13e0, move |_, _| int(queue.borrow_mut().remove(0)));
        // The first effect is accepted, the second refused (and deleted).
        let accepted = e.mem.alloc(16);
        let refused = e.mem.alloc(16);
        e.register_double(0x00b0_00c4, move |_, a| int((a[0] == accepted) as u32));
        e.register_double(0x00b0_0000, |_, _| Ret::default());
        put_vtable(&mut e, accepted, 0x0270_0000, &[(0xc4, 0x00b0_00c4)]);
        put_vtable(
            &mut e,
            refused,
            0x0271_0000,
            &[(0xc4, 0x00b0_00c4), (0, 0x00b0_0000)],
        );
        let effects = Rc::new(RefCell::new(vec![accepted, refused]));
        let queue = effects.clone();
        e.register_double(0x0081_f580, move |_, _| int(queue.borrow_mut().remove(0)));
        stub(&mut e, &[0x0097_3fd0]);
        e.call(0x0093_dd80, &args![this]);
        assert_eq!(called(&e, 0x007b_8f10), vec![vec![0x5800]]);
        let creations = called(&e, 0x0081_f580);
        assert_eq!(creations.len(), 2);
        assert_eq!(creations[0][..3], [0x6001, 0x22, 0x9a00]);
        assert_eq!(f(&creations[0], 3), -1.0);
        assert_eq!(creations[1][1], 0x33);
        assert_eq!(called(&e, 0x0097_3fd0), vec![vec![PROCESS_LISTS, accepted]]);
        assert_eq!(called(&e, 0x00b0_0000), vec![vec![refused, 1]]);
    }

    #[test]
    fn temp_effects_need_the_data_handler_object() {
        let mut e = engine();
        let this = perceived_world(&mut e, &[(0x22, 1)]);
        e.map(0x011a_0000, 0x6_0000);
        e.register(0x007b_8f10, |_, _| int(0));
        e.call(0x0093_dd80, &args![this]);
        assert!(called(&e, 0x00aa_13e0).is_empty());
    }

    /// The messages the fast-travel check showed: (setting object, icon).
    fn shown_messages(e: &Engine) -> Vec<(u32, u32)> {
        let settings: Vec<u32> = called(e, 0x0040_3df0).iter().map(|w| w[0]).collect();
        let icons: Vec<u32> = called(e, 0x0070_52f0).iter().map(|w| w[2]).collect();
        settings.into_iter().zip(icons).collect()
    }

    /// The world of the fast-travel check where everything is allowed:
    /// the player singleton is `this` (travel flag set, health and the
    /// other value 5, no lists, no active effects), the cell test allows.
    fn travel_check_world(e: &mut Engine) -> Ptr<PlayerCharacter> {
        let this = player(e);
        e.map(0x0101_0000, 0x2_0000);
        e.map(0x011a_0000, 0x6_0000);
        e.set_global(PLAYER_SINGLETON, this.addr());
        e.set_global(0x0101_2070, 1.0f64);
        e.set_global(TWO_FLOAT, 2.0f32);
        e.mem.set_u8(this.addr() + 0x66d, 1);
        stub(
            e,
            &[
                0x0057_5d10,
                0x0097_64a0,
                0x0097_1c30,
                0x0082_56d0,
                0x0040_3df0,
                0x0040_6d00,
                0x0070_52f0,
                0x008d_8520,
                0x005e_58f0,
                0x0088_49c0,
                0x008d_6f30,
                0x0054_4520,
                0x0058_6210,
                LIST_CLEAR,
                LIST_DELETING_DESTRUCT,
            ],
        );
        e.register(0x0057_5d70, |_, _| int(0x7300));
        // The player: no entry-point check (virtual +0x358 false); the
        // owner's virtual +0x8 gives 5; the magic target's +0x8 gives no effects.
        e.register_double(0x00b0_0358, |_, _| int(0));
        e.register_double(0x00b0_0008, |_, _| int(5));
        e.register_double(0x00b0_1008, |_, _| int(0));
        put_vtable(e, this.addr(), 0x0200_0000, &[(0x358, 0x00b0_0358)]);
        put_vtable(e, this.addr() + 0xa4, 0x0201_0000, &[(8, 0x00b0_0008)]);
        put_vtable(e, this.addr() + 0x94, 0x0202_0000, &[(8, 0x00b0_1008)]);
        this
    }

    #[test]
    fn fast_travel_is_allowed_when_nothing_forbids_it() {
        let mut e = engine();
        let this = travel_check_world(&mut e);
        assert!(e.call(0x0093_d660, &args![this]).bool());
        assert!(shown_messages(&e).is_empty());
    }

    #[test]
    fn fast_travel_is_refused_where_the_process_lists_say_so() {
        let mut e = engine();
        let this = travel_check_world(&mut e);
        e.register(0x0057_5d10, |_, _| int(1));
        e.register(0x0097_64a0, |_, a| int(a[1]));
        assert!(!e.call(0x0093_d660, &args![this]).bool());
        assert_eq!(shown_messages(&e), vec![(0x011d_3dd8, FAST_TRAVEL_ICON)]);
        // The interior flag was passed on and the text was formatted.
        assert_eq!(called(&e, 0x0097_64a0), vec![vec![PROCESS_LISTS, 1]]);
        let text = &called(&e, 0x0040_6d00)[0];
        assert_eq!(text[1], 0x1f4);
        let message = &called(&e, 0x0070_52f0)[0];
        assert_eq!((message[0], message[1], message[3]), (text[0], 0, 0));
        assert_eq!(f(message, 4), 2.0);
        assert_eq!(message[5], 0);
        assert!(called(&e, 0x0097_1c30).is_empty());
    }

    #[test]
    fn fast_travel_is_refused_while_a_list_is_pending_and_clears_it() {
        let mut e = engine();
        let this = travel_check_world(&mut e);
        e.register(0x0097_1c30, |_, _| int(0x5200));
        assert!(!e.call(0x0093_d660, &args![this]).bool());
        assert_eq!(shown_messages(&e), vec![(0x011d_4114, FAST_TRAVEL_ICON)]);
        assert_eq!(
            called(&e, 0x0097_1c30),
            vec![vec![PROCESS_LISTS, this.addr(), 0x15, 0]]
        );
        assert_eq!(called(&e, LIST_CLEAR), vec![vec![0x5200]]);
        assert_eq!(called(&e, LIST_DELETING_DESTRUCT), vec![vec![0x5200, 1]]);
    }

    #[test]
    fn an_empty_pending_list_does_not_forbid_fast_travel() {
        let mut e = engine();
        let this = travel_check_world(&mut e);
        e.register(0x0097_1c30, |_, _| int(0x5200));
        e.register(0x0082_56d0, |_, _| int(1));
        // With the list at its end the check goes on.
        assert!(e.call(0x0093_d660, &args![this]).bool());
        assert!(shown_messages(&e).is_empty());
    }

    /// A magic target whose effect list is a single node `0x5100` whose
    /// item is an object whose virtual +0x3C returns `forbids`.
    fn effect_world(e: &mut Engine, forbids: bool) -> Ptr<PlayerCharacter> {
        let this = travel_check_world(e);
        let node = e.mem.alloc(8);
        let effect = e.mem.alloc(8);
        e.mem.set_u32(node, effect);
        e.register_double(0x00b0_1008, move |_, _| int(node));
        e.register(0x008d_8520, |_, _| int(1));
        e.register_double(LIST_ITEM_ADDRESS, move |_, a| int(a[0]));
        e.register(LIST_NEXT, |_, _| int(0));
        e.register_double(0x00b0_003c, move |_, _| int(forbids as u32));
        put_vtable(e, effect, 0x0203_0000, &[(0x3c, 0x00b0_003c)]);
        this
    }

    #[test]
    fn fast_travel_is_refused_when_an_active_effect_forbids_it() {
        let mut e = engine();
        let this = effect_world(&mut e, true);
        assert!(!e.call(0x0093_d660, &args![this]).bool());
        assert_eq!(shown_messages(&e), vec![(0x011d_255c, FAST_TRAVEL_ICON)]);
    }

    #[test]
    fn fast_travel_ignores_effects_that_do_not_forbid_it() {
        let mut e = engine();
        let this = effect_world(&mut e, false);
        assert!(e.call(0x0093_d660, &args![this]).bool());
        assert_eq!(called(&e, 0x00b0_003c).len(), 1);
        assert!(shown_messages(&e).is_empty());
    }

    #[test]
    fn fast_travel_effects_are_only_checked_with_the_acquire_object() {
        let mut e = engine();
        let this = effect_world(&mut e, true);
        e.register(0x008d_8520, |_, _| int(0));
        assert!(e.call(0x0093_d660, &args![this]).bool());
        assert!(called(&e, 0x00b0_003c).is_empty());
    }

    #[test]
    fn fast_travel_needs_the_travel_flag() {
        let mut e = engine();
        let this = travel_check_world(&mut e);
        e.mem.set_u8(this.addr() + 0x66d, 0);
        assert!(!e.call(0x0093_d660, &args![this]).bool());
        assert_eq!(shown_messages(&e), vec![(0x011d_3630, FAST_TRAVEL_ICON)]);
    }

    #[test]
    fn fast_travel_waits_for_the_entry_point_delay() {
        let mut e = engine();
        let this = travel_check_world(&mut e);
        e.register_double(0x00b0_0358, |_, _| int(1));
        e.register(0x005e_58f0, |e, a| {
            e.mem.set_f32(a[2], 0.5);
            Ret::default()
        });
        assert!(!e.call(0x0093_d660, &args![this]).bool());
        // The message has no icon.
        assert_eq!(shown_messages(&e), vec![(0x011d_50c8, 0)]);
        let entry = &called(&e, 0x005e_58f0)[0];
        assert_eq!((entry[0], entry[1]), (0x33, this.addr()));
        // A delay of at least one allows it.
        let mut e = engine();
        let this = travel_check_world(&mut e);
        e.register_double(0x00b0_0358, |_, _| int(1));
        e.register(0x005e_58f0, |e, a| {
            e.mem.set_f32(a[2], 1.0);
            Ret::default()
        });
        assert!(e.call(0x0093_d660, &args![this]).bool());
    }

    #[test]
    fn fast_travel_needs_both_actor_values() {
        for (first, second) in [(0, 5), (5, 0)] {
            let mut e = engine();
            let this = travel_check_world(&mut e);
            e.register_double(0x00b0_0008, move |_, a| {
                int(if a[1] == 0x16 { first } else { second })
            });
            assert!(!e.call(0x0093_d660, &args![this]).bool());
            assert_eq!(shown_messages(&e), vec![(0x011d_255c, FAST_TRAVEL_ICON)]);
        }
    }

    #[test]
    fn fast_travel_is_refused_when_the_player_is_busy() {
        let mut e = engine();
        let this = travel_check_world(&mut e);
        e.register(0x0088_49c0, |_, _| int(1));
        assert!(!e.call(0x0093_d660, &args![this]).bool());
        assert_eq!(shown_messages(&e), vec![(0x011d_4120, FAST_TRAVEL_ICON)]);
    }

    #[test]
    fn fast_travel_follows_the_parent_cell_test() {
        // With a parent cell: allowed unless `00544520` says no.
        for (refuses, allowed) in [(0, true), (1, false)] {
            let mut e = engine();
            let this = travel_check_world(&mut e);
            e.register(0x008d_6f30, |_, _| int(0x3300));
            e.register_double(0x0054_4520, move |_, _| int(refuses));
            assert_eq!(e.call(0x0093_d660, &args![this]).bool(), allowed);
            assert_eq!(called(&e, 0x0054_4520), vec![vec![0x3300]]);
            assert_eq!(shown_messages(&e).len(), !allowed as usize);
            if !allowed {
                assert_eq!(shown_messages(&e), vec![(0x011d_4738, FAST_TRAVEL_ICON)]);
            }
        }
    }

    #[test]
    fn fast_travel_follows_the_world_space_test_without_a_parent_cell() {
        for (refuses, allowed) in [(0, true), (1, false)] {
            let mut e = engine();
            let this = travel_check_world(&mut e);
            e.register_double(0x0058_6210, move |_, _| int(refuses));
            assert_eq!(e.call(0x0093_d660, &args![this]).bool(), allowed);
            assert_eq!(called(&e, 0x0058_6210), vec![vec![0x7300]]);
        }
        // Without a world space either, it is refused.
        let mut e = engine();
        let this = travel_check_world(&mut e);
        e.register(0x0057_5d70, |_, _| int(0));
        assert!(!e.call(0x0093_d660, &args![this]).bool());
        assert_eq!(shown_messages(&e), vec![(0x011d_4738, FAST_TRAVEL_ICON)]);
    }

    /// The world of `CenterOnCell`: the exterior world (PositionPlayer and
    /// the cell clearing run for real) with the lookups stubbed.
    fn center_world(e: &mut Engine) -> PositionWorld {
        let world = exterior_world(e);
        stub(
            e,
            &[
                0x0040_fbe0,
                0x0046_1ae0,
                0x0046_1cf0,
                0x0054_4c30,
                0x0054_4c60,
                0x0054_cfd0,
                0x0045_72e0,
            ],
        );
        world
    }

    /// `0054cfd0` fills the position (1, 2, 3) and the rotation (4, 5, 6).
    fn register_placement(e: &mut Engine) {
        e.register(0x0054_cfd0, |e, a| {
            for (i, v) in [1.0f32, 2.0, 3.0].into_iter().enumerate() {
                e.mem.set_f32(a[1] + 4 * i as u32, v);
            }
            for (i, v) in [4.0f32, 5.0, 6.0].into_iter().enumerate() {
                e.mem.set_f32(a[2] + 4 * i as u32, v);
            }
            Ret::default()
        });
    }

    #[test]
    fn centering_on_an_interior_cell_places_the_player() {
        let mut e = engine();
        let world = center_world(&mut e);
        register_placement(&mut e);
        e.register(0x0042_5fd0, |_, _| int(1));
        let placed_at = watch_floats(&mut e, 0x00b0_02a8, 1, 3);
        let turned = watch_floats(&mut e, 0x0086_d490, 1, 3);
        e.call(0x0093_db60, &args![world.this, 0u32, 0x6100u32]);
        assert_eq!(called(&e, 0x0040_fbe0), vec![vec![world.this.addr()]]);
        assert!(called(&e, 0x0046_1ae0).is_empty());
        let placement = &called(&e, 0x0054_cfd0)[0];
        assert_eq!(placement[0], 0x6100);
        assert_eq!(called(&e, 0x0057_5bb0)[0], vec![world.this.addr(), 0x6100]);
        assert_eq!(placed_at.borrow()[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(turned.borrow()[0], vec![4.0, 5.0, 6.0]);
        // The weather is reset by the placement.
        assert_eq!(called(&e, 0x0046_dd00).len(), 1);
        // An interior cell has no land height to adjust to.
        assert!(called(&e, 0x0045_72e0).is_empty());
    }

    #[test]
    fn centering_on_a_cell_by_editor_id_looks_it_up() {
        let mut e = engine();
        let world = center_world(&mut e);
        register_placement(&mut e);
        e.set_global(DATA_HANDLER, 0x5800u32);
        e.register(0x0042_5fd0, |_, _| int(1));
        e.register(0x0046_1ae0, |_, _| int(0x6100));
        e.call(0x0093_db60, &args![world.this, 0x9990u32, 0u32]);
        assert_eq!(called(&e, 0x0046_1ae0), vec![vec![0x5800, 0x9990]]);
        assert_eq!(called(&e, 0x0057_5bb0)[0], vec![world.this.addr(), 0x6100]);
    }

    #[test]
    fn centering_on_an_exterior_cell_loads_it_and_sets_the_land_height() {
        for (height, expected) in [(12.5f32, 12.5f32), (-3.0, 0.0)] {
            let mut e = engine();
            let world = center_world(&mut e);
            register_placement(&mut e);
            e.set_global(EXTERIOR_CELL_LOADER, 0x5900u32);
            e.register(0x0054_ddd0, |_, _| int(0x7200));
            e.register(0x0054_4c30, |_, _| int(7));
            e.register(0x0054_4c60, |_, _| int(9));
            e.register(0x0045_11e0, |_, _| int(1));
            e.register_double(0x0045_72e0, move |e, a| {
                e.mem.set_f32(a[2], height);
                Ret::default()
            });
            let placed_at = watch_floats(&mut e, 0x00b0_02a8, 1, 3);
            e.call(0x0093_db60, &args![world.this, 0u32, 0x6100u32]);
            assert_eq!(called(&e, 0x0052_8540), vec![vec![0x5900]]);
            // The cell's world space is loaded at the cell's coordinates.
            assert_eq!(called(&e, 0x0058_5b30), vec![vec![0x7200, 7, 9]]);
            assert_eq!(called(&e, 0x0045_72e0)[0][0], 0x5000);
            // The last placement has the land height as z (the first is the
            // placement in PositionPlayer).
            let sets = placed_at.borrow();
            assert_eq!(sets.last().unwrap(), &vec![1.0, 2.0, expected]);
        }
    }

    #[test]
    fn centering_on_an_unknown_editor_id_loads_the_cell_from_the_data_files() {
        let mut e = engine();
        let world = center_world(&mut e);
        register_placement(&mut e);
        e.set_global(DATA_HANDLER, 0x5800u32);
        e.register(0x0046_1ae0, |_, _| int(0));
        e.register(0x0046_1cf0, |e, a| {
            e.mem.set_u32(a[2], 3);
            e.mem.set_u32(a[3], 4);
            int(0x7200)
        });
        e.register(0x0058_5b30, |_, _| int(0x6400));
        e.register(0x0045_11e0, |_, _| int(0));
        e.call(0x0093_db60, &args![world.this, 0x9990u32, 0u32]);
        assert_eq!(called(&e, 0x0046_1cf0)[0][..2], [0x5800, 0x9990]);
        // The grid is cleared for the world space, then the cell loaded at
        // the coordinates the data files gave.
        assert_eq!(called(&e, 0x0045_7d70)[0], vec![0x5000, 1, 0x7200, 0]);
        assert_eq!(called(&e, 0x0058_5b30), vec![vec![0x7200, 3, 4]]);
        assert_eq!(called(&e, 0x0057_5bb0)[0], vec![world.this.addr(), 0x6400]);
    }

    #[test]
    fn centering_on_nothing_does_nothing_more() {
        let mut e = engine();
        let world = center_world(&mut e);
        register_placement(&mut e);
        e.set_global(DATA_HANDLER, 0x5800u32);
        e.call(0x0093_db60, &args![world.this, 0x9990u32, 0u32]);
        assert!(called(&e, 0x0054_cfd0).is_empty());
        assert!(called(&e, 0x0057_5bb0).is_empty());
    }

    #[test]
    fn spawning_effects_for_the_four_iterators() {
        let mut e = engine();
        let this = player(&mut e);
        e.map(0x0101_0000, 0x2_0000);
        e.map(0x011a_0000, 0x6_0000);
        e.map(0x5000, 0x2000);
        e.map(0x0300_0000, 0x100);
        e.set_global(DATA_HANDLER, 0x5800u32);
        e.set_global(0x011c_95c8, 0x6600u32);
        e.mem.set_f32(0x0300_0000, 0.5);
        e.register(SETTING_FLOAT_GETTER, |_, _| int(0x0300_0000));
        e.mem.set_u32(0x5800 + 0x634, 0x9a00);
        stub(&mut e, &[0x0052_71f0, 0x0052_72b0, 0x0097_3fd0]);
        // The walker yields two actors in all, to the first iterator.
        let remaining = Rc::new(RefCell::new(vec![0x31u32, 0x32]));
        let queue = remaining.clone();
        e.register_double(0x0052_73c0, move |e, a| {
            if queue.borrow().is_empty() {
                int(0)
            } else {
                let actor = queue.borrow_mut().remove(0);
                e.mem.set_u32(a[2], actor);
                int(1)
            }
        });
        let blocks = Rc::new(RefCell::new(vec![0x6001u32, 0x6002]));
        let queue = blocks.clone();
        e.register_double(0x00aa_13e0, move |_, _| int(queue.borrow_mut().remove(0)));
        let accepted = e.mem.alloc(16);
        let refused = e.mem.alloc(16);
        let effects = Rc::new(RefCell::new(vec![accepted, refused]));
        let queue = effects.clone();
        e.register_double(0x0081_f580, move |_, _| int(queue.borrow_mut().remove(0)));
        e.register_double(0x00b0_00c4, move |_, a| int((a[0] == accepted) as u32));
        e.register_double(0x00b0_0000, |_, _| Ret::default());
        put_vtable(&mut e, accepted, 0x0270_0000, &[(0xc4, 0x00b0_00c4)]);
        put_vtable(
            &mut e,
            refused,
            0x0271_0000,
            &[(0xc4, 0x00b0_00c4), (0, 0x00b0_0000)],
        );
        e.call(0x0093_df50, &args![this]);
        // Four iterators were built, in the order of the kinds.
        let kinds: Vec<u32> = called(&e, 0x0052_71f0).iter().map(|w| w[1]).collect();
        assert_eq!(kinds, vec![0x28, 0x29, 0x2f, 0x2e]);
        assert!(called(&e, 0x0052_71f0).iter().all(|w| w[2] == this.addr()));
        // Each iterator is walked until it is done: two yields and four ends.
        let walks = called(&e, 0x0052_73c0);
        assert_eq!(walks.len(), 6);
        assert_eq!((walks[0][0], walks[0][4]), (0x6600, 3));
        let created = called(&e, 0x0081_f580);
        assert_eq!(created.len(), 2);
        assert_eq!((created[0][1], created[0][2]), (0x31, 0x9a00));
        assert_eq!(f(&created[0], 3), 0.5);
        assert_eq!(called(&e, 0x0097_3fd0), vec![vec![PROCESS_LISTS, accepted]]);
        assert_eq!(called(&e, 0x00b0_0000), vec![vec![refused, 1]]);
        // The iterators are released in reverse.
        let released: Vec<u32> = called(&e, 0x0052_72b0)
            .iter()
            .map(|w| w[0] - 0x1c)
            .collect();
        let built: Vec<u32> = called(&e, 0x0052_71f0).iter().map(|w| w[0]).collect();
        assert_eq!(released, built.iter().rev().copied().collect::<Vec<_>>());
    }

    #[test]
    fn no_effects_are_spawned_without_the_object() {
        let mut e = engine();
        let this = player(&mut e);
        e.map(0x0101_0000, 0x2_0000);
        e.map(0x011a_0000, 0x6_0000);
        e.map(0x5000, 0x2000);
        e.set_global(DATA_HANDLER, 0x5800u32);
        e.map(0x0300_0000, 0x100);
        e.register(SETTING_FLOAT_GETTER, |_, _| int(0x0300_0000));
        stub(&mut e, &[0x0052_71f0, 0x0052_72b0, 0x0052_73c0]);
        e.call(0x0093_df50, &args![this]);
        assert!(called(&e, 0x0052_73c0).is_empty());
        assert_eq!(called(&e, 0x0052_72b0).len(), 4);
    }

    /// The world of `SetSlowMoCamera`: a kill camera setting, sound
    /// functions recorded, the VATS object quiet.
    fn slow_mo_world(e: &mut Engine, setting: u32) -> Ptr<PlayerCharacter> {
        let this = player(e);
        e.map(0x0101_0000, 0x2_0000);
        e.map(0x011a_0000, 0x6_0000);
        e.set_global(ZERO_DOUBLE, 0.0f64);
        e.set_global(PLAYER_SINGLETON, this.addr());
        e.set(this, PlayerCharacter::eKillCameraSetting, setting);
        stub(
            e,
            &[
                SOUND_HANDLE_CONSTRUCT,
                SOUND_HANDLE_ASSIGN,
                SOUND_HANDLE_DESTRUCT,
                SOUND_HANDLE_PLAY,
                0x008d_6f30,
                0x0084_e3a0,
                0x0044_ddc0,
                0x009c_a2c0,
                0x008b_bc10,
                0x0070_9c40,
                0x004e_af60,
            ],
        );
        e.register(0x0045_3a70, |_, _| int(0x0400_0000));
        e.register(0x00ad_7550, |_, a| int(a[1]));
        e.map(0x0300_0000, 0x100);
        e.mem.set_f32(0x0300_0000, 3.0);
        e.register(SETTING_FLOAT_GETTER, |_, _| int(0x0300_0000));
        e.register_double(0x00b0_0008, |_, _| int(1));
        put_vtable(e, this.addr() + 0xa4, 0x0201_0000, &[(8, 0x00b0_0008)]);
        this
    }

    #[test]
    fn slow_motion_does_nothing_without_a_kill_camera_setting() {
        let mut e = engine();
        let this = slow_mo_world(&mut e, 0);
        e.call(0x0093_e530, &args![this, 1u32, 2.0f32, false, 0u32]);
        assert_eq!(call_addresses(&e), vec![0x0093_e530]);
    }

    #[test]
    fn slow_motion_stops_in_the_special_cell() {
        let mut e = engine();
        let this = slow_mo_world(&mut e, 1);
        e.register(0x008d_6f30, |_, _| int(0x3300));
        e.register(0x0084_e3a0, |_, _| int(0x0016_1e98));
        e.call(0x0093_e530, &args![this, 1u32, 2.0f32, false, 0u32]);
        assert_eq!(called(&e, 0x0084_e3a0), vec![vec![0x3300]]);
        assert!(called(&e, SOUND_HANDLE_CONSTRUCT).is_empty());
    }

    #[test]
    fn slow_motion_plays_a_sound_and_scales_the_time_for_other_settings() {
        let mut e = engine();
        let this = slow_mo_world(&mut e, 1);
        e.set_global(0x0102_90c0, 0.4f64);
        e.call(0x0093_e530, &args![this, 1u32, 5.0f32, false, 0u32]);
        let lookup = &called(&e, 0x00ad_7550)[0];
        assert_eq!(
            (lookup[0], lookup[2], lookup[3]),
            (0x0400_0000, 0x0106_f370, 0x121)
        );
        let handle = called(&e, SOUND_HANDLE_CONSTRUCT)[0][0];
        assert_eq!(
            called(&e, SOUND_HANDLE_ASSIGN),
            vec![vec![handle, lookup[1]]]
        );
        assert_eq!(called(&e, SOUND_HANDLE_PLAY), vec![vec![handle, 0]]);
        assert_eq!(called(&e, SOUND_HANDLE_DESTRUCT).len(), 2);
        assert_eq!(
            e.get(this, PlayerCharacter::fTimeInSlowMoCam),
            (5.0f32 as f64 * 0.4f64) as f32
        );
        assert!(called(&e, 0x009c_a2c0).is_empty());
    }

    #[test]
    fn slow_motion_does_nothing_more_without_a_mode_or_during_vats() {
        let mut e = engine();
        let this = slow_mo_world(&mut e, 2);
        e.call(0x0093_e530, &args![this, 0u32, 5.0f32, false, 0u32]);
        assert_eq!(e.get(this, PlayerCharacter::fTimeInSlowMoCam), 0.0);
        assert_eq!(called(&e, SOUND_HANDLE_DESTRUCT).len(), 2);
        let mut e = engine();
        let this = slow_mo_world(&mut e, 2);
        e.register(0x0044_ddc0, |_, _| int(1));
        e.call(0x0093_e530, &args![this, 1u32, 5.0f32, false, 0u32]);
        assert_eq!(e.get(this, PlayerCharacter::fTimeInSlowMoCam), 0.0);
    }

    #[test]
    fn slow_motion_for_setting_two_starts_the_vats_camera() {
        let mut e = engine();
        let this = slow_mo_world(&mut e, 2);
        e.register(0x004e_af60, |_, _| int(0));
        e.register(0x008b_bc10, |_, _| int(1));
        e.set_global(0x011f_21d1, 0u8);
        e.call(0x0093_e530, &args![this, 3u32, 5.0f32, false, 0x77u32]);
        assert_eq!(e.get(this, PlayerCharacter::fTimeInSlowMoCam), 5.0);
        assert_eq!(called(&e, 0x009c_a2c0), vec![vec![VATS_OBJECT, 3, 0x77]]);
        // The third-person flag comes from the player when none is saved.
        assert_eq!(e.global::<u8>(0x011f_21d1), 1);
        assert_eq!(called(&e, 0x0070_9c40), vec![vec![0]]);
    }

    #[test]
    fn slow_motion_uses_the_saved_third_person_flag() {
        let mut e = engine();
        let this = slow_mo_world(&mut e, 2);
        e.set(this, PlayerCharacter::b3rdPersonSaved, true);
        e.set(this, PlayerCharacter::bsave3rdPerson, true);
        e.call(0x0093_e530, &args![this, 3u32, 5.0f32, false, 0u32]);
        assert_eq!(e.global::<u8>(0x011f_21d1), 0);
        assert!(called(&e, 0x004e_af60).is_empty());
        assert!(called(&e, 0x0070_9c40).is_empty());
    }

    #[test]
    fn slow_motion_cooldown_decides_whether_the_queued_camera_starts() {
        // Queued with no cool-down: the cool-down is set from the setting.
        let mut e = engine();
        let this = slow_mo_world(&mut e, 1);
        e.set_global(0x0102_90c0, 0.4f64);
        e.call(0x0093_e530, &args![this, 1u32, 5.0f32, true, 0u32]);
        assert_eq!(e.get(this, PlayerCharacter::fKillCamCooldown), 3.0);
        assert!(e.get(this, PlayerCharacter::fTimeInSlowMoCam) > 0.0);
        // Queued during a cool-down: nothing starts.
        let mut e = engine();
        let this = slow_mo_world(&mut e, 1);
        e.set(this, PlayerCharacter::fKillCamCooldown, 1.0);
        e.call(0x0093_e530, &args![this, 1u32, 5.0f32, true, 0u32]);
        assert_eq!(e.get(this, PlayerCharacter::fTimeInSlowMoCam), 0.0);
        assert_eq!(e.get(this, PlayerCharacter::fKillCamCooldown), 1.0);
    }

    #[test]
    fn slow_motion_restores_the_time_multiplier_unless_the_player_has_the_perk() {
        let mut e = engine();
        let this = slow_mo_world(&mut e, 1);
        e.set_global(0x0102_90c0, 0.4f64);
        e.register_double(0x00b0_0008, |_, _| int(0));
        e.set_global(0x011a_c3a4, 1.0f32);
        e.set_global(0x011a_c3a8, 0.0f32);
        e.call(0x0093_e530, &args![this, 1u32, 5.0f32, false, 0u32]);
        assert_eq!(
            called(&e, 0x00b0_0008),
            vec![vec![this.addr() + 0xa4, 0x33]]
        );
        assert_eq!(e.global::<f32>(0x011a_c3a8), 1.0);
    }

    #[test]
    fn the_time_multiplier_is_saved_and_applied() {
        let mut e = engine();
        e.map(0x011a_0000, 0x6_0000);
        e.set_global(0x011a_c3a4, 0.25f32);
        e.call(0x0093_e750, &args![]);
        assert_eq!(e.global::<f32>(0x011a_c3a8), 0.25);
        stub(&mut e, &[0x00aa_4db0]);
        e.call(0x0093_e840, &args![GAME_TIMER]);
        let call = &called(&e, 0x00aa_4db0)[0];
        assert_eq!((call[0], f(call, 1), call[2]), (GAME_TIMER, 0.25, 0));
    }

    /// The world of `0093e770`: the animation getters and the VATS object.
    fn slow_mo_end_world(e: &mut Engine) -> Ptr<PlayerCharacter> {
        let this = player(e);
        e.map(0x011a_0000, 0x6_0000);
        e.set_global(0x011a_c3a8, 0.5f32);
        e.set_global(PLAYER_SINGLETON, this.addr());
        stub(
            e,
            &[
                0x00aa_4db0,
                0x004e_af60,
                0x0095_0a60,
                0x0088_b030,
                0x004a_4040,
                0x0095_f530,
                0x009c_8950,
                0x009c_6ba0,
                0x009c_6c30,
            ],
        );
        e.set(this, PlayerCharacter::fTimeInSlowMoCam, 4.0);
        this
    }

    #[test]
    fn ending_the_slow_motion_restores_time_and_resets_the_timer() {
        let mut e = engine();
        let this = slow_mo_end_world(&mut e);
        e.call(0x0093_e770, &args![this, 0i32, false]);
        let multiplier = &called(&e, 0x00aa_4db0)[0];
        assert_eq!((multiplier[0], f(multiplier, 1)), (GAME_TIMER, 0.5));
        assert_eq!(e.get(this, PlayerCharacter::fTimeInSlowMoCam), 0.0);
        // No animation: nothing told 1.0.
        assert_eq!(called(&e, 0x0095_0a60), vec![vec![this.addr(), 1]]);
        assert!(called(&e, 0x0088_b030).is_empty());
        assert!(called(&e, 0x009c_8950).is_empty());
    }

    #[test]
    fn ending_the_slow_motion_resets_the_animation_when_there_is_one() {
        let mut e = engine();
        let this = slow_mo_end_world(&mut e);
        e.register(0x004e_af60, |_, _| int(1));
        e.register(0x0095_0a60, |_, a| int(0x4a00 + a[1]));
        e.call(0x0093_e770, &args![this, 0i32, false]);
        assert_eq!(
            called(&e, 0x0095_0a60),
            vec![vec![this.addr(), 0], vec![this.addr(), 0]]
        );
        let reset = &called(&e, 0x0088_b030)[0];
        assert_eq!((reset[0], f(reset, 1)), (0x4a00, 1.0));
    }

    #[test]
    fn ending_the_vats_slow_motion_quits_the_playback() {
        let mut e = engine();
        let this = slow_mo_end_world(&mut e);
        e.call(0x0093_e770, &args![this, 2i32, true]);
        assert_eq!(called(&e, 0x0095_f530), vec![vec![this.addr(), 0, 3]]);
        assert_eq!(called(&e, 0x009c_8950), vec![vec![VATS_OBJECT, 0, 1]]);
        assert_eq!(called(&e, 0x009c_6ba0), vec![vec![VATS_OBJECT]]);
        assert_eq!(called(&e, 0x009c_6c30), vec![vec![VATS_OBJECT, 0, 1]]);
        assert_eq!(e.global::<u8>(0x011f_21d0), 1);
        assert!(called(&e, 0x004a_4040).is_empty());
    }

    #[test]
    fn ending_the_vats_slow_motion_without_the_flag_asks_first() {
        let mut e = engine();
        let this = slow_mo_end_world(&mut e);
        e.register(0x004a_4040, |_, _| int(1));
        e.call(0x0093_e770, &args![this, 2i32, false]);
        assert!(called(&e, 0x009c_8950).is_empty());
        assert_eq!(e.get(this, PlayerCharacter::fTimeInSlowMoCam), 0.0);
        e.register(0x004a_4040, |_, _| int(0));
        e.call(0x0093_e770, &args![this, 2i32, false]);
        assert_eq!(called(&e, 0x009c_8950).len(), 1);
    }
    /// Every function `Update` calls by address (outside the vtable calls).
    const UPDATE_CALLEES: &[u32] = &[
        0x0040_1170,
        0x0040_3df0,
        0x0040_4010,
        0x0040_4eb0,
        0x0040_4ee0,
        0x0040_8840,
        0x0040_8d60,
        0x0041_6870,
        0x0041_8940,
        0x0041_8a40,
        0x0041_a090,
        0x0041_cd70,
        0x0041_cda0,
        0x0043_01b0,
        0x0043_09e0,
        0x0043_6aa0,
        0x0043_7bd0,
        0x0043_7bf0,
        0x0043_9180,
        0x0043_9e90,
        0x0043_9ef0,
        0x0043_9f50,
        0x0043_d4d0,
        0x0044_6b50,
        0x0044_7950,
        0x0044_ddc0,
        0x0045_0b80,
        0x0045_6c70,
        0x0045_78c0,
        0x0045_7990,
        0x0045_7fe0,
        0x0045_81e0,
        0x0045_8b20,
        0x0045_c670,
        0x0046_4e10,
        0x0047_4a80,
        0x0048_1610,
        0x0048_1680,
        0x0048_18e0,
        0x0048_21a0,
        0x0048_39c0,
        0x0048_3a00,
        0x0048_f7f0,
        0x0049_1040,
        0x0049_38e0,
        0x0049_74a0,
        0x0049_85f0,
        0x0049_94f0,
        0x0049_97b0,
        0x004a_0c10,
        0x004a_4040,
        0x004a_7290,
        0x004a_d030,
        0x004b_3ab0,
        0x004b_3ae0,
        0x004b_71d0,
        0x004b_9930,
        0x004b_da70,
        0x004c_0c90,
        0x004d_1360,
        0x004e_44b0,
        0x004e_af60,
        0x004f_8960,
        0x004f_e160,
        0x0050_8070,
        0x0050_98e0,
        0x0050_d4a0,
        0x0051_1840,
        0x0051_9020,
        0x0052_4ac0,
        0x0052_4b60,
        0x0052_5430,
        0x0052_5980,
        0x0052_99a0,
        0x0052_9c90,
        0x0054_44c0,
        0x0054_74b0,
        0x0054_a070,
        0x0055_5c20,
        0x0055_8310,
        0x0055_9450,
        0x0056_10f0,
        0x0056_15d0,
        0x0056_20e0,
        0x0056_8680,
        0x0056_9160,
        0x0056_f930,
        0x0057_1760,
        0x0057_21e0,
        0x0057_3170,
        0x0057_5d10,
        0x0057_b460,
        0x0058_9850,
        0x0059_5f50,
        0x0059_5fc0,
        0x005a_03d0,
        0x005a_03f0,
        0x005a_1e50,
        0x005a_2030,
        0x005c_0880,
        0x005c_7870,
        0x005d_2860,
        0x005d_43c0,
        0x005e_2a70,
        0x005e_58f0,
        0x005f_2420,
        0x005f_2670,
        0x005f_4db0,
        0x0061_55f0,
        0x0063_c8a0,
        0x0064_50c0,
        0x0068_a7d0,
        0x0069_3ef0,
        0x006a_9540,
        0x006e_cd40,
        0x0070_2360,
        0x0070_3350,
        0x0070_38a0,
        0x0070_3e80,
        0x0070_50d0,
        0x0070_52f0,
        0x0070_54f0,
        0x0070_5640,
        0x0070_5a00,
        0x0070_62e0,
        0x0070_9430,
        0x0070_9bc0,
        0x0070_9be0,
        0x0070_9c40,
        0x0071_6440,
        0x0071_7a40,
        0x0072_6070,
        0x0077_1700,
        0x0077_2d10,
        0x0077_8930,
        0x007a_a720,
        0x007a_f430,
        0x007b_8f10,
        0x007d_0a70,
        0x007f_52c0,
        0x0080_fb00,
        0x0082_2e00,
        0x0082_ec10,
        0x0084_d030,
        0x0084_e3a0,
        0x0085_09a0,
        0x0085_09f0,
        0x0085_1230,
        0x0085_12f0,
        0x0086_fd90,
        0x0087_7720,
        0x0087_ce50,
        0x0088_43a0,
        0x0088_46e0,
        0x0088_49c0,
        0x0088_4f80,
        0x0088_5520,
        0x0088_6360,
        0x0088_85e0,
        0x0088_b030,
        0x0088_c830,
        0x0088_d640,
        0x0089_4900,
        0x0089_4cc0,
        0x0089_4d60,
        0x0089_4f90,
        0x0089_50f0,
        0x0089_5110,
        0x0089_f4e0,
        0x008a_0b10,
        0x008a_16d0,
        0x008a_6840,
        0x008a_6970,
        0x008a_73e0,
        0x008a_7570,
        0x008a_dcb0,
        0x008b_a600,
        0x008b_b650,
        0x008b_bc10,
        0x008b_bdb0,
        0x008b_c240,
        0x008c_3c40,
        0x008c_e390,
        0x008d_3550,
        0x008d_51f0,
        0x008d_5210,
        0x008d_6f30,
        0x008d_8520,
        0x008f_ec10,
        0x0092_2650,
        0x0093_06d0,
        0x0093_1d70,
        0x0094_4270,
        0x0094_42a0,
        0x0094_42c0,
        0x0094_42e0,
        0x0094_4300,
        0x0094_4320,
        0x0094_4340,
        0x0094_4360,
        0x0094_4380,
        0x0094_43a0,
        0x0094_43c0,
        0x0094_43f0,
        0x0094_4400,
        0x0094_4430,
        0x0094_4460,
        0x0094_44a0,
        0x0094_44d0,
        0x0094_45b0,
        0x0094_62c0,
        0x0094_7b10,
        0x0094_81d0,
        0x0094_8310,
        0x0094_ae40,
        0x0094_dbe0,
        0x0094_df80,
        0x0095_0090,
        0x0095_00a0,
        0x0095_0110,
        0x0095_0340,
        0x0095_03d0,
        0x0095_0530,
        0x0095_0a60,
        0x0095_1a10,
        0x0095_3c50,
        0x0095_3c80,
        0x0095_3d00,
        0x0095_3d40,
        0x0095_de30,
        0x0095_f6a0,
        0x0095_f6c0,
        0x0096_04f0,
        0x0096_13c0,
        0x0096_2190,
        0x0096_2350,
        0x0096_2d00,
        0x0096_3bf0,
        0x0096_3e00,
        0x0096_3eb0,
        0x0096_4100,
        0x0096_4260,
        0x0096_6a20,
        0x0096_73d0,
        0x0096_78a0,
        0x0096_7950,
        0x0096_7a00,
        0x0096_7ae0,
        0x0096_7da0,
        0x0096_9c30,
        0x0096_e570,
        0x0097_3de0,
        0x0097_4af0,
        0x0097_64a0,
        0x009a_b9a0,
        0x009b_b080,
        0x009c_71c0,
        0x009c_8950,
        0x009c_a1f0,
        0x009e_a360,
        0x009e_a570,
        0x00a2_3390,
        0x00a2_4280,
        0x00a2_4660,
        0x00aa_4db0,
        0x00ad_73b0,
        0x00ad_7480,
        0x00ad_7c80,
        0x00ad_7da0,
        0x00ad_7ec0,
        0x00ad_8930,
        0x00b5_d9f0,
        0x00c3_dfa0,
        0x00ca_1410,
        0x00ec_43fb,
        0x00ec_7d40,
        SOUND_HANDLE_CONSTRUCT,
        SOUND_HANDLE_ASSIGN,
        SOUND_HANDLE_DESTRUCT,
        SOUND_HANDLE_PLAY,
        SOUND_HANDLE_STOP,
        SOUND_HANDLE_IS_VALID,
        SOUND_HANDLE_FADE_OUT_AND_RELEASE,
        AUDIO_INSTANCE,
        AUDIO_GET_SOUND_HANDLE_BY_NAME,
        CELL_IS_INTERIOR,
        LIST_CONSTRUCT,
        LIST_DESTRUCT,
        LIST_IS_EMPTY,
        LIST_ITEM_ADDRESS,
        LIST_NEXT,
        NI_POINTER_ASSIGN,
    ];

    /// A world in which `Update` runs from start to end with everything it
    /// calls doing nothing: all the globals are 0, every setting is 0.0, and
    /// every virtual function of the player and of its process returns 0.
    fn update_world(e: &mut Engine) -> Ptr<PlayerCharacter> {
        let this = player(e);
        e.map(0x0100_0000, 0x0130_0000);
        e.map(0, 0x1000);
        e.register(0, |_, _| Ret::default());
        e.map(0x0300_0000, 0x1000);
        for addr in UPDATE_CALLEES {
            e.register(*addr, |_, _| Ret::default());
        }
        for getter in [
            SETTING_FLOAT_GETTER,
            SETTING_BYTE_GETTER,
            SETTING_INT_GETTER,
        ] {
            e.register(getter, |_, _| int(0x0300_0000));
        }
        e.register(0x00b0_0000, |_, _| Ret::default());
        let slots: Vec<(u32, u32)> = (0..0x200).map(|i| (i * 4, 0x00b0_0000)).collect();
        put_vtable(e, this.addr(), 0x0201_0000, &slots);
        let process = Ptr::<PlayerCharacter>::new(e.mem.alloc(0x40));
        put_vtable(e, process.addr(), 0x0202_0000, &slots);
        put_vtable(e, this.addr() + ACTOR_VALUE_OWNER_BASE, 0x0203_0000, &slots);
        e.mem.set_u32(this.addr() + 0x68, process.addr());
        e.set_global(PLAYER_SINGLETON, this.addr());
        this
    }

    #[test]
    fn update_in_a_quiet_world_runs_to_the_end_and_releases_its_locals() {
        let mut e = engine();
        let this = update_world(&mut e);
        player_character_update(&mut e, this, 0.5);
        let log = call_addresses(&e);
        let n = log.len();
        assert_eq!(&log[n - 2..], &[SOUND_HANDLE_DESTRUCT, 0x0040_4ee0]);
        assert_eq!(called(&e, 0x0040_4eb0).len(), 1);
        assert_eq!(
            called(&e, 0x0040_4eb0)[0][1..],
            [0x34, 1, UPDATE_GUARD_FILE, 0xc41]
        );
        assert_eq!(called(&e, 0x0040_4ee0).len(), 1);
    }

    #[test]
    fn update_with_a_forced_activation_activates_and_ends_early() {
        let mut e = engine();
        let this = update_world(&mut e);
        e.register(0x0094_4320, |_, _| int(0x4455));
        player_character_update(&mut e, this, 0.5);
        assert_eq!(
            called(&e, 0x0057_3170),
            vec![vec![0x4455, this.addr(), 0, 0, 1]]
        );
        assert_eq!(called(&e, 0x0051_9020), vec![vec![this.addr(), 0]]);
        let log = call_addresses(&e);
        let n = log.len();
        assert_eq!(&log[n - 2..], &[SOUND_HANDLE_DESTRUCT, 0x0040_4ee0]);
        assert!(called(&e, 0x0095_f6c0).is_empty());
    }

    #[test]
    fn update_counts_the_warning_timers_down_and_resets_them_when_out() {
        let mut e = engine();
        let this = update_world(&mut e);
        e.set_global(TIMER_RESET, -1.0f32);
        e.set(this, PlayerCharacter::fStealWarningTimer, 2.0);
        e.set(this, PlayerCharacter::fPickPocketWarningTimer, 0.0);
        player_character_update(&mut e, this, 0.5);
        assert_eq!(e.get(this, PlayerCharacter::fStealWarningTimer), 1.5);
        assert_eq!(e.get(this, PlayerCharacter::fPickPocketWarningTimer), -1.0);
    }

    #[test]
    fn update_runs_the_end_of_the_frame_work_once() {
        let mut e = engine();
        let this = update_world(&mut e);
        player_character_update(&mut e, this, 0.25);
        assert_eq!(
            called(&e, 0x0095_f6c0),
            vec![vec![this.addr(), 0.25f32.to_bits()]]
        );
        assert_eq!(called(&e, 0x0055_5c20).len(), 1);
    }
}
