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
}
