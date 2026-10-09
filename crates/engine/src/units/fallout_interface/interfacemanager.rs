//! `fallout/interface/interfacemanager.cpp` (Xbox PDB source unit), subsystem `fallout/interface`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit is the interface manager (`InterfaceManager`, Xbox PDB): the
//! one object (the word at `011d8a80`) that owns the menu scene graphs, the
//! cursor, the menu stack and the per-frame interface update. This file is
//! the unit's main file: it declares the layouts and helpers the part files
//! (`interfacemanager_p2.rs`, ...) share, and holds the functions from
//! `00709fd0` up to `00717230`.
//!
//! Offsets of the manager object on PC match the Xbox PDB up to the effect
//! managers (`+0x178 HUDEffects`, `+0x1dc VATSEffect`); after those the PC
//! build differs (the fields from `+0x4ac` on have no PDB name here).
//!
//! The compiler's exception-unwinding frames are not translated. Every
//! function that opens the scope guard (`00404eb0` before, `00404ee0`
//! after; neither has a PDB name, the guard is a 4-byte local that is given
//! the source line) does so through [`with_scope_guard`].

#[allow(unused_imports)]
use crate::prelude::*;

/// The interface manager singleton (a pointer to the object).
pub(crate) const MANAGER_SINGLETON: u32 = 0x011d_8a80;
/// `bHas360Controller` (Xbox PDB, static): set by `Initialize` from
/// `XInputGetState`.
pub(crate) const HAS_360_CONTROLLER: u32 = 0x011d_8a84;
/// `fCurrAlphaDPS`, `bAlphaGoingDown`, `iLastDPSFadeTime` (Xbox PDB, statics
/// of the manager, matched by type and the constructor's initial values).
pub(crate) const CURR_ALPHA_DPS: u32 = 0x011d_8a68;
pub(crate) const ALPHA_GOING_DOWN: u32 = 0x011d_8a50;
pub(crate) const LAST_DPS_FADE_TIME: u32 = 0x011d_8a64;
/// The `"...\InterfaceManager.cpp"` path the scope guard is given.
pub(crate) const SOURCE_FILE: u32 = 0x0106_ef50;
/// `"MenuRoot"` and `"Menu3DRoot"`.
pub(crate) const MENU_ROOT_NAME: u32 = 0x0106_ef34;
pub(crate) const MENU_3D_ROOT_NAME: u32 = 0x0106_ef40;
/// `"Cursor"`.
const CURSOR_NAME: u32 = 0x0106_f010;
/// `"Data\Menus\globals.xml"`.
const GLOBALS_XML_PATH: u32 = 0x0106_eff8;
/// `"_background_fill_alpha"` and `"_Has360Controller"`.
const BACKGROUND_FILL_ALPHA_NAME: u32 = 0x0106_efe0;
const HAS_360_CONTROLLER_NAME: u32 = 0x0106_efcc;
/// `"InterfaceManager: DebugText Root"`.
const DEBUG_TEXT_ROOT_NAME: u32 = 0x0106_efa8;
/// `255.0` (`double`).
const TWO_FIFTY_FIVE: u32 = 0x0101_e568;
/// `2.0` (`double`).
const TWO: u32 = 0x0101_1590;
/// The 16:10 aspect ratio, `1.6` (`double`), and `0.6` (`double`), which
/// `1 / ratio` is compared with (5:3 is `1.666..`, so this is the 10:6 test
/// for the inverse).
const WIDESCREEN_RATIO: u32 = 0x0106_efa0;
const WIDESCREEN_INVERSE_RATIO: u32 = 0x0102_90b8;
/// The angle (`float`) the one-to-one distance is derived from.
const ONE_TO_ONE_ANGLE: u32 = 0x0106_ef4c;
/// The `float` the cursor node's look direction takes as its second
/// component (`-1.0`).
const CURSOR_DIRECTION_Y: u32 = 0x0101_2054;
/// Default `fEnd` of a [`Timer`] (`float`).
const TIMER_DEFAULT_END: u32 = 0x0101_7d00;

/// `XInputGetState(user, state)` (`stdcall`); 0 means a controller answered.
const XINPUT_GET_STATE: u32 = 0x009f_996e;
/// `operator new(size)` and `operator delete(block)`.
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `NiAlloc(size)` (the Gamebryo allocator, `cdecl`).
const NI_ALLOC: u32 = 0x00aa_13e0;
/// Scope guard constructor `(guard, 0xd, 1, source file, line)` and
/// destructor.
const SCOPE_GUARD_BEGIN: u32 = 0x0040_4eb0;
const SCOPE_GUARD_END: u32 = 0x0040_4ee0;
/// `NiPointer<T>` members, called on the embedded pointer: constructor from
/// a pointer (adds a reference), assignment (releases the old pointer),
/// `get` and destructor.
const NI_POINTER_CONSTRUCT: u32 = 0x0063_3c90;
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
const NI_POINTER_GET: u32 = 0x0055_9450;
const NI_POINTER_DESTROY: u32 = 0x0045_cec0;
/// An empty member constructor (the body only returns `this`).
const MEMBER_CONSTRUCTOR_EMPTY: u32 = 0x0068_15c0;
/// Constructor / destructor of an embedded `BSSimpleList`.
const SIMPLE_LIST_CONSTRUCT: u32 = 0x0096_a2d0;
const SIMPLE_LIST_DESTROY: u32 = 0x0046_ffb0;
/// `NiPoint3::NiPoint3(x, y, z)`: stores the three floats, returns `this`.
const NI_POINT3_CONSTRUCT: u32 = 0x0041_6870;
/// Constructor of the struct the node update takes: `(this, float, byte,
/// byte)` stores the float and the two bytes and zeroes the rest (the
/// Gamebryo update data).
const NI_UPDATE_DATA_CONSTRUCT: u32 = 0x0043_d410;
/// Updates a node from that struct (`(node, &data)`).
const NODE_UPDATE: u32 = 0x00a5_9c60;
/// `NiAVObject::UpdateProperties` (Xbox PDB).
const NODE_UPDATE_PROPERTIES: u32 = 0x00a5_a040;
/// `(node, bool)`: forwards to `0043b370(node, flag, 1)`; the cursor and
/// debug text roots are given 0 and 1 here.
const NODE_SET_FLAG: u32 = 0x0045_0f90;
/// `FOHUDEffectManager` constructor (Xbox PDB) and destructor.
const HUD_EFFECT_MANAGER_CONSTRUCT: u32 = 0x007f_7610;
const HUD_EFFECT_MANAGER_DESTROY: u32 = 0x007f_7790;
/// `FOVATSEffectManager` constructor (Xbox PDB) and destructor.
const VATS_EFFECT_MANAGER_CONSTRUCT: u32 = 0x007f_fec0;
const VATS_EFFECT_MANAGER_DESTROY: u32 = 0x0080_01b0;
/// `FOPipboyManager` constructor.
const PIPBOY_MANAGER_CONSTRUCT: u32 = 0x007f_7a70;
/// Tile trait setters, called on a tile: `(trait id, int)` (converted to
/// float and forwarded to [`TILE_SET_FLOAT`]), `(trait id, float, bool)` and
/// `(trait id, string, bool)`.
const TILE_SET_INT: u32 = 0x0070_0320;
const TILE_SET_FLOAT: u32 = 0x00a0_12d0;
const TILE_SET_STRING: u32 = 0x00a0_1350;
/// `Tile::UpdateTile(bool)` (Xbox PDB).
const TILE_UPDATE_TILE: u32 = 0x00a0_4640;
/// `Tile::TextToTrait(name)` (Xbox PDB, `cdecl`): the trait id of a name.
const TILE_TEXT_TO_TRAIT: u32 = 0x00a0_1860;
/// `Tile::AddDirtyTile(tile)` (Xbox PDB, `cdecl`).
const TILE_ADD_DIRTY_TILE: u32 = 0x00a0_7690;
/// `Tile::UpdateAll(bool)` (Xbox PDB, `cdecl`).
const TILE_UPDATE_ALL: u32 = 0x00a0_4200;
/// `Tile::ReadFile(path)` (Xbox PDB).
const TILE_READ_FILE: u32 = 0x00a0_1b00;
/// `Tile::SetParent(parent, bool)` (Xbox PDB).
const TILE_SET_PARENT: u32 = 0x00a0_87d0;
/// `Tile::LoadTextTable` (Xbox PDB).
const TILE_LOAD_TEXT_TABLE: u32 = 0x009f_f970;
/// `Tile::FreeTraitList` (Xbox PDB).
const TILE_FREE_TRAIT_LIST: u32 = 0x00a0_0a30;
/// `Tile::~Tile` and `Tile::Release` (Xbox PDB).
const TILE_DESTRUCT: u32 = 0x009f_f340;
const TILE_RELEASE: u32 = 0x009f_f690;
/// `Menu::RemoveFromXMLArchive(path, bool)` and `Menu::FreeXMLArchive`
/// (Xbox PDB).
const MENU_REMOVE_FROM_XML_ARCHIVE: u32 = 0x00a1_ca20;
const MENU_FREE_XML_ARCHIVE: u32 = 0x00a1_c800;
/// `InterfaceManager::GetScreenWidth` (Xbox PDB, `float` in `ST0`), and the
/// getters used next to it by `Init`.
const GET_SCREEN_WIDTH: u32 = 0x0071_5d40;
const GET_SCREEN_HEIGHT: u32 = 0x0071_5da0;
const GET_INTERFACE_MARGIN_A: u32 = 0x0071_77c0;
const GET_INTERFACE_MARGIN_B: u32 = 0x0071_7820;
const GET_INTERFACE_SCALE: u32 = 0x0070_7b60;
/// `InterfaceManager::CreateSceneGraph(parent, name, flag)` (Xbox PDB).
const CREATE_SCENE_GRAPH: u32 = 0x0071_2e90;
/// The root pointers `5bac70` and `5bac60` read (the parent each scene
/// graph is created under).
const SCENE_GRAPH_PARENT_3D: u32 = 0x005b_ac70;
const SCENE_GRAPH_PARENT: u32 = 0x005b_ac60;
/// `MenuManager::Instance(create)` (Xbox PDB, `cdecl`).
const MENU_MANAGER_INSTANCE: u32 = 0x0071_e290;
/// `NiAlphaProperty` constructor and the setters it is given: bit `0x1` of
/// the flag word, bit `0x2000`, the mask `0x1e` (source mode) and the mask
/// `0x1e0` (destination mode).
const ALPHA_PROPERTY_CONSTRUCT: u32 = 0x0043_91c0;
const ALPHA_PROPERTY_SET_BLEND: u32 = 0x0049_ed90;
const ALPHA_PROPERTY_SET_NO_SORTER: u32 = 0x0063_6000;
const ALPHA_PROPERTY_SET_SOURCE_MODE: u32 = 0x0043_9340;
const ALPHA_PROPERTY_SET_DESTINATION_MODE: u32 = 0x0043_9390;
/// `BSShaderAccumulator::BSShaderAccumulator(this, 100 or 99, 1, 0x2f7)`
/// (Xbox PDB) and its two single-word setters (`+0x194`, `+0x19c`).
const SHADER_ACCUMULATOR_CONSTRUCT: u32 = 0x00b6_60d0;
const SHADER_ACCUMULATOR_SET_194: u32 = 0x004a_1020;
const SHADER_ACCUMULATOR_SET_19C: u32 = 0x004a_1040;
/// `ShadowSceneNode::ShadowSceneNode` (Xbox PDB) and
/// `BSShaderManager::SetShadowSceneNode(this, slot)` (Xbox PDB).
const SHADOW_SCENE_NODE_CONSTRUCT: u32 = 0x00b5_e0f0;
const SET_SHADOW_SCENE_NODE: u32 = 0x0070_b760;
/// Constructors of the objects the manager owns: the 0xc-byte view caster
/// and the 0x3c-byte root tile.
const VIEW_CASTER_CONSTRUCT: u32 = 0x0063_19e0;
const MENU_ROOT_TILE_CONSTRUCT: u32 = 0x0070_9840;
/// `Timer` list head helpers: the table constructor `0070a8e0` calls, and
/// the callee that destroys the 0x4d4 member.
const KEY_TABLE_CONSTRUCT: u32 = 0x0070_6b90;
/// Reads the word at `+0x14` of the object at `011f6394` (the tick count
/// the fade code compares with).
const FADE_CLOCK_READ: u32 = 0x0082_5c00;
const FADE_CLOCK: u32 = 0x011f_6394;
/// `cdecl(float) -> ST0` wrappers of the two functions the one-to-one
/// distance is computed from.
const ANGLE_FUNCTION_A: u32 = 0x004e_44b0;
const ANGLE_FUNCTION_B: u32 = 0x004e_4470;
/// Setters of the pointer `5585e0` / `703060` and the font manager.
const FONT_MANAGER_INIT: u32 = 0x00a1_67c0;
const SYSTEM_COLOR_MANAGER_GET_INSTANCE: u32 = 0x0071_8b60;
const NI_NODE_CONSTRUCT: u32 = 0x00a5_ecb0;
const NODE_SET_NAME: u32 = 0x00a5_b950;
const FIXED_STRING_CONSTRUCT: u32 = 0x0043_8170;
const FIXED_STRING_DESTROY: u32 = 0x0043_81b0;
const FOG_PROPERTY_CONSTRUCT: u32 = 0x00bb_8180;
const TILE_IMAGE_NODE: u32 = 0x0056_c7f0;
const NODE_SET_TRANSLATE: u32 = 0x004b_c1f0;
const GET_FRAME_SCENE_NODE: u32 = 0x007f_a950;
const GET_SECOND_SCENE_NODE: u32 = 0x0055_85e0;
const SET_DEBUG_TEXT_VISIBLE: u32 = 0x0070_3060;
const GET_DESKTOP_WIDTH: u32 = 0x0070_6e50;
const GET_DESKTOP_HEIGHT: u32 = 0x0070_6e20;
const STRING_LIST_SET_LIMIT: u32 = 0x0045_ce80;
const LOADING_MENU_CREATE: u32 = 0x0070_5e00;
const HAS_360_CONTROLLER_GETTER: u32 = 0x004b_71d0;
const FLOAT_HOLDER_GET: u32 = 0x0040_3e20;
const STRING_HOLDER_GET: u32 = 0x0040_3df0;
const HUD_MAIN_MENU_CREATE: u32 = 0x0076_bfe0;
const HUD_MAIN_MENU_SET_MENU_MODE: u32 = 0x0077_1700;

layout! {
    /// `InterfaceManager` (Xbox PDB), 0x580 bytes on PC (0x478 on the Xbox).
    /// Offsets up to `+0x178` are the PDB's; the fields after the effect
    /// managers are PC only and named by their offset.
    pub struct InterfaceManager: 0x580 {
        /// `bFirstInit` (Xbox PDB).
        0x000 bFirstInit: u8,
        /// `bSecondInit` (Xbox PDB).
        0x001 bSecondInit: u8,
        /// `spSceneGraph` (Xbox PDB): `NiPointer<SceneGraph>`.
        0x004 spSceneGraph: u32,
        /// `sp3DSceneGraph` (Xbox PDB): `NiPointer<SceneGraph>`.
        0x008 sp3DSceneGraph: u32,
        /// `cMenuMode` (Xbox PDB).
        0x00c cMenuMode: u32,
        /// `bExternalForcedMenuMode` (Xbox PDB).
        0x010 bExternalForcedMenuMode: u8,
        /// `bLockMenuModeForFade` (Xbox PDB).
        0x011 bLockMenuModeForFade: u8,
        /// `cStatsPageNumber`, `cInventoryPageNumber`, `cMagicPageNumber`,
        /// `cMapPageNumber` (Xbox PDB), each a `char`.
        0x012 cStatsPageNumber: u8,
        0x013 cInventoryPageNumber: u8,
        0x014 cMagicPageNumber: u8,
        0x015 cMapPageNumber: u8,
        /// `iPickDistance` (Xbox PDB).
        0x018 iPickDistance: i32,
        /// `iSoundPauses` (Xbox PDB).
        0x020 iSoundPauses: u8,
        /// `iVoicePauses` (Xbox PDB).
        0x021 iVoicePauses: u8,
        /// `pPlayerLight` (Xbox PDB).
        0x024 pPlayerLight: u32,
        /// `pCursor` (Xbox PDB): `TileImage *`.
        0x028 pCursor: u32,
        /// `fMouseWheel` (Xbox PDB).
        0x044 fMouseWheel: f32,
        /// `fMouseHeldTime` (Xbox PDB).
        0x048 fMouseHeldTime: f32,
        /// `fDragOffsetLastX` and `fDragOffsetLastY` (Xbox PDB).
        0x054 fDragOffsetLastX: f32,
        0x058 fDragOffsetLastY: f32,
        /// `pDragTarget` (Xbox PDB).
        0x04c pDragTarget: u32,
        /// `iDragStartX` (Xbox PDB).
        0x050 iDragStartX: i32,
        /// `iDragStartY` (Xbox PDB).
        0x05c iDragStartY: i32,
        /// `iCursorOffsetX` (Xbox PDB).
        0x060 iCursorOffsetX: i32,
        /// `iCursorOffsetY` (Xbox PDB).
        0x064 iCursorOffsetY: i32,
        /// `iCurrentPickIndex` (Xbox PDB).
        0x078 iCurrentPickIndex: i32,
        /// `bDebugTextVisible` (Xbox PDB).
        0x07c bDebugTextVisible: u8,
        /// `bShowMouse` (Xbox PDB).
        0x07d bShowMouse: u8,
        /// `pInterfaceRoot` (Xbox PDB): `NiNode *`.
        0x080 pInterfaceRoot: u32,
        /// `pCursorRoot` (Xbox PDB): `NiNode *`.
        0x084 pCursorRoot: u32,
        /// `pPlayerRoot` (Xbox PDB).
        0x088 pPlayerRoot: u32,
        /// `spInterfaceAccum` (Xbox PDB): `NiPointer<BSShaderAccumulator>`.
        0x08c spInterfaceAccum: u32,
        /// `sp3dInterfaceAccum` (Xbox PDB).
        0x090 sp3dInterfaceAccum: u32,
        /// `pShadowNode` (Xbox PDB): `ShadowSceneNode *`.
        0x094 pShadowNode: u32,
        /// `pUIPlayerNode` (Xbox PDB): `ShadowSceneNode *`.
        0x098 pUIPlayerNode: u32,
        /// `pMenusRoot` (Xbox PDB): the root `Tile`.
        0x09c pMenusRoot: u32,
        /// `pStringRoot` (Xbox PDB): the tile of the globals file.
        0x0a0 pStringRoot: u32,
        /// `pDebugTextRoot` (Xbox PDB): `NiNode *`.
        0x0a4 pDebugTextRoot: u32,
        /// `fOneToOneDistance` (Xbox PDB).
        0x0a8 fOneToOneDistance: f32,
        /// `spAlphaProp` (Xbox PDB): `NiPointer<NiAlphaProperty>`.
        0x0ac spAlphaProp: u32,
        /// `bNeedToUpdate` (Xbox PDB).
        0x0b0 bNeedToUpdate: u8,
        /// `iCharHit` (Xbox PDB).
        0x0b2 iCharHit: u16,
        /// `pTargetReticle` (Xbox PDB).
        0x0b4 pTargetReticle: u32,
        /// `pSafeZone` (Xbox PDB).
        0x0b8 pSafeZone: u32,
        /// `pMouseOverTarget` (Xbox PDB).
        0x0bc pMouseOverTarget: u32,
        /// `iLastXDefault` (Xbox PDB).
        0x0c0 iLastXDefault: u32,
        /// `bPreLoadMainMenus` (Xbox PDB).
        0x0c8 bPreLoadMainMenus: u8,
        /// `bFirstChanceLoad` (Xbox PDB).
        0x0c9 bFirstChanceLoad: u8,
        /// `pOverTileTarget`, `pOverTileMenu`, `pDragOverTileTarget`,
        /// `pDragOverTileMenu` (Xbox PDB).
        0x0cc pOverTileTarget: u32,
        0x0d0 pOverTileMenu: u32,
        0x0d4 pDragOverTileTarget: u32,
        0x0d8 pDragOverTileMenu: u32,
        /// `bFullHelp` (Xbox PDB).
        0x0dd bFullHelp: u8,
        /// `cLastMessageButtonClicked` (Xbox PDB).
        0x0e4 cLastMessageButtonClicked: u8,
        /// `fpResult` (Xbox PDB).
        0x0e8 fpResult: u32,
        /// `bTextureRelease` (Xbox PDB).
        0x0ec bTextureRelease: u8,
        /// `bMouseInMotion` (Xbox PDB).
        0x0ed bMouseInMotion: u8,
        /// `pPickRef` (Xbox PDB).
        0x0f0 pPickRef: u32,
        /// `pReticleRef` (Xbox PDB).
        0x0f4 pReticleRef: u32,
        /// `pCrossHairRef` (Xbox PDB).
        0x0f8 pCrossHairRef: u32,
        /// `pActivateRef` (Xbox PDB).
        0x0fc pActivateRef: u32,
        /// `pTelekinesisRef` (Xbox PDB).
        0x100 pTelekinesisRef: u32,
        /// `bFuzzyActivatePick` (Xbox PDB).
        0x110 bFuzzyActivatePick: u8,
        /// `iEnterStack` (Xbox PDB): ten words on PC.
        0x114 iEnterStack: u32,
        /// `pViewCaster` (Xbox PDB).
        0x13c pViewCaster: u32,
        /// `bClickMultithreaded` (Xbox PDB).
        0x148 bClickMultithreaded: u8,
        /// `bShiftDown` (Xbox PDB).
        0x149 bShiftDown: u8,
        /// `iModifierKeys` (Xbox PDB).
        0x14c iModifierKeys: i32,
        /// `iRepeatingKey` (Xbox PDB).
        0x150 iRepeatingKey: i32,
        /// `uKeyDownTime` (Xbox PDB).
        0x154 uKeyDownTime: u32,
        /// `iLastGamepadEvent` (Xbox PDB).
        0x15c iLastGamepadEvent: i32,
        /// `uGamepadRepeatStartTime` (Xbox PDB).
        0x160 uGamepadRepeatStartTime: u32,
        /// `pTimers` (Xbox PDB): the head [`Timer`].
        0x164 pTimers: u32,
        /// `bIsInRenderedMenu` (Xbox PDB).
        0x168 bIsInRenderedMenu: u8,
        /// `pCurrentRenderedMenu` (Xbox PDB).
        0x16c pCurrentRenderedMenu: u32,
        /// `bMouseOverRenderedMenu` (Xbox PDB).
        0x170 bMouseOverRenderedMenu: u8,
        /// `pPipboy` (Xbox PDB): `FOPipboyManager *`.
        0x174 pPipboy: u32,
        /// PC only: float the constructor sets to 0.
        0x4ac field_4ac: f32,
        /// PC only: float the constructor sets to 0.
        0x4b0 field_4b0: f32,
        /// PC only: word the constructor sets to 0.
        0x4b8 field_4b8: u32,
        /// PC only: a mode word; `3` is tested by `00709c00`.
        0x4bc field_4bc: u32,
        /// PC only: word the constructor sets to 0.
        0x4c0 field_4c0: u32,
        /// PC only: byte the constructor sets to 0.
        0x4cc field_4cc: u8,
        /// PC only: byte the constructor sets to 0.
        0x4cd field_4cd: u8,
        /// PC only: float the constructor sets to 1.0.
        0x4d0 field_4d0: f32,
    }

    /// `InterfaceManager::Timer` (Xbox PDB), 0x14 bytes: a node of the timer
    /// list that `pTimers` heads.
    pub struct Timer: 0x14 {
        /// `pIndex` (Xbox PDB).
        0x00 pIndex: u32,
        /// `fElapsed` (Xbox PDB).
        0x04 fElapsed: f32,
        /// `fEnd` (Xbox PDB).
        0x08 fEnd: f32,
        /// `pPrev` (Xbox PDB).
        0x0c pPrev: u32,
        /// `pNext` (Xbox PDB).
        0x10 pNext: u32,
    }

    /// `TileImage` (Xbox PDB), 0x48 bytes: a `Tile` (0x38 bytes) plus the
    /// image fields.
    pub struct TileImage: 0x48 {
        /// `uiFlags` of the `Tile` base (Xbox PDB); bit `0x2000` is tested by
        /// `00709a20`.
        0x30 uiFlags: u32,
        /// `fScale` (Xbox PDB).
        0x38 fScale: f32,
        /// `spTexture` (Xbox PDB): `NiPointer<NiTexture>`.
        0x3c spTexture: u32,
        /// `spTSP` (Xbox PDB): `NiPointer<TileShaderProperty>`.
        0x40 spTSP: u32,
        /// `bIsScissor` (Xbox PDB).
        0x44 bIsScissor: u8,
    }
}

/// The vtable of `TileImage` (`__scalar_deleting_destructor_` first).
const TILE_IMAGE_VTABLE: u32 = 0x0106_f01c;
/// `Tile::IsReleased`: true when bit `0x2000` of the tile flags is set
/// (`00709a20`); the tile's destructor skips `Tile::Release` then.
const TILE_FLAG_TEST: u32 = 0x0070_9a20;
/// Base-class constructor of `TileImage` (`Tile::Tile`).
const TILE_CONSTRUCT: u32 = 0x0070_9860;
/// `TileImage::GetType` returns this.
const TILE_IMAGE_TYPE: u32 = 0x386;
/// `TileImage::GetTypeName` returns this string.
const TILE_IMAGE_TYPE_NAME: u32 = 0x0106_f044;

/// Runs `body` between the scope guard's constructor and destructor. The
/// guard is a 4-byte local that the game builds with the source file and
/// line (`00404eb0`) and tears down on every exit (`00404ee0`).
pub(crate) fn with_scope_guard<R>(
    e: &mut Engine,
    line: u32,
    body: impl FnOnce(&mut Engine) -> R,
) -> R {
    e.with_stack(4, |e, guard| {
        e.call(
            SCOPE_GUARD_BEGIN,
            &args![guard, 0xdu32, 1u32, SOURCE_FILE, line],
        );
        let result = body(e);
        e.call(SCOPE_GUARD_END, &args![guard]);
        result
    })
}

/// `new T` followed by `T::T(...)`: allocates `size` bytes through
/// `allocator` and runs `construct` on the block, or yields null when the
/// allocation fails.
fn construct_new(
    e: &mut Engine,
    allocator: u32,
    size: u32,
    construct: impl FnOnce(&mut Engine, Ptr) -> Ptr,
) -> Ptr {
    let block: Ptr = e.call(allocator, &args![size]).ptr();
    if block.is_null() {
        Ptr::NULL
    } else {
        construct(e, block)
    }
}

/// `delete object` through the virtual destructor (slot 0, flag 1).
fn delete_virtual(e: &mut Engine, object: u32) {
    e.vcall(object, 0, &args![1u32]);
}

fn tile_set_int(e: &mut Engine, tile: u32, trait_id: u32, value: u32) {
    e.call(TILE_SET_INT, &args![tile, trait_id, value]);
}

fn tile_set_float(e: &mut Engine, tile: u32, trait_id: u32, value: f32) {
    e.call(TILE_SET_FLOAT, &args![tile, trait_id, value, 1u32]);
}

/// The object a `NiPointer` member at `member` holds.
fn ni_pointer_get(e: &mut Engine, member: u32) -> u32 {
    e.call(NI_POINTER_GET, &args![member]).u32()
}

// Addresses used by the functions of this file (see each function).
const DEBUG_TEXT_SHUTDOWN: u32 = 0x00a0_dae0;
const FONT_MANAGER_SHUTDOWN: u32 = 0x00a1_6850;
/// A member function whose body only stores `this` (it does nothing).
const EMPTY_MEMBER_FUNCTION: u32 = 0x0048_3710;
/// Byte set while the root tiles are deleted.
const TILE_DELETING: u32 = 0x011d_8909;
const MENU_MANAGER_SHUTDOWN: u32 = 0x0071_e360;
const FREE_TIMERS: u32 = 0x0071_6450;
const LIST_CLEAR: u32 = 0x0047_0470;
const LIST_AT_011D8B54: u32 = 0x011d_8b54;
const STATIC_OBJECT_SHUTDOWN_A: u32 = 0x0071_8bf0;
const MESSAGE_MENU_SHUTDOWN: u32 = 0x007a_a480;
const CLEAR_TEMP_MODEL: u32 = 0x0070_7820;
const START_MENU_SHUTDOWN: u32 = 0x007c_e6b0;
const STATIC_OBJECT_SHUTDOWN_B: u32 = 0x0071_3c00;
const POINTER_LIST_REMOVE_ALL: u32 = 0x004e_d900;
const SCRIPT_POINTER_LIST: u32 = 0x011f_3324;
const GLOBAL_OBJECT_8CE8: u32 = 0x011d_8ce8;
const OBJECT_8CE8_DESTRUCT: u32 = 0x0071_b040;
const GLOBAL_OBJECT_A0C4: u32 = 0x011d_a0c4;
const VIEW_CASTER_DESTRUCT: u32 = 0x0063_1a50;
const FLOAT_HOLDER_BACKGROUND: u32 = 0x011d_3174;
const STRING_HOLDER_A: u32 = 0x011d_475c;
const STRING_HOLDER_B: u32 = 0x011f_3368;
const STRING_LIST_A: u32 = 0x011d_eec0;
const STRING_LIST_B: u32 = 0x011d_8d44;
/// `BSFogProperty` setter of two floats (`+0x2c`, `+0x30`).
const SET_FOG_RANGE: u32 = 0x005b_f9a0;
/// The global the `Controls` object is found from, and the getter.
const CONTROLS_OWNER: u32 = 0x011d_ea0c;
const CONTROLS_GET: u32 = 0x0087_7720;
/// `Controls::ClearKeystrokes` (Xbox PDB).
const CLEAR_KEYSTROKES: u32 = 0x00a2_37b0;
/// `HUDMainMenu::SetInfoForRef` (Xbox PDB, `cdecl`).
const HUD_SET_INFO_FOR_REF: u32 = 0x0077_5a00;
/// `MenuConsole::Instance(create)` (Xbox PDB, `cdecl`) and
/// `MenuConsole::OnEnterMenuMode` (Xbox PDB).
const MENU_CONSOLE_INSTANCE: u32 = 0x0071_b160;
const MENU_CONSOLE_ON_ENTER_MENU_MODE: u32 = 0x0071_d770;
/// The manager object itself (`004b7210`, the word at `011d8a80`).
const GET_MANAGER: u32 = 0x004b_7210;
/// `BSAudio::QInstance` (Xbox PDB) and the pause functions called on it.
const AUDIO_INSTANCE: u32 = 0x0045_3a70;
const AUDIO_PAUSE_TYPE: u32 = 0x00ad_8510;
const AUDIO_PAUSE_TYPE_6: u32 = 0x0070_bbe0;
const AUDIO_UNPAUSE_TYPE_6: u32 = 0x0070_bc00;
const AUDIO_UNPAUSE_TYPE_4000_0000: u32 = 0x0070_bbc0;
/// True when the manager's mode word is not 1 (`007023a0`).
const MENU_MODE_IS_NOT_ONE: u32 = 0x0070_23a0;
/// `InterfaceManager::UpdateAllTimers` (Xbox PDB).
const UPDATE_ALL_TIMERS: u32 = 0x0071_6320;

// Globals, objects and callees used by `Idle` and the gamepad/pick code.
const PLAYER_SINGLETON: u32 = 0x011d_ea3c;
const LAST_MENU_MODE: u32 = 0x0119_f530;
const CONTROLLER_STATE: u32 = 0x011d_8a6c;
const LAST_HAS_CONTROLLER: u32 = 0x011d_8c70;
const LAST_HAS_CONTROLLER_GUARD: u32 = 0x011d_8c74;
const MESSAGE_TITLE_HOLDER: u32 = 0x011d_38b8;
const TEXT_HOLDER_CONTROLLER_WAS_SET: u32 = 0x011d_49b4;
const TEXT_HOLDER_CONTROLLER_WAS_CLEAR: u32 = 0x011d_2760;
const SHOW_MESSAGE_BOX: u32 = 0x0070_3e80;
const CONTROLLER_CHANGED: u32 = 0x0071_9630;
const PRELOAD_MAIN_MENUS: u32 = 0x0071_7660;
/// Controls object queries (see each use for the arguments).
const CONTROLS_QUERY_00A239E0: u32 = 0x00a2_39e0;
const CONTROLS_QUERY_00A23A50: u32 = 0x00a2_3a50;
const CONTROLS_QUERY_00A24660: u32 = 0x00a2_4660;
const CONTROLS_QUERY_00A24180: u32 = 0x00a2_4180;
const CONTROLS_QUERY_00A238A0: u32 = 0x00a2_38a0;
const CONTROLS_AXIS_00A23390: u32 = 0x00a2_3390;
const CONTROLS_NEXT_EVENT: u32 = 0x00a2_3820;
const CONTROLS_CLEAR_USER_ACTIONS: u32 = 0x00a2_53d0;
const GET_PIPBOY: u32 = 0x0070_4370;
const PIPBOY_UPDATE_LIGHT_EFFECT: u32 = 0x007f_a540;
const HUD_EFFECTS_UPDATE: u32 = 0x007f_7830;
const VATS_EFFECTS_UPDATE: u32 = 0x0080_0370;
const GAME_STATE_ID: u32 = 0x0045_8030;
const CONDITION_CHECK_005A03F0: u32 = 0x005a_03f0;
const PLAYER_FLAG_00950090: u32 = 0x0095_0090;
const ACTOR_GET_IRON_SIGHTS: u32 = 0x008b_bc10;
const WEAPON_STATE_GETTER: u32 = 0x0044_ddc0;
const FORM_GETTER_0044DDC0: u32 = 0x0044_ddc0;
const WEAPON_STATE_OBJECT: u32 = 0x011f_2250;
const HUD_SET_MENU_MODE: u32 = 0x0077_1700;
const GET_ENTER_STACK_TOP: u32 = 0x0071_4f00;
const QUERY_00719AE0: u32 = 0x0071_9ae0;
const ACTION_00718930: u32 = 0x0071_8930;
const IS_TOP_MENU_ID: u32 = 0x0070_2450;
const IS_TOP_MENU_FADED_IN: u32 = 0x0070_24e0;
const IS_PIPBOY_MENU_TOPMOST: u32 = 0x0071_7920;
const OBJECT_FLAG_0070ED80: u32 = 0x0070_ed80;
const PIPBOY_MENU_CLASS: u32 = 0x0070_ede0;
const TILE_GET_MENU_BY_CLASS: u32 = 0x00a0_9030;
const IS_CONSOLE_VISIBLE: u32 = 0x0070_3d50;
const IS_IN_GAME_LOADING_MENU_OPEN: u32 = 0x0070_5ea0;
const XUI_IS_UP: u32 = 0x0070_edf0;
const GET_MOUSE_OVER_TARGET: u32 = 0x0097_ae90;
const TILE_IS_VISIBLE: u32 = 0x00a0_40a0;
const TILE_IS_TRUE: u32 = 0x00a0_1230;
const GET_DEFAULT_FOCUS: u32 = 0x0071_5ca0;
const CONSOLE_PRIMARY_DOWN: u32 = 0x004a_4020;
const FRAME_TIME_GETTER: u32 = 0x0084_d030;
const GET_SCREEN_SCALE: u32 = 0x0070_ecb0;
const NI_POINT2_CONSTRUCT: u32 = 0x0045_2dc0;
const GET_MENU_HEIGHT: u32 = 0x0071_7860;
const PIPBOY_MARGIN: u32 = 0x0106_f070;
const TILE_GET_VALUE: u32 = 0x00a0_11b0;
const TILE_GET_VALUE_Q: u32 = 0x00a0_0e90;
const TILE_GET_MENU: u32 = 0x00a0_3c90;
const PICK_TILE: u32 = 0x0071_26c0;
const DO_LEAVE: u32 = 0x0071_7ef0;
const DO_ENTER: u32 = 0x0071_7e70;
const DO_WHEEL_MOVE: u32 = 0x0071_8080;
const SET_CURSOR_OFFSET_X: u32 = 0x009e_3360;
const SET_CURSOR_OFFSET_Y: u32 = 0x008d_8500;
const GET_CURSOR_OFFSET_X: u32 = 0x005f_5f80;
const GET_CURSOR_OFFSET_Y: u32 = 0x0070_ec90;
const TILE_GET_POSITION_X: u32 = 0x00a0_13d0;
const TILE_GET_POSITION_Y: u32 = 0x00a0_1440;
const TILE_PLAY_TILE_SOUND: u32 = 0x00a0_b110;
const TILE_CHILD_LIST_FIND: u32 = 0x0049_c680;
const CLEAR_MOUSE_OVER_TARGET: u32 = 0x0070_6cb0;
const TILE_PARENT: u32 = 0x0045_cd60;
const GET_WHEEL_STEPS: u32 = 0x0070_ec70;
const PICK_LIST_COUNT: u32 = 0x005a_e380;
const MENU_STATE: u32 = 0x0059_bb30;
const FTOL: u32 = 0x00ec_62c0;
const RENDERED_MENU_OR_PIPBOY: u32 = 0x0070_7af0;
const MENU_FLAG_QUERY_004A4040: u32 = 0x004a_4040;
const SOUND_HANDLE_CONSTRUCT: u32 = 0x0041_a250;
const SOUND_HANDLE_ASSIGN: u32 = 0x0041_8900;
const AUDIO_GET_SOUND_HANDLE_BY_NAME: u32 = 0x00ad_7550;
const UI_MENU_MODE_SOUND: u32 = 0x0106_f060;
const SOUND_HANDLE_PLAY: u32 = 0x00ad_8830;
const SOUND_HANDLE_RELEASE: u32 = 0x00ad_8d10;
const SET_STATS_MENU_VISIBLE: u32 = 0x0070_4c10;
const SET_INVENTORY_MENU_VISIBLE: u32 = 0x0070_48f0;
const SET_MAP_MENU_VISIBLE: u32 = 0x0070_4170;
const MENU_CONSOLE_IDLE: u32 = 0x0071_b210;
const KEY_REPEAT_RESET: u32 = 0x0071_66f0;
const TRANSLATE_KEY_EVENT: u32 = 0x0071_54b0;
const KEY_REPEAT_00716730: u32 = 0x0071_6730;
const MODIFIER_FOUR_DOWN: u32 = 0x0070_ecf0;
const GET_FRONTMOST_MENU: u32 = 0x0072_0e60;
const FORMAT_STRING: u32 = 0x0040_6d00;
const FORMAT_PC_BUTTON: u32 = 0x0106_f04c;
const PC_BUTTON_PREFIX: u32 = 0x0106_f054;
/// `[this + 4]` of the object (a plain getter used on tiles, lists and
/// menus).
const GET_FIELD_AT_4: u32 = 0x0072_6070;
const TILE_GET_STRING: u32 = 0x00a0_11f0;
const TILE_GET_CHILD_BY_NAME: u32 = 0x00a0_3da0;
const PLAY_MENU_SOUND: u32 = 0x0071_7280;
const SHOW_ICON_MESSAGE: u32 = 0x0070_52f0;
const MESSAGE_HOLDER_19: u32 = 0x011d_4cb4;
const MESSAGE_HOLDER_1A: u32 = 0x011d_3fd0;
const MESSAGE_ICON_SIZE: u32 = 0x0101_62c0;
const MESSAGE_ICON_PATH: u32 = 0x0102_08a0;
const CONSOLE_KEY_FLAG_GET: u32 = 0x0070_ed10;
const CONSOLE_KEY_FLAG_SET: u32 = 0x0070_ed20;
const MENU_CONSOLE_TOGGLE_VISIBLE: u32 = 0x0071_d580;
const ADD_TO_ENTER_STACK: u32 = 0x0071_4d90;
const POP_FROM_ENTER_STACK: u32 = 0x0071_4fd0;
const MENU_MANAGER_IS_MENU_OPEN: u32 = 0x0072_0f90;
const START_MENU_ALLOWED: u32 = 0x0119_f348;
const START_MENU_CLOSE: u32 = 0x007c_e7a0;
const START_MENU_CREATE: u32 = 0x007c_b7d0;
const START_MENU_SETTING: u32 = 0x011c_3ea4;
const QUEUE_PENDING_00070EC00: u32 = 0x0070_ec00;
const QUEUE_MENU_CREATE: u32 = 0x0070_9470;
const DEBUG_TEXT_UPDATE: u32 = 0x00a0_df80;
const MENU_LOOP_ABORT: u32 = 0x011d_8a85;
const LIST_NEXT_ELEMENT: u32 = 0x0057_cbe0;
const CHECK_MENU_BUTTON: u32 = 0x0071_7a40;
const CLEAR_MENU_BUTTON: u32 = 0x0071_7de0;
const TUTORIAL_MANAGER_UPDATE: u32 = 0x0071_82e0;
const GET_MENUS_ROOT: u32 = 0x0058_6150;
const LEVEL_UP_MENU_CREATE: u32 = 0x0078_4c80;
const CURSOR_STRING_PENDING: u32 = 0x011d_8904;
const LEVEL_UP_MENU_PENDING: u32 = 0x011d_8905;
const PLAYER_CAST_EAT_DRINK_ITEMS: u32 = 0x0095_d090;
const PLAYER_CAST_QUEUED_ENCHANTMENTS: u32 = 0x0095_ec40;
// `UpdatePipboy`.
const CONTAINER_MENU_CLOSE: u32 = 0x0075_b750;
const RENDERED_MENU_CLOSE: u32 = 0x007f_fd50;
const ACTOR_GET_ANIM_ACTION: u32 = 0x008a_7570;
const PLAYER_GET_ANIMATION: u32 = 0x0095_0a60;
const ANIMATION_CLEAR_GROUP: u32 = 0x0049_6080;
const PLAYER_FORCE_TEMP_1ST_PERSON: u32 = 0x0095_0460;
const ANIMATION_PLAY_GROUP: u32 = 0x0049_4740;
const GET_PIPBOY_STATIC: u32 = 0x0070_5990;
const PIPBOY_FADE_LIGHT_EFFECT: u32 = 0x007f_a7a0;
const PIPBOY_ACCESS_UP_SOUND: u32 = 0x0106_f0c4;
const IMAGE_SPACE_GET_HIT: u32 = 0x005d_2860;
const IMAGE_SPACE_STOP: u32 = 0x0052_9c90;
const PLAYER_STATE_00962590: u32 = 0x0096_2590;
const INTERFACE_SHOW_MENUS: u32 = 0x0070_3500;
const ANIMATION_GET_SEQUENCE: u32 = 0x0049_1040;
const MAP_MENU_NEEDS_TIDY: u32 = 0x007a_1750;
const MAP_MENU_TIDY: u32 = 0x0079_ffb0;
const PLAYER_IS_PIPBOY_ACTIVE: u32 = 0x0096_7ae0;
const ANIMATION_GROUP_ID: u32 = 0x0043_01b0;
const MAP_MENU_CLEAR_MAP_MEMORY: u32 = 0x007a_1490;
const RELOAD_DYNAMIC_IDLE_ON_1ST_PERSON: u32 = 0x0092_1250;
const GET_SAVED_ACQUIRE_OBJECT: u32 = 0x008d_8520;
const ACTOR_IS_WEAPON_DRAWN: u32 = 0x008a_16d0;
const WEAPON_TYPE_INDEX: u32 = 0x0044_6390;
const WEAPON_TABLE: u32 = 0x0118_a838;
const CLEAR_GUN_WOBBLE: u32 = 0x008d_6a80;
const ANIMATION_NODE: u32 = 0x0055_85e0;
const SEQUENCE_END_TIME: u32 = 0x0063_9aa0;
const ANIMATION_ZERO_GLOBAL_TRANSFORM: u32 = 0x0048_f7f0;
const ANIM_GROUP_GET_TIME: u32 = 0x005f_3780;
const ANIMATION_UPDATE: u32 = 0x0049_1180;
const ANIMATION_UPDATE_BIP_ONLY: u32 = 0x0049_bca0;
const ANIMATION_UPDATE_SCENE_GRAPH_NO_CONTROLLER: u32 = 0x0049_3bd0;
const IDLE_MANAGER_BUSY: u32 = 0x011c_ab24;
const LOG_WARNING: u32 = 0x005b_5e40;
const MISSING_DYNAMIC_IDLE_MESSAGE: u32 = 0x0106_f078;
const PIPBOY_MANAGER_UPDATE: u32 = 0x007f_8a80;
const ENTER_RENDERED_MENU: u32 = 0x0071_81d0;
const PLAYER_QUERY_005737E0: u32 = 0x0057_37e0;
const PLAYER_QUERY_004EAF60: u32 = 0x004e_af60;
const PLAYER_QUERY_0093A740: u32 = 0x0093_a740;
const PLAYER_QUERY_005721E0: u32 = 0x0057_21e0;
const HOT_KEYS_OWNER: u32 = 0x011d_96c0;
const TILE_IS_ACCEPTING_EVENTS: u32 = 0x0071_6910;
const PLAYER_BUSY_FLAG: u32 = 0x011e_0780;
const WEAPON_STATE_FLOAT: u32 = 0x008d_1cf0;
const FLAGS_OWNER_8804: u32 = 0x011d_8804;
const FLAGS_QUERY_00701450: u32 = 0x0070_1450;
const SUB_OBJECT_QUERY_00701740: u32 = 0x0070_1740;
const FLAGS_OWNER: u32 = 0x011d_aac0;
const HAS_FLAG: u32 = 0x004a_4080;
const INVENTORY_MENU: u32 = 0x011d_9ea4;
const KEYRING_TRAIT: u32 = 0x011d_9eb8;
// `GetTopGamepadButton` and `DoGamepadEvent`.
const REPEAT_DELAY_HOLDER_FAST: u32 = 0x011d_8aa4;
const REPEAT_DELAY_HOLDER_FIRST: u32 = 0x011d_8b38;
const DO_GAMEPAD: u32 = 0x0071_7f80;
const KEY_REPEAT_00705B10: u32 = 0x0070_5b10;
const LIST_PREVIOUS_ELEMENT: u32 = 0x0068_3520;
const SCAN_FOR_MAX_FOCUS: u32 = 0x0071_60f0;
const SET_CURRENT_FOCUS_TARGET: u32 = 0x0071_5860;
const TILE_GET_FIRST_REF_COPY: u32 = 0x00a0_3f70;
const LIST_PUSH_FRONT_005AE3D0: u32 = 0x005a_e3d0;
const LIST_CONTAINS: u32 = 0x005f_65d0;
// `00710880`.
const NI_PICK_CONSTRUCT: u32 = 0x00e9_8f20;
const NI_PICK_DESTRUCT: u32 = 0x00e9_8fa0;
const NI_PICK_SET_FLAG: u32 = 0x0045_8b30;
const GET_PICK_ROOT: u32 = 0x0084_e3a0;
const PICK_ROOT_OWNER: u32 = 0x011d_ea10;
const NI_PICK_SET_ROOT: u32 = 0x0070_5fc0;
const NI_PICK_SET_MODE: u32 = 0x006e_cd40;
const GET_SCENE_GRAPH: u32 = 0x0045_c670;
const SCENE_GRAPH_SET_CAMERA_FOV: u32 = 0x00c5_2020;
const SCENE_GRAPH_GET_CAMERA: u32 = 0x0066_29f0;
const CAMERA_BUILD_PICK_RAY: u32 = 0x00a7_1080;
const NI_PICK_PICK_OBJECTS: u32 = 0x00e9_8e20;
const NI_PICK_GET_RESULTS: u32 = 0x0050_0940;
const RESULTS_COUNT: u32 = 0x0044_edb0;
const RESULTS_GET: u32 = 0x0096_8670;
const RESULT_OBJECT: u32 = 0x0045_8b50;
const FIND_REFERENCE_FOR_3D: u32 = 0x0056_f930;
const LIST_ADD: u32 = 0x0090_5820;
// `DisplayCurrentPickRef`.
const GET_SETTING_VALUE: u32 = 0x0043_d4d0;
const DEBUG_TEXT_TOP_SETTING: u32 = 0x011d_ebac;
const FONT_LEVEL_SETTING: u32 = 0x011f_33c8;
const FONT_MANAGER_GET: u32 = 0x005b_d5b0;
const FONT_MANAGER_GET_FONT: u32 = 0x005b_d5c0;
const FONT_HEIGHT: u32 = 0x005b_d580;
const SET_PICK_REF_DISPLAY: u32 = 0x0071_4d70;
const FORM_ID_GETTER: u32 = 0x0084_e3a0;
const REFR_NAME: u32 = 0x0055_d520;
const FORMAT_INTO_STRING: u32 = 0x0040_6f60;
const FORMAT_NAME_AND_ID: u32 = 0x0103_a1c8;
const DEBUG_TEXT_X: u32 = 0x0103_a1c4;
const DEBUG_TEXT_INSTANCE: u32 = 0x00a0_d9e0;
const DEBUG_TEXT_PRINT: u32 = 0x00a0_f8b0;
const EMPTY_TEXT: u32 = 0x0101_1584;
const DEBUG_TEXT_BOTTOM: u32 = 0x011d_8c78;
const STRING_CONSTRUCT: u32 = 0x0040_37b0;
const STRING_DESTROY: u32 = 0x0040_37d0;
const REFR_EXTRA_DATA_LIST: u32 = 0x005d_43c0;
const EXTRA_GET_SCRIPT: u32 = 0x0041_8800;
const SCRIPT_NAME_FORMAT: u32 = 0x0106_f274;
const FURNITURE_FORMAT: u32 = 0x0106_f254;
const REFR_GET_OWNER: u32 = 0x0056_7790;
const OWNER_FORMAT: u32 = 0x0106_f23c;
const EXTRA_GET_COUNT: u32 = 0x0041_8770;
const COUNT_FORMAT: u32 = 0x0106_f230;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const FROM_TYPE: u32 = 0x0118_41cc;
const ACTOR_TYPE: u32 = 0x0118_46d4;
const ACTOR_PACKAGE_FORMAT: u32 = 0x0106_f208;
const ACTOR_FURNITURE_FORMAT: u32 = 0x0106_f1e8;
const REFR_IS_FURNITURE: u32 = 0x0056_8680;
const REFR_GET_ROTATION: u32 = 0x0043_0830;
const RADIANS_TO_DEGREES: u32 = 0x0102_f248;
const HEADING_FORMAT: u32 = 0x0106_f1d4;
const MARKER_CONSTRUCT: u32 = 0x0050_9890;
const REFR_GET_MARKER_AT_INDEX: u32 = 0x0056_8500;
const REFR_GET_FURNITURE_DATA: u32 = 0x007a_f430;
const MARKER_KIND: u32 = 0x0050_94b0;
const FURNITURE_GET_MARKER_ENABLED: u32 = 0x0050_9450;
const FURNITURE_IS_SIT_MARKER: u32 = 0x0050_9510;
const SIT_TEXT: u32 = 0x0106_4acc;
const SLEEP_TEXT: u32 = 0x0106_37d4;
const DISABLED_MARKER_FORMAT: u32 = 0x0106_f178;
const REFR_GET_SCALE: u32 = 0x0056_7400;
const FURNITURE_GET_MARKER_TARGET_OFFSET: u32 = 0x0050_9920;
const REFR_GET_MARKER_USED: u32 = 0x0056_7f80;
const USED_TEXT: u32 = 0x0106_f1cc;
const UNUSED_TEXT: u32 = 0x0106_f1c4;
const FURNITURE_GET_MARKER_ANGLE: u32 = 0x0050_99c0;
const MARKER_FORMAT: u32 = 0x0106_f190;
const REFR_GET_LOCK: u32 = 0x0056_9160;
const LOCK_IS_LOCKED: u32 = 0x0050_21a0;
const LOCKED_TEXT: u32 = 0x0106_0314;
const UNLOCKED_TEXT: u32 = 0x0106_f16c;
const LOCK_GET_LEVEL: u32 = 0x0043_09e0;
const LOCK_LEVEL_TABLE: u32 = 0x0118_4a98;
const LOCK_FORMAT: u32 = 0x0106_f15c;
const MAP_MARKER_GET_LOCATION_NAME: u32 = 0x0040_8da0;
const KEY_FORMAT: u32 = 0x0106_f150;
const REFR_GET_TELEPORT_DATA: u32 = 0x0056_8e50;
const DOOR_TELEPORT_GET_CELL: u32 = 0x0043_a2b0;
const UNKNOWN_TEXT: u32 = 0x0106_f148;
const PERSISTENT_TEXT: u32 = 0x0106_f13c;
const EXTRA_IS_PERSISTENT: u32 = 0x0041_d460;
const TELEPORT_FORMAT: u32 = 0x0106_f11c;
const REFR_FLAGS: u32 = 0x0057_2d10;
const STRING_COPY: u32 = 0x0040_6d30;
const FLAGS_PREFIX: u32 = 0x0106_f10c;
const STRING_APPEND: u32 = 0x0040_6d50;
const FLAG_TEXT_1: u32 = 0x0106_f100;
const FLAG_TEXT_2: u32 = 0x0106_f0f4;
const FLAG_TEXT_4: u32 = 0x0106_f0ec;
const FLAG_TEXT_8: u32 = 0x0106_f0d8;
const TRIM_TEXT: u32 = 0x0046_4f30;
// `007118d0`.
const CONTROLS_FLAG_00711E00: u32 = 0x0071_1e00;
const STICK_SCALE_HOLDER: u32 = 0x011d_8c5c;
const MOUSE_SETTING_00711DA0: u32 = 0x0071_1da0;
const MOUSE_SETTING_00711D80: u32 = 0x0071_1d80;
const MOUSE_LEVEL_00711D60: u32 = 0x0071_1d60;
const MOUSE_SETTING_00711DC0: u32 = 0x0071_1dc0;
const MOUSE_SETTING_00711DE0: u32 = 0x0071_1de0;
const NODE_TRANSLATION: u32 = 0x0043_c490;
const EDGE_RIGHT: u32 = 0x0071_1e40;
const EDGE_LEFT: u32 = 0x0071_1e20;
const EDGE_TOP: u32 = 0x0071_1e80;
const EDGE_BOTTOM: u32 = 0x0071_1e60;
const CURSOR_TILT_SCALE: u32 = 0x0106_f290;
const NODE_SET_TRANSLATE_VECTOR: u32 = 0x0044_0460;
const FLOAT_MIN: u32 = 0x0040_ebd0;
const FLOAT_MAX: u32 = 0x0040_4010;
const ABS: u32 = 0x00ec_7d40;
const MOUSE_LEVEL_DIVISOR: u32 = 0x0102_fc70;
const CURSOR_HORIZONTAL_SCALE: u32 = 0x0106_e960;
const CURSOR_VERTICAL_SCALE: u32 = 0x0106_e7f8;
const ONE_DOUBLE: u32 = 0x0101_2070;

pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00709fd0, interface_manager_initialize(i32, u8)),
        entry!(0x0070a0b0, fn_0070a0b0()),
        entry!(
            0x0070a100,
            interface_manager_scalar_deleting_destructor(
                Ptr<InterfaceManager>,
                u32,
            ) -> Ptr<InterfaceManager>
        ),
        entry!(
            0x0070a130,
            interface_manager_construct(Ptr<InterfaceManager>) -> Ptr<InterfaceManager>
        ),
        entry!(0x0070a8a0, timer_construct(Ptr<Timer>) -> Ptr<Timer>),
        entry!(0x0070a8e0, fn_0070a8e0(Ptr) -> Ptr),
        entry!(0x0070a900, fn_0070a900(Ptr, u8)),
        entry!(
            0x0070a920,
            interface_manager_destruct(Ptr<InterfaceManager>)
        ),
        entry!(0x0070aca0, fn_0070aca0()),
        entry!(0x0070acb0, fn_0070acb0()),
        entry!(0x0070ad00, fn_0070ad00(Ptr, u32) -> Ptr),
        entry!(0x0070ad30, fn_0070ad30()),
        entry!(0x0070ad80, fn_0070ad80(Ptr, u32) -> Ptr),
        entry!(
            0x0070adb0,
            interface_manager_init(Ptr<InterfaceManager>, u8)
        ),
        entry!(
            0x0070b240,
            interface_manager_init_second(Ptr<InterfaceManager>)
        ),
        entry!(
            0x0070b5d0,
            tile_image_construct(Ptr<TileImage>) -> Ptr<TileImage>
        ),
        entry!(0x0070b670, tile_image_get_type(Ptr<TileImage>) -> u32),
        entry!(0x0070b680, tile_image_get_type_name(Ptr<TileImage>) -> u32),
        entry!(
            0x0070b690,
            tile_image_scalar_deleting_destructor(Ptr<TileImage>, u32) -> Ptr<TileImage>
        ),
        entry!(0x0070b6c0, tile_image_destruct(Ptr<TileImage>)),
        entry!(
            0x0070b8f0,
            interface_manager_pre_idle_stuff(Ptr<InterfaceManager>)
        ),
        entry!(0x0070bba0, fn_0070bba0(Ptr)),
        entry!(0x0070c4a0, interface_manager_idle(Ptr<InterfaceManager>)),
        entry!(0x0070ec20, fn_0070ec20(Ptr, i32, i32)),
        entry!(0x0070ec50, fn_0070ec50(Ptr) -> u8),
        entry!(0x0070ecd0, fn_0070ecd0(Ptr<InterfaceManager>) -> bool),
        entry!(0x0070ed30, inventory_menu_is_keyring_open() -> bool),
        entry!(0x0070edc0, fn_0070edc0(Ptr) -> u8),
        entry!(0x0070ee30, fn_0070ee30() -> bool),
        entry!(
            0x0070ee80,
            interface_manager_update_pipboy(Ptr<InterfaceManager>)
        ),
        entry!(0x0070f490, fn_0070f490(Ptr, i32) -> u32),
        entry!(0x0070f4e0, fn_0070f4e0(Ptr<InterfaceManager>, u32, u32)),
        entry!(0x0070f670, fn_0070f670(Ptr) -> u32),
        entry!(0x0070f690, fn_0070f690(Ptr<InterfaceManager>, u32)),
        entry!(
            0x0070f6e0,
            interface_manager_do_gamepad_event(Ptr<InterfaceManager>, i32)
        ),
        entry!(
            0x00710060,
            interface_manager_get_top_gamepad_button(Ptr<InterfaceManager>, u8, u8, Ptr) -> i32
        ),
        entry!(0x00710880, fn_00710880(Ptr<InterfaceManager>)),
        entry!(0x00710ab0, fn_00710ab0(Ptr) -> f32),
        entry!(
            0x00710ad0,
            interface_manager_display_current_pick_ref(Ptr<InterfaceManager>)
        ),
        entry!(0x007118d0, fn_007118d0(Ptr<InterfaceManager>)),
        // @@ENTRIES@@
    ]
}

// Translated from 00709fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterfaceManager::Initialize` (Xbox PDB, static, `cdecl`): records
/// whether an Xbox 360 controller answers, creates the manager if there is
/// none, runs the first (`phase` 1, with `flag`) or second (`phase` 2)
/// initialization, and clears the sound and voice pause counters.
pub fn interface_manager_initialize(e: &mut Engine, phase: i32, flag: u8) {
    let state = e.with_stack(0x10, |e, state| {
        e.call(XINPUT_GET_STATE, &args![0u32, state]).u32()
    });
    e.set_global(HAS_360_CONTROLLER, (state == 0) as u8);
    if e.global::<u32>(MANAGER_SINGLETON) == 0 {
        let manager = construct_new(e, OPERATOR_NEW, 0x580, |e, block| {
            interface_manager_construct(e, block.cast()).cast()
        });
        e.set_global(MANAGER_SINGLETON, manager.addr());
    }
    let manager: Ptr<InterfaceManager> = Ptr::new(e.global(MANAGER_SINGLETON));
    if phase == 1 {
        interface_manager_init(e, manager, flag);
    } else if phase == 2 {
        interface_manager_init_second(e, manager);
    }
    let manager: Ptr<InterfaceManager> = Ptr::new(e.global(MANAGER_SINGLETON));
    e.set(manager, InterfaceManager::iSoundPauses, 0);
    let manager: Ptr<InterfaceManager> = Ptr::new(e.global(MANAGER_SINGLETON));
    e.set(manager, InterfaceManager::iVoicePauses, 0);
}

// Translated from 0070a0b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destroys the interface manager (through its deleting destructor) and
/// clears the singleton; does nothing when there is none.
pub fn fn_0070a0b0(e: &mut Engine) {
    let manager: Ptr<InterfaceManager> = Ptr::new(e.global(MANAGER_SINGLETON));
    if !manager.is_null() {
        interface_manager_scalar_deleting_destructor(e, manager, 1);
        e.set_global(MANAGER_SINGLETON, 0u32);
    }
}

// Translated from 0070a100 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterfaceManager::__scalar_deleting_destructor_` (Xbox PDB): destroys
/// the manager and, when bit 0 of `flags` is set, frees its memory.
pub fn interface_manager_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<InterfaceManager>,
    flags: u32,
) -> Ptr<InterfaceManager> {
    interface_manager_destruct(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0070a130 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterfaceManager::InterfaceManager` (Xbox PDB): constructs the embedded
/// members, zeroes or defaults every field, derives the one-to-one
/// distance from the screen width, creates the alpha property, the two
/// scene graphs (`Menu3DRoot`, `MenuRoot`), the menu manager, the view
/// caster, the timer list head and the two shader accumulators. Returns
/// `this`.
pub fn interface_manager_construct(
    e: &mut Engine,
    this: Ptr<InterfaceManager>,
) -> Ptr<InterfaceManager> {
    let base = this.addr();
    e.call(NI_POINTER_CONSTRUCT, &args![base + 0x4, 0u32]);
    e.call(NI_POINTER_CONSTRUCT, &args![base + 0x8, 0u32]);
    e.call(MEMBER_CONSTRUCTOR_EMPTY, &args![base + 0x2c]);
    e.call(MEMBER_CONSTRUCTOR_EMPTY, &args![base + 0x38]);
    e.call(SIMPLE_LIST_CONSTRUCT, &args![base + 0x70]);
    e.call(NI_POINTER_CONSTRUCT, &args![base + 0x8c, 0u32]);
    e.call(NI_POINTER_CONSTRUCT, &args![base + 0x90, 0u32]);
    e.call(NI_POINTER_CONSTRUCT, &args![base + 0xac, 0u32]);
    e.call(MEMBER_CONSTRUCTOR_EMPTY, &args![base + 0x104]);
    e.call(HUD_EFFECT_MANAGER_CONSTRUCT, &args![base + 0x178]);
    e.call(VATS_EFFECT_MANAGER_CONSTRUCT, &args![base + 0x1dc]);
    e.call(SIMPLE_LIST_CONSTRUCT, &args![base + 0x4c4]);
    fn_0070a8e0(e, Ptr::new(base + 0x4d4));
    with_scope_guard(e, 0x11d, |e| {
        e.set(this, InterfaceManager::field_4d0, 1.0);
        e.set(this, InterfaceManager::bFirstInit, 0);
        e.set(this, InterfaceManager::bSecondInit, 0);
        e.set(this, InterfaceManager::pInterfaceRoot, 0);
        e.set(this, InterfaceManager::pCursorRoot, 0);
        e.set(this, InterfaceManager::pPlayerRoot, 0);
        e.set(this, InterfaceManager::pMenusRoot, 0);
        e.set(this, InterfaceManager::pDebugTextRoot, 0);
        // `NiPoint3(0, 0, 0)` built in a temporary and copied into the
        // activate pick location (`+0x104`) word by word.
        let origin = e.with_stack(0xc, |e, temp| {
            let at = e
                .call(NI_POINT3_CONSTRUCT, &args![temp, 0.0f32, 0.0f32, 0.0f32])
                .u32();
            [e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8)]
        });
        for (i, word) in origin.iter().enumerate() {
            e.mem.set_u32(base + 0x104 + 4 * i as u32, *word);
        }
        e.set(this, InterfaceManager::bFuzzyActivatePick, 0);
        e.set(this, InterfaceManager::pActivateRef, 0);
        e.set(this, InterfaceManager::pTelekinesisRef, 0);
        e.set(this, InterfaceManager::pPickRef, 0);
        e.set(this, InterfaceManager::pReticleRef, 0);
        e.set(this, InterfaceManager::pCrossHairRef, 0);
        e.set(this, InterfaceManager::pPlayerLight, 0);
        e.set(this, InterfaceManager::fOneToOneDistance, 0.0);
        e.set(this, InterfaceManager::cMenuMode, 1);
        e.set(this, InterfaceManager::bLockMenuModeForFade, 0);
        e.set(this, InterfaceManager::bExternalForcedMenuMode, 0);
        e.set(this, InterfaceManager::cStatsPageNumber, 0xff);
        e.set(this, InterfaceManager::cInventoryPageNumber, 0xff);
        e.set(this, InterfaceManager::cMagicPageNumber, 0xff);
        e.set(this, InterfaceManager::cMapPageNumber, 0xff);
        e.set(this, InterfaceManager::iCharHit, 0);
        e.set(this, InterfaceManager::pMouseOverTarget, 0);
        e.set(this, InterfaceManager::pOverTileTarget, 0);
        e.set(this, InterfaceManager::pOverTileMenu, 0);
        e.set(this, InterfaceManager::pDragOverTileTarget, 0);
        e.set(this, InterfaceManager::pDragOverTileMenu, 0);
        e.call(NI_POINTER_ASSIGN, &args![base + 0xac, 0u32]);
        e.set(this, InterfaceManager::iPickDistance, 0x50);
        e.set(this, InterfaceManager::fMouseWheel, 0.0);
        e.set(this, InterfaceManager::iDragStartX, 0);
        e.set(this, InterfaceManager::iDragStartY, 0);
        e.set(this, InterfaceManager::iCursorOffsetX, 0);
        e.set(this, InterfaceManager::iCursorOffsetY, 0);
        e.set(this, InterfaceManager::fMouseHeldTime, 0.0);
        e.set(this, InterfaceManager::pCursor, 0);
        e.set(this, InterfaceManager::bShiftDown, 0);
        e.set(this, InterfaceManager::iModifierKeys, 0);
        e.set(this, InterfaceManager::iRepeatingKey, 0);
        e.set(this, InterfaceManager::uKeyDownTime, 0);
        e.set(this, InterfaceManager::iLastGamepadEvent, 0);
        e.set(this, InterfaceManager::pTargetReticle, 0);
        e.set(this, InterfaceManager::pSafeZone, 0);
        e.set(this, InterfaceManager::pDragTarget, 0);
        e.set(this, InterfaceManager::bFullHelp, 0);
        e.set(this, InterfaceManager::cLastMessageButtonClicked, 0xff);
        e.set(this, InterfaceManager::fpResult, 0);
        e.set(this, InterfaceManager::bTextureRelease, 0);
        e.set(this, InterfaceManager::field_4cc, 0);
        e.set(this, InterfaceManager::bMouseInMotion, 1);
        e.set(this, InterfaceManager::iLastXDefault, 100);
        e.set(this, InterfaceManager::bFirstChanceLoad, 0);
        e.set(this, InterfaceManager::bPreLoadMainMenus, 1);
        e.set_global(CURR_ALPHA_DPS, 0.0f32);
        e.set_global(ALPHA_GOING_DOWN, 0u8);
        let tick = e.call(FADE_CLOCK_READ, &args![FADE_CLOCK]).u32();
        e.set_global(LAST_DPS_FADE_TIME, tick);
        for i in 0..10u32 {
            e.mem.set_u32(base + 0x114 + 4 * i, 0);
        }

        // One interface unit in world distance: half the screen width over
        // the first angle function, times the second.
        let angle: f32 = e.global(ONE_TO_ONE_ANGLE);
        let half_width = e.call(GET_SCREEN_WIDTH, &args![]).f64() / e.global::<f64>(TWO);
        let first = e.call(ANGLE_FUNCTION_A, &args![angle]).f64();
        let ratio = (half_width / first) as f32;
        let second = e.call(ANGLE_FUNCTION_B, &args![angle]).f64();
        e.set(
            this,
            InterfaceManager::fOneToOneDistance,
            (second * ratio as f64) as f32,
        );

        if ni_pointer_get(e, base + 0xac) == 0 {
            let alpha = construct_new(e, NI_ALLOC, 0x1c, |e, block| {
                e.call(ALPHA_PROPERTY_CONSTRUCT, &args![block]).ptr()
            });
            e.call(NI_POINTER_ASSIGN, &args![base + 0xac, alpha]);
            for (setter, value) in [
                (ALPHA_PROPERTY_SET_BLEND, 1u32),
                (ALPHA_PROPERTY_SET_NO_SORTER, 0),
                (ALPHA_PROPERTY_SET_SOURCE_MODE, 6),
                (ALPHA_PROPERTY_SET_DESTINATION_MODE, 7),
            ] {
                let property = ni_pointer_get(e, base + 0xac);
                e.call(setter, &args![property, value]);
            }
        }
        let parent = e.call(SCENE_GRAPH_PARENT_3D, &args![]).u32();
        let graph = e
            .call(
                CREATE_SCENE_GRAPH,
                &args![this, parent, MENU_3D_ROOT_NAME, 1u32],
            )
            .u32();
        e.call(NI_POINTER_ASSIGN, &args![base + 0x8, graph]);
        let parent = e.call(SCENE_GRAPH_PARENT, &args![]).u32();
        let graph = e
            .call(
                CREATE_SCENE_GRAPH,
                &args![this, parent, MENU_ROOT_NAME, 0u32],
            )
            .u32();
        e.call(NI_POINTER_ASSIGN, &args![base + 0x4, graph]);
        e.call(MENU_MANAGER_INSTANCE, &args![1u32]);
        let view_caster = construct_new(e, OPERATOR_NEW, 0xc, |e, block| {
            e.call(VIEW_CASTER_CONSTRUCT, &args![block]).ptr()
        });
        e.set(this, InterfaceManager::pViewCaster, view_caster.addr());
        let timers = construct_new(e, OPERATOR_NEW, 0x14, |e, block| {
            timer_construct(e, block.cast()).cast()
        });
        e.set(this, InterfaceManager::pTimers, timers.addr());
        // The head timer's `pPrev` points at itself.
        e.mem.set_u32(timers.addr() + 0xc, timers.addr());
        e.set(this, InterfaceManager::field_4ac, 0.0);
        e.set(this, InterfaceManager::field_4b0, 0.0);
        e.set(this, InterfaceManager::bMouseOverRenderedMenu, 0);
        e.set(this, InterfaceManager::bIsInRenderedMenu, 0);
        e.set(this, InterfaceManager::pCurrentRenderedMenu, 0);
        e.set(this, InterfaceManager::pPipboy, 0);
        e.set(this, InterfaceManager::field_4bc, 0);
        e.set(this, InterfaceManager::field_4c0, 0);
        e.set(this, InterfaceManager::pShadowNode, 0);
        // The interface accumulator (id 100: setter `+0x19c` = 10, byte
        // `+0x32` = 1) and the 3D one (id 99).
        let accumulator = construct_new(e, NI_ALLOC, 0x280, |e, block| {
            e.call(
                SHADER_ACCUMULATOR_CONSTRUCT,
                &args![block, 100u32, 1u32, 0x2f7u32],
            )
            .ptr()
        });
        e.call(NI_POINTER_ASSIGN, &args![base + 0x8c, accumulator]);
        let held = ni_pointer_get(e, base + 0x8c);
        e.call(SHADER_ACCUMULATOR_SET_19C, &args![held, 10u32]);
        let held = ni_pointer_get(e, base + 0x8c);
        fn_0070a900(e, Ptr::new(held), 1);
        let accumulator = construct_new(e, NI_ALLOC, 0x280, |e, block| {
            e.call(
                SHADER_ACCUMULATOR_CONSTRUCT,
                &args![block, 99u32, 1u32, 0x2f7u32],
            )
            .ptr()
        });
        e.call(NI_POINTER_ASSIGN, &args![base + 0x90, accumulator]);
        e.set(this, InterfaceManager::bClickMultithreaded, 0);
        e.set(this, InterfaceManager::field_4b8, 0);
        e.set(this, InterfaceManager::bShowMouse, 0);
        e.set(this, InterfaceManager::field_4cd, 0);
        e.set(this, InterfaceManager::iCurrentPickIndex, 0);
    });
    this
}

// Translated from 0070a8a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterfaceManager::Timer` constructor (Xbox PDB layout): no neighbours,
/// zero elapsed time, the default end time. Returns `this`.
pub fn timer_construct(e: &mut Engine, this: Ptr<Timer>) -> Ptr<Timer> {
    e.set(this, Timer::pPrev, 0);
    e.set(this, Timer::pNext, 0);
    e.set(this, Timer::fElapsed, 0.0);
    let default_end: f32 = e.global(TIMER_DEFAULT_END);
    e.set(this, Timer::fEnd, default_end);
    e.set(this, Timer::pIndex, 0);
    this
}

// Translated from 0070a8e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the table embedded at `+0x4d4` of the manager: runs the
/// table initializer `00706b90` on it and returns it.
pub fn fn_0070a8e0(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(KEY_TABLE_CONSTRUCT, &args![this]);
    this
}

// Translated from 0070a900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the byte at `+0x32` of the object (called on the
/// interface shader accumulator).
pub fn fn_0070a900(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x32, value);
}

// Translated from 0070a920 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterfaceManager::~InterfaceManager` (Xbox PDB): deletes the target
/// reticle, shuts down the helper singletons, detaches both scene graphs,
/// deletes the root tiles, the cursor, the view caster, the pipboy and the
/// shadow nodes, clears the singleton and finally destroys the embedded
/// members in reverse order. The compiler's unwinding states are not
/// translated.
pub fn interface_manager_destruct(e: &mut Engine, this: Ptr<InterfaceManager>) {
    let base = this.addr();
    let reticle = e.get(this, InterfaceManager::pTargetReticle);
    if reticle != 0 {
        delete_virtual(e, reticle);
    }
    e.set(this, InterfaceManager::pTargetReticle, 0);
    fn_0070ad30(e);
    fn_0070acb0(e);
    e.call(DEBUG_TEXT_SHUTDOWN, &args![]);
    e.call(FONT_MANAGER_SHUTDOWN, &args![]);
    for member in [base + 0x4, base + 0x8] {
        if ni_pointer_get(e, member) != 0 {
            let graph = ni_pointer_get(e, member);
            e.call(EMPTY_MEMBER_FUNCTION, &args![graph]);
            e.call(NI_POINTER_ASSIGN, &args![member, 0u32]);
        }
    }
    e.set_global(TILE_DELETING, 1u8);
    let string_root = e.get(this, InterfaceManager::pStringRoot);
    if string_root != 0 {
        delete_virtual(e, string_root);
    }
    let menus_root = e.get(this, InterfaceManager::pMenusRoot);
    if menus_root != 0 {
        delete_virtual(e, menus_root);
    }
    e.set_global(TILE_DELETING, 0u8);
    e.call(MENU_MANAGER_SHUTDOWN, &args![]);
    let cursor = e.get(this, InterfaceManager::pCursor);
    if cursor != 0 {
        delete_virtual(e, cursor);
    }
    e.call(NI_POINTER_ASSIGN, &args![base + 0xac, 0u32]);
    e.set_global(MANAGER_SINGLETON, 0u32);
    e.call(TILE_FREE_TRAIT_LIST, &args![]);
    fn_0070aca0(e);
    let view_caster = e.get(this, InterfaceManager::pViewCaster);
    if view_caster != 0 {
        fn_0070ad80(e, Ptr::new(view_caster), 1);
    }
    e.call(FREE_TIMERS, &args![this]);
    e.call(MENU_FREE_XML_ARCHIVE, &args![]);
    e.call(LIST_CLEAR, &args![LIST_AT_011D8B54]);
    e.call(STATIC_OBJECT_SHUTDOWN_A, &args![]);
    e.call(MESSAGE_MENU_SHUTDOWN, &args![]);
    e.call(CLEAR_TEMP_MODEL, &args![]);
    let pipboy = e.get(this, InterfaceManager::pPipboy);
    if pipboy != 0 {
        delete_virtual(e, pipboy);
    }
    e.call(NI_POINTER_ASSIGN, &args![base + 0x8c, 0u32]);
    let shadow = e.get(this, InterfaceManager::pShadowNode);
    if shadow != 0 {
        delete_virtual(e, shadow);
    }
    let ui_player = e.get(this, InterfaceManager::pUIPlayerNode);
    if ui_player != 0 {
        delete_virtual(e, ui_player);
    }
    e.call(START_MENU_SHUTDOWN, &args![]);
    e.call(STATIC_OBJECT_SHUTDOWN_B, &args![]);
    e.call(EMPTY_MEMBER_FUNCTION, &args![base + 0x4d4]);
    e.call(SIMPLE_LIST_DESTROY, &args![base + 0x4c4]);
    e.call(VATS_EFFECT_MANAGER_DESTROY, &args![base + 0x1dc]);
    e.call(HUD_EFFECT_MANAGER_DESTROY, &args![base + 0x178]);
    e.call(NI_POINTER_DESTROY, &args![base + 0xac]);
    e.call(NI_POINTER_DESTROY, &args![base + 0x90]);
    e.call(NI_POINTER_DESTROY, &args![base + 0x8c]);
    e.call(SIMPLE_LIST_DESTROY, &args![base + 0x70]);
    e.call(NI_POINTER_DESTROY, &args![base + 0x8]);
    e.call(NI_POINTER_DESTROY, &args![base + 0x4]);
}

// Translated from 0070aca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Empties the `NiTPointerList<unsigned int>` at `011f3324`
/// (`NiTPointerListBase<NiTPointerAllocator<unsigned_int>_Script_P>::RemoveAll`,
/// the engine map's name for `004ed900`).
pub fn fn_0070aca0(e: &mut Engine) {
    e.call(POINTER_LIST_REMOVE_ALL, &args![SCRIPT_POINTER_LIST]);
}

// Translated from 0070acb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deletes the object the global at `011d8ce8` points to (through
/// `0070ad00`) and clears the global.
pub fn fn_0070acb0(e: &mut Engine) {
    let object = e.global::<u32>(GLOBAL_OBJECT_8CE8);
    if object != 0 {
        fn_0070ad00(e, Ptr::new(object), 1);
    }
    e.set_global(GLOBAL_OBJECT_8CE8, 0u32);
}

// Translated from 0070ad00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deleting destructor of the object `fn_0070acb0` owns: runs `0071b040`
/// on it and frees it when bit 0 of `flags` is set. Returns `this`.
pub fn fn_0070ad00(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(OBJECT_8CE8_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0070ad30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deletes (virtual destructor, flag 1) the object the global at
/// `011da0c4` points to and clears the global.
pub fn fn_0070ad30(e: &mut Engine) {
    let object = e.global::<u32>(GLOBAL_OBJECT_A0C4);
    if object != 0 {
        delete_virtual(e, object);
    }
    e.set_global(GLOBAL_OBJECT_A0C4, 0u32);
}

// Translated from 0070ad80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deleting destructor of the view caster (`0x0c` bytes): runs `00631a50`
/// on it and frees it when bit 0 of `flags` is set. Returns `this`.
pub fn fn_0070ad80(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(VIEW_CASTER_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0070adb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterfaceManager::Init` (Xbox PDB): first initialization. Optionally
/// loads the text table, builds the root tile (`MenuRoot`) with the screen
/// metrics as traits, reads `globals.xml`, creates the debug text root and
/// the pipboy manager, and corrects the line limit for 16:10 screens.
/// `load_text_table` is the byte argument (non-zero also creates the
/// loading menu at the end).
pub fn interface_manager_init(e: &mut Engine, this: Ptr<InterfaceManager>, load_text_table: u8) {
    with_scope_guard(e, 0x21a, |e| {
        if load_text_table != 0 {
            e.call(TILE_LOAD_TEXT_TABLE, &args![]);
        }
        let root = construct_new(e, OPERATOR_NEW, 0x3c, |e, block| {
            e.call(MENU_ROOT_TILE_CONSTRUCT, &args![block]).ptr()
        });
        e.set(this, InterfaceManager::pMenusRoot, root.addr());
        let root = e.get(this, InterfaceManager::pMenusRoot);
        // `Tile::Init` (virtual slot 4): no parent, the name, flag 0.
        e.vcall(root, 4, &args![0u32, MENU_ROOT_NAME, 0u32]);
        let root = e.get(this, InterfaceManager::pMenusRoot);
        tile_set_int(e, root, 0xfa8, 1);
        tile_set_int(e, root, 0x1771, 0x3eb);
        tile_set_float(e, root, 0xfa9, 0.0);
        for (trait_id, getter) in [
            (0xfb1u32, GET_SCREEN_WIDTH),
            (0xfb0, GET_SCREEN_HEIGHT),
            (0xfc0, GET_INTERFACE_MARGIN_A),
            (0xfbf, GET_INTERFACE_MARGIN_B),
            (0xff8, GET_INTERFACE_SCALE),
        ] {
            let value = e.call(getter, &args![]).f32();
            let root = e.get(this, InterfaceManager::pMenusRoot);
            tile_set_float(e, root, trait_id, value);
        }
        let root = e.get(this, InterfaceManager::pMenusRoot);
        e.call(TILE_ADD_DIRTY_TILE, &args![root]);
        e.call(TILE_UPDATE_ALL, &args![0u32]);
        e.call(SYSTEM_COLOR_MANAGER_GET_INSTANCE, &args![]);
        let root = e.get(this, InterfaceManager::pMenusRoot);
        let globals = e.call(TILE_READ_FILE, &args![root, GLOBALS_XML_PATH]).u32();
        e.set(this, InterfaceManager::pStringRoot, globals);
        e.call(MENU_REMOVE_FROM_XML_ARCHIVE, &args![GLOBALS_XML_PATH, 0u32]);
        let globals = e.get(this, InterfaceManager::pStringRoot);
        e.call(TILE_SET_PARENT, &args![globals, 0u32, 0u32]);

        // `_background_fill_alpha` = (value * 255) as a float, from the
        // float the holder at `011d3174` keeps.
        let holder = e
            .call(FLOAT_HOLDER_GET, &args![FLOAT_HOLDER_BACKGROUND])
            .u32();
        let fraction: f32 = e.mem.f32(holder);
        let scale: f64 = e.global(TWO_FIFTY_FIVE);
        let alpha = (fraction as f64 * scale) as f32;
        let trait_id = e
            .call(TILE_TEXT_TO_TRAIT, &args![BACKGROUND_FILL_ALPHA_NAME])
            .u32();
        let globals = e.get(this, InterfaceManager::pStringRoot);
        tile_set_float(e, globals, trait_id, alpha);

        let has_controller = e.call(HAS_360_CONTROLLER_GETTER, &args![]).u8() as u32;
        let trait_id = e
            .call(TILE_TEXT_TO_TRAIT, &args![HAS_360_CONTROLLER_NAME])
            .u32();
        let globals = e.get(this, InterfaceManager::pStringRoot);
        tile_set_int(e, globals, trait_id, has_controller);

        let debug_root = construct_new(e, NI_ALLOC, 0xac, |e, block| {
            e.call(NI_NODE_CONSTRUCT, &args![block, 0u32]).ptr()
        });
        e.set(this, InterfaceManager::pDebugTextRoot, debug_root.addr());
        e.with_stack(4, |e, name| {
            let handle = e
                .call(FIXED_STRING_CONSTRUCT, &args![name, DEBUG_TEXT_ROOT_NAME])
                .u32();
            let debug_root = e.get(this, InterfaceManager::pDebugTextRoot);
            e.call(NODE_SET_NAME, &args![debug_root, handle]);
            e.call(FIXED_STRING_DESTROY, &args![name]);
        });
        let debug_root = e.get(this, InterfaceManager::pDebugTextRoot);
        e.call(NODE_SET_FLAG, &args![debug_root, 0u32]);
        // The interface root attaches the debug text root (virtual slot
        // `0xdc`, second argument 1).
        let interface_root = e.get(this, InterfaceManager::pInterfaceRoot);
        let debug_root = e.get(this, InterfaceManager::pDebugTextRoot);
        e.vcall(interface_root, 0xdc, &args![debug_root, 1u32]);
        let node = e.call(GET_FRAME_SCENE_NODE, &args![this]).u32();
        e.call(NODE_UPDATE_PROPERTIES, &args![node]);
        e.with_stack(0xc, |e, update_data| {
            e.call(
                NI_UPDATE_DATA_CONSTRUCT,
                &args![update_data, 0.0f32, 0u32, 0u32],
            );
            e.call(NODE_UPDATE, &args![node, update_data]);
        });

        // 16:10 (and its inverse 10:16 portrait) screens get a smaller
        // limit on two string lists.
        let width = e.call(GET_DESKTOP_WIDTH, &args![]).f64();
        let height = e.call(GET_DESKTOP_HEIGHT, &args![]).f64();
        let ratio = (width / height) as f32;
        let widescreen: f64 = e.global(WIDESCREEN_RATIO);
        let inverse: f64 = e.global(WIDESCREEN_INVERSE_RATIO);
        if ratio as f64 == widescreen || 1.0 / ratio as f64 == inverse {
            e.call(STRING_LIST_SET_LIMIT, &args![STRING_LIST_A, 0x50u32]);
            e.call(STRING_LIST_SET_LIMIT, &args![STRING_LIST_B, 0x50u32]);
        }

        let fog = construct_new(e, NI_ALLOC, 0x64, |e, block| {
            e.call(FOG_PROPERTY_CONSTRUCT, &args![block]).ptr()
        });
        e.with_stack(4, |e, held| {
            e.call(NI_POINTER_CONSTRUCT, &args![held, fog]);
            let fog = ni_pointer_get(e, held.addr());
            e.call(SET_FOG_RANGE, &args![fog, 0.0f32, 0.0f32]);
            e.call(GET_SECOND_SCENE_NODE, &args![this]);
            e.call(SET_DEBUG_TEXT_VISIBLE, &args![this, 0u32]);
            let pipboy = construct_new(e, OPERATOR_NEW, 0x170, |e, block| {
                e.call(PIPBOY_MANAGER_CONSTRUCT, &args![block]).ptr()
            });
            e.set(this, InterfaceManager::pPipboy, pipboy.addr());
            e.set(this, InterfaceManager::bFirstInit, 1);
            if load_text_table != 0 {
                e.call(LOADING_MENU_CREATE, &args![0u32, 0u32]);
            }
            e.call(NI_POINTER_DESTROY, &args![held]);
        });
    });
}

// Translated from 0070b240 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterfaceManager::InitSecond` (Xbox PDB): second initialization. Starts
/// the font manager, creates the cursor tile (`Cursor`, 32x32) and its
/// node orientation, the two shadow scene nodes and their accumulators,
/// creates the HUD main menu and marks the manager initialized.
pub fn interface_manager_init_second(e: &mut Engine, this: Ptr<InterfaceManager>) {
    with_scope_guard(e, 0x29b, |e| {
        e.call(FONT_MANAGER_INIT, &args![]);
        let cursor = construct_new(e, OPERATOR_NEW, 0x48, |e, block| {
            tile_image_construct(e, block.cast()).cast()
        })
        .addr();
        // `Tile::Init` (virtual slot 4): no parent, the name, flag 0.
        e.vcall(cursor, 4, &args![0u32, CURSOR_NAME, 0u32]);
        tile_set_int(e, cursor, 0xfad, 2000);
        tile_set_int(e, cursor, 0xfb1, 0x20);
        tile_set_int(e, cursor, 0xfb0, 0x20);
        for (trait_id, holder) in [(0xfccu32, STRING_HOLDER_A), (0xff9, STRING_HOLDER_B)] {
            let string = e.call(STRING_HOLDER_GET, &args![holder]).u32();
            e.call(TILE_SET_STRING, &args![cursor, trait_id, string, 1u32]);
        }
        for trait_id in [0xfa1u32, 0xfa2, 0xfa3] {
            tile_set_int(e, cursor, trait_id, 0);
        }
        for trait_id in [0xfa9u32, 0xfb2, 0xfb3, 0xfb4] {
            tile_set_int(e, cursor, trait_id, 0xff);
        }
        e.call(TILE_UPDATE_TILE, &args![cursor, 1u32]);
        let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
        let cursor_root = e.get(this, InterfaceManager::pCursorRoot);
        e.vcall(cursor_root, 0xdc, &args![node, 1u32]);
        let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
        let direction_y: f32 = e.global(CURSOR_DIRECTION_Y);
        e.call(
            NODE_SET_TRANSLATE,
            &args![node, 0.0f32, direction_y, 0.0f32],
        );
        let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
        e.call(NODE_UPDATE_PROPERTIES, &args![node]);
        e.with_stack(0xc, |e, update_data| {
            e.call(
                NI_UPDATE_DATA_CONSTRUCT,
                &args![update_data, 0.0f32, 0u32, 0u32],
            );
            let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
            e.call(NODE_UPDATE, &args![node, update_data]);
        });
        e.set(this, InterfaceManager::pCursor, cursor);
        fn_007118d0(e, this);
        let cursor = e.get(this, InterfaceManager::pCursor);
        let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
        e.call(NODE_SET_FLAG, &args![node, 1u32]);

        let shadow = construct_new(e, NI_ALLOC, 0x200, |e, block| {
            e.call(SHADOW_SCENE_NODE_CONSTRUCT, &args![block]).ptr()
        });
        e.set(this, InterfaceManager::pShadowNode, shadow.addr());
        e.call(SET_SHADOW_SCENE_NODE, &args![shadow, 1u32]);
        let shadow = construct_new(e, NI_ALLOC, 0x200, |e, block| {
            e.call(SHADOW_SCENE_NODE_CONSTRUCT, &args![block]).ptr()
        });
        e.set(this, InterfaceManager::pUIPlayerNode, shadow.addr());
        e.call(SET_SHADOW_SCENE_NODE, &args![shadow, 3u32]);
        // Both accumulators are given the first shadow node (setter
        // `+0x194`).
        for member in [0x8cu32, 0x90] {
            let shadow_node = e.get(this, InterfaceManager::pShadowNode);
            let accumulator = ni_pointer_get(e, this.addr() + member);
            e.call(SHADER_ACCUMULATOR_SET_194, &args![accumulator, shadow_node]);
        }
        let hud = e.call(HUD_MAIN_MENU_CREATE, &args![]).u32();
        e.call(HUD_MAIN_MENU_SET_MENU_MODE, &args![4u32]);
        if hud != 0 {
            tile_set_int(e, hud, 0xfa3, 0);
        }
        let cursor = e.get(this, InterfaceManager::pCursor);
        let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
        e.call(NODE_SET_FLAG, &args![node, 1u32]);
        let cursor = e.get(this, InterfaceManager::pCursor);
        tile_set_int(e, cursor, 0xfa3, 0);
        let cursor = e.get(this, InterfaceManager::pCursor);
        e.call(TILE_UPDATE_TILE, &args![cursor, 1u32]);
        e.set(this, InterfaceManager::bSecondInit, 1);
    });
}

// Translated from 0070b5d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TileImage::TileImage` (Xbox PDB): the `Tile` base constructor, the
/// `TileImage` vtable, empty texture and shader property pointers, scale
/// 1.0 and `bIsScissor` false. Returns `this`.
pub fn tile_image_construct(e: &mut Engine, this: Ptr<TileImage>) -> Ptr<TileImage> {
    e.call(TILE_CONSTRUCT, &args![this]);
    e.mem.set_u32(this.addr(), TILE_IMAGE_VTABLE);
    e.call(NI_POINTER_CONSTRUCT, &args![this.addr() + 0x3c, 0u32]);
    e.call(NI_POINTER_CONSTRUCT, &args![this.addr() + 0x40, 0u32]);
    e.set(this, TileImage::fScale, 1.0);
    e.call(NI_POINTER_ASSIGN, &args![this.addr() + 0x3c, 0u32]);
    e.set(this, TileImage::bIsScissor, 0);
    this
}

// Translated from 0070b670 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TileImage::GetType` (Xbox PDB): the tile type id.
pub fn tile_image_get_type(_e: &mut Engine, _this: Ptr<TileImage>) -> u32 {
    TILE_IMAGE_TYPE
}

// Translated from 0070b680 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TileImage::GetTypeName` (Xbox PDB): the address of the type name
/// string.
pub fn tile_image_get_type_name(_e: &mut Engine, _this: Ptr<TileImage>) -> u32 {
    TILE_IMAGE_TYPE_NAME
}

// Translated from 0070b690 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TileImage::__scalar_deleting_destructor_` (Xbox PDB): destroys the tile
/// and, when bit 0 of `flags` is set, frees it. Returns `this`.
pub fn tile_image_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<TileImage>,
    flags: u32,
) -> Ptr<TileImage> {
    tile_image_destruct(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0070b6c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TileImage::~TileImage` (Xbox PDB): restores the `TileImage` vtable,
/// clears the shader property, calls `Tile::Release` unless the tile
/// flags say it was already released (bit `0x2000`), destroys the two
/// pointer members and runs `Tile::~Tile`.
pub fn tile_image_destruct(e: &mut Engine, this: Ptr<TileImage>) {
    e.mem.set_u32(this.addr(), TILE_IMAGE_VTABLE);
    e.call(NI_POINTER_ASSIGN, &args![this.addr() + 0x40, 0u32]);
    if !e.call(TILE_FLAG_TEST, &args![this]).bool() {
        e.call(TILE_RELEASE, &args![this]);
    }
    e.call(NI_POINTER_DESTROY, &args![this.addr() + 0x40]);
    e.call(NI_POINTER_DESTROY, &args![this.addr() + 0x3c]);
    e.call(TILE_DESTRUCT, &args![this]);
}

// Translated from 0070b8f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterfaceManager::PreIdleStuff` (Xbox PDB): per-frame mode transitions
/// before the idle. Leaving mode 3 (menus fading out, not locked) clears
/// the keystrokes, enters mode 5, restores the cursor and the HUD, enters
/// the console and pauses sound and voice once more; mode 5 becomes 2.
/// While in mode 1 the pause counters follow whether the top menu is a
/// game-pausing one (`007023a0`).
pub fn interface_manager_pre_idle_stuff(e: &mut Engine, this: Ptr<InterfaceManager>) {
    with_scope_guard(e, 0x33a, |e| {
        let controls_owner = e.global::<u32>(CONTROLS_OWNER);
        let controls = e.call(CONTROLS_GET, &args![controls_owner]).u32();
        e.call(UPDATE_ALL_TIMERS, &args![this]);
        if e.get(this, InterfaceManager::cMenuMode) == 3
            && e.get(this, InterfaceManager::bLockMenuModeForFade) == 0
        {
            e.call(CLEAR_KEYSTROKES, &args![controls]);
            e.set(this, InterfaceManager::cMenuMode, 5);
            let has_controller = e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool();
            if !has_controller || e.get(this, InterfaceManager::bShowMouse) != 0 {
                let cursor = e.get(this, InterfaceManager::pCursor);
                let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
                e.call(NODE_SET_FLAG, &args![node, 0u32]);
                let cursor = e.get(this, InterfaceManager::pCursor);
                tile_set_int(e, cursor, 0xfa3, 1);
            }
            let string = e.call(STRING_HOLDER_GET, &args![STRING_HOLDER_A]).u32();
            let cursor = e.get(this, InterfaceManager::pCursor);
            e.call(TILE_SET_STRING, &args![cursor, 0xfccu32, string, 1u32]);
            let cursor = e.get(this, InterfaceManager::pCursor);
            e.call(TILE_UPDATE_TILE, &args![cursor, 1u32]);
            e.call(HUD_SET_INFO_FOR_REF, &args![0u32, 0u32, 0u32]);
            e.set(this, InterfaceManager::pMouseOverTarget, 0);
            let has_controller = e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool();
            if has_controller && e.get(this, InterfaceManager::bShowMouse) == 0 {
                e.set(this, InterfaceManager::bMouseInMotion, 0);
            }
            let console = e.call(MENU_CONSOLE_INSTANCE, &args![0u32]).u32();
            if console != 0 {
                e.call(MENU_CONSOLE_ON_ENTER_MENU_MODE, &args![console]);
            }
            pause_sound_and_voice(e);
        } else if e.get(this, InterfaceManager::cMenuMode) == 5 {
            e.set(this, InterfaceManager::cMenuMode, 2);
        }
        if e.get(this, InterfaceManager::cMenuMode) == 1 {
            if e.call(MENU_MODE_IS_NOT_ONE, &args![this]).bool() {
                if e.get(this, InterfaceManager::bExternalForcedMenuMode) == 0 {
                    e.set(this, InterfaceManager::bExternalForcedMenuMode, 1);
                    pause_sound_and_voice(e);
                }
            } else if e.get(this, InterfaceManager::bExternalForcedMenuMode) != 0 {
                e.set(this, InterfaceManager::bExternalForcedMenuMode, 0);
                unpause_voice_and_sound(e);
            }
        }
    });
}

/// Adds one to the sound pause counter (`+0x20` of the manager object that
/// `004b7210` returns) and pauses the audio, then the same for the voice
/// counter (`+0x21`).
fn pause_sound_and_voice(e: &mut Engine) {
    let manager = e.call(GET_MANAGER, &args![]).u32();
    let sound = e.mem.u8(manager + 0x20).wrapping_add(1);
    e.mem.set_u8(manager + 0x20, sound);
    let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
    fn_0070bba0(e, Ptr::new(audio));
    let manager = e.call(GET_MANAGER, &args![]).u32();
    let voice = e.mem.u8(manager + 0x21).wrapping_add(1);
    e.mem.set_u8(manager + 0x21, voice);
    let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
    e.call(AUDIO_PAUSE_TYPE_6, &args![audio]);
}

/// The inverse of [`pause_sound_and_voice`], each counter only when it is
/// not zero: voice first, then sound.
fn unpause_voice_and_sound(e: &mut Engine) {
    let manager = e.call(GET_MANAGER, &args![]).u32();
    if e.mem.u8(manager + 0x21) != 0 {
        let manager = e.call(GET_MANAGER, &args![]).u32();
        let voice = e.mem.u8(manager + 0x21).wrapping_sub(1);
        e.mem.set_u8(manager + 0x21, voice);
        let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
        e.call(AUDIO_UNPAUSE_TYPE_6, &args![audio]);
    }
    let manager = e.call(GET_MANAGER, &args![]).u32();
    if e.mem.u8(manager + 0x20) != 0 {
        let manager = e.call(GET_MANAGER, &args![]).u32();
        let sound = e.mem.u8(manager + 0x20).wrapping_sub(1);
        e.mem.set_u8(manager + 0x20, sound);
        let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
        e.call(AUDIO_UNPAUSE_TYPE_4000_0000, &args![audio]);
    }
}

// Translated from 0070bba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Pauses the audio sounds of type `0x40000000` (`00ad8510(this,
/// 0x40000000, 1)`); called on the audio instance.
pub fn fn_0070bba0(e: &mut Engine, this: Ptr) {
    e.call(AUDIO_PAUSE_TYPE, &args![this, 0x4000_0000u32, 1u32]);
}

// Translated from 0070c4a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterfaceManager::Idle` (Xbox PDB): the per-frame interface update.
///
/// In order: reads the Xbox 360 controller and announces a change of
/// connection; shows or hides the cursor for the input device; updates the
/// HUD mode when the menu mode changed; in modes 2 and 5 (menus open or
/// opening) runs the mouse handling (pick, drag, enter/leave/click through
/// the menus, wheel), the gamepad events, the console and the menu keys;
/// then handles the console and start menu keys, the pipboy hot keys and
/// the tutorial manager. Nothing after the scope guard.
///
/// The three mouse-button queries `00a23a50(0, n)` (or `00a24660(5, n)` on
/// the console's own controls) are kept as `input_0`, `input_1`, `input_2`;
/// the code never names them.
pub fn interface_manager_idle(e: &mut Engine, this: Ptr<InterfaceManager>) {
    with_scope_guard(e, 0x47a, |e| {
        let mut state = IdleState::default();
        idle_controller_state(e, this);
        if e.get(this, InterfaceManager::bPreLoadMainMenus) != 0 {
            e.call(PRELOAD_MAIN_MENUS, &args![this]);
        }
        let controls_owner = e.global::<u32>(CONTROLS_OWNER);
        state.controls = e.call(CONTROLS_GET, &args![controls_owner]).u32();
        e.call(CONTROLS_QUERY_00A239E0, &args![state.controls, 1u32]);
        e.call(CONTROLS_QUERY_00A239E0, &args![state.controls, 2u32]);
        idle_cursor_visibility(e, this);
        state.menu_blocks_pointer = idle_pointer_blocked(e, this);
        let rendered_menu = e.get(this, InterfaceManager::pCurrentRenderedMenu);
        if rendered_menu != 0 {
            e.vcall(rendered_menu, 0x14, &args![]);
        }
        let pipboy = e.call(GET_PIPBOY, &args![this]).u32();
        if fn_0070ec50(e, Ptr::new(pipboy)) != 0 {
            let tick = e.call(FADE_CLOCK_READ, &args![FADE_CLOCK]).u32();
            let pipboy = e.call(GET_PIPBOY, &args![this]).u32();
            e.call(PIPBOY_UPDATE_LIGHT_EFFECT, &args![pipboy, tick as f32]);
        }
        e.call(HUD_EFFECTS_UPDATE, &args![this.addr() + 0x178]);
        e.call(VATS_EFFECTS_UPDATE, &args![this.addr() + 0x1dc]);
        idle_hud_mode(e, this);
        idle_gamepad_menu_key(e, this);
        if e.get(this, InterfaceManager::cMenuMode) == 1 {
            idle_game_mode(e, this);
        }
        let mode = e.get(this, InterfaceManager::cMenuMode);
        if mode == 2 || mode == 5 {
            idle_menu_mode(e, this, &mut state);
        }
        idle_console_and_hot_keys(e, this, &mut state);
    });
}

/// What `Idle` keeps in locals across its parts.
#[derive(Default)]
struct IdleState {
    /// The controls object (`00877720`).
    controls: u32,
    /// The console (`MenuConsole::Instance`), or 0.
    console: u32,
    /// True when a menu keeps the pointer from acting (`menu_blocks_pointer`).
    menu_blocks_pointer: bool,
    /// The three mouse-button queries.
    input_0: u32,
    input_1: u32,
    input_2: u32,
    /// The tile under the pointer and its menu.
    picked_tile: u32,
    picked_menu: u32,
}

/// Part of `Idle`: copies the previous controller state words, polls
/// `XInputGetState` and reports a change of the connected state.
fn idle_controller_state(e: &mut Engine, this: Ptr<InterfaceManager>) {
    for (from, to) in [
        (0x011d_8a6cu32, 0x011d_8a54u32),
        (0x011d_8a70, 0x011d_8a58),
        (0x011d_8a74, 0x011d_8a5c),
        (0x011d_8a78, 0x011d_8a60),
    ] {
        let word = e.mem.u32(from);
        e.mem.set_u32(to, word);
    }
    let result = e
        .call(XINPUT_GET_STATE, &args![0u32, CONTROLLER_STATE])
        .u32();
    e.set_global(HAS_360_CONTROLLER, (result == 0) as u8);
    // `static bool lastHasController` with its init guard.
    if e.global::<u32>(LAST_HAS_CONTROLLER_GUARD) & 1 == 0 {
        let guard: u32 = e.global(LAST_HAS_CONTROLLER_GUARD);
        e.set_global(LAST_HAS_CONTROLLER_GUARD, guard | 1);
        let now: u8 = e.global(HAS_360_CONTROLLER);
        e.set_global(LAST_HAS_CONTROLLER, now);
    }
    let last: u8 = e.global(LAST_HAS_CONTROLLER);
    let now: u8 = e.global(HAS_360_CONTROLLER);
    if last != now {
        // A message box about the change: the first string comes from the
        // holder at `011d38b8`, the second from one of two holders depending
        // on whether the controller was connected before (`last` is 1).
        let text = if last == 1 {
            TEXT_HOLDER_CONTROLLER_WAS_SET
        } else {
            TEXT_HOLDER_CONTROLLER_WAS_CLEAR
        };
        let title = e
            .call(STRING_HOLDER_GET, &args![MESSAGE_TITLE_HOLDER])
            .u32();
        let body = e.call(STRING_HOLDER_GET, &args![text]).u32();
        e.call(
            SHOW_MESSAGE_BOX,
            &args![body, 0u32, 0u32, 0u32, 0u32, 0x17u32, 0.0f32, 0.0f32, title, 0u32],
        );
        e.call(CONTROLLER_CHANGED, &args![this, now as u32]);
        let now: u8 = e.global(HAS_360_CONTROLLER);
        e.set_global(LAST_HAS_CONTROLLER, now);
    }
}

/// Part of `Idle`: with a controller and no console the cursor tile is
/// hidden (`bShowMouse` false); otherwise it is shown unless the manager is
/// in mode 1.
fn idle_cursor_visibility(e: &mut Engine, this: Ptr<InterfaceManager>) {
    if e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool() {
        if !e.call(IS_CONSOLE_VISIBLE, &args![]).bool() {
            e.set(this, InterfaceManager::bShowMouse, 0);
            set_cursor_hidden(e, this, false);
        } else {
            e.set(this, InterfaceManager::bShowMouse, 1);
            set_cursor_hidden(e, this, true);
        }
    } else if e.get(this, InterfaceManager::cMenuMode) != 1 {
        e.set(this, InterfaceManager::bShowMouse, 1);
        set_cursor_hidden(e, this, true);
    }
}

/// The three calls that show or hide the cursor tile: its node flag
/// (`0` when hiding, `1` when showing) and trait `0xfa3` (`1` when hiding).
fn set_cursor_hidden(e: &mut Engine, this: Ptr<InterfaceManager>, hidden: bool) {
    let cursor = e.get(this, InterfaceManager::pCursor);
    let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
    e.call(NODE_SET_FLAG, &args![node, !hidden as u32]);
    let cursor = e.get(this, InterfaceManager::pCursor);
    tile_set_int(e, cursor, 0xfa3, hidden as u32);
}

/// Part of `Idle`: `menu_blocks_pointer`, true when the pipboy (or the menu the code
/// names) keeps the pointer from acting.
fn idle_pointer_blocked(e: &mut Engine, this: Ptr<InterfaceManager>) -> bool {
    if !e.call(IS_PIPBOY_MENU_TOPMOST, &args![this]).bool() {
        return true;
    }
    if e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool()
        && (is_top_menu(e, 0x40b) || is_top_menu(e, 0x425))
    {
        return true;
    }
    if is_top_menu(e, 0x3ea) && inventory_menu_is_keyring_open(e) {
        return true;
    }
    if e.call(OBJECT_FLAG_0070ED80, &args![]).bool() && e.global::<u8>(HAS_360_CONTROLLER) != 0 {
        return true;
    }
    let class = e.call(PIPBOY_MENU_CLASS, &args![]).u32();
    e.call(TILE_GET_MENU_BY_CLASS, &args![class]).u32() != 0
}

/// `Interface::IsTopMenuID(id)` (Xbox PDB, `cdecl`).
fn is_top_menu(e: &mut Engine, id: u32) -> bool {
    e.call(IS_TOP_MENU_ID, &args![id]).bool()
}

/// Part of `Idle`: when the menu mode differs from the one last seen
/// (`0119f530`), pauses/unpauses the audio and picks the HUD mode for the
/// new menu mode.
fn idle_hud_mode(e: &mut Engine, this: Ptr<InterfaceManager>) {
    let mode = e.get(this, InterfaceManager::cMenuMode);
    if e.global::<u32>(LAST_MENU_MODE) == mode {
        return;
    }
    if mode == 1 {
        loop {
            let manager = e.call(GET_MANAGER, &args![]).u32();
            if e.mem.u8(manager + 0x21) == 0 {
                break;
            }
            let manager = e.call(GET_MANAGER, &args![]).u32();
            let voice = e.mem.u8(manager + 0x21).wrapping_sub(1);
            e.mem.set_u8(manager + 0x21, voice);
            let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
            e.call(AUDIO_UNPAUSE_TYPE_6, &args![audio]);
        }
        loop {
            let manager = e.call(GET_MANAGER, &args![]).u32();
            if e.mem.u8(manager + 0x20) == 0 {
                break;
            }
            let manager = e.call(GET_MANAGER, &args![]).u32();
            let sound = e.mem.u8(manager + 0x20).wrapping_sub(1);
            e.mem.set_u8(manager + 0x20, sound);
            let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
            e.call(AUDIO_UNPAUSE_TYPE_4000_0000, &args![audio]);
        }
        let player = e.global::<u32>(PLAYER_SINGLETON);
        let hud_mode = if e.vcall(player, 0x22c, &args![0u32]).bool() {
            7
        } else if e.vcall(player, 0x214, &args![]).u32() != 0 {
            1
        } else if e.call(GAME_STATE_ID, &args![]).u32() == 0x18 {
            // No HUD mode is set while this state id is current.
            0
        } else if e
            .call(CONDITION_CHECK_005A03F0, &args![player, 1u32])
            .bool()
        {
            0xc
        } else if e.call(PLAYER_FLAG_00950090, &args![player]).bool() {
            0x14
        } else {
            let iron_sights = e.call(ACTOR_GET_IRON_SIGHTS, &args![player]).bool();
            if iron_sights
                && e.call(WEAPON_STATE_GETTER, &args![WEAPON_STATE_OBJECT])
                    .u32()
                    == 0
            {
                0x16
            } else if e
                .call(WEAPON_STATE_GETTER, &args![WEAPON_STATE_OBJECT])
                .u32()
                == 4
            {
                8
            } else {
                2
            }
        };
        if hud_mode != 0 {
            e.call(HUD_SET_MENU_MODE, &args![hud_mode]);
        }
    } else if mode == 5 {
        let top = e.call(GET_ENTER_STACK_TOP, &args![this]).i32();
        let hud_mode = match top {
            0x3e9 => 0x11,
            1 | 0x13 | 0x3ea | 0x3eb | 0x3f6 | 0x3ff | 0x40b | 0x425 => 3,
            0x3ef => 5,
            0x3f0 => 9,
            0x3f1 => 6,
            0x403 | 0x418 => 0xe,
            0x41f => 0xf,
            0x420 => 7,
            0x421 => 0x10,
            0x424 => 0x12,
            0x432 => 0x13,
            0x438..=0x43b => 0x19,
            _ => 4,
        };
        e.call(HUD_SET_MENU_MODE, &args![hud_mode as u32]);
    }
    e.set_global(LAST_MENU_MODE, mode);
}

/// Part of `Idle`: when `00719ae0` reports true and none of the four
/// `00a24660(n, 0)` queries (n = 2, 3, 0, 1) is set, calls `00718930(0)`.
fn idle_gamepad_menu_key(e: &mut Engine, this: Ptr<InterfaceManager>) {
    if !e.call(QUERY_00719AE0, &args![this]).bool() {
        return;
    }
    for button in [2u32, 3, 0, 1] {
        let controls_owner = e.global::<u32>(CONTROLS_OWNER);
        let controls = e.call(CONTROLS_GET, &args![controls_owner]).u32();
        if e.call(CONTROLS_QUERY_00A24660, &args![controls, button, 0u32])
            .u32()
            != 0
        {
            return;
        }
    }
    e.call(ACTION_00718930, &args![this, 0u32]);
}

/// Part of `Idle` in menu mode 1 (the game): makes sure the HUD exists,
/// shows the pending cursor string, creates the level-up menu when queued
/// and lets the player cast the queued eat/drink and enchantment effects.
fn idle_game_mode(e: &mut Engine, this: Ptr<InterfaceManager>) {
    let hud = e.call(TILE_GET_MENU_BY_CLASS, &args![0x3ecu32]).u32();
    if hud == 0 {
        e.call(HUD_MAIN_MENU_CREATE, &args![]);
    }
    if e.global::<u8>(CURSOR_STRING_PENDING) != 0 {
        let string = e.call(STRING_HOLDER_GET, &args![STRING_HOLDER_A]).u32();
        let cursor = e.get(this, InterfaceManager::pCursor);
        e.call(TILE_SET_STRING, &args![cursor, 0xfccu32, string, 1u32]);
        let cursor = e.get(this, InterfaceManager::pCursor);
        e.call(TILE_UPDATE_TILE, &args![cursor, 1u32]);
        e.set_global(CURSOR_STRING_PENDING, 0u8);
    }
    if e.global::<u8>(LEVEL_UP_MENU_PENDING) != 0 {
        e.call(LEVEL_UP_MENU_CREATE, &args![]);
        e.set_global(LEVEL_UP_MENU_PENDING, 0u8);
    }
    let player = e.global::<u32>(PLAYER_SINGLETON);
    e.call(PLAYER_CAST_EAT_DRINK_ITEMS, &args![player]);
    let player = e.global::<u32>(PLAYER_SINGLETON);
    e.call(PLAYER_CAST_QUEUED_ENCHANTMENTS, &args![player]);
}

/// Part of `Idle` in menu modes 2 and 5: cursor and default focus, the
/// mouse buttons, picking and enter/leave/click/drag through the menu
/// virtuals, the wheel, the gamepad and the menu keys.
fn idle_menu_mode(e: &mut Engine, this: Ptr<InterfaceManager>, state: &mut IdleState) {
    idle_menu_cursor_and_focus(e, this);
    idle_read_mouse(e, this, state);
    if e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool()
        || e.get(this, InterfaceManager::bMouseInMotion) != 0
    {
        idle_pick(e, this, state);
    }
    idle_menu_key_state(e, this, state);
    idle_click(e, this, state);
    idle_button_and_wheel(e, this, state);
    idle_gamepad_events(e, this, state);
}

/// `ftol(Tile::GetValue(tile, 0xfaa))`: the id of the tile's menu entry that
/// the menu virtuals take.
fn tile_menu_id(e: &mut Engine, tile: u32) -> i32 {
    let value = e.call(TILE_GET_VALUE, &args![tile, 0xfaau32]).f64();
    e.call(FTOL, &args![value]).i32()
}

fn pointer_shown(e: &mut Engine, this: Ptr<InterfaceManager>) -> bool {
    !e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool()
        || e.get(this, InterfaceManager::bShowMouse) != 0
}

/// The menu state word at `+0x24` of a menu (`0059bb30`, a plain getter the
/// linker folded with `D3DTexture_LockRect`); 1 means the menu is active.
fn menu_is_active(e: &mut Engine, menu: u32) -> bool {
    e.call(MENU_STATE, &args![menu]).u32() == 1
}

/// Part of `idle_menu_mode`: with a controller and no console, shows the
/// cursor tile as hidden or shown; keeps the default focus when the pointer
/// is not in motion.
fn idle_menu_cursor_and_focus(e: &mut Engine, this: Ptr<InterfaceManager>) {
    let controller = e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool();
    if controller && e.get(this, InterfaceManager::bShowMouse) == 0 {
        if !e.call(IS_CONSOLE_VISIBLE, &args![]).bool() {
            e.set(this, InterfaceManager::bMouseInMotion, 0);
        }
    } else {
        let loading = e.call(IS_IN_GAME_LOADING_MENU_OPEN, &args![]).bool();
        if loading && !e.call(XUI_IS_UP, &args![]).bool() {
            let cursor = e.get(this, InterfaceManager::pCursor);
            let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
            e.call(NODE_SET_FLAG, &args![node, 1u32]);
            let cursor = e.get(this, InterfaceManager::pCursor);
            tile_set_int(e, cursor, 0xfa3, 0);
        } else {
            let cursor = e.get(this, InterfaceManager::pCursor);
            let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
            e.call(NODE_SET_FLAG, &args![node, 0u32]);
            let cursor = e.get(this, InterfaceManager::pCursor);
            tile_set_int(e, cursor, 0xfa3, 1);
        }
    }
    fn_007118d0(e, this);
    if e.get(this, InterfaceManager::bMouseInMotion) == 0 {
        let over = e.call(GET_MOUSE_OVER_TARGET, &args![this]).u32();
        let keep_focus = over != 0 && {
            let over = e.call(GET_MOUSE_OVER_TARGET, &args![this]).u32();
            e.call(TILE_IS_VISIBLE, &args![over]).bool() && {
                let over = e.call(GET_MOUSE_OVER_TARGET, &args![this]).u32();
                e.call(TILE_IS_TRUE, &args![over, 0xfafu32]).bool()
            }
        };
        if !keep_focus {
            e.call(GET_DEFAULT_FOCUS, &args![this]);
        }
    }
}

/// Part of `idle_menu_mode`: reads the three mouse-button queries (from the
/// console's own controls when it is visible and the pointer is hidden) and
/// accumulates the held time; stores the wheel as a float.
fn idle_read_mouse(e: &mut Engine, this: Ptr<InterfaceManager>, state: &mut IdleState) {
    state.input_1 = 0;
    state.input_2 = 0;
    state.input_0 = 0;
    let controller = e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool();
    if !controller || e.get(this, InterfaceManager::bShowMouse) != 0 {
        state.input_1 = e
            .call(CONTROLS_QUERY_00A23A50, &args![state.controls, 0u32, 1u32])
            .u32();
        state.input_2 = e
            .call(CONTROLS_QUERY_00A23A50, &args![state.controls, 0u32, 2u32])
            .u32();
        state.input_0 = e
            .call(CONTROLS_QUERY_00A23A50, &args![state.controls, 0u32, 0u32])
            .u32();
        accumulate_held_time(e, this, state.input_0);
    } else {
        let console = e.call(MENU_CONSOLE_INSTANCE, &args![1u32]).u32();
        if e.call(CONSOLE_PRIMARY_DOWN, &args![console]).bool() {
            state.input_1 = e
                .call(CONTROLS_QUERY_00A24660, &args![state.controls, 5u32, 1u32])
                .u32();
            state.input_2 = e
                .call(CONTROLS_QUERY_00A24660, &args![state.controls, 5u32, 2u32])
                .u32();
            state.input_0 = e
                .call(CONTROLS_QUERY_00A24660, &args![state.controls, 5u32, 0u32])
                .u32();
            accumulate_held_time(e, this, state.input_0);
        }
    }
    let wheel = e
        .call(CONTROLS_QUERY_00A239E0, &args![state.controls, 3u32])
        .i32();
    e.set(this, InterfaceManager::fMouseWheel, wheel as f32);
}

/// `fMouseHeldTime += frame time` while the button is down, 0 otherwise.
fn accumulate_held_time(e: &mut Engine, this: Ptr<InterfaceManager>, held: u32) {
    if held == 0 {
        e.set(this, InterfaceManager::fMouseHeldTime, 0.0);
    } else {
        let frame_time = e.call(FRAME_TIME_GETTER, &args![FADE_CLOCK]).f64();
        let total = frame_time + e.get(this, InterfaceManager::fMouseHeldTime) as f64;
        e.set(this, InterfaceManager::fMouseHeldTime, total as f32);
    }
}

/// The pointer position the drag code works in: the cursor fields
/// `+0x38`/`+0x40` divided by the screen scale (`0070ecb0`), as the
/// `NiPoint2` the code builds from them.
fn pointer_scaled(e: &mut Engine, this: Ptr<InterfaceManager>) -> (f32, f32) {
    let scale = e.call(GET_SCREEN_SCALE, &args![]).f64();
    let y = (e.mem.f32(this.addr() + 0x40) as f64 / scale) as f32;
    let scale = e.call(GET_SCREEN_SCALE, &args![]).f64();
    let x = (e.mem.f32(this.addr() + 0x38) as f64 / scale) as f32;
    e.with_stack(8, |e, point| {
        e.call(NI_POINT2_CONSTRUCT, &args![point, x, y]);
        (e.mem.f32(point.addr()), e.mem.f32(point.addr() + 4))
    })
}

/// With the pipboy menu topmost the point is in pipboy coordinates:
/// `value / menu height - 195`.
fn pointer_to_pipboy_space(
    e: &mut Engine,
    this: Ptr<InterfaceManager>,
    point: (f32, f32),
) -> (f32, f32) {
    let (mut x, mut y) = point;
    if e.call(IS_PIPBOY_MENU_TOPMOST, &args![this]).bool() {
        let margin: f64 = e.global(PIPBOY_MARGIN);
        let height = e.call(GET_MENU_HEIGHT, &args![]).f64();
        x = (x as f64 / height - margin) as f32;
        let height = e.call(GET_MENU_HEIGHT, &args![]).f64();
        y = (y as f64 / height - margin) as f32;
    }
    (x, y)
}

/// Part of `idle_menu_mode`, when the pointer is in motion (or a
/// controller is attached): finds the tile under the pointer, updates the
/// drag of the dragged tile, and runs drag-over, release, enter and leave
/// on the menu virtuals.
fn idle_pick(e: &mut Engine, this: Ptr<InterfaceManager>, state: &mut IdleState) {
    state.picked_tile = e.call(PICK_TILE, &args![this, 0u32]).u32();
    state.picked_menu = 0;
    if state.picked_tile != 0 {
        state.picked_menu = e.call(TILE_GET_MENU, &args![state.picked_tile]).u32();
        let over = e.get(this, InterfaceManager::pMouseOverTarget);
        if pointer_shown(e, this) && over != 0 {
            tile_set_int(e, over, 0xfc3, 0);
            let target = e.get(this, InterfaceManager::pOverTileTarget);
            let id = tile_menu_id(e, over);
            let menu = e.call(TILE_GET_MENU, &args![over]).u32();
            e.call(DO_LEAVE, &args![this, menu, id, target]);
            e.set(this, InterfaceManager::pMouseOverTarget, 0);
        }
    }
    let drag = e.get(this, InterfaceManager::pDragTarget);
    if pointer_shown(e, this) && drag != 0 {
        let point = pointer_scaled(e, this);
        let (x, y) = pointer_to_pipboy_space(e, this, point);
        let drag = e.get(this, InterfaceManager::pDragTarget);
        let offset = tile_get_float(e, drag, 0xfea);
        let offset_x = e.call(FTOL, &args![x as f64 - offset]).i32();
        e.call(SET_CURSOR_OFFSET_X, &args![this, offset_x]);
        let offset = tile_get_float(e, drag, 0xfeb);
        let offset_y = e.call(FTOL, &args![y as f64 - offset]).i32();
        e.call(SET_CURSOR_OFFSET_Y, &args![this, offset_y]);
        let tile_x = e.call(TILE_GET_POSITION_X, &args![drag]).f64();
        let origin_x = tile_get_float(e, drag, 0xfa1);
        tile_set_float(e, drag, 0xff0, (x as f64 - (tile_x - origin_x)) as f32);
        let tile_y = e.call(TILE_GET_POSITION_Y, &args![drag]).f64();
        let origin_y = tile_get_float(e, drag, 0xfa2);
        tile_set_float(e, drag, 0xff1, (y as f64 - (tile_y - origin_y)) as f32);
    }
    let current = e.get(this, InterfaceManager::pOverTileTarget);
    if state.picked_tile != current && state.input_0 != 0 {
        // Dragging over another tile: the old menu hears the drag leave
        // (slot 0x1c), the new one the drag enter (slot 0x18).
        let drag_menu = e.get(this, InterfaceManager::pDragOverTileMenu);
        if drag_menu != 0 && menu_is_active(e, drag_menu) {
            let drag_tile = e.get(this, InterfaceManager::pDragOverTileTarget);
            let id = tile_menu_id(e, drag_tile);
            let current = e.get(this, InterfaceManager::pOverTileTarget);
            let drag_menu = e.get(this, InterfaceManager::pDragOverTileMenu);
            e.vcall(drag_menu, 0x1c, &args![id, drag_tile, current]);
        }
        e.set(
            this,
            InterfaceManager::pDragOverTileTarget,
            state.picked_tile,
        );
        e.set(this, InterfaceManager::pDragOverTileMenu, state.picked_menu);
        let drag_menu = e.get(this, InterfaceManager::pDragOverTileMenu);
        if drag_menu != 0 && menu_is_active(e, drag_menu) {
            let drag_tile = e.get(this, InterfaceManager::pDragOverTileTarget);
            let id = tile_menu_id(e, drag_tile);
            let current = e.get(this, InterfaceManager::pOverTileTarget);
            let drag_menu = e.get(this, InterfaceManager::pDragOverTileMenu);
            e.vcall(drag_menu, 0x18, &args![id, drag_tile, current]);
        }
    }
    if state.input_2 != 0 {
        idle_release(e, this, state);
    }
    idle_enter_leave(e, this, state);
}

/// `Tile::GetValue(tile, trait)` as the `double` it is read as in `ST0`.
fn tile_get_float(e: &mut Engine, tile: u32, trait_id: u32) -> f64 {
    e.call(TILE_GET_VALUE, &args![tile, trait_id]).f64()
}

/// The release of the button: the menu under the pointer is told (slot
/// `0xc`) and the tile's click trait pulses, when the press began on the
/// tile that is still under the pointer; the drag ends.
fn idle_release(e: &mut Engine, this: Ptr<InterfaceManager>, state: &mut IdleState) {
    let menu = e.get(this, InterfaceManager::pOverTileMenu);
    let tile = e.get(this, InterfaceManager::pOverTileTarget);
    let pressed = e.get(this, InterfaceManager::pDragOverTileTarget);
    if menu != 0
        && pressed == tile
        && e.call(TILE_GET_VALUE_Q, &args![tile, 0xfaau32]).u32() != 0
        && menu_is_active(e, menu)
    {
        e.call(TILE_PLAY_TILE_SOUND, &args![tile, 0xfcbu32]);
        let tile = e.get(this, InterfaceManager::pOverTileTarget);
        let menu = e.get(this, InterfaceManager::pOverTileMenu);
        if tile != 0 {
            tile_set_int(e, tile, 0xfc7, 1);
            tile_set_int(e, tile, 0xfc7, 0);
            if menu != 0 {
                let id = tile_menu_id(e, tile);
                e.vcall(menu, 0xc, &args![id, tile]);
            }
        }
        if e.get(this, InterfaceManager::pOverTileTarget) == 0 {
            state.picked_tile = 0;
            state.picked_menu = 0;
        }
    }
    e.set(this, InterfaceManager::pDragOverTileTarget, 0);
    e.set(this, InterfaceManager::pDragOverTileMenu, 0);
    let drag = e.get(this, InterfaceManager::pDragTarget);
    if drag != 0 {
        tile_set_int(e, drag, 0xfea, -1i32 as u32);
        let drag = e.get(this, InterfaceManager::pDragTarget);
        tile_set_int(e, drag, 0xfeb, -1i32 as u32);
        e.set(this, InterfaceManager::pDragTarget, 0);
    }
}

/// The pointer moved to another tile with no button down: leaves the old
/// tile and enters the new one (`DoLeave`, `DoEnter`, the menu virtuals).
fn idle_enter_leave(e: &mut Engine, this: Ptr<InterfaceManager>, state: &mut IdleState) {
    let current = e.get(this, InterfaceManager::pOverTileTarget);
    if state.picked_tile == current || state.input_0 != 0 || !pointer_shown(e, this) {
        return;
    }
    let menu = e.get(this, InterfaceManager::pOverTileMenu);
    if menu != 0 && menu_is_active(e, menu) {
        let tile = e.get(this, InterfaceManager::pOverTileTarget);
        tile_set_int(e, tile, 0xfc3, 0);
        // Is the picked tile a child of the tile being left?
        let found = e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), state.picked_tile);
            e.call(TILE_CHILD_LIST_FIND, &args![tile + 4, slot, 0u32])
                .u32()
        });
        let tile = e.get(this, InterfaceManager::pOverTileTarget);
        let id = tile_menu_id(e, tile);
        let menu = e.get(this, InterfaceManager::pOverTileMenu);
        if found != 0 {
            e.vcall(menu, 0x14, &args![id, tile]);
        } else {
            e.call(DO_LEAVE, &args![this, menu, id, tile]);
        }
    }
    if state.picked_menu != 0 && menu_is_active(e, state.picked_menu) {
        e.set(this, InterfaceManager::pOverTileTarget, state.picked_tile);
        e.set(this, InterfaceManager::pOverTileMenu, state.picked_menu);
    } else {
        e.set(this, InterfaceManager::pOverTileTarget, 0);
        e.set(this, InterfaceManager::pOverTileMenu, 0);
    }
    let menu = e.get(this, InterfaceManager::pOverTileMenu);
    if menu != 0 && menu_is_active(e, menu) {
        if pointer_shown(e, this) {
            let tile = e.get(this, InterfaceManager::pOverTileTarget);
            e.set(this, InterfaceManager::pMouseOverTarget, tile);
        }
        let tile = e.get(this, InterfaceManager::pOverTileTarget);
        e.call(TILE_PLAY_TILE_SOUND, &args![tile, 0xfe8u32]);
        let tile = e.get(this, InterfaceManager::pOverTileTarget);
        tile_set_int(e, tile, 0xfc3, 1);
        let tile = e.get(this, InterfaceManager::pOverTileTarget);
        let id = tile_menu_id(e, tile);
        let menu = e.get(this, InterfaceManager::pOverTileMenu);
        e.call(DO_ENTER, &args![this, menu, id, tile]);
    }
    if e.get(this, InterfaceManager::bMouseInMotion) != 0 {
        e.call(CLEAR_MOUSE_OVER_TARGET, &args![this]);
    }
}

/// Part of `idle_menu_mode`, when the pointer is shown: keeps the flag at
/// `+0x4b4` and decides `bMouseOverRenderedMenu` (`+0x170`); when the
/// rendered/pipboy menu is unchanged and `+0x4b8` is set, plays the
/// `UIMenuMode` sound and opens the stats, inventory or map page.
fn idle_menu_key_state(e: &mut Engine, this: Ptr<InterfaceManager>, state: &mut IdleState) {
    if !pointer_shown(e, this) {
        return;
    }
    if e.call(RENDERED_MENU_OR_PIPBOY, &args![this]).u32() == 0 {
        e.mem.set_u8(this.addr() + 0x4b4, 0);
    }
    let current = e.call(RENDERED_MENU_OR_PIPBOY, &args![this]).u32();
    let pipboy = e.call(GET_PIPBOY, &args![this]).u32();
    let rendered = e.get(this, InterfaceManager::pCurrentRenderedMenu);
    if current == pipboy {
        let blocked = e.get(this, InterfaceManager::bMouseInMotion) == 0
            || rendered == 0
            || e.call(MENU_FLAG_QUERY_004A4040, &args![]).bool()
            || is_top_menu(e, 0x3e9)
            || is_top_menu(e, 0x41b)
            || e.mem.u8(this.addr() + 0x4b4) == 0;
        e.set(
            this,
            InterfaceManager::bMouseOverRenderedMenu,
            !blocked as u8,
        );
        if (e.mem.u32(this.addr() + 0x4b8) as i32) > 0 {
            idle_play_menu_mode_sound(e, this);
        }
    } else {
        let blocked = e.get(this, InterfaceManager::bMouseInMotion) == 0
            || rendered == 0
            || e.call(MENU_FLAG_QUERY_004A4040, &args![]).bool()
            || is_top_menu(e, 0x3e9)
            || is_top_menu(e, 0x41b)
            || is_top_menu(e, 0x423)
            || {
                let rendered = e.get(this, InterfaceManager::pCurrentRenderedMenu);
                let held = (state.input_2 as i32 > 0) as u32;
                !e.vcall(
                    rendered,
                    0x18,
                    &args![held, this.addr() + 0x4ac, this.addr() + 0x4b0],
                )
                .bool()
            };
        e.set(
            this,
            InterfaceManager::bMouseOverRenderedMenu,
            !blocked as u8,
        );
    }
}

/// Plays the `UIMenuMode` sound and opens the page `+0x4b8` names (1 stats,
/// 2 inventory, 3 map); any other value releases the sound handle.
fn idle_play_menu_mode_sound(e: &mut Engine, this: Ptr<InterfaceManager>) {
    e.with_stack(12, |e, handle| {
        e.call(SOUND_HANDLE_CONSTRUCT, &args![handle]);
        e.with_stack(12, |e, found| {
            let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
            let result = e
                .call(
                    AUDIO_GET_SOUND_HANDLE_BY_NAME,
                    &args![audio, found, UI_MENU_MODE_SOUND, 0x121u32],
                )
                .u32();
            e.call(SOUND_HANDLE_ASSIGN, &args![handle, result]);
            e.call(EMPTY_MEMBER_FUNCTION, &args![found]);
        });
        match e.mem.u32(this.addr() + 0x4b8) {
            1 => {
                e.call(SOUND_HANDLE_PLAY, &args![handle, 0u32]);
                e.call(SET_STATS_MENU_VISIBLE, &args![1u32, 0u32]);
            }
            2 => {
                e.call(SOUND_HANDLE_PLAY, &args![handle, 0u32]);
                e.call(SET_INVENTORY_MENU_VISIBLE, &args![1u32, 0u32, 1u32]);
            }
            3 => {
                e.call(SOUND_HANDLE_PLAY, &args![handle, 0u32]);
                e.call(SET_MAP_MENU_VISIBLE, &args![1u32, 0u32]);
            }
            _ => {
                e.call(SOUND_HANDLE_RELEASE, &args![handle]);
            }
        }
        e.call(EMPTY_MEMBER_FUNCTION, &args![handle]);
    });
}

/// Part of `idle_menu_mode`, on `input_1` with the pointer in motion: the
/// tile under the pointer is clicked (menu virtual slot 8); a draggable
/// tile (`0xfe9`) starts a drag. Without a menu under the pointer the
/// console's pick display is refreshed.
fn idle_click(e: &mut Engine, this: Ptr<InterfaceManager>, state: &mut IdleState) {
    if e.get(this, InterfaceManager::bMouseInMotion) == 0 || state.input_1 == 0 {
        return;
    }
    let tile = e.get(this, InterfaceManager::pOverTileTarget);
    let menu = e.get(this, InterfaceManager::pOverTileMenu);
    if tile != 0 && menu != 0 && menu_is_active(e, menu) {
        e.set(this, InterfaceManager::pDragOverTileTarget, tile);
        e.set(this, InterfaceManager::pDragOverTileMenu, menu);
        let id = tile_menu_id(e, tile);
        e.vcall(menu, 8, &args![id, tile]);
        let tile = e.get(this, InterfaceManager::pOverTileTarget);
        if tile != 0 && e.call(TILE_IS_TRUE, &args![tile, 0xfe9u32]).bool() {
            idle_start_drag(e, this, tile);
        }
    } else {
        let console = e.call(MENU_CONSOLE_INSTANCE, &args![1u32]).u32();
        if e.call(CONSOLE_PRIMARY_DOWN, &args![console]).bool() {
            fn_00710880(e, this);
            interface_manager_display_current_pick_ref(e, this);
        }
    }
}

/// The drag start of [`idle_click`]: the dragged tile is `tile`, the
/// pointer position (in menu space) is stored as the drag start and the
/// tile's drag traits are set from it.
fn idle_start_drag(e: &mut Engine, this: Ptr<InterfaceManager>, tile: u32) {
    e.set(this, InterfaceManager::pDragTarget, tile);
    let point = pointer_scaled(e, this);
    let drag = e.get(this, InterfaceManager::pDragTarget);
    let tile_x = e.call(TILE_GET_POSITION_X, &args![drag]).f32();
    let tile_y = e.call(TILE_GET_POSITION_Y, &args![drag]).f32();
    let (x, y) = pointer_to_pipboy_space(e, this, point);
    let start_x = e.call(FTOL, &args![x as f64]).i32();
    e.set(this, InterfaceManager::iDragStartX, start_x);
    let start_y = e.call(FTOL, &args![y as f64]).i32();
    e.set(this, InterfaceManager::iDragStartY, start_y);
    e.call(SET_CURSOR_OFFSET_X, &args![this, 0u32]);
    e.call(SET_CURSOR_OFFSET_Y, &args![this, 0u32]);
    let origin_x = tile_get_float(e, drag, 0xfa1);
    tile_set_float(
        e,
        drag,
        0xff0,
        (x as f64 - (tile_x as f64 - origin_x)) as f32,
    );
    let origin_y = tile_get_float(e, drag, 0xfa2);
    tile_set_float(
        e,
        drag,
        0xff1,
        (y as f64 - (tile_y as f64 - origin_y)) as f32,
    );
    let start_x = e.get(this, InterfaceManager::iDragStartX);
    tile_set_int(e, drag, 0xfea, start_x as u32);
    let start_y = e.get(this, InterfaceManager::iDragStartY);
    tile_set_int(e, drag, 0xfeb, start_y as u32);
    tile_set_float(e, drag, 0xfec, (x as f64 - tile_x as f64) as f32);
    tile_set_float(e, drag, 0xfed, (y as f64 - tile_y as f64) as f32);
    e.set(this, InterfaceManager::fDragOffsetLastX, 0.0);
    e.set(this, InterfaceManager::fDragOffsetLastY, 0.0);
}

/// Part of `idle_menu_mode`: with `input_0` (a button held) the menu under
/// the pointer and the drag-over menu are told (slots 0x20 and 0x24) and
/// the dragged tile follows the cursor offset; with no button held the
/// wheel scrolls the menu (`DoWheelMove`) or steps the console's pick list.
fn idle_button_and_wheel(e: &mut Engine, this: Ptr<InterfaceManager>, state: &mut IdleState) {
    if state.input_0 != 0 {
        let menu = e.get(this, InterfaceManager::pOverTileMenu);
        if menu != 0 {
            let tile = e.get(this, InterfaceManager::pOverTileTarget);
            let id = tile_menu_id(e, tile);
            e.vcall(menu, 0x20, &args![id, tile]);
        }
        let drag_menu = e.get(this, InterfaceManager::pDragOverTileMenu);
        let tile = e.get(this, InterfaceManager::pOverTileTarget);
        if drag_menu != 0 && tile != 0 {
            let id = tile_menu_id(e, tile);
            e.vcall(drag_menu, 0x24, &args![id, tile]);
        }
        let drag = e.get(this, InterfaceManager::pDragTarget);
        if drag != 0 {
            let offset_x = e.call(GET_CURSOR_OFFSET_X, &args![this]).i32();
            let last_x = e.get(this, InterfaceManager::fDragOffsetLastX);
            tile_set_float(e, drag, 0xfee, (offset_x as f64 - last_x as f64) as f32);
            let offset_y = e.call(GET_CURSOR_OFFSET_Y, &args![this]).i32();
            let last_y = e.get(this, InterfaceManager::fDragOffsetLastY);
            let drag = e.get(this, InterfaceManager::pDragTarget);
            tile_set_float(e, drag, 0xfef, (offset_y as f64 - last_y as f64) as f32);
            let offset_x = e.call(GET_CURSOR_OFFSET_X, &args![this]).i32();
            e.set(this, InterfaceManager::fDragOffsetLastX, offset_x as f32);
            let offset_y = e.call(GET_CURSOR_OFFSET_Y, &args![this]).i32();
            e.set(this, InterfaceManager::fDragOffsetLastY, offset_y as f32);
        }
    } else if e.get(this, InterfaceManager::fMouseWheel) != 0.0 {
        let menu = e.get(this, InterfaceManager::pOverTileMenu);
        if menu != 0 {
            let mut tile = e.get(this, InterfaceManager::pOverTileTarget);
            // The nearest tile (starting at the one under the pointer)
            // that wants the wheel (`0xff2`), walking up through
            // `Tile::GetParent`.
            while !e.call(TILE_IS_TRUE, &args![tile, 0xff2u32]).bool()
                && e.call(TILE_PARENT, &args![tile]).u32() != 0
            {
                tile = e.call(TILE_PARENT, &args![tile]).u32();
            }
            if tile != 0 && e.call(TILE_IS_TRUE, &args![tile, 0xff2u32]).bool() {
                let wheel = e.call(GET_WHEEL_STEPS, &args![this]).i32();
                tile_set_int(e, tile, 0xff3, (wheel / -120) as u32);
                tile_set_int(e, tile, 0xff3, 0);
                let id = tile_menu_id(e, tile);
                let menu = e.get(this, InterfaceManager::pOverTileMenu);
                e.call(DO_WHEEL_MOVE, &args![this, menu, id, tile]);
            }
        } else {
            let console = e.call(MENU_CONSOLE_INSTANCE, &args![1u32]).u32();
            if e.call(CONSOLE_PRIMARY_DOWN, &args![console]).bool() {
                let wheel = e.call(GET_WHEEL_STEPS, &args![this]).i32();
                let step = if wheel >= 0 { 1 } else { -1 };
                let index = e.get(this, InterfaceManager::iCurrentPickIndex) + step;
                e.set(this, InterfaceManager::iCurrentPickIndex, index);
                let last = e.call(PICK_LIST_COUNT, &args![this.addr() + 0x70]).u32();
                if index < -1 {
                    e.set(
                        this,
                        InterfaceManager::iCurrentPickIndex,
                        last.wrapping_sub(1) as i32,
                    );
                } else if (last.wrapping_sub(1)) < index as u32 {
                    e.set(this, InterfaceManager::iCurrentPickIndex, -1);
                }
                interface_manager_display_current_pick_ref(e, this);
            }
        }
    }
}

/// Reads the next controls event: `(kind, key code)` from
/// `Controls`' event queue (`00a23820`, which writes the key code to the
/// word at `slot`).
fn read_controls_event(e: &mut Engine, slot: Ptr) -> u32 {
    let owner = e.global::<u32>(CONTROLS_OWNER);
    let controls = e.call(CONTROLS_GET, &args![owner]).u32();
    e.call(CONTROLS_NEXT_EVENT, &args![controls, slot]).u32()
}

/// Reads events until one that is not kind 2, running `007166f0(0)` and
/// `007154b0` on each kind-2 event (key modifier changes), and returns the
/// `007154b0` result for the last one.
fn translate_controls_events(e: &mut Engine, this: Ptr<InterfaceManager>, slot: Ptr) -> u32 {
    let mut kind = read_controls_event(e, slot);
    while kind == 2 {
        e.call(KEY_REPEAT_RESET, &args![this, 0u32]);
        let code = e.mem.u32(slot.addr());
        e.call(TRANSLATE_KEY_EVENT, &args![this, kind, code]);
        kind = read_controls_event(e, slot);
    }
    let code = e.mem.u32(slot.addr());
    e.call(TRANSLATE_KEY_EVENT, &args![this, kind, code]).u32()
}

/// Part of `idle_menu_mode`: gamepad events, then the key events: each is
/// translated to a menu key (`007154b0`), offered to the console, then to
/// the frontmost menu (virtual slot `0x30`), then to the `_PCButton_<key>`
/// child tile of the menu; keys nobody took become gamepad events
/// (`DoGamepadEvent`).
fn idle_gamepad_events(e: &mut Engine, this: Ptr<InterfaceManager>, state: &mut IdleState) {
    if e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool()
        && e.get(this, InterfaceManager::bShowMouse) == 0
    {
        interface_manager_do_gamepad_event(e, this, 0);
    }
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), 0);
        let mut key = translate_controls_events(e, this, slot);
        if key == 0 {
            key = e.call(KEY_REPEAT_00716730, &args![this, 1.0f32]).u32();
        }
        let mut consumed = false;
        state.console = e.call(MENU_CONSOLE_INSTANCE, &args![0u32]).u32();
        if key != 0 && state.console != 0 {
            loop {
                let code = e.mem.u32(slot.addr());
                consumed = if code == 0x29 {
                    false
                } else {
                    e.call(MENU_CONSOLE_IDLE, &args![state.console, key]).bool()
                };
                if consumed {
                    key = translate_controls_events(e, this, slot);
                }
                if !(key != 0 && consumed) {
                    break;
                }
            }
        }
        if key == 0
            && pointer_shown(e, this)
            && e.call(
                CONTROLS_QUERY_00A24660,
                &args![state.controls, 0x1cu32, 1u32],
            )
            .u32()
                != 0
            && is_top_menu(e, 0x3e9)
        {
            key = 1;
            fn_0070ec20(e, Ptr::new(state.controls), 0x1c, 0);
        }
        if key != 0 && !consumed {
            let code = e.mem.u32(slot.addr());
            idle_dispatch_key(e, this, state, key, code);
        }
    });
    // Sad-face messages for the two controller buttons `0x19` and `0x1a`.
    for (button, holder) in [(0x19u32, MESSAGE_HOLDER_19), (0x1a, MESSAGE_HOLDER_1A)] {
        if e.call(
            CONTROLS_QUERY_00A24660,
            &args![state.controls, button, 1u32],
        )
        .u32()
            != 0
            && !e.call(XUI_IS_UP, &args![]).bool()
        {
            let text = e.call(STRING_HOLDER_GET, &args![holder]).u32();
            let size: f32 = e.global(MESSAGE_ICON_SIZE);
            e.call(
                SHOW_ICON_MESSAGE,
                &args![text, 0u32, MESSAGE_ICON_PATH, 0u32, size, 0u32],
            );
        }
    }
}

/// A key `Idle` has translated and nobody consumed: the frontmost menu may
/// take it (slot `0x30`), else the matching `_PCButton_` tile is clicked,
/// else the key becomes a gamepad event.
fn idle_dispatch_key(
    e: &mut Engine,
    this: Ptr<InterfaceManager>,
    state: &mut IdleState,
    key: u32,
    code: u32,
) {
    let menu_manager = e.call(MENU_MANAGER_INSTANCE, &args![1u32]).u32();
    let frontmost = e.call(GET_FRONTMOST_MENU, &args![menu_manager]).u32();
    let mut taken = false;
    let mut pipboy_key = false;
    if e.call(IS_PIPBOY_MENU_TOPMOST, &args![this]).bool() {
        let controls_owner = e.global::<u32>(CONTROLS_OWNER);
        let controls = e.call(CONTROLS_GET, &args![controls_owner]).u32();
        let page_key = e
            .call(CONTROLS_QUERY_00A24660, &args![controls, 0xeu32, 1u32])
            .u32()
            != 0
            || {
                let controls = e.call(CONTROLS_GET, &args![controls_owner]).u32();
                e.call(CONTROLS_QUERY_00A24660, &args![controls, 0xeu32, 2u32])
                    .u32()
                    != 0
            };
        if page_key {
            let controls = e.call(CONTROLS_GET, &args![controls_owner]).u32();
            pipboy_key = e
                .call(CONTROLS_QUERY_00A24180, &args![controls, 0xfu32, 0u32])
                .u32()
                == 0;
        }
    }
    if frontmost != 0 && !pipboy_key {
        taken = e.vcall(frontmost, 0x30, &args![key]).bool();
    }
    if !taken && frontmost != 0 && !pipboy_key {
        idle_click_key_tile(e, frontmost, code);
    }
    if !taken {
        let event = match key {
            0x8000_0001 => 4,
            0x8000_0002 => 3,
            0x8000_0003 => 1,
            0x8000_0004 => 2,
            0x8000_0008 => {
                if e.call(MODIFIER_FOUR_DOWN, &args![this]).bool() {
                    0xb
                } else if fn_0070ecd0(e, this) {
                    0xc
                } else {
                    -2
                }
            }
            0x8000_0009 => 0xf,
            0x8000_000a => 0x10,
            _ => 0,
        };
        let event = if event == 4 && e.call(MODIFIER_FOUR_DOWN, &args![this]).bool() {
            0xd
        } else {
            event
        };
        let event = if event == 3 && e.call(MODIFIER_FOUR_DOWN, &args![this]).bool() {
            0xe
        } else {
            event
        };
        if event != 0 {
            interface_manager_do_gamepad_event(e, this, event);
            e.call(CLEAR_KEYSTROKES, &args![state.controls]);
        }
    }
}

/// The `_PCButton_<key>` child tile of the frontmost menu: when it is
/// enabled (`0xfa3`) it is clicked (`0xfaf` set: sound, click pulse and the
/// menu's slot `0xc`), else the "not allowed" menu sound plays.
fn idle_click_key_tile(e: &mut Engine, frontmost: u32, code: u32) {
    let controls_owner = e.global::<u32>(CONTROLS_OWNER);
    let controls = e.call(CONTROLS_GET, &args![controls_owner]).u32();
    let key_char = e
        .call(CONTROLS_QUERY_00A238A0, &args![controls, code, 0xffu32])
        .u32();
    let name = e.with_stack(0x10, |e, buffer| {
        e.call(
            FORMAT_STRING,
            &args![
                buffer,
                0x10u32,
                FORMAT_PC_BUTTON,
                PC_BUTTON_PREFIX,
                key_char
            ],
        );
        let trait_id = e.call(TILE_TEXT_TO_TRAIT, &args![buffer]).u32();
        let root = e.call(GET_FIELD_AT_4, &args![frontmost]).u32();
        let string = e.call(TILE_GET_STRING, &args![root, trait_id]).u32();
        let root = e.call(GET_FIELD_AT_4, &args![frontmost]).u32();
        e.call(TILE_GET_CHILD_BY_NAME, &args![root, string]).u32()
    });
    if name != 0 && e.call(TILE_IS_TRUE, &args![name, 0xfa3u32]).bool() {
        if e.call(TILE_IS_TRUE, &args![name, 0xfafu32]).bool() {
            e.call(TILE_PLAY_TILE_SOUND, &args![name, 0xfcbu32]);
            tile_set_int(e, name, 0xfc7, 1);
            tile_set_int(e, name, 0xfc7, 0);
            let id = tile_menu_id(e, name);
            e.vcall(frontmost, 0xc, &args![id, name]);
        } else {
            e.call(PLAY_MENU_SOUND, &args![2u32]);
        }
    }
}

/// The last part of `Idle`, after the mode-dependent work: the console
/// toggle (key `0x1d`), the start menu key (`0x1c`), the menus' per-frame
/// virtual (slot `0x2c`), the pipboy hot keys (`0x3b`..`0x3d`), the dialog
/// pause, and the pipboy/tutorial updates.
fn idle_console_and_hot_keys(e: &mut Engine, this: Ptr<InterfaceManager>, state: &mut IdleState) {
    let mut console_wanted = false;
    if e.call(CONSOLE_KEY_FLAG_GET, &args![]).bool() {
        e.call(CONSOLE_KEY_FLAG_SET, &args![0u32]);
        let console = e.call(MENU_CONSOLE_INSTANCE, &args![1u32]).u32();
        if !e.call(CONSOLE_PRIMARY_DOWN, &args![console]).bool() {
            console_wanted = true;
        }
    }
    if e.call(
        CONTROLS_QUERY_00A24660,
        &args![state.controls, 0x1du32, 1u32],
    )
    .u32()
        != 0
        || console_wanted
    {
        state.console = e.call(MENU_CONSOLE_INSTANCE, &args![1u32]).u32();
        e.call(CLEAR_KEYSTROKES, &args![state.controls]);
        if e.call(MENU_CONSOLE_TOGGLE_VISIBLE, &args![state.console])
            .bool()
        {
            e.call(ADD_TO_ENTER_STACK, &args![this, 3u32]);
        } else {
            e.call(POP_FROM_ENTER_STACK, &args![this, 3u32, 0u32]);
        }
    }
    if e.call(
        CONTROLS_QUERY_00A24660,
        &args![state.controls, 0x1cu32, 1u32],
    )
    .u32()
        != 0
    {
        idle_start_menu_key(e, this);
    }
    e.call(DEBUG_TEXT_UPDATE, &args![]);
    // The unreachable `Interface::InDialog` / `SetDialogPause` path of the
    // compiled code (guarded by a local that is always 0) is omitted.
    e.set_global(MENU_LOOP_ABORT, 0u8);
    let root = e.get(this, InterfaceManager::pMenusRoot);
    let first = e.call(NI_POINTER_GET, &args![root + 4]).u32();
    e.with_stack(4, |e, cursor| {
        e.mem.set_u32(cursor.addr(), first);
        while e.mem.u32(cursor.addr()) != 0 && e.global::<u8>(MENU_LOOP_ABORT) == 0 {
            // Returns the element pointer and advances the cursor.
            let element = e.call(LIST_NEXT_ELEMENT, &args![root + 4, cursor]).u32();
            let child = e.mem.u32(element);
            if child != 0 {
                let menu = e.call(TILE_GET_MENU, &args![child]).u32();
                if menu != 0 {
                    e.vcall(menu, 0x2c, &args![]);
                }
            }
        }
    });
    if e.global::<u8>(MENU_LOOP_ABORT) != 0 {
        e.set_global(MENU_LOOP_ABORT, 0u8);
        return;
    }
    let mode_word = e.get(this, InterfaceManager::field_4bc);
    let key_down = if mode_word == 3 {
        if !e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool() {
            e.call(
                CONTROLS_QUERY_00A24660,
                &args![state.controls, 0xeu32, 2u32],
            )
            .u32()
                != 0
                && !state.menu_blocks_pointer
        } else {
            (e.call(CHECK_MENU_BUTTON, &args![10u32, 1u32]).i32() == 1
                || e.call(CHECK_MENU_BUTTON, &args![6u32, 1u32]).i32() == 1)
                && !state.menu_blocks_pointer
        }
    } else {
        let player = e.global::<u32>(PLAYER_SINGLETON);
        fn_0070edc0(e, Ptr::new(player)) != 0
    };
    if key_down && e.get(this, InterfaceManager::field_4bc) == 0 {
        fn_0070f4e0(e, this, 0, 0);
    } else {
        let weapon_state = e
            .call(WEAPON_STATE_GETTER, &args![WEAPON_STATE_OBJECT])
            .u32();
        let player = e.global::<u32>(PLAYER_SINGLETON);
        let anything = weapon_state != 0 || e.vcall(player, 0x22c, &args![0u32]).bool() || key_down;
        let mode_word = e.get(this, InterfaceManager::field_4bc) as i32;
        if anything && pointer_shown(e, this) && mode_word > 0 && mode_word <= 3 {
            fn_0070f690(e, this, 0);
            let owner = e.global::<u32>(CONTROLS_OWNER);
            let controls = e.call(CONTROLS_GET, &args![owner]).u32();
            e.call(CONTROLS_CLEAR_USER_ACTIONS, &args![controls]);
        }
    }
    idle_pipboy_hot_keys(e, this, state);
    let top = e.call(GET_ENTER_STACK_TOP, &args![this]).u32();
    if top == 0 || e.call(IS_PIPBOY_MENU_TOPMOST, &args![this]).bool() {
        interface_manager_update_pipboy(e, this);
    }
    e.call(TUTORIAL_MANAGER_UPDATE, &args![this.addr() + 0x4d4]);
}

/// The start menu key (`0x1c`): opens the start menu or closes it again,
/// or queues the pause menu, depending on the menus already open.
fn idle_start_menu_key(e: &mut Engine, this: Ptr<InterfaceManager>) {
    let manager = e.call(MENU_MANAGER_INSTANCE, &args![1u32]).u32();
    if e.call(MENU_MANAGER_IS_MENU_OPEN, &args![manager, 0x3efu32])
        .bool()
    {
        return;
    }
    if e.call(XUI_IS_UP, &args![]).bool() {
        return;
    }
    let mode_word = e.get(this, InterfaceManager::field_4bc);
    if mode_word != 3 && mode_word != 0 {
        return;
    }
    if e.call(MENU_FLAG_QUERY_004A4040, &args![]).bool()
        && is_top_menu(e, 0x3f5)
        && e.call(IS_TOP_MENU_FADED_IN, &args![]).bool()
    {
        let top = e.call(GET_ENTER_STACK_TOP, &args![this]).i32();
        let start_menu = e.call(PIPBOY_MENU_CLASS, &args![]).i32();
        if top == start_menu && !fn_0070ee30(e) {
            e.call(START_MENU_CLOSE, &args![1u32]);
        }
    } else if e.global::<u8>(START_MENU_ALLOWED) != 0 {
        let flag = e.call(GET_SETTING_VALUE, &args![START_MENU_SETTING]).u32();
        if e.mem.u32(flag) == 1 {
            e.call(START_MENU_CREATE, &args![1u32, 0u32]);
        } else if !e.call(QUEUE_PENDING_00070EC00, &args![]).bool() {
            e.call(
                QUEUE_MENU_CREATE,
                &args![6u32, 0u32, 0u32, 0u32, 1u32, 0u32],
            );
        }
    }
}

/// The pipboy hot keys (`0x3b` stats, `0x3c` inventory, `0x3d` map) in the
/// two pipboy modes: in mode 0 they open the pipboy page, in mode 3 they
/// switch to it (or leave the pipboy when it is already shown).
fn idle_pipboy_hot_keys(e: &mut Engine, this: Ptr<InterfaceManager>, state: &mut IdleState) {
    let mut menu = 0u32;
    let mut pressed = false;
    for (key, id) in [(0x3bu32, 0x3ebu32), (0x3c, 0x3ea), (0x3d, 0x3ff)] {
        if e.call(CONTROLS_QUERY_00A24180, &args![state.controls, key, 1u32])
            .u32()
            != 0
        {
            menu = id;
            pressed = true;
            break;
        }
    }
    if pressed && e.get(this, InterfaceManager::field_4bc) == 0 {
        fn_0070f4e0(e, this, 0, menu);
    }
    if pressed && e.get(this, InterfaceManager::field_4bc) == 3 {
        let root = e.call(GET_MENUS_ROOT, &args![this]).u32();
        let shown = tile_get_float(e, root, 0x1771);
        if menu as f64 == shown {
            fn_0070f690(e, this, 0);
        } else if menu == 0x3ea {
            e.call(SET_INVENTORY_MENU_VISIBLE, &args![1u32, 0u32, 1u32]);
        } else if menu == 0x3eb {
            e.call(SET_STATS_MENU_VISIBLE, &args![1u32, 0u32]);
        } else if menu == 0x3ff {
            e.call(SET_MAP_MENU_VISIBLE, &args![1u32, 0u32]);
        }
    }
}

// Translated from 0070ec20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the byte at `+0x1b94 + b * 0x1c + a` of the controls object to
/// `0xff` (clears one key binding slot: `a` is the action, `b` the slot).
pub fn fn_0070ec20(e: &mut Engine, this: Ptr, a: i32, b: i32) {
    let offset = (b.wrapping_mul(0x1c)).wrapping_add(a) as u32;
    e.mem
        .set_u8(this.addr().wrapping_add(offset).wrapping_add(0x1b94), 0xff);
}

// Translated from 0070ec50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the byte at `+0x162` of the object (called on the pipboy
/// manager).
pub fn fn_0070ec50(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x162)
}

// Translated from 0070ecd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when bit 0 of `iModifierKeys` (`+0x14c`) is set.
pub fn fn_0070ecd0(e: &mut Engine, this: Ptr<InterfaceManager>) -> bool {
    e.get(this, InterfaceManager::iModifierKeys) & 1 != 0
}

// Translated from 0070ed30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InventoryMenu::IsKeyringOpen` (Xbox PDB): true when the inventory menu
/// exists and its keyring tile (the tile `00726070` returns for it, trait
/// stored at `011d9eb8`) is true.
pub fn inventory_menu_is_keyring_open(e: &mut Engine) -> bool {
    let menu = e.global::<u32>(INVENTORY_MENU);
    if menu == 0 {
        return false;
    }
    let trait_id = e.global::<u32>(KEYRING_TRAIT);
    let tile = e.call(GET_FIELD_AT_4, &args![menu]).u32();
    e.call(TILE_IS_TRUE, &args![tile, trait_id]).bool()
}

// Translated from 0070edc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the byte at `+0xd54` of the object (called on the player).
pub fn fn_0070edc0(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0xd54)
}

// Translated from 0070ee30 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when the object at `011daac0` has both flag bits `0x8000` and `0x8`
/// (`004a4080`).
pub fn fn_0070ee30(e: &mut Engine) -> bool {
    let flags_owner = e.global::<u32>(FLAGS_OWNER);
    e.call(HAS_FLAG, &args![flags_owner, 0x8000u32]).bool() && {
        let flags_owner = e.global::<u32>(FLAGS_OWNER);
        e.call(HAS_FLAG, &args![flags_owner, 8u32]).bool()
    }
}

// Translated from 0070ee80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterfaceManager::UpdatePipboy` (Xbox PDB): the pipboy state machine
/// (`+0x4bc`): 1 raising it (closes the container menu or the terminal, or
/// starts the pipboy-up animation, sound and menu), 2 waiting for the
/// animation to reach the pipboy pose, 3 open (keeps the map tidy), 4
/// lowering, 5 waiting for the player to stop being in the pipboy.
/// Callbacks stored at `+0x4c0` run when a transition finishes.
pub fn interface_manager_update_pipboy(e: &mut Engine, this: Ptr<InterfaceManager>) {
    let player = e.global::<u32>(PLAYER_SINGLETON);
    match e.get(this, InterfaceManager::field_4bc) {
        1 => {
            if is_top_menu(e, 0x3f0) {
                e.call(CONTAINER_MENU_CLOSE, &args![]);
            } else if is_top_menu(e, 0x421) || is_top_menu(e, 0x41f) {
                let rendered = e.get(this, InterfaceManager::pCurrentRenderedMenu);
                e.call(RENDERED_MENU_CLOSE, &args![rendered]);
            } else {
                let action = e.call(ACTOR_GET_ANIM_ACTION, &args![player]).u32();
                // The animation actions during which the pipboy cannot be
                // raised: 0-6, 8-0xd and 0x11.
                let blocked = matches!(action, 0..=6 | 8..=0xd | 0x11);
                if !blocked {
                    pipboy_raise(e, this, player);
                }
            }
        }
        2 => {
            let mut animation = e.call(PLAYER_GET_ANIMATION, &args![player, 1u32]).u32();
            if animation != 0
                && e.call(ANIMATION_GET_SEQUENCE, &args![animation, 2u32])
                    .u32()
                    != 0
            {
                if fn_0070f490(e, Ptr::new(animation), 2) == 1 {
                    pipboy_pose_reached(e, this, player, &mut animation);
                }
            } else {
                let pipboy = e.call(GET_PIPBOY_STATIC, &args![]).u32();
                e.call(PIPBOY_FADE_LIGHT_EFFECT, &args![pipboy, 0u32]);
                e.set(this, InterfaceManager::field_4bc, 4);
            }
            if e.call(MAP_MENU_NEEDS_TIDY, &args![]).bool() {
                e.call(MAP_MENU_TIDY, &args![0u32]);
            }
        }
        3 => {
            if e.call(MAP_MENU_NEEDS_TIDY, &args![]).bool() {
                e.call(MAP_MENU_TIDY, &args![0u32]);
            }
        }
        4 => {
            if (e.call(POP_FROM_ENTER_STACK, &args![this, 1u32, 0u32]).i32()) >= 0 {
                let pipboy = e.call(GET_PIPBOY_STATIC, &args![]).u32();
                e.call(PIPBOY_FADE_LIGHT_EFFECT, &args![pipboy, 0u32]);
                if e.vcall(player, 0x214, &args![]).i32() == 4 {
                    let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![player]).u32();
                    e.call(RELOAD_DYNAMIC_IDLE_ON_1ST_PERSON, &args![process, player]);
                }
                let animation = e.call(PLAYER_GET_ANIMATION, &args![player, 0u32]).u32();
                let group = e.call(ANIMATION_GROUP_ID, &args![animation, 0u32]).u16();
                e.vcall(player, 0x4b0, &args![group as u32, 1u32]);
                e.call(MAP_MENU_CLEAR_MAP_MEMORY, &args![]);
                loop {
                    let manager = e.call(GET_MANAGER, &args![]).u32();
                    if e.mem.u8(manager + 0x20) == 0 {
                        break;
                    }
                    let manager = e.call(GET_MANAGER, &args![]).u32();
                    let sound = e.mem.u8(manager + 0x20).wrapping_sub(1);
                    e.mem.set_u8(manager + 0x20, sound);
                    let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
                    e.call(AUDIO_UNPAUSE_TYPE_4000_0000, &args![audio]);
                }
                e.set(this, InterfaceManager::field_4bc, 5);
            }
        }
        5 if !e.call(PLAYER_IS_PIPBOY_ACTIVE, &args![player]).bool() => {
            let animation = e.call(PLAYER_GET_ANIMATION, &args![player, 0u32]).u32();
            let group = e.call(ANIMATION_GROUP_ID, &args![animation, 2u32]).u16();
            e.vcall(player, 0x4b0, &args![group as u32, 1u32]);
            e.set(this, InterfaceManager::field_4bc, 0);
            run_pipboy_callback(e, this);
        }
        _ => {}
    }
}

/// Runs and clears the callback stored at `+0x4c0`, if any.
fn run_pipboy_callback(e: &mut Engine, this: Ptr<InterfaceManager>) {
    let callback = e.get(this, InterfaceManager::field_4c0);
    if callback != 0 {
        e.call(callback, &args![]);
        e.set(this, InterfaceManager::field_4c0, 0);
    }
}

/// State 1 of `UpdatePipboy`: clears the player's current animation groups,
/// plays the pipboy-up group (`0xe2`), fades the light effect in, plays
/// `UIPipBoyAccessUp`, stops the get-hit image space modifier, pauses the
/// sound once more and opens the pipboy menu (state 2).
fn pipboy_raise(e: &mut Engine, this: Ptr<InterfaceManager>, player: u32) {
    let mut animation = e.call(PLAYER_GET_ANIMATION, &args![player, 0u32]).u32();
    if animation != 0 {
        e.call(ANIMATION_CLEAR_GROUP, &args![animation, 1u32, 0.0f32]);
        animation = e.call(PLAYER_GET_ANIMATION, &args![player, 1u32]).u32();
    }
    if animation == 0 {
        return;
    }
    e.call(ANIMATION_CLEAR_GROUP, &args![animation, 1u32, 0.0f32]);
    e.call(PLAYER_FORCE_TEMP_1ST_PERSON, &args![player, 0u32]);
    e.call(
        ANIMATION_PLAY_GROUP,
        &args![animation, 0xe2u32, 1u32, -1i32, -1i32],
    );
    let pipboy = e.call(GET_PIPBOY_STATIC, &args![]).u32();
    e.call(PIPBOY_FADE_LIGHT_EFFECT, &args![pipboy, 1u32]);
    e.with_stack(12, |e, found| {
        let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
        let handle = e
            .call(
                AUDIO_GET_SOUND_HANDLE_BY_NAME,
                &args![audio, found, PIPBOY_ACCESS_UP_SOUND, 0x121u32],
            )
            .u32();
        e.call(SOUND_HANDLE_PLAY, &args![handle, 0u32]);
        e.call(EMPTY_MEMBER_FUNCTION, &args![found]);
    });
    let get_hit = e.call(IMAGE_SPACE_GET_HIT, &args![]).u32();
    e.call(IMAGE_SPACE_STOP, &args![get_hit]);
    let manager = e.call(GET_MANAGER, &args![]).u32();
    let sound = e.mem.u8(manager + 0x20).wrapping_add(1);
    e.mem.set_u8(manager + 0x20, sound);
    let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
    fn_0070bba0(e, Ptr::new(audio));
    e.call(PLAYER_STATE_00962590, &args![player]);
    e.call(ADD_TO_ENTER_STACK, &args![this, 1u32]);
    e.call(INTERFACE_SHOW_MENUS, &args![]);
    e.set(this, InterfaceManager::field_4bc, 2);
}

/// State 2 of `UpdatePipboy` once the animation reached the pipboy pose:
/// shows the HUD for it, lets a drawn weapon's wobble clear, advances the
/// animation to the end of the pose, runs the pending callback and enters
/// the rendered pipboy menu (state 3).
fn pipboy_pose_reached(
    e: &mut Engine,
    this: Ptr<InterfaceManager>,
    player: u32,
    animation: &mut u32,
) {
    e.call(HUD_SET_MENU_MODE, &args![3u32]);
    if e.call(ACTOR_IS_WEAPON_DRAWN, &args![player]).bool() {
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![player]).u32();
        let weapon = if e.vcall(process, 0x148, &args![]).u32() == 0 {
            0
        } else {
            let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![player]).u32();
            let object = e.vcall(process, 0x148, &args![]).u32();
            e.call(FORM_GETTER_0044DDC0, &args![object]).u32()
        };
        if weapon != 0 && e.call(PLAYER_GET_ANIMATION, &args![player, 1u32]).u32() != 0 {
            let first = e.call(PLAYER_GET_ANIMATION, &args![player, 1u32]).u32();
            if e.call(ANIMATION_NODE, &args![first]).u32() != 0 {
                let second = e.call(PLAYER_GET_ANIMATION, &args![player, 1u32]).u32();
                let node = e.call(ANIMATION_NODE, &args![second]).u32();
                let index = e.call(WEAPON_TYPE_INDEX, &args![weapon]).u32();
                let entry = e.mem.u32(WEAPON_TABLE + index * 4);
                e.call(CLEAR_GUN_WOBBLE, &args![entry, node]);
            }
        }
    }
    let sequence = e
        .call(ANIMATION_GET_SEQUENCE, &args![*animation, 2u32])
        .u32();
    let end_time = e.call(SEQUENCE_END_TIME, &args![sequence]).f64();
    let mut time = (-end_time) as f32;
    let sequence = e
        .call(ANIMATION_GET_SEQUENCE, &args![*animation, 2u32])
        .u32();
    let global_transform = e
        .call(ANIMATION_ZERO_GLOBAL_TRANSFORM, &args![sequence])
        .u32();
    let group_time = e
        .call(ANIM_GROUP_GET_TIME, &args![global_transform, 1u32])
        .f64();
    time = (group_time + time as f64) as f32;
    e.call(ANIMATION_UPDATE, &args![*animation, player, 0.0f32, time]);
    e.call(
        ANIMATION_UPDATE_BIP_ONLY,
        &args![*animation, time, 0u32, 1u32],
    );
    e.call(
        ANIMATION_UPDATE_SCENE_GRAPH_NO_CONTROLLER,
        &args![*animation],
    );
    e.set_global(IDLE_MANAGER_BUSY, 1u8);
    let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![player]).u32();
    if e.vcall(process, 0x4d8, &args![player]).bool() {
        let first = e.call(PLAYER_GET_ANIMATION, &args![player, 0u32]).u32();
        e.call(ANIMATION_CLEAR_GROUP, &args![first, 0u32, 0.0f32]);
    } else {
        e.call(LOG_WARNING, &args![MISSING_DYNAMIC_IDLE_MESSAGE]);
    }
    e.set_global(IDLE_MANAGER_BUSY, 0u8);
    run_pipboy_callback(e, this);
    let manager = e.call(GET_MANAGER, &args![]).u32();
    let pipboy = e.call(GET_PIPBOY, &args![manager]).u32();
    e.call(PIPBOY_MANAGER_UPDATE, &args![pipboy]);
    let manager = e.call(GET_MANAGER, &args![]).u32();
    let pipboy = e.call(GET_PIPBOY, &args![manager]).u32();
    let manager = e.call(GET_MANAGER, &args![]).u32();
    e.call(ENTER_RENDERED_MENU, &args![manager, pipboy]);
    e.set(this, InterfaceManager::field_4bc, 3);
}

// Translated from 0070f490 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `this + 0x5c + index * 4`, with `index` 0x14 read as 1 and
/// 0x15 as 4 (the same code as `00491040`, which the linker folded; the
/// pipboy code calls it on an animation object).
pub fn fn_0070f490(e: &mut Engine, this: Ptr, index: i32) -> u32 {
    let index = match index {
        0x14 => 1,
        0x15 => 4,
        other => other,
    };
    e.mem.u32(
        this.addr()
            .wrapping_add(0x5c)
            .wrapping_add((index as u32).wrapping_mul(4)),
    )
}

// Translated from 0070f4e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Asks for the pipboy to open: only when the player is idle enough (not
/// in the states `005a03f0(4)`, `0x22c`, not mid-transition, no hot keys
/// wheel up, tile events accepted, the weapon state quiet, ...).
/// `value`, when not zero, is stored in the `0x1771` trait of the root
/// tile; the callback is run at once while the pipboy is open (state 3), or
/// stored for the transition when the pipboy is closed (state 0 becomes 1).
pub fn fn_0070f4e0(e: &mut Engine, this: Ptr<InterfaceManager>, callback: u32, value: u32) {
    let player = e.global::<u32>(PLAYER_SINGLETON);
    if e.call(CONDITION_CHECK_005A03F0, &args![player, 4u32])
        .bool()
    {
        return;
    }
    if e.vcall(player, 0x22c, &args![0u32]).bool() {
        return;
    }
    let first = e.call(PLAYER_QUERY_005737E0, &args![player]).u8();
    let second = e.call(PLAYER_QUERY_004EAF60, &args![player]).u8();
    if first != second {
        return;
    }
    if e.call(PLAYER_QUERY_0093A740, &args![player]).bool() {
        return;
    }
    if e.call(PLAYER_QUERY_005721E0, &args![player]).bool() {
        return;
    }
    let hot_keys = e.global::<u32>(HOT_KEYS_OWNER);
    if hot_keys != 0 && fn_0070f670(e, Ptr::new(hot_keys)) != 0 {
        return;
    }
    if !e.call(TILE_IS_ACCEPTING_EVENTS, &args![this, 0u32]).bool() {
        return;
    }
    if e.vcall(player, 0x230, &args![]).bool() {
        return;
    }
    if e.call(FORM_GETTER_0044DDC0, &args![WEAPON_STATE_OBJECT])
        .u32()
        != 0
    {
        return;
    }
    if e.global::<u8>(PLAYER_BUSY_FLAG) != 0 {
        return;
    }
    if e.call(WEAPON_STATE_FLOAT, &args![WEAPON_STATE_OBJECT])
        .f64()
        != 0.0
    {
        return;
    }
    let owner = e.global::<u32>(FLAGS_OWNER_8804);
    if e.call(FLAGS_QUERY_00701450, &args![owner, 1u32]).bool() {
        return;
    }
    if value != 0 {
        let root = e.get(this, InterfaceManager::pMenusRoot);
        tile_set_int(e, root, 0x1771, value);
    }
    let mode_word = e.get(this, InterfaceManager::field_4bc);
    if mode_word == 3 && callback != 0 {
        e.call(callback, &args![]);
    } else if mode_word == 0 {
        e.set(this, InterfaceManager::field_4bc, 1);
        e.set(this, InterfaceManager::field_4c0, callback);
    }
}

// Translated from 0070f670 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `00701740` on the sub-object at `+0x1c8` and returns its
/// result.
pub fn fn_0070f670(e: &mut Engine, this: Ptr) -> u32 {
    e.call(SUB_OBJECT_QUERY_00701740, &args![this.addr() + 0x1c8])
        .u32()
}

// Translated from 0070f690 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs `callback` at once when the pipboy is in state 0 and a callback
/// is given; in state 3 stores it and starts the lowering (state 4).
pub fn fn_0070f690(e: &mut Engine, this: Ptr<InterfaceManager>, callback: u32) {
    let mode_word = e.get(this, InterfaceManager::field_4bc);
    if mode_word == 0 && callback != 0 {
        e.call(callback, &args![]);
    } else if mode_word == 3 {
        e.set(this, InterfaceManager::field_4bc, 4);
        e.set(this, InterfaceManager::field_4c0, callback);
    }
}

// Translated from 0070f6e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterfaceManager::DoGamepadEvent` (Xbox PDB): handles one gamepad event
/// (`event` 0 reads the top one with `GetTopGamepadButton`). The menus are
/// offered the event from the topmost down (`DoGamepad`); a direction event
/// (1 to 4) moves the focus to the nearest tile that reacts to it; other
/// buttons click the tile bound to them; with no menu the console's pick
/// list steps.
pub fn interface_manager_do_gamepad_event(e: &mut Engine, this: Ptr<InterfaceManager>, event: i32) {
    let mut event = event;
    let mut released = false;
    let root = e.call(GET_MENUS_ROOT, &args![this]).u32();
    let mut tail = e.call(GET_FIELD_AT_4, &args![root + 4]).u32();
    // Every visible root child gets `DoGamepad(menu, -1, 0.0)` first, from
    // the last child to the first; the first that takes it ends the event.
    while tail != 0 {
        let child = e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), tail);
            let root = e.call(GET_MENUS_ROOT, &args![this]).u32();
            let element = e.call(LIST_PREVIOUS_ELEMENT, &args![root + 4, slot]).u32();
            tail = e.mem.u32(slot.addr());
            e.mem.u32(element)
        });
        let menu = if child != 0 {
            e.call(TILE_GET_MENU, &args![child]).u32()
        } else {
            0
        };
        if menu != 0 && e.call(TILE_IS_VISIBLE, &args![child]).bool() {
            let taken = e.call(DO_GAMEPAD, &args![this, menu, -1i32, 0.0f32]).bool();
            if taken {
                return;
            }
        }
    }
    if e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool()
        && e.get(this, InterfaceManager::bShowMouse) == 0
    {
        let mut flag_a = 0u8;
        let mut flag_b = 0u8;
        let over = e.call(GET_MOUSE_OVER_TARGET, &args![this]).u32();
        if over != 0 && e.call(TILE_GET_MENU, &args![over]).u32() != 0 {
            let menu = e.call(TILE_GET_MENU, &args![over]).u32();
            if !menu_is_active(e, menu) {
                return;
            }
        }
        if over != 0 {
            flag_a = e.call(TILE_IS_TRUE, &args![over, 0xfcfu32]).u8();
            flag_b = e.call(TILE_IS_TRUE, &args![over, 0xfd0u32]).u8();
        }
        if event == 0 {
            event = e.with_stack(4, |e, out| {
                let result = interface_manager_get_top_gamepad_button(e, this, flag_a, flag_b, out);
                released = e.mem.u8(out.addr()) != 0;
                result
            });
        }
        if event != 0 && event != 4 && event != 3 && event != 1 && event != 2 {
            e.call(KEY_REPEAT_00705B10, &args![this, 0u32]);
        }
    }
    let menu_manager = e.call(MENU_MANAGER_INSTANCE, &args![1u32]).u32();
    let frontmost = e.call(GET_FRONTMOST_MENU, &args![menu_manager]).u32();
    let frontmost_tile = if frontmost != 0 {
        e.call(GET_FIELD_AT_4, &args![frontmost]).u32()
    } else {
        0
    };
    if frontmost != 0
        && !released
        && e.call(DO_GAMEPAD, &args![this, frontmost, event, 1.0f32])
            .bool()
    {
        return;
    }
    if released {
        if frontmost != 0 {
            e.vcall(frontmost, 0x3c, &args![event, 1.0f32]);
        }
        return;
    }
    if frontmost != 0 && frontmost_tile != 0 && event != 0 {
        if matches!(event, 1..=4) {
            gamepad_direction_event(e, this, frontmost, frontmost_tile, event);
        } else {
            gamepad_button_event(e, this, frontmost, frontmost_tile, event);
        }
    } else {
        let console = e.call(MENU_CONSOLE_INSTANCE, &args![1u32]).u32();
        if e.call(CONSOLE_PRIMARY_DOWN, &args![console]).bool() && (event == 1 || event == 2) {
            let step = if event != 2 { 1 } else { -1 };
            let index = e.get(this, InterfaceManager::iCurrentPickIndex) + step;
            e.set(this, InterfaceManager::iCurrentPickIndex, index);
            let count = e.call(PICK_LIST_COUNT, &args![this.addr() + 0x70]).u32();
            if index < -1 {
                e.set(
                    this,
                    InterfaceManager::iCurrentPickIndex,
                    count.wrapping_sub(1) as i32,
                );
            } else if count.wrapping_sub(1) < index as u32 {
                e.set(this, InterfaceManager::iCurrentPickIndex, -1);
            }
            interface_manager_display_current_pick_ref(e, this);
        }
    }
}

/// The trait that names the tile reacting to each direction event.
fn direction_trait(event: i32) -> u32 {
    match event {
        1 => 0xfd7,
        2 => 0xfd8,
        4 => 0xfd9,
        3 => 0xfda,
        _ => 0,
    }
}

/// Part of `DoGamepadEvent`, for a direction event (1 to 4) and a frontmost
/// menu with a root tile: moves the focus.
fn gamepad_direction_event(
    e: &mut Engine,
    this: Ptr<InterfaceManager>,
    frontmost: u32,
    root_tile: u32,
    event: i32,
) {
    e.set(this, InterfaceManager::bMouseInMotion, 0);
    let trait_id = direction_trait(event);
    // The trait id the reacting tile was found under (out parameter).
    e.with_stack(4, |e, found_trait| {
        e.mem.set_u32(found_trait.addr(), 0xfc3);
        let current = e.call(GET_MOUSE_OVER_TARGET, &args![this]).u32();
        if current == 0 {
            // No tile has the focus: take the one with the largest focus
            // value, or the first reacting tile of the menu.
            let mut best = e.with_stack(4, |e, seed| {
                e.mem.set_u32(seed.addr(), 0x8000_0000);
                e.call(SCAN_FOR_MAX_FOCUS, &args![this, seed, 0u32]).u32()
            });
            e.call(SET_CURRENT_FOCUS_TARGET, &args![this, best, 0xfc3u32, 1u32]);
            if best == 0 {
                best = e
                    .call(
                        TILE_GET_FIRST_REF_COPY,
                        &args![root_tile, trait_id, found_trait],
                    )
                    .u32();
                if best != 0 {
                    e.call(TILE_PLAY_TILE_SOUND, &args![best, 0xfcbu32]);
                    tile_set_int(e, best, 0xfc7, 1);
                    tile_set_int(e, best, 0xfc7, 0);
                    let id = tile_menu_id(e, best);
                    e.vcall(frontmost, 0xc, &args![id, best]);
                } else if e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool()
                    || e.get(this, InterfaceManager::bShowMouse) == 0
                {
                    e.call(
                        DO_GAMEPAD,
                        &args![this, frontmost, event, trait_id as i32 as f32],
                    );
                }
            }
            return;
        }
        // The tiles already tried are kept in a `BSSimpleList` (`0096a2d0`,
        // add `005ae3d0`, contains `005f65d0`, clear `00470470`, destroy
        // `0046ffb0`); the add and contains functions take the address of
        // the variable that holds the tile.
        let visited = Ptr::<()>::new(e.mem.alloc(8));
        let current_cell = Ptr::<()>::new(e.mem.alloc(4));
        let candidate_cell = Ptr::<()>::new(e.mem.alloc(4));
        let mut candidate = e
            .call(
                TILE_GET_FIRST_REF_COPY,
                &args![current, trait_id, found_trait],
            )
            .u32();
        e.call(SIMPLE_LIST_CONSTRUCT, &args![visited]);
        e.mem.set_u32(current_cell.addr(), current);
        e.call(LIST_PUSH_FRONT_005AE3D0, &args![visited, current_cell]);
        loop {
            e.mem.set_u32(candidate_cell.addr(), candidate);
            if candidate == 0
                || e.call(LIST_CONTAINS, &args![visited, candidate_cell])
                    .bool()
                || (e.call(TILE_IS_TRUE, &args![candidate, 0xfa3u32]).bool()
                    && e.call(TILE_IS_TRUE, &args![candidate, 0xfafu32]).bool())
            {
                break;
            }
            e.call(LIST_PUSH_FRONT_005AE3D0, &args![visited, candidate_cell]);
            candidate = e
                .call(
                    TILE_GET_FIRST_REF_COPY,
                    &args![candidate, trait_id, found_trait],
                )
                .u32();
        }
        if candidate != 0
            && e.call(LIST_CONTAINS, &args![visited, candidate_cell])
                .bool()
            && (!e.call(TILE_IS_TRUE, &args![candidate, 0xfa3u32]).bool()
                || !e.call(TILE_IS_TRUE, &args![candidate, 0xfafu32]).bool())
        {
            candidate = 0;
        }
        e.call(LIST_CLEAR, &args![visited]);
        let found = e.mem.u32(found_trait.addr());
        if candidate != 0 {
            if candidate != current && found == 0xfc3 {
                e.call(TILE_PLAY_TILE_SOUND, &args![candidate, 0xfe8u32]);
            }
            if candidate != current && found == 0xfc7 {
                e.call(TILE_PLAY_TILE_SOUND, &args![candidate, 0xfcbu32]);
            }
            e.call(
                SET_CURRENT_FOCUS_TARGET,
                &args![this, candidate, found, 1u32],
            );
        } else {
            let taken = e
                .call(DO_GAMEPAD, &args![this, frontmost, event, 1.0f32])
                .bool();
            let over = e.get(this, InterfaceManager::pMouseOverTarget);
            if !taken && current != over {
                e.call(SET_CURRENT_FOCUS_TARGET, &args![this, current, found, 1u32]);
                if current != 0 {
                    e.call(TILE_PLAY_TILE_SOUND, &args![current, 0xfe8u32]);
                }
            } else if current != 0 && found == 0xfc7 {
                e.call(TILE_PLAY_TILE_SOUND, &args![current, 0xfcbu32]);
            }
        }
        e.call(SIMPLE_LIST_DESTROY, &args![visited]);
        e.mem.free(candidate_cell.addr());
        e.mem.free(current_cell.addr());
        e.mem.free(visited.addr());
    });
}

/// Part of `DoGamepadEvent`, for a button event (not a direction): offers
/// it to every visible root menu with its button state, then clicks the tile
/// bound to the button (a trait per button) in the focused tile's menu.
fn gamepad_button_event(
    e: &mut Engine,
    this: Ptr<InterfaceManager>,
    frontmost: u32,
    root_tile: u32,
    event: i32,
) {
    let mut event = event;
    let root = e.call(GET_MENUS_ROOT, &args![this]).u32();
    let mut tail = e.call(GET_FIELD_AT_4, &args![root + 4]).u32();
    while tail != 0 {
        let child = e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), tail);
            let root = e.call(GET_MENUS_ROOT, &args![this]).u32();
            let element = e.call(LIST_PREVIOUS_ELEMENT, &args![root + 4, slot]).u32();
            tail = e.mem.u32(slot.addr());
            e.mem.u32(element)
        });
        let menu = if child != 0 {
            e.call(TILE_GET_MENU, &args![child]).u32()
        } else {
            0
        };
        if menu != 0 && e.call(TILE_IS_VISIBLE, &args![child]).bool() {
            let state = e.call(CHECK_MENU_BUTTON, &args![event, 1u32]).i32();
            if e.call(DO_GAMEPAD, &args![this, menu, event, state as f32])
                .bool()
            {
                e.call(CLEAR_MENU_BUTTON, &args![event]);
                return;
            }
        }
    }
    let mut trait_id = match event {
        9 => 0xfdd,
        10 => 0xfde,
        0xb => 0xfdf,
        0xc => 0xfe0,
        0xe => 0xfe2,
        0xd => 0xfe1,
        0x10 => 0xfe4,
        0xf => 0xfe3,
        5 => 0xfe7,
        _ => 0,
    };
    let mut focus = e.call(GET_MOUSE_OVER_TARGET, &args![this]).u32();
    if focus == 0 {
        focus = e.get(this, InterfaceManager::pOverTileTarget);
    }
    if focus == 0 {
        focus = root_tile;
    }
    while focus != 0 && !e.call(TILE_IS_VISIBLE, &args![focus]).bool() {
        focus = e.call(TILE_PARENT, &args![focus]).u32();
    }
    if focus == 0 {
        return;
    }
    let (mut candidate, found) = e.with_stack(4, |e, found_trait| {
        let mut candidate = e
            .call(
                TILE_GET_FIRST_REF_COPY,
                &args![focus, trait_id, found_trait],
            )
            .u32();
        let mut walker = focus;
        while candidate == 0 && walker != 0 {
            walker = e.call(TILE_PARENT, &args![walker]).u32();
            if walker != 0 {
                candidate = e
                    .call(
                        TILE_GET_FIRST_REF_COPY,
                        &args![walker, trait_id, found_trait],
                    )
                    .u32();
            }
        }
        (candidate, e.mem.u32(found_trait.addr()))
    });
    if event == -2 {
        candidate = focus;
        event = 9;
        trait_id = 0xfdd;
    }
    let _ = trait_id;
    if candidate != 0
        && e.call(TILE_IS_VISIBLE, &args![candidate]).bool()
        && (found == 0xfc7 || found == 0xfc5)
    {
        if e.call(TILE_IS_TRUE, &args![candidate, 0xfafu32]).bool() {
            if found == 0xfc5 {
                let keys = e.get(this, InterfaceManager::iModifierKeys);
                e.set(this, InterfaceManager::iModifierKeys, keys | 4);
            }
            e.call(TILE_PLAY_TILE_SOUND, &args![candidate, 0xfcbu32]);
            tile_set_int(e, candidate, 0xfc7, 1);
            tile_set_int(e, candidate, 0xfc7, 0);
            let menu = e.call(TILE_GET_MENU, &args![candidate]).u32();
            let id = tile_menu_id(e, candidate);
            e.vcall(menu, 0xc, &args![id, candidate]);
            if found == 0xfc5 {
                let keys = e.get(this, InterfaceManager::iModifierKeys);
                e.set(this, InterfaceManager::iModifierKeys, keys & 0xfffb);
            }
            e.call(CLEAR_MENU_BUTTON, &args![event]);
            event = 0xa;
            if event != 0 {
                e.call(CLEAR_MENU_BUTTON, &args![6u32]);
            }
        } else {
            e.call(PLAY_MENU_SOUND, &args![2u32]);
        }
    } else if event == 9 && e.call(TILE_IS_TRUE, &args![focus, 0xfafu32]).bool() {
        e.call(TILE_PLAY_TILE_SOUND, &args![focus, 0xfcbu32]);
        tile_set_int(e, focus, 0xfc7, 1);
        tile_set_int(e, focus, 0xfc7, 0);
        let menu = e.call(TILE_GET_MENU, &args![focus]).u32();
        let id = tile_menu_id(e, focus);
        e.vcall(menu, 0xc, &args![id, focus]);
        e.call(CLEAR_MENU_BUTTON, &args![event]);
    }
    let _ = frontmost;
}

// Translated from 00710060 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterfaceManager::GetTopGamepadButton` (Xbox PDB): the gamepad button
/// that should act now, with key-repeat handling. Needs a controller and no
/// mouse. The buttons 9, 10, 0xb, 0xc, 0x10, 6 and 0xe act on their press
/// (`CheckMenuButton` 2) or hold (1) and `*released` is set when the button
/// is not held (state 1 is the held state). The four directions 1 to 4
/// repeat: the first press sets `iLastGamepadEvent` and the repeat time
/// (`+0x160`, from the tick count and the delay setting at `011d8b38`, or
/// `011d8aa4` for the repeats); the shoulder buttons 0xf, 0x10, 0xd, 0xe and
/// the button 5 act on their press. `flag_a` and `flag_b` say whether the
/// tile under the pointer wants the repeat for the left/right (1, 2) or
/// up/down (3, 4) pairs. Returns 0 when no button acts.
pub fn interface_manager_get_top_gamepad_button(
    e: &mut Engine,
    this: Ptr<InterfaceManager>,
    flag_a: u8,
    flag_b: u8,
    released: Ptr,
) -> i32 {
    let mut result = 0i32;
    if !e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool()
        || e.get(this, InterfaceManager::bShowMouse) != 0
    {
        return 0;
    }
    let owner = e.global::<u32>(CONTROLS_OWNER);
    e.call(CONTROLS_GET, &args![owner]);
    let mut reset_repeat = true;
    for button in [9i32, 10, 0xb, 0xc, 0x10, 6, 0xe] {
        let state = e.call(CHECK_MENU_BUTTON, &args![button, 1u32]).i32();
        if state == 2 || e.call(CHECK_MENU_BUTTON, &args![button, 1u32]).i32() == 1 {
            result = button;
            e.set(this, InterfaceManager::iLastGamepadEvent, 0);
            reset_repeat = false;
            let held = e.call(CHECK_MENU_BUTTON, &args![button, 1u32]).i32();
            e.mem.set_u8(released.addr(), (held == 1) as u8);
            return finish_top_gamepad_button(e, this, result, reset_repeat);
        }
    }
    // The four directions: pressed (state 2) and idle (state 0) flags.
    let pressed_right = e.call(CHECK_MENU_BUTTON, &args![4u32, 1u32]).i32() == 2;
    let pressed_left = e.call(CHECK_MENU_BUTTON, &args![3u32, 1u32]).i32() == 2;
    let pressed_up = e.call(CHECK_MENU_BUTTON, &args![1u32, 1u32]).i32() == 2;
    let pressed_down = e.call(CHECK_MENU_BUTTON, &args![2u32, 1u32]).i32() == 2;
    let idle_right = e.call(CHECK_MENU_BUTTON, &args![4u32, 1u32]).i32() == 0;
    let idle_left = e.call(CHECK_MENU_BUTTON, &args![3u32, 1u32]).i32() == 0;
    let idle_up = e.call(CHECK_MENU_BUTTON, &args![1u32, 1u32]).i32() == 0;
    let idle_down = e.call(CHECK_MENU_BUTTON, &args![2u32, 1u32]).i32() == 0;
    let pressed_11 = e.call(CHECK_MENU_BUTTON, &args![0x11u32, 1u32]).i32() == 2;
    let pressed_12 = e.call(CHECK_MENU_BUTTON, &args![0x12u32, 1u32]).i32() == 2;
    // Which single direction is pressed alone (and no other is pressed or
    // idle... the other three are neither pressed nor idle-released).
    if pressed_up && !pressed_right && !pressed_left && !idle_right && !idle_left {
        result = 1;
    } else if pressed_down && !pressed_right && !pressed_left && !idle_right && !idle_left {
        result = 2;
    } else if pressed_right && !pressed_up && !pressed_down && !idle_up && !idle_down {
        result = 4;
    } else if pressed_left && !pressed_up && !pressed_down && !idle_up && !idle_down {
        result = 3;
    } else if pressed_11 {
        result = 0x11;
    } else if pressed_12 {
        result = 0x12;
    }
    let last = e.get(this, InterfaceManager::iLastGamepadEvent);
    let mut held_direction = 0;
    if idle_up && last == 1 {
        held_direction = 1;
    } else if idle_down && last == 2 {
        held_direction = 2;
    } else if idle_right && last == 4 {
        held_direction = 4;
    } else if idle_left && last == 3 {
        held_direction = 3;
    }
    if result == 0
        && held_direction == 0
        && (idle_right as u32 + idle_left as u32 + idle_up as u32 + idle_down as u32) != 0
    {
        if idle_up {
            result = 1;
        } else if idle_down {
            result = 2;
        } else if idle_right {
            result = 4;
        } else if idle_left {
            result = 3;
        }
    }
    let wants_repeat = (flag_a != 0
        && (pressed_up as u32 + pressed_down as u32 + idle_up as u32 + idle_down as u32) != 0)
        || (flag_b != 0
            && (pressed_right as u32 + pressed_left as u32 + idle_right as u32 + idle_left as u32)
                != 0);
    let last = e.get(this, InterfaceManager::iLastGamepadEvent);
    if result == 0 || last == result {
        let last = e.get(this, InterfaceManager::iLastGamepadEvent);
        let tick = e.call(FADE_CLOCK_READ, &args![FADE_CLOCK]).u32();
        if last == held_direction && e.get(this, InterfaceManager::uGamepadRepeatStartTime) <= tick
        {
            result = last;
            if wants_repeat {
                set_repeat_time(e, this, REPEAT_DELAY_HOLDER_FAST);
            }
        } else if last == held_direction {
            result = 0;
        } else {
            result = 0;
            e.set(this, InterfaceManager::iLastGamepadEvent, 0);
        }
    } else {
        e.set(this, InterfaceManager::iLastGamepadEvent, result);
        if wants_repeat {
            set_repeat_time(e, this, REPEAT_DELAY_HOLDER_FIRST);
        } else {
            let tick = e.call(FADE_CLOCK_READ, &args![FADE_CLOCK]).u32();
            e.set(this, InterfaceManager::uGamepadRepeatStartTime, tick << 1);
        }
    }
    let any_direction = pressed_right
        || pressed_left
        || pressed_up
        || pressed_down
        || idle_right
        || idle_left
        || idle_up
        || idle_down;
    if any_direction {
        reset_repeat = false;
    }
    if result == 0 {
        let button_f = e.call(CHECK_MENU_BUTTON, &args![0xfu32, 1u32]).i32() == 2;
        let button_10 = e.call(CHECK_MENU_BUTTON, &args![0x10u32, 1u32]).i32() == 2;
        let button_d = e.call(CHECK_MENU_BUTTON, &args![0xdu32, 1u32]).i32() == 2;
        let button_e = e.call(CHECK_MENU_BUTTON, &args![0xeu32, 1u32]).i32() == 2;
        if button_f || button_10 || button_d || button_e {
            e.set(this, InterfaceManager::iLastGamepadEvent, 0);
            reset_repeat = false;
        }
        if button_f && !button_10 {
            result = 0xf;
        } else if button_10 && !button_f {
            result = 0x10;
        } else if button_d && !button_e {
            result = 0xd;
        } else if button_e && !button_d {
            result = 0xe;
        }
    }
    if result == 0 && e.call(CHECK_MENU_BUTTON, &args![5u32, 1u32]).i32() == 2 {
        result = 5;
        e.set(this, InterfaceManager::iLastGamepadEvent, 0);
        reset_repeat = false;
    }
    finish_top_gamepad_button(e, this, result, reset_repeat)
}

/// `uGamepadRepeatStartTime = trunc(float at holder + tick count)`.
fn set_repeat_time(e: &mut Engine, this: Ptr<InterfaceManager>, holder: u32) {
    let tick = e.call(FADE_CLOCK_READ, &args![FADE_CLOCK]).u32();
    let delay_ptr = e.call(FLOAT_HOLDER_GET, &args![holder]).u32();
    let delay = e.mem.f32(delay_ptr) as f64;
    let time = (delay + tick as f64) as i64 as u32;
    e.set(this, InterfaceManager::uGamepadRepeatStartTime, time);
}

/// The end of `GetTopGamepadButton`: with nothing acting, the repeat state
/// is reset (no last event, repeat time twice the tick count).
fn finish_top_gamepad_button(
    e: &mut Engine,
    this: Ptr<InterfaceManager>,
    result: i32,
    reset_repeat: bool,
) -> i32 {
    if reset_repeat {
        e.set(this, InterfaceManager::iLastGamepadEvent, 0);
        let tick = e.call(FADE_CLOCK_READ, &args![FADE_CLOCK]).u32();
        e.set(this, InterfaceManager::uGamepadRepeatStartTime, tick << 1);
    }
    result
}

// Translated from 00710880 (decompiled, FalloutNV.exe 1.4.0.525)
/// Rebuilds the console's pick list (`+0x70`) by casting a ray through the
/// pointer position (`NiPick`) and collecting the references hit; the pick
/// index returns to 0. The list is only replaced when the references differ
/// from the ones already listed.
pub fn fn_00710880(e: &mut Engine, this: Ptr<InterfaceManager>) {
    let base = this.addr();
    // Locals: origin and direction (12 bytes each), the `NiPick` (0x34) and
    // the new list (8).
    e.with_stack(0x34, |e, pick| {
        e.with_stack(0xc, |e, origin| {
            e.with_stack(0xc, |e, direction| {
                e.with_stack(8, |e, list| {
                    e.call(MEMBER_CONSTRUCTOR_EMPTY, &args![origin]);
                    e.call(MEMBER_CONSTRUCTOR_EMPTY, &args![direction]);
                    e.call(NI_PICK_CONSTRUCT, &args![pick, 0u32, 8u32]);
                    e.call(SIMPLE_LIST_CONSTRUCT, &args![list]);
                    let mut same_as_before = true;
                    e.call(NI_PICK_SET_FLAG, &args![pick, 1u32]);
                    let root = e
                        .call(GET_PICK_ROOT, &args![e.global::<u32>(PICK_ROOT_OWNER)])
                        .u32();
                    e.call(NI_PICK_SET_ROOT, &args![pick, root]);
                    e.call(NI_PICK_SET_MODE, &args![pick, 0u32]);
                    let player = e.global::<u32>(PLAYER_SINGLETON);
                    let fov = fn_00710ab0(e, Ptr::new(player));
                    let scene = e.call(GET_SCENE_GRAPH, &args![]).u32();
                    e.call(
                        SCENE_GRAPH_SET_CAMERA_FOV,
                        &args![scene, fov, 0u32, 0u32, 0u32],
                    );
                    let y = e.call(FTOL, &args![e.mem.f32(base + 0x40) as f64]).i32();
                    let x = e.call(FTOL, &args![e.mem.f32(base + 0x38) as f64]).i32();
                    let scene = e.call(GET_SCENE_GRAPH, &args![]).u32();
                    let camera = e.call(SCENE_GRAPH_GET_CAMERA, &args![scene]).u32();
                    e.call(
                        CAMERA_BUILD_PICK_RAY,
                        &args![camera, x, y, origin, direction, 0u32],
                    );
                    if e.call(NI_PICK_PICK_OBJECTS, &args![pick, origin, direction, 0u32])
                        .bool()
                    {
                        let mut node = base + 0x70;
                        let mut index = 0u32;
                        loop {
                            let results = e.call(NI_PICK_GET_RESULTS, &args![pick]).u32();
                            let count = e.call(RESULTS_COUNT, &args![results]).u32();
                            if index >= count {
                                break;
                            }
                            let results = e.call(NI_PICK_GET_RESULTS, &args![pick]).u32();
                            let entry = e.call(RESULTS_GET, &args![results, index]).u32();
                            let object = e.call(RESULT_OBJECT, &args![entry]).u32();
                            let reference = e.call(FIND_REFERENCE_FOR_3D, &args![object]).u32();
                            let seen = e.with_stack(4, |e, slot| {
                                e.mem.set_u32(slot.addr(), reference);
                                let seen = e.call(LIST_CONTAINS, &args![list, slot]).bool();
                                if !seen {
                                    e.call(LIST_ADD, &args![list, slot]);
                                }
                                seen
                            });
                            if !seen {
                                let same_item = node == 0 || {
                                    let item = e.call(MEMBER_CONSTRUCTOR_EMPTY, &args![node]).u32();
                                    e.mem.u32(item) == reference
                                };
                                if !same_item {
                                    same_as_before = false;
                                } else if same_as_before && node != 0 {
                                    node = e.call(GET_FIELD_AT_4, &args![node]).u32();
                                }
                            }
                            index += 1;
                        }
                    }
                    e.call(LIST_CLEAR, &args![base + 0x70]);
                    e.set(this, InterfaceManager::iCurrentPickIndex, 0);
                    if !same_as_before {
                        let mut node = list.addr();
                        while node != 0 {
                            let item = e.call(MEMBER_CONSTRUCTOR_EMPTY, &args![node]).u32();
                            if e.mem.u32(item) == 0 {
                                break;
                            }
                            let item = e.call(MEMBER_CONSTRUCTOR_EMPTY, &args![node]).u32();
                            e.call(LIST_ADD, &args![base + 0x70, item]);
                            node = e.call(GET_FIELD_AT_4, &args![node]).u32();
                        }
                    }
                    e.call(LIST_CLEAR, &args![list]);
                    e.call(SIMPLE_LIST_DESTROY, &args![list]);
                    e.call(NI_PICK_DESTRUCT, &args![pick]);
                });
            });
        });
    });
}

// Translated from 00710ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the `float` at `+0x670` of the object (called on the player; the
/// pick code passes it to the camera as the field of view).
pub fn fn_00710ab0(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + 0x670)
}

// Translated from 00710ad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InterfaceManager::DisplayCurrentPickRef` (Xbox PDB): the debug text
/// for the reference the console is looking at. Walks the pick list (`+0x70`)
/// to entry `iCurrentPickIndex`; with no entry clears `pPickRef` and prints
/// an empty line, otherwise sets `pPickRef` and prints its name and form id,
/// then (with `bFullHelp`) its script, current furniture, owner, count, the
/// furniture markers, lock, teleport door and flags. Lines the previous call
/// printed below the new last line are blanked.
pub fn interface_manager_display_current_pick_ref(e: &mut Engine, this: Ptr<InterfaceManager>) {
    let base = this.addr();
    let mut pick = 0u32;
    let line_height = debug_text_line_height(e);
    let start_y = setting_value(e, DEBUG_TEXT_TOP_SETTING);
    let mut y = start_y + 0x1e;
    let mut index = 0i32;
    let mut node = base + 0x70;
    while node != 0 {
        let item = e.call(MEMBER_CONSTRUCTOR_EMPTY, &args![node]).u32();
        if e.mem.u32(item) == 0 || e.get(this, InterfaceManager::iCurrentPickIndex) < index {
            break;
        }
        pick = e.mem.u32(item);
        index += 1;
        node = e.call(GET_FIELD_AT_4, &args![node]).u32();
    }
    if pick == 0 {
        e.set(this, InterfaceManager::pPickRef, 0);
        e.call(SET_PICK_REF_DISPLAY, &args![this, 0u32]);
        // The code works the line height out again here and does not use it.
        debug_text_line_height(e);
        let top = setting_value(e, DEBUG_TEXT_TOP_SETTING);
        debug_text_print_literal(e, EMPTY_TEXT, top);
    } else {
        with_string(e, |e, text| {
            e.call(SET_PICK_REF_DISPLAY, &args![this, pick]);
            e.set(this, InterfaceManager::pPickRef, pick);
            let form_id = e.call(FORM_ID_GETTER, &args![pick]).u32();
            let name = e.call(REFR_NAME, &args![pick]).u32();
            e.call(
                FORMAT_INTO_STRING,
                &args![text, FORMAT_NAME_AND_ID, name, form_id],
            );
            debug_text_print(e, text, start_y);
            if e.get(this, InterfaceManager::bFullHelp) != 0 {
                pick_ref_full_help(e, this, text, pick, &mut y, line_height);
            }
        });
    }
    let last = y;
    let bottom = e.global::<i32>(DEBUG_TEXT_BOTTOM);
    while y < bottom {
        debug_text_print_literal(e, EMPTY_TEXT, y);
        y += line_height;
    }
    e.set_global(DEBUG_TEXT_BOTTOM, last);
}

/// The word a setting holder keeps (`0043d4d0` returns a pointer to it).
fn setting_value(e: &mut Engine, holder: u32) -> i32 {
    let at = e.call(GET_SETTING_VALUE, &args![holder]).u32();
    e.mem.u32(at) as i32
}

/// Line height of the debug text: the height of the font the setting
/// `011f33c8` minus one selects, plus 3.
fn debug_text_line_height(e: &mut Engine) -> i32 {
    let level = setting_value(e, FONT_LEVEL_SETTING);
    let fonts = e.call(FONT_MANAGER_GET, &args![]).u32();
    let font = e
        .call(FONT_MANAGER_GET_FONT, &args![fonts, level - 1])
        .u32();
    let height = e.call(FONT_HEIGHT, &args![font]).f64();
    e.call(FTOL, &args![height]).i32() + 3
}

/// Runs `body` with a `BSStringT` local (`004037b0` / `004037d0`).
fn with_string<R>(e: &mut Engine, body: impl FnOnce(&mut Engine, Ptr) -> R) -> R {
    e.with_stack(8, |e, text| {
        e.call(STRING_CONSTRUCT, &args![text]);
        let result = body(e, text);
        e.call(STRING_DESTROY, &args![text]);
        result
    })
}

/// Prints the text of the string `text` at column `640.0` (the global at
/// `0103a1c4`) and row `y`: `DebugText::Instance(1)->Print(text, x, y, 2, -1,
/// -1.0, 0, 0)`.
fn debug_text_print(e: &mut Engine, text: Ptr, y: i32) {
    let chars = ni_pointer_get(e, text.addr());
    debug_text_print_literal(e, chars, y);
}

fn debug_text_print_literal(e: &mut Engine, chars: u32, y: i32) {
    let x: f32 = e.global(DEBUG_TEXT_X);
    let colour: f32 = e.global(CURSOR_DIRECTION_Y);
    let instance = e.call(DEBUG_TEXT_INSTANCE, &args![1u32]).u32();
    e.call(
        DEBUG_TEXT_PRINT,
        &args![instance, chars, x, y as f32, 2u32, -1i32, colour, 0u32, 0u32],
    );
}

/// One formatted debug line: `format` fills `text`, then it is printed at
/// `*y`, and `*y` moves down a line.
fn pick_line(e: &mut Engine, text: Ptr, y: &mut i32, line_height: i32, words: &[u32]) {
    let mut call = vec![text.addr()];
    call.extend_from_slice(words);
    e.call(FORMAT_INTO_STRING, &call);
    debug_text_print(e, text, *y);
    *y += line_height;
}

/// The `bFullHelp` part of `DisplayCurrentPickRef`.
fn pick_ref_full_help(
    e: &mut Engine,
    this: Ptr<InterfaceManager>,
    text: Ptr,
    pick: u32,
    y: &mut i32,
    line_height: i32,
) {
    let extra = e.call(REFR_EXTRA_DATA_LIST, &args![pick]).u32();
    if e.call(EXTRA_GET_SCRIPT, &args![extra]).u32() != 0 {
        let extra = e.call(REFR_EXTRA_DATA_LIST, &args![pick]).u32();
        let script = e.call(EXTRA_GET_SCRIPT, &args![extra]).u32();
        let extra = e.call(REFR_EXTRA_DATA_LIST, &args![pick]).u32();
        let script_again = e.call(EXTRA_GET_SCRIPT, &args![extra]).u32();
        let id = e.call(FORM_ID_GETTER, &args![script_again]).u32();
        let name = e.vcall(script, 0x130, &args![]).u32();
        pick_line(
            e,
            text,
            y,
            line_height,
            &args![SCRIPT_NAME_FORMAT, name, id],
        );
    }
    if e.vcall(pick, 0x100, &args![]).bool() {
        let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![pick]).u32();
        if process != 0 {
            let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![pick]).u32();
            if e.vcall(process, 0x4c8, &args![]).u32() != 0 {
                let first = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![pick]).u32();
                let second = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![pick]).u32();
                let furniture_a = e.vcall(second, 0x4c8, &args![]).u32();
                let furniture_b = e.vcall(first, 0x4c8, &args![]).u32();
                let id = e.call(FORM_ID_GETTER, &args![furniture_b]).u32();
                let name = e.vcall(furniture_a, 0x130, &args![]).u32();
                pick_line(e, text, y, line_height, &args![FURNITURE_FORMAT, name, id]);
            }
        }
    }
    if e.call(REFR_GET_OWNER, &args![pick]).u32() != 0 {
        let owner = e.call(REFR_GET_OWNER, &args![pick]).u32();
        let owner_again = e.call(REFR_GET_OWNER, &args![pick]).u32();
        let id = e.call(FORM_ID_GETTER, &args![owner_again]).u32();
        let name = e.vcall(owner, 0x130, &args![]).u32();
        pick_line(e, text, y, line_height, &args![OWNER_FORMAT, name, id]);
    }
    let extra = e.call(REFR_EXTRA_DATA_LIST, &args![pick]).u32();
    if e.call(EXTRA_GET_COUNT, &args![extra]).u16() as i16 != 1 {
        let extra = e.call(REFR_EXTRA_DATA_LIST, &args![pick]).u32();
        let count = e.call(EXTRA_GET_COUNT, &args![extra]).u16() as i16 as i32;
        pick_line(e, text, y, line_height, &args![COUNT_FORMAT, count]);
    }
    if e.vcall(pick, 0x100, &args![]).bool() {
        let actor = e
            .call(
                DYNAMIC_CAST,
                &args![pick, 0u32, FROM_TYPE, ACTOR_TYPE, 0u32],
            )
            .u32();
        if e.vcall(actor, 0x214, &args![]).u32() != 0 {
            let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![actor]).u32();
            if e.vcall(process, 0x4c8, &args![]).u32() == 0 {
                let process = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![actor]).u32();
                let word = e.vcall(process, 0x4d0, &args![]).u32();
                pick_line(e, text, y, line_height, &args![ACTOR_PACKAGE_FORMAT, word]);
            } else {
                let a = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![actor]).u32();
                let b = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![actor]).u32();
                let c = e.call(GET_SAVED_ACQUIRE_OBJECT, &args![actor]).u32();
                let furniture = e.vcall(c, 0x4c8, &args![]).u32();
                let word = e.vcall(a, 0x4d0, &args![]).u32();
                let other = e.vcall(b, 0x4c8, &args![]).u32();
                let id = e.call(FORM_ID_GETTER, &args![other]).u32();
                let name = e.vcall(furniture, 0x130, &args![]).u32();
                pick_line(
                    e,
                    text,
                    y,
                    line_height,
                    &args![ACTOR_FURNITURE_FORMAT, name, id, word],
                );
            }
        }
    }
    if e.call(REFR_IS_FURNITURE, &args![pick]).bool() {
        pick_furniture_lines(e, text, pick, y, line_height);
    }
    pick_lock_and_door_lines(e, this, text, pick, y, line_height);
}

/// The furniture part: the heading angle and one line for each marker.
fn pick_furniture_lines(e: &mut Engine, text: Ptr, pick: u32, y: &mut i32, line_height: i32) {
    let direction = e.call(REFR_GET_ROTATION, &args![pick]).u32();
    let z = e.mem.f32(direction + 8) as f64;
    let factor: f64 = e.global(RADIANS_TO_DEGREES);
    let degrees = e.call(FTOL, &args![z * factor]).i32();
    pick_line(e, text, y, line_height, &args![HEADING_FORMAT, degrees]);
    e.with_stack(0x14, |e, marker| {
        e.call(MARKER_CONSTRUCT, &args![marker, 0xffu32]);
        let mut index = 0i32;
        while e
            .call(REFR_GET_MARKER_AT_INDEX, &args![pick, index, marker])
            .bool()
        {
            let furniture = e.call(REFR_GET_FURNITURE_DATA, &args![pick]).u32();
            let marker_kind = e.call(MARKER_KIND, &args![marker]).u32();
            let enabled = e
                .call(FURNITURE_GET_MARKER_ENABLED, &args![furniture, index])
                .bool();
            let is_sit =
                |e: &mut Engine| e.call(FURNITURE_IS_SIT_MARKER, &args![marker_kind]).bool();
            if !enabled {
                let kind = if is_sit(e) { SIT_TEXT } else { SLEEP_TEXT };
                pick_line(
                    e,
                    text,
                    y,
                    line_height,
                    &args![DISABLED_MARKER_FORMAT, kind, marker_kind],
                );
            } else {
                let scale = e
                    .call(REFR_GET_SCALE, &args![e.global::<u32>(PLAYER_SINGLETON)])
                    .f32();
                let mut delta = [0f32; 3];
                e.with_stack(12, |e, out| {
                    let marker_pos = e.call(MARKER_KIND, &args![marker]).u32();
                    e.call(
                        FURNITURE_GET_MARKER_TARGET_OFFSET,
                        &args![furniture, out, marker_pos, scale],
                    );
                    for (i, d) in delta.iter_mut().enumerate() {
                        *d = e.mem.f32(out.addr() + 4 * i as u32);
                    }
                });
                let used = if e
                    .call(REFR_GET_MARKER_USED, &args![pick, index, 1u32])
                    .bool()
                {
                    USED_TEXT
                } else {
                    UNUSED_TEXT
                };
                let kind = if is_sit(e) { SIT_TEXT } else { SLEEP_TEXT };
                let angle = e
                    .call(FURNITURE_GET_MARKER_ANGLE, &args![furniture, marker_kind])
                    .f64();
                let degrees = e
                    .call(FTOL, &args![angle * e.global::<f64>(RADIANS_TO_DEGREES)])
                    .i32();
                pick_line(
                    e,
                    text,
                    y,
                    line_height,
                    &args![
                        MARKER_FORMAT,
                        kind,
                        marker_kind,
                        delta[0] as f64,
                        delta[1] as f64,
                        delta[2] as f64,
                        degrees,
                        used
                    ],
                );
            }
            index += 1;
        }
    });
}

/// The lock, teleport door and flag lines.
fn pick_lock_and_door_lines(
    e: &mut Engine,
    _this: Ptr<InterfaceManager>,
    text: Ptr,
    pick: u32,
    y: &mut i32,
    line_height: i32,
) {
    if e.call(REFR_GET_LOCK, &args![pick]).u32() != 0 {
        let lock = e.call(REFR_GET_LOCK, &args![pick]).u32();
        let locked = if e.call(LOCK_IS_LOCKED, &args![lock]).bool() {
            LOCKED_TEXT
        } else {
            UNLOCKED_TEXT
        };
        let level = e.call(LOCK_GET_LEVEL, &args![lock, pick]).u32();
        let holder = e.mem.u32(LOCK_LEVEL_TABLE + level * 4);
        let level_text = e.call(STRING_HOLDER_GET, &args![holder]).u32();
        pick_line(
            e,
            text,
            y,
            line_height,
            &args![LOCK_FORMAT, level_text, locked],
        );
        let key = e.mem.u32(lock + 4);
        if key != 0 {
            let name = e
                .call(MAP_MARKER_GET_LOCATION_NAME, &args![key + 0x30])
                .u32();
            pick_line(e, text, y, line_height, &args![KEY_FORMAT, name]);
        }
    }
    if e.call(REFR_GET_TELEPORT_DATA, &args![pick]).u32() != 0 {
        let door = e.call(REFR_GET_TELEPORT_DATA, &args![pick]).u32();
        let cell = e.call(DOOR_TELEPORT_GET_CELL, &args![door]).u32();
        let reference = ni_pointer_get(e, door);
        let mut cell_name = UNKNOWN_TEXT;
        let mut door_name = UNKNOWN_TEXT;
        let mut named = false;
        if cell != 0 {
            let name = e
                .call(MAP_MARKER_GET_LOCATION_NAME, &args![cell + 0x18])
                .u32();
            if name != 0 {
                cell_name = e
                    .call(MAP_MARKER_GET_LOCATION_NAME, &args![cell + 0x18])
                    .u32();
                named = true;
            }
        }
        if !named && reference != 0 {
            let extra = e.call(REFR_EXTRA_DATA_LIST, &args![reference]).u32();
            if e.call(EXTRA_IS_PERSISTENT, &args![extra]).u32() != 0 {
                cell_name = PERSISTENT_TEXT;
            }
        }
        if reference != 0 {
            let name = e.call(REFR_NAME, &args![reference]).u32();
            if name != 0 {
                door_name = e.call(REFR_NAME, &args![reference]).u32();
            }
        }
        pick_line(
            e,
            text,
            y,
            line_height,
            &args![TELEPORT_FORMAT, cell_name, door_name],
        );
    }
    let flags = e.call(REFR_FLAGS, &args![pick]).u32();
    if flags != 0 {
        e.with_stack(0x100, |e, buffer| {
            e.call(STRING_COPY, &args![buffer, 0x100u32, FLAGS_PREFIX]);
            for (bit, suffix) in [
                (1u32, FLAG_TEXT_1),
                (2, FLAG_TEXT_2),
                (4, FLAG_TEXT_4),
                (8, FLAG_TEXT_8),
            ] {
                if flags & bit != 0 {
                    e.call(STRING_APPEND, &args![buffer, 0x100u32, suffix]);
                }
            }
            let line = e.call(TRIM_TEXT, &args![buffer, 0u32]).u32();
            pick_line(e, text, y, line_height, &args![line]);
        });
    }
}

// Translated from 007118d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Moves the cursor tile's node with the mouse or the controller's cursor
/// stick (only when the cursor is wanted: no controller or the mouse in
/// use, and not while the console steals it). The movement comes from the
/// two mouse axes (or the stick axes scaled by the setting at `011d8c5c`),
/// speeded up by the acceleration settings, converted to screen units, kept
/// inside the screen and stored in the node's translation; when the position
/// changed, `sCursorPos`, `bMouseInMotion` and the real cursor position
/// (`+0x38`, `+0x3c`, `+0x40`) are updated, and the node is updated.
pub fn fn_007118d0(e: &mut Engine, this: Ptr<InterfaceManager>) {
    let controller = e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool();
    if controller && e.get(this, InterfaceManager::bShowMouse) == 0 {
        let console = e.call(MENU_CONSOLE_INSTANCE, &args![1u32]).u32();
        if !e.call(CONSOLE_PRIMARY_DOWN, &args![console]).bool() {
            return;
        }
    }
    with_scope_guard(e, 0xc51, |e| {
        let owner = e.global::<u32>(CONTROLS_OWNER);
        let controls = e.call(CONTROLS_GET, &args![owner]).u32();
        let controller = e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool();
        if controller && e.get(this, InterfaceManager::bShowMouse) == 0 {
            return;
        }
        if e.call(CONTROLS_FLAG_00711E00, &args![controls]).u32() != 0 {
            return;
        }
        let cursor = e.get(this, InterfaceManager::pCursor);
        let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
        e.call(NODE_SET_FLAG, &args![node, 0u32]);
        let cursor = e.get(this, InterfaceManager::pCursor);
        tile_set_int(e, cursor, 0xfa3, 1);
        let (mut mouse_x, mut mouse_y);
        if e.call(HAS_360_CONTROLLER_GETTER, &args![]).bool()
            && e.get(this, InterfaceManager::bShowMouse) == 0
        {
            mouse_x = e
                .call(CONTROLS_AXIS_00A23390, &args![controls, 0u32, 7u32])
                .i32();
            mouse_y = e
                .call(CONTROLS_AXIS_00A23390, &args![controls, 0u32, 8u32])
                .i32();
            let stick_scale = float_holder_value(e, STICK_SCALE_HOLDER);
            mouse_x = e.call(FTOL, &args![stick_scale * mouse_x as f64]).i32();
            let stick_scale = float_holder_value(e, STICK_SCALE_HOLDER);
            mouse_y = e.call(FTOL, &args![stick_scale * -(mouse_y as f64)]).i32();
        } else {
            mouse_x = e
                .call(CONTROLS_QUERY_00A239E0, &args![controls, 1u32])
                .i32();
            mouse_y = e
                .call(CONTROLS_QUERY_00A239E0, &args![controls, 2u32])
                .i32();
        }
        let mut speed = 1.0f32;
        if mouse_x != 0 || mouse_y != 0 {
            if e.call(MOUSE_SETTING_00711DA0, &args![controls]).f64() > 0.0 {
                let base = e.call(MOUSE_SETTING_00711DA0, &args![controls]).f64();
                let step = e.call(MOUSE_SETTING_00711D80, &args![controls]).f64();
                let level = e.call(MOUSE_LEVEL_00711D60, &args![controls]).u8() as i8 as i32;
                let twenty: f64 = e.global(MOUSE_LEVEL_DIVISOR);
                speed = ((level as f64 / twenty) * step + base) as f32;
            }
            if e.call(MOUSE_SETTING_00711DC0, &args![controls]).f64() > 0.0 {
                let bias = e.call(MOUSE_SETTING_00711DE0, &args![controls]).f64();
                let abs_x = e.call(ABS, &args![mouse_x]).i32();
                let abs_y = e.call(ABS, &args![mouse_y]).i32();
                let total = abs_x.wrapping_add(abs_y);
                let divisor = e.call(MOUSE_SETTING_00711DC0, &args![controls]).f64();
                speed = ((total as f64 / divisor + bias) * speed as f64) as f32;
            }
        }
        let cursor = e.get(this, InterfaceManager::pCursor);
        let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
        let translation = e.call(NODE_TRANSLATION, &args![node]).u32();
        let old = [
            e.mem.f32(translation),
            e.mem.f32(translation + 4),
            e.mem.f32(translation + 8),
        ];
        let width = e.call(GET_SCREEN_WIDTH, &args![]).f64();
        let horizontal: f64 = e.global(CURSOR_HORIZONTAL_SCALE);
        let moved_x =
            (((width * mouse_x as f64) / horizontal) * speed as f64 + old[0] as f64) as f32;
        let half_width = clamp_edge(e, moved_x, EDGE_RIGHT, EDGE_LEFT);
        let tilt = tile_get_float(e, e.get(this, InterfaceManager::pCursor), 0xfad);
        let moved_y_part = tilt * e.global::<f64>(CURSOR_TILT_SCALE);
        let height = e.call(GET_SCREEN_HEIGHT, &args![]).f64();
        let vertical: f64 = e.global(CURSOR_VERTICAL_SCALE);
        let moved_z =
            (old[2] as f64 - ((height * mouse_y as f64) / vertical) * speed as f64) as f32;
        let z = clamp_edge(e, moved_z, EDGE_TOP, EDGE_BOTTOM);
        let position = [half_width, moved_y_part as f32, z];
        let cursor = e.get(this, InterfaceManager::pCursor);
        let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
        e.with_stack(12, |e, vector| {
            for (i, v) in position.iter().enumerate() {
                e.mem.set_f32(vector.addr() + 4 * i as u32, *v);
            }
            e.call(NODE_SET_TRANSLATE_VECTOR, &args![node, vector]);
        });
        if e.mem.f32(this.addr() + 0x2c) != position[0]
            || e.mem.f32(this.addr() + 0x34) != position[2]
        {
            for (i, v) in position.iter().enumerate() {
                e.mem.set_f32(this.addr() + 0x2c + 4 * i as u32, *v);
            }
            e.set(this, InterfaceManager::bMouseInMotion, 1);
            let desktop_width = e.call(GET_DESKTOP_WIDTH, &args![]).f64();
            let width = e.call(GET_SCREEN_WIDTH, &args![]).f64();
            let cursor_x = e.mem.f32(this.addr() + 0x2c) as f64;
            let desktop_width_again = e.call(GET_DESKTOP_WIDTH, &args![]).f64();
            let two: f64 = e.global(TWO);
            let real_x = (desktop_width_again / two + (desktop_width / width) * cursor_x) as f32;
            e.mem.set_f32(this.addr() + 0x38, real_x);
            let cursor = e.get(this, InterfaceManager::pCursor);
            let tilt = tile_get_float(e, cursor, 0xfad);
            e.mem.set_f32(
                this.addr() + 0x3c,
                (tilt * e.global::<f64>(CURSOR_TILT_SCALE)) as f32,
            );
            let desktop_height = e.call(GET_DESKTOP_HEIGHT, &args![]).f64();
            let half_height = desktop_height / e.global::<f64>(TWO);
            let desktop_height_again = e.call(GET_DESKTOP_HEIGHT, &args![]).f64();
            let screen_height = e.call(GET_SCREEN_HEIGHT, &args![]).f64();
            let cursor_z = e.mem.f32(this.addr() + 0x34) as f64;
            let real_z = (half_height - (desktop_height_again / screen_height) * cursor_z) as f32;
            e.mem.set_f32(this.addr() + 0x40, real_z);
        }
        e.with_stack(12, |e, update_data| {
            e.call(
                NI_UPDATE_DATA_CONSTRUCT,
                &args![update_data, 0.0f32, 0u32, 0u32],
            );
            let cursor = e.get(this, InterfaceManager::pCursor);
            let node = e.call(TILE_IMAGE_NODE, &args![cursor]).u32();
            e.call(NODE_UPDATE, &args![node, update_data]);
        });
    });
}

/// `*holder` as a `float` widened to `double` (`00403e20` returns a pointer
/// to the float).
fn float_holder_value(e: &mut Engine, holder: u32) -> f64 {
    let at = e.call(FLOAT_HOLDER_GET, &args![holder]).u32();
    e.mem.f32(at) as f64
}

/// Clamps a cursor coordinate to the screen: `max(min(value, edge_high - 2),
/// edge_low - 2)` for x and `+ 1` for z, with the two clamp helpers
/// `0040ebd0` (min) and `00404010` (max).
fn clamp_edge(e: &mut Engine, value: f32, high: u32, low: u32) -> f32 {
    let high_edge = e.call(high, &args![]).f64();
    let margin: f64 = if high == EDGE_RIGHT {
        -e.global::<f64>(TWO)
    } else {
        e.global::<f64>(ONE_DOUBLE)
    };
    let upper = (high_edge + margin) as f32;
    let min = e.call(FLOAT_MIN, &args![upper, value]).f32();
    let low_edge = e.call(low, &args![]).f64();
    let lower = (low_edge + margin) as f32;
    e.call(FLOAT_MAX, &args![lower, min]).f32()
}

// @@FUNCS-END@@

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    type Log = Rc<RefCell<Vec<Vec<u32>>>>;

    /// Every callee outside this file: tests replace them all with doubles
    /// that return 0, then give the ones they care about a behaviour.
    const EXTERNAL: &[u32] = &[
        CURSOR_NAME,
        GLOBALS_XML_PATH,
        BACKGROUND_FILL_ALPHA_NAME,
        HAS_360_CONTROLLER_NAME,
        DEBUG_TEXT_ROOT_NAME,
        TWO_FIFTY_FIVE,
        TWO,
        WIDESCREEN_RATIO,
        WIDESCREEN_INVERSE_RATIO,
        ONE_TO_ONE_ANGLE,
        CURSOR_DIRECTION_Y,
        TIMER_DEFAULT_END,
        XINPUT_GET_STATE,
        OPERATOR_NEW,
        OPERATOR_DELETE,
        NI_ALLOC,
        SCOPE_GUARD_BEGIN,
        SCOPE_GUARD_END,
        NI_POINTER_CONSTRUCT,
        NI_POINTER_ASSIGN,
        NI_POINTER_GET,
        NI_POINTER_DESTROY,
        MEMBER_CONSTRUCTOR_EMPTY,
        SIMPLE_LIST_CONSTRUCT,
        SIMPLE_LIST_DESTROY,
        NI_POINT3_CONSTRUCT,
        NI_UPDATE_DATA_CONSTRUCT,
        NODE_UPDATE,
        NODE_UPDATE_PROPERTIES,
        NODE_SET_FLAG,
        HUD_EFFECT_MANAGER_CONSTRUCT,
        HUD_EFFECT_MANAGER_DESTROY,
        VATS_EFFECT_MANAGER_CONSTRUCT,
        VATS_EFFECT_MANAGER_DESTROY,
        PIPBOY_MANAGER_CONSTRUCT,
        TILE_SET_INT,
        TILE_SET_FLOAT,
        TILE_SET_STRING,
        TILE_UPDATE_TILE,
        TILE_TEXT_TO_TRAIT,
        TILE_ADD_DIRTY_TILE,
        TILE_UPDATE_ALL,
        TILE_READ_FILE,
        TILE_SET_PARENT,
        TILE_LOAD_TEXT_TABLE,
        TILE_FREE_TRAIT_LIST,
        TILE_DESTRUCT,
        TILE_RELEASE,
        MENU_REMOVE_FROM_XML_ARCHIVE,
        MENU_FREE_XML_ARCHIVE,
        GET_SCREEN_WIDTH,
        GET_SCREEN_HEIGHT,
        GET_INTERFACE_MARGIN_A,
        GET_INTERFACE_MARGIN_B,
        GET_INTERFACE_SCALE,
        CREATE_SCENE_GRAPH,
        SCENE_GRAPH_PARENT_3D,
        SCENE_GRAPH_PARENT,
        MENU_MANAGER_INSTANCE,
        ALPHA_PROPERTY_CONSTRUCT,
        ALPHA_PROPERTY_SET_BLEND,
        ALPHA_PROPERTY_SET_NO_SORTER,
        ALPHA_PROPERTY_SET_SOURCE_MODE,
        ALPHA_PROPERTY_SET_DESTINATION_MODE,
        SHADER_ACCUMULATOR_CONSTRUCT,
        SHADER_ACCUMULATOR_SET_194,
        SHADER_ACCUMULATOR_SET_19C,
        SHADOW_SCENE_NODE_CONSTRUCT,
        SET_SHADOW_SCENE_NODE,
        VIEW_CASTER_CONSTRUCT,
        MENU_ROOT_TILE_CONSTRUCT,
        KEY_TABLE_CONSTRUCT,
        FADE_CLOCK_READ,
        FADE_CLOCK,
        ANGLE_FUNCTION_A,
        ANGLE_FUNCTION_B,
        FONT_MANAGER_INIT,
        SYSTEM_COLOR_MANAGER_GET_INSTANCE,
        NI_NODE_CONSTRUCT,
        NODE_SET_NAME,
        FIXED_STRING_CONSTRUCT,
        FIXED_STRING_DESTROY,
        FOG_PROPERTY_CONSTRUCT,
        TILE_IMAGE_NODE,
        NODE_SET_TRANSLATE,
        GET_FRAME_SCENE_NODE,
        GET_SECOND_SCENE_NODE,
        SET_DEBUG_TEXT_VISIBLE,
        GET_DESKTOP_WIDTH,
        GET_DESKTOP_HEIGHT,
        STRING_LIST_SET_LIMIT,
        LOADING_MENU_CREATE,
        HAS_360_CONTROLLER_GETTER,
        FLOAT_HOLDER_GET,
        STRING_HOLDER_GET,
        HUD_MAIN_MENU_CREATE,
        HUD_MAIN_MENU_SET_MENU_MODE,
        TILE_IMAGE_VTABLE,
        TILE_FLAG_TEST,
        TILE_CONSTRUCT,
        TILE_IMAGE_TYPE,
        TILE_IMAGE_TYPE_NAME,
        DEBUG_TEXT_SHUTDOWN,
        FONT_MANAGER_SHUTDOWN,
        EMPTY_MEMBER_FUNCTION,
        TILE_DELETING,
        MENU_MANAGER_SHUTDOWN,
        FREE_TIMERS,
        LIST_CLEAR,
        LIST_AT_011D8B54,
        STATIC_OBJECT_SHUTDOWN_A,
        MESSAGE_MENU_SHUTDOWN,
        CLEAR_TEMP_MODEL,
        START_MENU_SHUTDOWN,
        STATIC_OBJECT_SHUTDOWN_B,
        POINTER_LIST_REMOVE_ALL,
        SCRIPT_POINTER_LIST,
        GLOBAL_OBJECT_8CE8,
        OBJECT_8CE8_DESTRUCT,
        GLOBAL_OBJECT_A0C4,
        VIEW_CASTER_DESTRUCT,
        FLOAT_HOLDER_BACKGROUND,
        STRING_HOLDER_A,
        STRING_HOLDER_B,
        STRING_LIST_A,
        STRING_LIST_B,
        SET_FOG_RANGE,
        CONTROLS_OWNER,
        CONTROLS_GET,
        CLEAR_KEYSTROKES,
        HUD_SET_INFO_FOR_REF,
        MENU_CONSOLE_INSTANCE,
        MENU_CONSOLE_ON_ENTER_MENU_MODE,
        GET_MANAGER,
        AUDIO_INSTANCE,
        AUDIO_PAUSE_TYPE,
        AUDIO_PAUSE_TYPE_6,
        AUDIO_UNPAUSE_TYPE_6,
        AUDIO_UNPAUSE_TYPE_4000_0000,
        MENU_MODE_IS_NOT_ONE,
        UPDATE_ALL_TIMERS,
        PLAYER_SINGLETON,
        LAST_MENU_MODE,
        CONTROLLER_STATE,
        LAST_HAS_CONTROLLER,
        LAST_HAS_CONTROLLER_GUARD,
        MESSAGE_TITLE_HOLDER,
        TEXT_HOLDER_CONTROLLER_WAS_SET,
        TEXT_HOLDER_CONTROLLER_WAS_CLEAR,
        SHOW_MESSAGE_BOX,
        CONTROLLER_CHANGED,
        PRELOAD_MAIN_MENUS,
        CONTROLS_QUERY_00A239E0,
        CONTROLS_QUERY_00A23A50,
        CONTROLS_QUERY_00A24660,
        CONTROLS_QUERY_00A24180,
        CONTROLS_QUERY_00A238A0,
        CONTROLS_AXIS_00A23390,
        CONTROLS_NEXT_EVENT,
        CONTROLS_CLEAR_USER_ACTIONS,
        GET_PIPBOY,
        PIPBOY_UPDATE_LIGHT_EFFECT,
        HUD_EFFECTS_UPDATE,
        VATS_EFFECTS_UPDATE,
        GAME_STATE_ID,
        CONDITION_CHECK_005A03F0,
        PLAYER_FLAG_00950090,
        ACTOR_GET_IRON_SIGHTS,
        WEAPON_STATE_GETTER,
        FORM_GETTER_0044DDC0,
        WEAPON_STATE_OBJECT,
        HUD_SET_MENU_MODE,
        GET_ENTER_STACK_TOP,
        QUERY_00719AE0,
        ACTION_00718930,
        IS_TOP_MENU_ID,
        IS_TOP_MENU_FADED_IN,
        IS_PIPBOY_MENU_TOPMOST,
        OBJECT_FLAG_0070ED80,
        PIPBOY_MENU_CLASS,
        TILE_GET_MENU_BY_CLASS,
        IS_CONSOLE_VISIBLE,
        IS_IN_GAME_LOADING_MENU_OPEN,
        XUI_IS_UP,
        GET_MOUSE_OVER_TARGET,
        TILE_IS_VISIBLE,
        TILE_IS_TRUE,
        GET_DEFAULT_FOCUS,
        CONSOLE_PRIMARY_DOWN,
        FRAME_TIME_GETTER,
        GET_SCREEN_SCALE,
        NI_POINT2_CONSTRUCT,
        GET_MENU_HEIGHT,
        PIPBOY_MARGIN,
        TILE_GET_VALUE,
        TILE_GET_VALUE_Q,
        TILE_GET_MENU,
        PICK_TILE,
        DO_LEAVE,
        DO_ENTER,
        DO_WHEEL_MOVE,
        SET_CURSOR_OFFSET_X,
        SET_CURSOR_OFFSET_Y,
        GET_CURSOR_OFFSET_X,
        GET_CURSOR_OFFSET_Y,
        TILE_GET_POSITION_X,
        TILE_GET_POSITION_Y,
        TILE_PLAY_TILE_SOUND,
        TILE_CHILD_LIST_FIND,
        CLEAR_MOUSE_OVER_TARGET,
        TILE_PARENT,
        GET_WHEEL_STEPS,
        PICK_LIST_COUNT,
        MENU_STATE,
        FTOL,
        RENDERED_MENU_OR_PIPBOY,
        MENU_FLAG_QUERY_004A4040,
        SOUND_HANDLE_CONSTRUCT,
        SOUND_HANDLE_ASSIGN,
        AUDIO_GET_SOUND_HANDLE_BY_NAME,
        UI_MENU_MODE_SOUND,
        SOUND_HANDLE_PLAY,
        SOUND_HANDLE_RELEASE,
        SET_STATS_MENU_VISIBLE,
        SET_INVENTORY_MENU_VISIBLE,
        SET_MAP_MENU_VISIBLE,
        MENU_CONSOLE_IDLE,
        KEY_REPEAT_RESET,
        TRANSLATE_KEY_EVENT,
        KEY_REPEAT_00716730,
        MODIFIER_FOUR_DOWN,
        GET_FRONTMOST_MENU,
        FORMAT_STRING,
        FORMAT_PC_BUTTON,
        PC_BUTTON_PREFIX,
        GET_FIELD_AT_4,
        TILE_GET_STRING,
        TILE_GET_CHILD_BY_NAME,
        PLAY_MENU_SOUND,
        SHOW_ICON_MESSAGE,
        MESSAGE_HOLDER_19,
        MESSAGE_HOLDER_1A,
        MESSAGE_ICON_SIZE,
        MESSAGE_ICON_PATH,
        CONSOLE_KEY_FLAG_GET,
        CONSOLE_KEY_FLAG_SET,
        MENU_CONSOLE_TOGGLE_VISIBLE,
        ADD_TO_ENTER_STACK,
        POP_FROM_ENTER_STACK,
        MENU_MANAGER_IS_MENU_OPEN,
        START_MENU_ALLOWED,
        START_MENU_CLOSE,
        START_MENU_CREATE,
        START_MENU_SETTING,
        QUEUE_PENDING_00070EC00,
        QUEUE_MENU_CREATE,
        DEBUG_TEXT_UPDATE,
        MENU_LOOP_ABORT,
        LIST_NEXT_ELEMENT,
        CHECK_MENU_BUTTON,
        CLEAR_MENU_BUTTON,
        TUTORIAL_MANAGER_UPDATE,
        GET_MENUS_ROOT,
        LEVEL_UP_MENU_CREATE,
        CURSOR_STRING_PENDING,
        LEVEL_UP_MENU_PENDING,
        PLAYER_CAST_EAT_DRINK_ITEMS,
        PLAYER_CAST_QUEUED_ENCHANTMENTS,
        CONTAINER_MENU_CLOSE,
        RENDERED_MENU_CLOSE,
        ACTOR_GET_ANIM_ACTION,
        PLAYER_GET_ANIMATION,
        ANIMATION_CLEAR_GROUP,
        PLAYER_FORCE_TEMP_1ST_PERSON,
        ANIMATION_PLAY_GROUP,
        GET_PIPBOY_STATIC,
        PIPBOY_FADE_LIGHT_EFFECT,
        PIPBOY_ACCESS_UP_SOUND,
        IMAGE_SPACE_GET_HIT,
        IMAGE_SPACE_STOP,
        PLAYER_STATE_00962590,
        INTERFACE_SHOW_MENUS,
        ANIMATION_GET_SEQUENCE,
        MAP_MENU_NEEDS_TIDY,
        MAP_MENU_TIDY,
        PLAYER_IS_PIPBOY_ACTIVE,
        ANIMATION_GROUP_ID,
        MAP_MENU_CLEAR_MAP_MEMORY,
        RELOAD_DYNAMIC_IDLE_ON_1ST_PERSON,
        GET_SAVED_ACQUIRE_OBJECT,
        ACTOR_IS_WEAPON_DRAWN,
        WEAPON_TYPE_INDEX,
        WEAPON_TABLE,
        CLEAR_GUN_WOBBLE,
        ANIMATION_NODE,
        SEQUENCE_END_TIME,
        ANIMATION_ZERO_GLOBAL_TRANSFORM,
        ANIM_GROUP_GET_TIME,
        ANIMATION_UPDATE,
        ANIMATION_UPDATE_BIP_ONLY,
        ANIMATION_UPDATE_SCENE_GRAPH_NO_CONTROLLER,
        IDLE_MANAGER_BUSY,
        LOG_WARNING,
        MISSING_DYNAMIC_IDLE_MESSAGE,
        PIPBOY_MANAGER_UPDATE,
        ENTER_RENDERED_MENU,
        PLAYER_QUERY_005737E0,
        PLAYER_QUERY_004EAF60,
        PLAYER_QUERY_0093A740,
        PLAYER_QUERY_005721E0,
        HOT_KEYS_OWNER,
        TILE_IS_ACCEPTING_EVENTS,
        PLAYER_BUSY_FLAG,
        WEAPON_STATE_FLOAT,
        FLAGS_OWNER_8804,
        FLAGS_QUERY_00701450,
        SUB_OBJECT_QUERY_00701740,
        FLAGS_OWNER,
        HAS_FLAG,
        INVENTORY_MENU,
        KEYRING_TRAIT,
        REPEAT_DELAY_HOLDER_FAST,
        REPEAT_DELAY_HOLDER_FIRST,
        DO_GAMEPAD,
        KEY_REPEAT_00705B10,
        LIST_PREVIOUS_ELEMENT,
        SCAN_FOR_MAX_FOCUS,
        SET_CURRENT_FOCUS_TARGET,
        TILE_GET_FIRST_REF_COPY,
        LIST_PUSH_FRONT_005AE3D0,
        LIST_CONTAINS,
        NI_PICK_CONSTRUCT,
        NI_PICK_DESTRUCT,
        NI_PICK_SET_FLAG,
        GET_PICK_ROOT,
        PICK_ROOT_OWNER,
        NI_PICK_SET_ROOT,
        NI_PICK_SET_MODE,
        GET_SCENE_GRAPH,
        SCENE_GRAPH_SET_CAMERA_FOV,
        SCENE_GRAPH_GET_CAMERA,
        CAMERA_BUILD_PICK_RAY,
        NI_PICK_PICK_OBJECTS,
        NI_PICK_GET_RESULTS,
        RESULTS_COUNT,
        RESULTS_GET,
        RESULT_OBJECT,
        FIND_REFERENCE_FOR_3D,
        LIST_ADD,
        GET_SETTING_VALUE,
        DEBUG_TEXT_TOP_SETTING,
        FONT_LEVEL_SETTING,
        FONT_MANAGER_GET,
        FONT_MANAGER_GET_FONT,
        FONT_HEIGHT,
        SET_PICK_REF_DISPLAY,
        FORM_ID_GETTER,
        REFR_NAME,
        FORMAT_INTO_STRING,
        FORMAT_NAME_AND_ID,
        DEBUG_TEXT_X,
        DEBUG_TEXT_INSTANCE,
        DEBUG_TEXT_PRINT,
        EMPTY_TEXT,
        DEBUG_TEXT_BOTTOM,
        STRING_CONSTRUCT,
        STRING_DESTROY,
        REFR_EXTRA_DATA_LIST,
        EXTRA_GET_SCRIPT,
        SCRIPT_NAME_FORMAT,
        FURNITURE_FORMAT,
        REFR_GET_OWNER,
        OWNER_FORMAT,
        EXTRA_GET_COUNT,
        COUNT_FORMAT,
        DYNAMIC_CAST,
        FROM_TYPE,
        ACTOR_TYPE,
        ACTOR_PACKAGE_FORMAT,
        ACTOR_FURNITURE_FORMAT,
        REFR_IS_FURNITURE,
        REFR_GET_ROTATION,
        RADIANS_TO_DEGREES,
        HEADING_FORMAT,
        MARKER_CONSTRUCT,
        REFR_GET_MARKER_AT_INDEX,
        REFR_GET_FURNITURE_DATA,
        MARKER_KIND,
        FURNITURE_GET_MARKER_ENABLED,
        FURNITURE_IS_SIT_MARKER,
        SIT_TEXT,
        SLEEP_TEXT,
        DISABLED_MARKER_FORMAT,
        REFR_GET_SCALE,
        FURNITURE_GET_MARKER_TARGET_OFFSET,
        REFR_GET_MARKER_USED,
        USED_TEXT,
        UNUSED_TEXT,
        FURNITURE_GET_MARKER_ANGLE,
        MARKER_FORMAT,
        REFR_GET_LOCK,
        LOCK_IS_LOCKED,
        LOCKED_TEXT,
        UNLOCKED_TEXT,
        LOCK_GET_LEVEL,
        LOCK_LEVEL_TABLE,
        LOCK_FORMAT,
        MAP_MARKER_GET_LOCATION_NAME,
        KEY_FORMAT,
        REFR_GET_TELEPORT_DATA,
        DOOR_TELEPORT_GET_CELL,
        UNKNOWN_TEXT,
        PERSISTENT_TEXT,
        EXTRA_IS_PERSISTENT,
        TELEPORT_FORMAT,
        REFR_FLAGS,
        STRING_COPY,
        FLAGS_PREFIX,
        STRING_APPEND,
        FLAG_TEXT_1,
        FLAG_TEXT_2,
        FLAG_TEXT_4,
        FLAG_TEXT_8,
        TRIM_TEXT,
        CONTROLS_FLAG_00711E00,
        STICK_SCALE_HOLDER,
        MOUSE_SETTING_00711DA0,
        MOUSE_SETTING_00711D80,
        MOUSE_LEVEL_00711D60,
        MOUSE_SETTING_00711DC0,
        MOUSE_SETTING_00711DE0,
        NODE_TRANSLATION,
        EDGE_RIGHT,
        EDGE_LEFT,
        EDGE_TOP,
        EDGE_BOTTOM,
        CURSOR_TILT_SCALE,
        NODE_SET_TRANSLATE_VECTOR,
        FLOAT_MIN,
        FLOAT_MAX,
        ABS,
        MOUSE_LEVEL_DIVISOR,
        CURSOR_HORIZONTAL_SCALE,
        CURSOR_VERTICAL_SCALE,
        ONE_DOUBLE,
    ];

    /// An engine whose callees are all doubles, with the data pages the
    /// code reads mapped (and page 0, so a null object reads as zeros).
    fn world() -> Engine {
        let mut e = Engine::new();
        e.map(0, 0x1000);
        e.map(0x0100_0000, 0x0030_0000);
        for &address in EXTERNAL {
            e.register(address, |_, _| Ret::default());
        }
        // The empty member constructor returns its object.
        echo(&mut e, MEMBER_CONSTRUCTOR_EMPTY);
        e.call_log = Some(vec![]);
        e
    }

    fn returns(e: &mut Engine, address: u32, value: u32) {
        e.register_double(address, move |_, _| Ret {
            eax: value,
            ..Ret::default()
        });
    }

    fn returns_st0(e: &mut Engine, address: u32, value: f64) {
        e.register_double(address, move |_, _| Ret {
            st0: value,
            ..Ret::default()
        });
    }

    /// A constructor-like double: returns its first argument.
    fn echo(e: &mut Engine, address: u32) {
        e.register(address, |_, a| Ret {
            eax: a[0],
            ..Ret::default()
        });
    }

    /// A double that allocates the number of bytes in its first argument.
    fn allocator(e: &mut Engine, address: u32) {
        e.register(address, |e, a| Ret {
            eax: e.mem.alloc(a[0]),
            ..Ret::default()
        });
    }

    /// A double that records its argument words and returns `value`.
    fn recorder(e: &mut Engine, address: u32, value: u32) -> Log {
        let log: Log = Rc::new(RefCell::new(vec![]));
        let seen = log.clone();
        e.register_double(address, move |_, a| {
            seen.borrow_mut().push(a.to_vec());
            Ret {
                eax: value,
                ..Ret::default()
            }
        });
        log
    }

    /// The argument words of every logged call to `address`.
    fn calls(e: &Engine, address: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, words)| words.clone())
            .collect()
    }

    /// The addresses of the logged calls, in order, restricted to `only`.
    fn order(e: &Engine, only: &[u32]) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(a, _)| *a)
            .filter(|a| only.contains(a))
            .collect()
    }

    fn manager(e: &mut Engine) -> Ptr<InterfaceManager> {
        let object = e.new_object::<InterfaceManager>();
        e.set_global(MANAGER_SINGLETON, object.addr());
        object
    }

    /// An object whose virtual table has `targets` at the given byte
    /// offsets; each target is a recorder returning `value`.
    fn virtual_object(e: &mut Engine, targets: &[(u32, u32)], value: u32) -> (u32, Vec<Log>) {
        let table = e.mem.alloc(0x600);
        let object = e.mem.alloc(0x1000);
        e.mem.set_u32(object, table);
        let mut logs = vec![];
        for (offset, target) in targets {
            e.mem.set_u32(table + offset, *target);
            logs.push(recorder(e, *target, value));
        }
        (object, logs)
    }

    /// Sets the `float` result of a getter double that returns through
    /// memory (a pointer to the float).
    fn float_cell(e: &mut Engine, value: f32) -> u32 {
        let cell = e.mem.alloc(4);
        e.mem.set_f32(cell, value);
        cell
    }

    #[test]
    fn initialize_polls_the_controller_and_clears_the_pause_counters() {
        let mut e = world();
        let m = manager(&mut e);
        e.set(m, InterfaceManager::iSoundPauses, 3);
        e.set(m, InterfaceManager::iVoicePauses, 2);
        returns(&mut e, XINPUT_GET_STATE, 0);
        interface_manager_initialize(&mut e, 3, 0);
        assert_eq!(e.global::<u8>(HAS_360_CONTROLLER), 1);
        assert_eq!(e.get(m, InterfaceManager::iSoundPauses), 0);
        assert_eq!(e.get(m, InterfaceManager::iVoicePauses), 0);
        returns(&mut e, XINPUT_GET_STATE, 0x48f);
        interface_manager_initialize(&mut e, 3, 0);
        assert_eq!(e.global::<u8>(HAS_360_CONTROLLER), 0);
        assert_eq!(calls(&e, XINPUT_GET_STATE)[0][0], 0);
    }

    #[test]
    fn initialize_builds_the_manager_when_there_is_none() {
        let mut e = world();
        allocator(&mut e, OPERATOR_NEW);
        allocator(&mut e, NI_ALLOC);
        echo(&mut e, NI_POINT3_CONSTRUCT);
        interface_manager_initialize(&mut e, 3, 0);
        let m = e.global::<u32>(MANAGER_SINGLETON);
        assert_ne!(m, 0);
        assert_eq!(calls(&e, OPERATOR_NEW)[0], vec![0x580]);
        assert_eq!(e.mem.u32(m + 0xc), 1, "constructed: cMenuMode is 1");
    }

    #[test]
    fn destroying_the_manager_deletes_it_and_clears_the_singleton() {
        let mut e = world();
        let m = manager(&mut e);
        fn_0070a0b0(&mut e);
        assert_eq!(e.global::<u32>(MANAGER_SINGLETON), 0);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![m.addr()]]);
        fn_0070a0b0(&mut e);
        assert_eq!(calls(&e, OPERATOR_DELETE).len(), 1, "no manager: nothing");
    }

    #[test]
    fn the_deleting_destructor_frees_only_when_asked() {
        let mut e = world();
        let m = manager(&mut e);
        assert_eq!(
            interface_manager_scalar_deleting_destructor(&mut e, m, 0),
            m
        );
        assert!(calls(&e, OPERATOR_DELETE).is_empty());
        interface_manager_scalar_deleting_destructor(&mut e, m, 1);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![m.addr()]]);
    }

    #[test]
    fn the_constructor_sets_the_defaults_and_creates_the_scene_graphs() {
        let mut e = world();
        allocator(&mut e, OPERATOR_NEW);
        allocator(&mut e, NI_ALLOC);
        echo(&mut e, NI_POINT3_CONSTRUCT);
        echo(&mut e, ALPHA_PROPERTY_CONSTRUCT);
        echo(&mut e, VIEW_CASTER_CONSTRUCT);
        echo(&mut e, SHADER_ACCUMULATOR_CONSTRUCT);
        returns(&mut e, CREATE_SCENE_GRAPH, 0x5000);
        returns(&mut e, SCENE_GRAPH_PARENT_3D, 0x6001);
        returns(&mut e, SCENE_GRAPH_PARENT, 0x6002);
        returns_st0(&mut e, GET_SCREEN_WIDTH, 1280.0);
        returns_st0(&mut e, ANGLE_FUNCTION_A, 2.0);
        returns_st0(&mut e, ANGLE_FUNCTION_B, 3.0);
        e.mem.set_f64(TWO, 2.0);
        e.mem.set_f32(TIMER_DEFAULT_END, 0.001);
        e.mem.set_u32(FADE_CLOCK + 0x14, 77);
        returns(&mut e, FADE_CLOCK_READ, 77);
        let m = Ptr::<InterfaceManager>::new(e.mem.alloc(0x580));
        assert_eq!(interface_manager_construct(&mut e, m), m);
        assert_eq!(e.get(m, InterfaceManager::cMenuMode), 1);
        assert_eq!(e.get(m, InterfaceManager::iPickDistance), 0x50);
        assert_eq!(e.get(m, InterfaceManager::cStatsPageNumber), 0xff);
        assert_eq!(e.get(m, InterfaceManager::iLastXDefault), 100);
        assert_eq!(e.get(m, InterfaceManager::bPreLoadMainMenus), 1);
        assert_eq!(e.get(m, InterfaceManager::bMouseInMotion), 1);
        assert_eq!(e.get(m, InterfaceManager::field_4d0), 1.0);
        // 640 / 2.0, divided by the first helper, times the second.
        assert_eq!(e.get(m, InterfaceManager::fOneToOneDistance), 960.0);
        assert_eq!(e.global::<u32>(LAST_DPS_FADE_TIME), 77);
        // Two scene graphs, the 3D one first.
        let graphs = calls(&e, CREATE_SCENE_GRAPH);
        assert_eq!(graphs[0], vec![m.addr(), 0x6001, MENU_3D_ROOT_NAME, 1]);
        assert_eq!(graphs[1], vec![m.addr(), 0x6002, MENU_ROOT_NAME, 0]);
        assert_eq!(calls(&e, NI_POINTER_ASSIGN).len(), 6);
        // The timer list head links to itself.
        let timers = e.get(m, InterfaceManager::pTimers);
        assert_ne!(timers, 0);
        assert_eq!(e.mem.u32(timers + 0xc), timers);
        assert_ne!(e.get(m, InterfaceManager::pViewCaster), 0);
        // The accumulators are numbered 100 and 99.
        let accumulators = calls(&e, SHADER_ACCUMULATOR_CONSTRUCT);
        assert_eq!(accumulators[0][1..], [100, 1, 0x2f7]);
        assert_eq!(accumulators[1][1..], [99, 1, 0x2f7]);
        assert_eq!(
            calls(&e, SHADER_ACCUMULATOR_SET_19C)[0][1],
            10,
            "the interface accumulator gets 10"
        );
    }

    #[test]
    fn a_timer_starts_idle_with_the_default_end() {
        let mut e = world();
        e.mem.set_f32(TIMER_DEFAULT_END, 0.5);
        let t = e.new_object::<Timer>();
        e.set(t, Timer::pPrev, 9);
        assert_eq!(timer_construct(&mut e, t), t);
        assert_eq!(e.get(t, Timer::pPrev), 0);
        assert_eq!(e.get(t, Timer::pNext), 0);
        assert_eq!(e.get(t, Timer::fElapsed), 0.0);
        assert_eq!(e.get(t, Timer::fEnd), 0.5);
        assert_eq!(e.get(t, Timer::pIndex), 0);
    }

    #[test]
    fn the_key_table_constructor_calls_the_initializer() {
        let mut e = world();
        let p = Ptr::new(0x2000_0100);
        assert_eq!(fn_0070a8e0(&mut e, p), p);
        assert_eq!(calls(&e, KEY_TABLE_CONSTRUCT), vec![vec![0x2000_0100]]);
    }

    #[test]
    fn the_accumulator_byte_is_stored() {
        let mut e = world();
        let p = Ptr::<()>::new(e.mem.alloc(0x40));
        fn_0070a900(&mut e, p, 1);
        assert_eq!(e.mem.u8(p.addr() + 0x32), 1);
        fn_0070a900(&mut e, p, 0);
        assert_eq!(e.mem.u8(p.addr() + 0x32), 0);
    }

    #[test]
    fn the_destructor_deletes_what_the_manager_owns_in_order() {
        let mut e = world();
        let m = manager(&mut e);
        // Offsets of the owned objects and the order the code deletes them.
        let fields = [0xb4u32, 0xa0, 0x9c, 0x28, 0x174, 0x94, 0x98];
        let mut owned = vec![];
        for (n, field) in fields.iter().enumerate() {
            let (object, mut logs) = virtual_object(&mut e, &[(0, 0x0900_0000 + n as u32)], 0);
            e.mem.set_u32(m.addr() + field, object);
            owned.push((object, logs.remove(0)));
        }
        let view_caster = e.mem.alloc(0xc);
        e.set(m, InterfaceManager::pViewCaster, view_caster);
        let held = m.addr() + 4;
        e.register_double(NI_POINTER_GET, move |_, a| Ret {
            eax: if a[0] == held { 0x4444 } else { 0 },
            ..Ret::default()
        });
        interface_manager_destruct(&mut e, m);
        for (object, log) in &owned {
            assert_eq!(log.borrow().as_slice(), &[vec![*object, 1]]);
        }
        assert_eq!(e.global::<u32>(MANAGER_SINGLETON), 0);
        assert_eq!(e.get(m, InterfaceManager::pTargetReticle), 0);
        assert_eq!(calls(&e, VIEW_CASTER_DESTRUCT), vec![vec![view_caster]]);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![view_caster]]);
        // The scene graph that is set is detached and released.
        assert_eq!(calls(&e, EMPTY_MEMBER_FUNCTION)[0], vec![0x4444]);
        assert!(calls(&e, NI_POINTER_ASSIGN).contains(&vec![held, 0]));
        assert_eq!(e.global::<u8>(TILE_DELETING), 0);
        let sequence = order(
            &e,
            &[
                TILE_FREE_TRAIT_LIST,
                MENU_FREE_XML_ARCHIVE,
                CLEAR_TEMP_MODEL,
                START_MENU_SHUTDOWN,
                STATIC_OBJECT_SHUTDOWN_B,
            ],
        );
        assert_eq!(
            sequence,
            vec![
                TILE_FREE_TRAIT_LIST,
                MENU_FREE_XML_ARCHIVE,
                CLEAR_TEMP_MODEL,
                START_MENU_SHUTDOWN,
                STATIC_OBJECT_SHUTDOWN_B
            ]
        );
        // The embedded members are destroyed last, in reverse order.
        let tail: Vec<Vec<u32>> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .rev()
            .take(2)
            .map(|(_, w)| w.clone())
            .collect();
        assert_eq!(tail, vec![vec![m.addr() + 4], vec![m.addr() + 8]]);
    }

    #[test]
    fn the_script_pointer_list_is_emptied() {
        let mut e = world();
        fn_0070aca0(&mut e);
        assert_eq!(
            calls(&e, POINTER_LIST_REMOVE_ALL),
            vec![vec![SCRIPT_POINTER_LIST]]
        );
    }

    #[test]
    fn the_first_global_object_is_deleted_and_cleared() {
        let mut e = world();
        e.set_global(GLOBAL_OBJECT_8CE8, 0x2000_0100u32);
        fn_0070acb0(&mut e);
        assert_eq!(calls(&e, OBJECT_8CE8_DESTRUCT), vec![vec![0x2000_0100]]);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![0x2000_0100]]);
        assert_eq!(e.global::<u32>(GLOBAL_OBJECT_8CE8), 0);
        fn_0070acb0(&mut e);
        assert_eq!(calls(&e, OBJECT_8CE8_DESTRUCT).len(), 1);
    }

    #[test]
    fn the_first_deleting_destructor_frees_on_request() {
        let mut e = world();
        let p = Ptr::new(0x2000_0200);
        assert_eq!(fn_0070ad00(&mut e, p, 0), p);
        assert!(calls(&e, OPERATOR_DELETE).is_empty());
        fn_0070ad00(&mut e, p, 1);
        assert_eq!(calls(&e, OBJECT_8CE8_DESTRUCT).len(), 2);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![0x2000_0200]]);
    }

    #[test]
    fn the_second_global_object_gets_its_virtual_destructor() {
        let mut e = world();
        let (object, mut logs) = virtual_object(&mut e, &[(0, 0x0900_0001)], 0);
        e.set_global(GLOBAL_OBJECT_A0C4, object);
        fn_0070ad30(&mut e);
        assert_eq!(logs.remove(0).borrow().as_slice(), &[vec![object, 1]]);
        assert_eq!(e.global::<u32>(GLOBAL_OBJECT_A0C4), 0);
        fn_0070ad30(&mut e);
        assert_eq!(calls(&e, 0x0900_0001).len(), 1);
    }

    #[test]
    fn the_view_caster_deleting_destructor_frees_on_request() {
        let mut e = world();
        let p = Ptr::new(0x2000_0300);
        assert_eq!(fn_0070ad80(&mut e, p, 0), p);
        assert!(calls(&e, OPERATOR_DELETE).is_empty());
        fn_0070ad80(&mut e, p, 3);
        assert_eq!(calls(&e, VIEW_CASTER_DESTRUCT).len(), 2);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![0x2000_0300]]);
    }

    /// The stubs both `Init` tests need: allocation and constructors that
    /// return their object.
    fn init_world() -> (Engine, Ptr<InterfaceManager>) {
        let mut e = world();
        allocator(&mut e, OPERATOR_NEW);
        allocator(&mut e, NI_ALLOC);
        for constructor in [
            MENU_ROOT_TILE_CONSTRUCT,
            NI_NODE_CONSTRUCT,
            FOG_PROPERTY_CONSTRUCT,
            PIPBOY_MANAGER_CONSTRUCT,
            SHADOW_SCENE_NODE_CONSTRUCT,
            FIXED_STRING_CONSTRUCT,
        ] {
            echo(&mut e, constructor);
        }
        let m = manager(&mut e);
        (e, m)
    }

    #[test]
    fn init_builds_the_root_tile_and_the_debug_text_root() {
        let (mut e, m) = init_world();
        let (interface_root, mut root_logs) = virtual_object(&mut e, &[(0xdc, 0x0900_0010)], 0);
        e.set(m, InterfaceManager::pInterfaceRoot, interface_root);
        // The root tile: slot 4 is `Tile::Init`.
        e.register(MENU_ROOT_TILE_CONSTRUCT, |e, a| {
            let table = e.mem.alloc(0x40);
            e.mem.set_u32(a[0], table);
            e.mem.set_u32(table + 4, 0x0900_0011);
            Ret {
                eax: a[0],
                ..Ret::default()
            }
        });
        let init_log = recorder(&mut e, 0x0900_0011, 0);
        returns_st0(&mut e, GET_SCREEN_WIDTH, 1920.0);
        returns_st0(&mut e, GET_SCREEN_HEIGHT, 1080.0);
        returns_st0(&mut e, GET_INTERFACE_SCALE, 0.5);
        returns(&mut e, TILE_READ_FILE, 0x7777);
        returns(&mut e, TILE_TEXT_TO_TRAIT, 0x1234);
        returns(&mut e, HAS_360_CONTROLLER_GETTER, 1);
        returns(&mut e, FLOAT_HOLDER_GET, 0);
        let cell = float_cell(&mut e, 0.5);
        returns(&mut e, FLOAT_HOLDER_GET, cell);
        e.mem.set_f64(TWO_FIFTY_FIVE, 255.0);
        returns_st0(&mut e, GET_DESKTOP_WIDTH, 1500.0);
        returns_st0(&mut e, GET_DESKTOP_HEIGHT, 1000.0);
        e.mem.set_f64(WIDESCREEN_RATIO, 1.5);
        e.mem.set_f64(WIDESCREEN_INVERSE_RATIO, 0.6);
        e.mem.set_f64(TWO, 2.0);
        interface_manager_init(&mut e, m, 1);
        let root = e.get(m, InterfaceManager::pMenusRoot);
        assert_ne!(root, 0);
        assert_eq!(
            init_log.borrow().as_slice(),
            &[vec![root, 0, MENU_ROOT_NAME, 0]]
        );
        // The screen width goes to trait 0xfb1 as a float.
        let float_sets = calls(&e, TILE_SET_FLOAT);
        assert!(float_sets.contains(&vec![root, 0xfb1, 1920.0f32.to_bits(), 1]));
        assert!(float_sets.contains(&vec![root, 0xff8, 0.5f32.to_bits(), 1]));
        assert_eq!(calls(&e, TILE_LOAD_TEXT_TABLE).len(), 1);
        // `globals.xml` becomes the string root; its alpha trait is 0.5 * 255.
        assert_eq!(e.get(m, InterfaceManager::pStringRoot), 0x7777);
        assert!(float_sets.contains(&vec![0x7777, 0x1234, 127.5f32.to_bits(), 1]));
        assert_eq!(
            calls(&e, TILE_READ_FILE),
            vec![vec![root, GLOBALS_XML_PATH]]
        );
        // The has-controller trait is set from the getter.
        assert!(calls(&e, TILE_SET_INT).contains(&vec![0x7777, 0x1234, 1]));
        // The debug text root is a node attached to the interface root.
        let debug_root = e.get(m, InterfaceManager::pDebugTextRoot);
        assert_ne!(debug_root, 0);
        assert_eq!(
            root_logs.remove(0).borrow().as_slice(),
            &[vec![interface_root, debug_root, 1]]
        );
        // 1500 / 1000 equals the stored ratio: both string lists are limited.
        let limits = calls(&e, STRING_LIST_SET_LIMIT);
        assert_eq!(
            limits,
            vec![vec![STRING_LIST_A, 0x50], vec![STRING_LIST_B, 0x50]]
        );
        // The pipboy manager and the first-init flag.
        assert_ne!(e.get(m, InterfaceManager::pPipboy), 0);
        assert_eq!(e.get(m, InterfaceManager::bFirstInit), 1);
        assert_eq!(calls(&e, LOADING_MENU_CREATE), vec![vec![0, 0]]);
        // The scope guard is opened with the source line and closed.
        assert_eq!(
            calls(&e, SCOPE_GUARD_BEGIN)[0][1..],
            [0xd, 1, SOURCE_FILE, 0x21a]
        );
        assert_eq!(calls(&e, SCOPE_GUARD_END).len(), 1);
    }

    #[test]
    fn init_without_the_text_table_skips_it_and_the_loading_menu() {
        let (mut e, m) = init_world();
        e.register(MENU_ROOT_TILE_CONSTRUCT, |e, a| {
            let table = e.mem.alloc(0x40);
            e.mem.set_u32(a[0], table);
            e.mem.set_u32(table + 4, 0x0900_0012);
            Ret {
                eax: a[0],
                ..Ret::default()
            }
        });
        recorder(&mut e, 0x0900_0012, 0);
        let (interface_root, _) = virtual_object(&mut e, &[(0xdc, 0x0900_0013)], 0);
        e.set(m, InterfaceManager::pInterfaceRoot, interface_root);
        let cell = float_cell(&mut e, 0.0);
        returns(&mut e, FLOAT_HOLDER_GET, cell);
        e.mem.set_f64(WIDESCREEN_RATIO, 1.5);
        e.mem.set_f64(WIDESCREEN_INVERSE_RATIO, 0.6);
        returns_st0(&mut e, GET_DESKTOP_WIDTH, 1280.0);
        returns_st0(&mut e, GET_DESKTOP_HEIGHT, 1024.0);
        interface_manager_init(&mut e, m, 0);
        assert!(calls(&e, TILE_LOAD_TEXT_TABLE).is_empty());
        assert!(calls(&e, LOADING_MENU_CREATE).is_empty());
        assert!(
            calls(&e, STRING_LIST_SET_LIMIT).is_empty(),
            "1.25 is not 1.5"
        );
        assert_eq!(e.get(m, InterfaceManager::bFirstInit), 1);
    }

    #[test]
    fn init_second_creates_the_cursor_the_shadow_nodes_and_the_hud() {
        let (mut e, m) = init_world();
        e.put_vtable(TILE_IMAGE_VTABLE, &[0, 0x0900_0020]);
        let init_log = recorder(&mut e, 0x0900_0020, 0);
        echo(&mut e, TILE_CONSTRUCT);
        let (cursor_root, mut root_logs) = virtual_object(&mut e, &[(0xdc, 0x0900_0021)], 0);
        e.set(m, InterfaceManager::pCursorRoot, cursor_root);
        returns(&mut e, TILE_IMAGE_NODE, 0x5555);
        returns(&mut e, HUD_MAIN_MENU_CREATE, 0x8888);
        returns(&mut e, STRING_HOLDER_GET, 0x6666);
        returns(&mut e, NI_POINTER_GET, 0x4040);
        e.mem.set_f32(CURSOR_DIRECTION_Y, -1.0);
        interface_manager_init_second(&mut e, m);
        let cursor = e.get(m, InterfaceManager::pCursor);
        assert_ne!(cursor, 0);
        assert_eq!(e.mem.u32(cursor), TILE_IMAGE_VTABLE);
        assert_eq!(
            init_log.borrow().as_slice(),
            &[vec![cursor, 0, CURSOR_NAME, 0]]
        );
        // The cursor is 32 by 32 and its node joins the cursor root.
        assert!(calls(&e, TILE_SET_INT).contains(&vec![cursor, 0xfb1, 0x20]));
        assert!(calls(&e, TILE_SET_INT).contains(&vec![cursor, 0xfad, 2000]));
        assert_eq!(
            root_logs.remove(0).borrow().as_slice(),
            &[vec![cursor_root, 0x5555, 1]]
        );
        // The node looks along -y.
        assert_eq!(
            calls(&e, NODE_SET_TRANSLATE),
            vec![vec![
                0x5555,
                0.0f32.to_bits(),
                (-1.0f32).to_bits(),
                0.0f32.to_bits()
            ]]
        );
        // Two shadow scene nodes, in slots 1 and 3.
        let shadow = e.get(m, InterfaceManager::pShadowNode);
        let ui_player = e.get(m, InterfaceManager::pUIPlayerNode);
        assert_ne!(shadow, 0);
        assert_ne!(ui_player, 0);
        assert_eq!(
            calls(&e, SET_SHADOW_SCENE_NODE),
            vec![vec![shadow, 1], vec![ui_player, 3]]
        );
        assert_eq!(
            calls(&e, SHADER_ACCUMULATOR_SET_194),
            vec![vec![0x4040, shadow], vec![0x4040, shadow]]
        );
        // The HUD menu is shown and the manager is initialized.
        assert!(calls(&e, TILE_SET_INT).contains(&vec![0x8888, 0xfa3, 0]));
        assert_eq!(calls(&e, HUD_MAIN_MENU_SET_MENU_MODE), vec![vec![4]]);
        assert_eq!(e.get(m, InterfaceManager::bSecondInit), 1);
        assert_eq!(
            calls(&e, SCOPE_GUARD_BEGIN)[0][1..],
            [0xd, 1, SOURCE_FILE, 0x29b]
        );
    }

    #[test]
    fn a_tile_image_starts_with_its_vtable_a_full_scale_and_no_texture() {
        let mut e = world();
        echo(&mut e, TILE_CONSTRUCT);
        let t = e.new_object::<TileImage>();
        e.set(t, TileImage::bIsScissor, 1);
        assert_eq!(tile_image_construct(&mut e, t), t);
        assert_eq!(e.mem.u32(t.addr()), TILE_IMAGE_VTABLE);
        assert_eq!(e.get(t, TileImage::fScale), 1.0);
        assert_eq!(e.get(t, TileImage::bIsScissor), 0);
        assert_eq!(
            calls(&e, NI_POINTER_CONSTRUCT),
            vec![vec![t.addr() + 0x3c, 0], vec![t.addr() + 0x40, 0]]
        );
        assert_eq!(calls(&e, NI_POINTER_ASSIGN), vec![vec![t.addr() + 0x3c, 0]]);
    }

    #[test]
    fn a_tile_image_knows_its_type() {
        let mut e = world();
        let t = e.new_object::<TileImage>();
        assert_eq!(tile_image_get_type(&mut e, t), 0x386);
        assert_eq!(tile_image_get_type_name(&mut e, t), 0x0106_f044);
    }

    #[test]
    fn the_tile_image_deleting_destructor_frees_on_request() {
        let mut e = world();
        let t = e.new_object::<TileImage>();
        assert_eq!(tile_image_scalar_deleting_destructor(&mut e, t, 0), t);
        assert!(calls(&e, OPERATOR_DELETE).is_empty());
        tile_image_scalar_deleting_destructor(&mut e, t, 1);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![t.addr()]]);
        assert_eq!(calls(&e, TILE_DESTRUCT).len(), 2);
    }

    #[test]
    fn a_tile_image_is_released_unless_it_already_was() {
        let mut e = world();
        let t = e.new_object::<TileImage>();
        returns(&mut e, TILE_FLAG_TEST, 0);
        tile_image_destruct(&mut e, t);
        assert_eq!(calls(&e, TILE_RELEASE), vec![vec![t.addr()]]);
        assert_eq!(e.mem.u32(t.addr()), TILE_IMAGE_VTABLE);
        assert_eq!(
            order(
                &e,
                &[
                    NI_POINTER_ASSIGN,
                    TILE_RELEASE,
                    NI_POINTER_DESTROY,
                    TILE_DESTRUCT
                ]
            ),
            vec![
                NI_POINTER_ASSIGN,
                TILE_RELEASE,
                NI_POINTER_DESTROY,
                NI_POINTER_DESTROY,
                TILE_DESTRUCT
            ]
        );
        returns(&mut e, TILE_FLAG_TEST, 1);
        tile_image_destruct(&mut e, t);
        assert_eq!(calls(&e, TILE_RELEASE).len(), 1);
    }

    /// The audio stubs the pause code needs.
    fn audio_world() -> Engine {
        let mut e = world();
        let manager_block = e.mem.alloc(0x40);
        returns(&mut e, GET_MANAGER, manager_block);
        returns(&mut e, AUDIO_INSTANCE, 0x3030);
        e
    }

    #[test]
    fn leaving_the_fade_out_enters_mode_five_and_pauses_the_audio() {
        let mut e = audio_world();
        let m = manager(&mut e);
        let sounds = e.call(GET_MANAGER, &args![]).u32();
        e.call_log = Some(vec![]);
        e.set(m, InterfaceManager::cMenuMode, 3);
        e.set(m, InterfaceManager::pCursor, 0x7000);
        returns(&mut e, TILE_IMAGE_NODE, 0x5555);
        returns(&mut e, GET_FIELD_AT_4, 0);
        returns(&mut e, MENU_CONSOLE_INSTANCE, 0x1111);
        returns(&mut e, HAS_360_CONTROLLER_GETTER, 1);
        returns(&mut e, CONTROLS_GET, 0x2222);
        returns(&mut e, STRING_HOLDER_GET, 0x6666);
        e.set(m, InterfaceManager::bShowMouse, 1);
        interface_manager_pre_idle_stuff(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::cMenuMode), 5);
        assert_eq!(calls(&e, CLEAR_KEYSTROKES).len(), 1);
        // The mouse shows, so the cursor is hidden through its node.
        assert_eq!(calls(&e, NODE_SET_FLAG), vec![vec![0x5555, 0]]);
        assert!(calls(&e, TILE_SET_INT).contains(&vec![0x7000, 0xfa3, 1]));
        assert_eq!(calls(&e, HUD_SET_INFO_FOR_REF), vec![vec![0, 0, 0]]);
        assert_eq!(e.get(m, InterfaceManager::pMouseOverTarget), 0);
        // Console entered, both pause counters up by one.
        assert_eq!(
            calls(&e, MENU_CONSOLE_ON_ENTER_MENU_MODE),
            vec![vec![0x1111]]
        );
        assert_eq!(e.mem.u8(sounds + 0x20), 1);
        assert_eq!(e.mem.u8(sounds + 0x21), 1);
        assert_eq!(
            calls(&e, AUDIO_PAUSE_TYPE),
            vec![vec![0x3030, 0x4000_0000, 1]]
        );
        assert_eq!(calls(&e, AUDIO_PAUSE_TYPE_6), vec![vec![0x3030]]);
    }

    #[test]
    fn mode_five_settles_to_mode_two_and_a_locked_fade_is_left_alone() {
        let mut e = audio_world();
        let m = manager(&mut e);
        e.set(m, InterfaceManager::cMenuMode, 5);
        interface_manager_pre_idle_stuff(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::cMenuMode), 2);
        e.set(m, InterfaceManager::cMenuMode, 3);
        e.set(m, InterfaceManager::bLockMenuModeForFade, 1);
        interface_manager_pre_idle_stuff(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::cMenuMode), 3);
        assert!(calls(&e, CLEAR_KEYSTROKES).is_empty());
    }

    #[test]
    fn in_the_game_mode_the_audio_pauses_follow_the_menu_mode_check() {
        let mut e = audio_world();
        let m = manager(&mut e);
        let sounds = e.call(GET_MANAGER, &args![]).u32();
        e.set(m, InterfaceManager::cMenuMode, 1);
        // `007023a0` says the manager is not in mode 1: not paused.
        returns(&mut e, MENU_MODE_IS_NOT_ONE, 1);
        interface_manager_pre_idle_stuff(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::bExternalForcedMenuMode), 1);
        assert_eq!(e.mem.u8(sounds + 0x20), 1);
        assert_eq!(e.mem.u8(sounds + 0x21), 1);
        // Asked again with the flag already set: nothing more.
        interface_manager_pre_idle_stuff(&mut e, m);
        assert_eq!(e.mem.u8(sounds + 0x20), 1);
        // Now it reports mode 1: the flag clears and the audio resumes.
        returns(&mut e, MENU_MODE_IS_NOT_ONE, 0);
        interface_manager_pre_idle_stuff(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::bExternalForcedMenuMode), 0);
        assert_eq!(e.mem.u8(sounds + 0x20), 0);
        assert_eq!(e.mem.u8(sounds + 0x21), 0);
        assert_eq!(calls(&e, AUDIO_UNPAUSE_TYPE_6).len(), 1);
        assert_eq!(calls(&e, AUDIO_UNPAUSE_TYPE_4000_0000).len(), 1);
    }

    #[test]
    fn the_audio_pause_wrapper_pauses_type_0x40000000() {
        let mut e = world();
        fn_0070bba0(&mut e, Ptr::new(0x3030));
        assert_eq!(
            calls(&e, AUDIO_PAUSE_TYPE),
            vec![vec![0x3030, 0x4000_0000, 1]]
        );
    }

    /// A world with the two helpers that do real arithmetic: the float to
    /// integer conversion and `abs`, and the min/max helpers.
    fn arithmetic_world() -> Engine {
        let mut e = world();
        e.register(FTOL, |_, a| Ret {
            eax: f64::from_bits(a[0] as u64 | (a[1] as u64) << 32) as i32 as u32,
            ..Ret::default()
        });
        e.register(ABS, |_, a| Ret {
            eax: (a[0] as i32).wrapping_abs() as u32,
            ..Ret::default()
        });
        e.register(FLOAT_MIN, |_, a| {
            let (first, second) = (f32::from_bits(a[0]), f32::from_bits(a[1]));
            Ret {
                st0: if second <= first { second } else { first } as f64,
                ..Ret::default()
            }
        });
        e.register(FLOAT_MAX, |_, a| {
            let (first, second) = (f32::from_bits(a[0]), f32::from_bits(a[1]));
            Ret {
                st0: if second < first { first } else { second } as f64,
                ..Ret::default()
            }
        });
        e
    }

    /// A double for a controls query: `(this, key, state)` answers `answer`
    /// when `key` matches, else 0.
    fn key_query(e: &mut Engine, address: u32, key: u32, answer: u32) {
        e.register_double(address, move |_, a| Ret {
            eax: if a[1] == key { answer } else { 0 },
            ..Ret::default()
        });
    }

    #[test]
    fn a_binding_slot_is_cleared() {
        let mut e = world();
        let controls = Ptr::<()>::new(e.mem.alloc(0x2000));
        fn_0070ec20(&mut e, controls, 0x1c, 2);
        assert_eq!(e.mem.u8(controls.addr() + 0x1b94 + 2 * 0x1c + 0x1c), 0xff);
        assert_eq!(e.mem.u8(controls.addr() + 0x1b94), 0);
    }

    #[test]
    fn small_getters_read_their_fields() {
        let mut e = world();
        let object = Ptr::<()>::new(e.mem.alloc(0x1000));
        e.mem.set_u8(object.addr() + 0x162, 9);
        assert_eq!(fn_0070ec50(&mut e, object), 9);
        e.mem.set_u8(object.addr() + 0xd54, 4);
        assert_eq!(fn_0070edc0(&mut e, object), 4);
        e.mem.set_f32(object.addr() + 0x670, 1.25);
        assert_eq!(fn_00710ab0(&mut e, object), 1.25);
        let m = manager(&mut e);
        assert!(!fn_0070ecd0(&mut e, m));
        e.set(m, InterfaceManager::iModifierKeys, 0b110);
        assert!(!fn_0070ecd0(&mut e, m));
        e.set(m, InterfaceManager::iModifierKeys, 0b101);
        assert!(fn_0070ecd0(&mut e, m));
    }

    #[test]
    fn the_keyring_is_open_when_the_inventory_menu_tile_says_so() {
        let mut e = world();
        assert!(!inventory_menu_is_keyring_open(&mut e));
        e.set_global(INVENTORY_MENU, 0x2000_0400u32);
        e.set_global(KEYRING_TRAIT, 0xfd5u32);
        returns(&mut e, GET_FIELD_AT_4, 0x7100);
        returns(&mut e, TILE_IS_TRUE, 1);
        assert!(inventory_menu_is_keyring_open(&mut e));
        assert_eq!(calls(&e, GET_FIELD_AT_4), vec![vec![0x2000_0400]]);
        assert_eq!(calls(&e, TILE_IS_TRUE), vec![vec![0x7100, 0xfd5]]);
        returns(&mut e, TILE_IS_TRUE, 0);
        assert!(!inventory_menu_is_keyring_open(&mut e));
    }

    #[test]
    fn two_flag_bits_must_both_be_set() {
        let mut e = world();
        e.set_global(FLAGS_OWNER, 0x2000_0500u32);
        returns(&mut e, HAS_FLAG, 1);
        assert!(fn_0070ee30(&mut e));
        assert_eq!(
            calls(&e, HAS_FLAG),
            vec![vec![0x2000_0500, 0x8000], vec![0x2000_0500, 8]]
        );
        e.register(HAS_FLAG, |_, a| Ret {
            eax: (a[1] == 0x8000) as u32,
            ..Ret::default()
        });
        assert!(!fn_0070ee30(&mut e));
        e.register(HAS_FLAG, |_, _| Ret::default());
        assert!(!fn_0070ee30(&mut e));
    }

    fn top_menu(e: &mut Engine, id: u32) {
        e.register_double(IS_TOP_MENU_ID, move |_, a| Ret {
            eax: (a[0] == id) as u32,
            ..Ret::default()
        });
    }

    #[test]
    fn raising_the_pipboy_closes_the_container_menu_first() {
        let mut e = world();
        let m = manager(&mut e);
        e.set(m, InterfaceManager::field_4bc, 1);
        top_menu(&mut e, 0x3f0);
        update_pipboy(&mut e, m);
        assert_eq!(calls(&e, CONTAINER_MENU_CLOSE).len(), 1);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 1);
    }

    fn update_pipboy(e: &mut Engine, m: Ptr<InterfaceManager>) {
        interface_manager_update_pipboy(e, m)
    }

    #[test]
    fn raising_the_pipboy_closes_the_rendered_terminal_menus() {
        let mut e = world();
        let m = manager(&mut e);
        e.set(m, InterfaceManager::field_4bc, 1);
        e.set(m, InterfaceManager::pCurrentRenderedMenu, 0x2000_0600);
        top_menu(&mut e, 0x41f);
        update_pipboy(&mut e, m);
        assert_eq!(calls(&e, RENDERED_MENU_CLOSE), vec![vec![0x2000_0600]]);
        assert!(calls(&e, CONTAINER_MENU_CLOSE).is_empty());
    }

    #[test]
    fn the_pipboy_rises_unless_the_player_is_busy() {
        let mut e = audio_world();
        let m = manager(&mut e);
        e.set(m, InterfaceManager::field_4bc, 1);
        let player = e.mem.alloc(0x1000);
        e.set_global(PLAYER_SINGLETON, player);
        returns(&mut e, ACTOR_GET_ANIM_ACTION, 3);
        update_pipboy(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 1, "busy: nothing");
        assert!(calls(&e, PLAYER_GET_ANIMATION).is_empty());

        returns(&mut e, ACTOR_GET_ANIM_ACTION, 7);
        returns(&mut e, PLAYER_GET_ANIMATION, 0x2000_0700);
        returns(&mut e, GET_PIPBOY_STATIC, 0x8080);
        update_pipboy(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 2);
        let play = calls(&e, ANIMATION_PLAY_GROUP);
        assert_eq!(
            play,
            vec![vec![0x2000_0700, 0xe2, 1, 0xffff_ffff, 0xffff_ffff]]
        );
        assert_eq!(calls(&e, PIPBOY_FADE_LIGHT_EFFECT), vec![vec![0x8080, 1]]);
        assert_eq!(calls(&e, ADD_TO_ENTER_STACK), vec![vec![m.addr(), 1]]);
        assert_eq!(calls(&e, INTERFACE_SHOW_MENUS).len(), 1);
        assert_eq!(calls(&e, IMAGE_SPACE_STOP).len(), 1);
        assert_eq!(calls(&e, ANIMATION_CLEAR_GROUP).len(), 2);
        // The `UIPipBoyAccessUp` sound is looked up and played.
        assert_eq!(
            calls(&e, AUDIO_GET_SOUND_HANDLE_BY_NAME)[0][2..],
            [PIPBOY_ACCESS_UP_SOUND, 0x121]
        );
        assert_eq!(calls(&e, SOUND_HANDLE_PLAY).len(), 1);
    }

    #[test]
    fn the_pipboy_gives_up_waiting_when_the_animation_is_gone() {
        let mut e = world();
        let m = manager(&mut e);
        e.set(m, InterfaceManager::field_4bc, 2);
        e.set_global(PLAYER_SINGLETON, 0x2000_0800u32);
        returns(&mut e, GET_PIPBOY_STATIC, 0x8080);
        returns(&mut e, MAP_MENU_NEEDS_TIDY, 1);
        update_pipboy(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 4);
        assert_eq!(calls(&e, PIPBOY_FADE_LIGHT_EFFECT), vec![vec![0x8080, 0]]);
        assert_eq!(calls(&e, MAP_MENU_TIDY), vec![vec![0]]);
    }

    #[test]
    fn the_pipboy_pose_enters_the_rendered_menu() {
        let mut e = audio_world();
        let m = manager(&mut e);
        e.set(m, InterfaceManager::field_4bc, 2);
        let player = e.mem.alloc(0x1000);
        e.set_global(PLAYER_SINGLETON, player);
        // The animation and the word `0070f490(anim, 2)` reads: 1 means
        // the pose was reached.
        let animation = e.mem.alloc(0x100);
        e.mem.set_u32(animation + 0x64, 1);
        returns(&mut e, PLAYER_GET_ANIMATION, animation);
        returns(&mut e, ANIMATION_GET_SEQUENCE, 0x2000_0900);
        returns_st0(&mut e, SEQUENCE_END_TIME, 2.0);
        returns_st0(&mut e, ANIM_GROUP_GET_TIME, 5.0);
        // A pending callback runs and is cleared.
        let callback_log = recorder(&mut e, 0x0900_0030, 0);
        e.set(m, InterfaceManager::field_4c0, 0x0900_0030);
        // The saved acquire object answers "can reload" for the idle.
        let (process, _) = virtual_object(&mut e, &[(0x4d8, 0x0900_0031)], 1);
        returns(&mut e, GET_SAVED_ACQUIRE_OBJECT, process);
        returns(&mut e, GET_PIPBOY, 0x6060);
        interface_manager_update_pipboy(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 3);
        assert_eq!(calls(&e, HUD_SET_MENU_MODE), vec![vec![3]]);
        // The update time is the pose time minus the sequence end time.
        let update = calls(&e, ANIMATION_UPDATE);
        assert_eq!(
            update,
            vec![vec![animation, player, 0.0f32.to_bits(), 3.0f32.to_bits()]]
        );
        assert_eq!(
            calls(&e, ANIMATION_UPDATE_BIP_ONLY),
            vec![vec![animation, 3.0f32.to_bits(), 0, 1]]
        );
        assert_eq!(callback_log.borrow().len(), 1);
        assert_eq!(e.get(m, InterfaceManager::field_4c0), 0);
        assert_eq!(e.global::<u8>(IDLE_MANAGER_BUSY), 0);
        assert_eq!(calls(&e, ANIMATION_CLEAR_GROUP).len(), 1);
        assert_eq!(calls(&e, ENTER_RENDERED_MENU).len(), 1);
        assert_eq!(calls(&e, PIPBOY_MANAGER_UPDATE), vec![vec![0x6060]]);
    }

    #[test]
    fn the_pipboy_open_state_keeps_the_map_tidy() {
        let mut e = world();
        let m = manager(&mut e);
        e.set(m, InterfaceManager::field_4bc, 3);
        e.set_global(PLAYER_SINGLETON, 0x2000_0800u32);
        interface_manager_update_pipboy(&mut e, m);
        assert!(calls(&e, MAP_MENU_TIDY).is_empty());
        returns(&mut e, MAP_MENU_NEEDS_TIDY, 1);
        interface_manager_update_pipboy(&mut e, m);
        assert_eq!(calls(&e, MAP_MENU_TIDY), vec![vec![0]]);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 3);
    }

    #[test]
    fn lowering_the_pipboy_waits_for_the_enter_stack() {
        let mut e = audio_world();
        let m = manager(&mut e);
        let sounds = e.call(GET_MANAGER, &args![]).u32();
        e.mem.set_u8(sounds + 0x20, 2);
        e.set(m, InterfaceManager::field_4bc, 4);
        let (player, _) = virtual_object(&mut e, &[(0x214, 0x0900_0040), (0x4b0, 0x0900_0041)], 4);
        e.set_global(PLAYER_SINGLETON, player);
        returns(&mut e, POP_FROM_ENTER_STACK, 0xffff_ffff);
        update_pipboy(&mut e, m);
        assert_eq!(
            e.get(m, InterfaceManager::field_4bc),
            4,
            "pop failed: still lowering"
        );
        returns(&mut e, POP_FROM_ENTER_STACK, 0);
        returns(&mut e, GET_PIPBOY_STATIC, 0x8080);
        returns(&mut e, PLAYER_GET_ANIMATION, 0x2000_0a00);
        returns(&mut e, ANIMATION_GROUP_ID, 0x1234);
        let reload = recorder(&mut e, RELOAD_DYNAMIC_IDLE_ON_1ST_PERSON, 0);
        returns(&mut e, GET_SAVED_ACQUIRE_OBJECT, 0x2000_0b00);
        update_pipboy(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 5);
        // The player's virtual slot 0x214 answers 4: the idle is reloaded.
        assert_eq!(reload.borrow().as_slice(), &[vec![0x2000_0b00, player]]);
        // Slot 0x4b0 gets the animation group and 1.
        assert_eq!(calls(&e, 0x0900_0041), vec![vec![player, 0x1234, 1]]);
        assert_eq!(calls(&e, MAP_MENU_CLEAR_MAP_MEMORY).len(), 1);
        assert_eq!(e.mem.u8(sounds + 0x20), 0, "the sound pauses are released");
        assert_eq!(calls(&e, AUDIO_UNPAUSE_TYPE_4000_0000).len(), 2);
    }

    #[test]
    fn the_pipboy_returns_to_idle_when_the_player_leaves_it() {
        let mut e = world();
        let m = manager(&mut e);
        e.set(m, InterfaceManager::field_4bc, 5);
        let (player, _) = virtual_object(&mut e, &[(0x4b0, 0x0900_0050)], 0);
        e.set_global(PLAYER_SINGLETON, player);
        e.set(m, InterfaceManager::field_4c0, 0x0900_0051);
        let callback = recorder(&mut e, 0x0900_0051, 0);
        returns(&mut e, PLAYER_IS_PIPBOY_ACTIVE, 1);
        update_pipboy(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 5);
        returns(&mut e, PLAYER_IS_PIPBOY_ACTIVE, 0);
        returns(&mut e, ANIMATION_GROUP_ID, 0x4321);
        update_pipboy(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 0);
        assert_eq!(callback.borrow().len(), 1);
        assert_eq!(e.get(m, InterfaceManager::field_4c0), 0);
        assert_eq!(calls(&e, 0x0900_0050), vec![vec![player, 0x4321, 1]]);
        assert_eq!(calls(&e, ANIMATION_GROUP_ID)[0][1], 2);
    }

    #[test]
    fn the_animation_word_getter_maps_two_indices() {
        let mut e = world();
        let anim = Ptr::<()>::new(e.mem.alloc(0x100));
        for i in 0..8u32 {
            e.mem.set_u32(anim.addr() + 0x5c + 4 * i, 100 + i);
        }
        assert_eq!(fn_0070f490(&mut e, anim, 2), 102);
        assert_eq!(fn_0070f490(&mut e, anim, 0x14), 101);
        assert_eq!(fn_0070f490(&mut e, anim, 0x15), 104);
        assert_eq!(fn_0070f490(&mut e, anim, 0), 100);
    }

    fn pipboy_request_world() -> (Engine, Ptr<InterfaceManager>, u32) {
        let mut e = world();
        let m = manager(&mut e);
        let (player, _) = virtual_object(&mut e, &[(0x22c, 0x0900_0060), (0x230, 0x0900_0061)], 0);
        e.set_global(PLAYER_SINGLETON, player);
        e.set(m, InterfaceManager::pMenusRoot, 0x9999);
        returns(&mut e, TILE_IS_ACCEPTING_EVENTS, 1);
        (e, m, player)
    }

    #[test]
    fn a_pipboy_request_when_closed_stores_the_callback() {
        let (mut e, m, _) = pipboy_request_world();
        fn_0070f4e0(&mut e, m, 0x0900_0070, 0x3eb);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 1);
        assert_eq!(e.get(m, InterfaceManager::field_4c0), 0x0900_0070);
        assert_eq!(calls(&e, TILE_SET_INT), vec![vec![0x9999, 0x1771, 0x3eb]]);
        // Asked again while opening: nothing changes.
        fn_0070f4e0(&mut e, m, 0x0900_0071, 0);
        assert_eq!(e.get(m, InterfaceManager::field_4c0), 0x0900_0070);
    }

    #[test]
    fn a_pipboy_request_when_open_runs_the_callback_at_once() {
        let (mut e, m, _) = pipboy_request_world();
        e.set(m, InterfaceManager::field_4bc, 3);
        let callback = recorder(&mut e, 0x0900_0072, 0);
        fn_0070f4e0(&mut e, m, 0x0900_0072, 0);
        assert_eq!(callback.borrow().len(), 1);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 3);
    }

    #[test]
    fn a_pipboy_request_is_refused_while_the_player_is_busy() {
        let (mut e, m, _) = pipboy_request_world();
        returns(&mut e, CONDITION_CHECK_005A03F0, 1);
        fn_0070f4e0(&mut e, m, 0x0900_0073, 5);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 0);
        assert!(calls(&e, TILE_SET_INT).is_empty());
        returns(&mut e, CONDITION_CHECK_005A03F0, 0);
        // A weapon state value that is not 0.0 refuses too.
        returns_st0(&mut e, WEAPON_STATE_FLOAT, 0.5);
        fn_0070f4e0(&mut e, m, 0x0900_0073, 5);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 0);
        returns_st0(&mut e, WEAPON_STATE_FLOAT, 0.0);
        // The hot keys object blocks it when its sub-object says so.
        e.set_global(HOT_KEYS_OWNER, 0x2000_0c00u32);
        returns(&mut e, SUB_OBJECT_QUERY_00701740, 1);
        fn_0070f4e0(&mut e, m, 0x0900_0073, 5);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 0);
        assert_eq!(calls(&e, SUB_OBJECT_QUERY_00701740).len(), 1);
        returns(&mut e, SUB_OBJECT_QUERY_00701740, 0);
        fn_0070f4e0(&mut e, m, 0x0900_0073, 5);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 1);
        assert_eq!(
            calls(&e, SUB_OBJECT_QUERY_00701740),
            vec![vec![0x2000_0c00 + 0x1c8]; 2]
        );
    }

    #[test]
    fn the_hot_keys_query_forwards_to_the_sub_object() {
        let mut e = world();
        returns(&mut e, SUB_OBJECT_QUERY_00701740, 7);
        assert_eq!(fn_0070f670(&mut e, Ptr::new(0x2000_0d00)), 7);
        assert_eq!(
            calls(&e, SUB_OBJECT_QUERY_00701740),
            vec![vec![0x2000_0d00 + 0x1c8]]
        );
    }

    #[test]
    fn lowering_is_requested_in_the_open_state_only() {
        let mut e = world();
        let m = manager(&mut e);
        let callback = recorder(&mut e, 0x0900_0080, 0);
        // Closed with a callback: run at once.
        fn_0070f690(&mut e, m, 0x0900_0080);
        assert_eq!(callback.borrow().len(), 1);
        // Closed without one: nothing.
        fn_0070f690(&mut e, m, 0);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 0);
        // Open: lowering starts and the callback is kept.
        e.set(m, InterfaceManager::field_4bc, 3);
        fn_0070f690(&mut e, m, 0x0900_0080);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 4);
        assert_eq!(e.get(m, InterfaceManager::field_4c0), 0x0900_0080);
        assert_eq!(callback.borrow().len(), 1);
        // Another state: nothing.
        e.set(m, InterfaceManager::field_4bc, 2);
        fn_0070f690(&mut e, m, 0);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 2);
    }

    /// A manager with a cursor and a pipboy object, in the given menu mode,
    /// whose last-seen menu mode is already that mode.
    fn idle_world(mode: u32) -> (Engine, Ptr<InterfaceManager>) {
        let mut e = arithmetic_world();
        let m = manager(&mut e);
        e.set(m, InterfaceManager::cMenuMode, mode);
        e.set(m, InterfaceManager::pCursor, 0x2000_0e00);
        e.set_global(LAST_MENU_MODE, mode);
        let pipboy = e.mem.alloc(0x200);
        returns(&mut e, GET_PIPBOY, pipboy);
        let (player, _) = virtual_object(
            &mut e,
            &[
                (0x214, 0x0900_00c0),
                (0x22c, 0x0900_00c1),
                (0x230, 0x0900_00c2),
                (0x4b0, 0x0900_00c3),
            ],
            0,
        );
        e.set_global(PLAYER_SINGLETON, player);
        returns(&mut e, XINPUT_GET_STATE, 0x48f);
        e.mem.set_f64(TWO, 2.0);
        e.mem.set_f64(ONE_DOUBLE, 1.0);
        // The screen edges the cursor is clamped to.
        returns_st0(&mut e, EDGE_RIGHT, 640.0);
        returns_st0(&mut e, EDGE_LEFT, -640.0);
        returns_st0(&mut e, EDGE_TOP, 480.0);
        returns_st0(&mut e, EDGE_BOTTOM, -480.0);
        e.mem.set_f64(CURSOR_HORIZONTAL_SCALE, 1280.0);
        e.mem.set_f64(CURSOR_VERTICAL_SCALE, 960.0);
        e.mem.set_f64(MOUSE_LEVEL_DIVISOR, 20.0);
        (e, m)
    }

    #[test]
    fn idle_in_the_game_mode_keeps_the_hud_and_the_player_effects_going() {
        let (mut e, m) = idle_world(1);
        e.set_global(CURSOR_STRING_PENDING, 1u8);
        e.set_global(LEVEL_UP_MENU_PENDING, 1u8);
        returns(&mut e, STRING_HOLDER_GET, 0x6666);
        interface_manager_idle(&mut e, m);
        // No controller: the has-controller flag is clear and no message.
        assert_eq!(e.global::<u8>(HAS_360_CONTROLLER), 0);
        assert!(calls(&e, SHOW_MESSAGE_BOX).is_empty());
        // The scope guard carries the source line.
        assert_eq!(
            calls(&e, SCOPE_GUARD_BEGIN)[0][1..],
            [0xd, 1, SOURCE_FILE, 0x47a]
        );
        // The HUD main menu is created because no tile has class `0x3ec`.
        assert_eq!(calls(&e, TILE_GET_MENU_BY_CLASS)[0], vec![0x3ec]);
        assert_eq!(calls(&e, HUD_MAIN_MENU_CREATE).len(), 1);
        // The pending cursor string is set once and the level-up menu made.
        assert_eq!(
            calls(&e, TILE_SET_STRING),
            vec![vec![0x2000_0e00, 0xfcc, 0x6666, 1]]
        );
        assert_eq!(e.global::<u8>(CURSOR_STRING_PENDING), 0);
        assert_eq!(calls(&e, LEVEL_UP_MENU_CREATE).len(), 1);
        assert_eq!(e.global::<u8>(LEVEL_UP_MENU_PENDING), 0);
        let player = e.global::<u32>(PLAYER_SINGLETON);
        assert_eq!(calls(&e, PLAYER_CAST_EAT_DRINK_ITEMS), vec![vec![player]]);
        assert_eq!(
            calls(&e, PLAYER_CAST_QUEUED_ENCHANTMENTS),
            vec![vec![player]]
        );
        // The effect managers inside the manager update every frame.
        assert_eq!(calls(&e, HUD_EFFECTS_UPDATE), vec![vec![m.addr() + 0x178]]);
        assert_eq!(calls(&e, VATS_EFFECTS_UPDATE), vec![vec![m.addr() + 0x1dc]]);
        assert_eq!(
            calls(&e, TUTORIAL_MANAGER_UPDATE),
            vec![vec![m.addr() + 0x4d4]]
        );
        assert_eq!(calls(&e, SCOPE_GUARD_END).len(), 1);
    }

    #[test]
    fn idle_announces_a_controller_connecting_once() {
        let (mut e, m) = idle_world(1);
        returns(&mut e, XINPUT_GET_STATE, 0x48f);
        interface_manager_idle(&mut e, m);
        assert!(calls(&e, SHOW_MESSAGE_BOX).is_empty());
        // A controller appears: the message box and the notification.
        returns(&mut e, XINPUT_GET_STATE, 0);
        returns(&mut e, STRING_HOLDER_GET, 0x6666);
        returns(&mut e, HAS_360_CONTROLLER_GETTER, 1);
        interface_manager_idle(&mut e, m);
        assert_eq!(e.global::<u8>(HAS_360_CONTROLLER), 1);
        assert_eq!(
            calls(&e, SHOW_MESSAGE_BOX),
            vec![vec![
                0x6666,
                0,
                0,
                0,
                0,
                0x17,
                0.0f32.to_bits(),
                0.0f32.to_bits(),
                0x6666,
                0
            ]]
        );
        assert_eq!(calls(&e, STRING_HOLDER_GET)[0], vec![MESSAGE_TITLE_HOLDER]);
        assert_eq!(
            calls(&e, STRING_HOLDER_GET)[1],
            vec![TEXT_HOLDER_CONTROLLER_WAS_CLEAR]
        );
        assert_eq!(calls(&e, CONTROLLER_CHANGED), vec![vec![m.addr(), 1]]);
        assert_eq!(e.global::<u8>(LAST_HAS_CONTROLLER), 1);
        interface_manager_idle(&mut e, m);
        assert_eq!(calls(&e, SHOW_MESSAGE_BOX).len(), 1, "announced once");
    }

    #[test]
    fn idle_picks_the_hud_mode_when_returning_to_the_game() {
        let (mut e, m) = idle_world(1);
        e.set_global(LAST_MENU_MODE, 2u32);
        let manager_block = e.mem.alloc(0x40);
        e.mem.set_u8(manager_block + 0x20, 1);
        e.mem.set_u8(manager_block + 0x21, 2);
        returns(&mut e, GET_MANAGER, manager_block);
        returns(&mut e, AUDIO_INSTANCE, 0x3030);
        // The player's virtual slot 0x22c answers true: HUD mode 7.
        let (player, _) = virtual_object(&mut e, &[(0x22c, 0x0900_0090), (0x214, 0x0900_0091)], 1);
        e.set_global(PLAYER_SINGLETON, player);
        returns(&mut e, TILE_GET_MENU_BY_CLASS, 0x1);
        interface_manager_idle(&mut e, m);
        assert_eq!(calls(&e, HUD_SET_MENU_MODE), vec![vec![7]]);
        assert_eq!(e.global::<u32>(LAST_MENU_MODE), 1);
        assert_eq!(e.mem.u8(manager_block + 0x20), 0);
        assert_eq!(e.mem.u8(manager_block + 0x21), 0);
        assert_eq!(calls(&e, AUDIO_UNPAUSE_TYPE_6).len(), 2);
        assert_eq!(calls(&e, AUDIO_UNPAUSE_TYPE_4000_0000).len(), 1);
        // The HUD exists (a tile has class 0x3ec): not created.
        assert!(calls(&e, HUD_MAIN_MENU_CREATE).is_empty());
    }

    #[test]
    fn idle_maps_the_top_menu_to_a_hud_mode_in_the_opening_state() {
        for (top, expected) in [
            (0x3e9, 0x11),
            (0x13, 3),
            (0x3ef, 5),
            (0x438, 0x19),
            (0x401, 4),
            (0x41f, 0xf),
        ] {
            let (mut e, m) = idle_world(5);
            e.set_global(LAST_MENU_MODE, 2u32);
            returns(&mut e, GET_ENTER_STACK_TOP, top);
            interface_manager_idle(&mut e, m);
            assert_eq!(
                calls(&e, HUD_SET_MENU_MODE),
                vec![vec![expected]],
                "top {top:#x}"
            );
            assert_eq!(e.global::<u32>(LAST_MENU_MODE), 5);
        }
    }

    #[test]
    fn idle_in_the_menus_clicks_the_tile_under_the_pointer() {
        let (mut e, m) = idle_world(2);
        // The tile under the pointer and its menu (a virtual object).
        let (menu, mut menu_logs) = virtual_object(
            &mut e,
            &[
                (8, 0x0900_00a0),
                (0x18, 0x0900_00a3),
                (0x20, 0x0900_00a1),
                (0x24, 0x0900_00a2),
            ],
            1,
        );
        e.set(m, InterfaceManager::bMouseInMotion, 1);
        e.set(m, InterfaceManager::pOverTileTarget, 0x7100);
        e.set(m, InterfaceManager::pOverTileMenu, menu);
        returns(&mut e, PICK_TILE, 0x7100);
        returns(&mut e, TILE_GET_MENU, menu);
        returns(&mut e, MENU_STATE, 1);
        returns_st0(&mut e, TILE_GET_VALUE, 42.0);
        returns(&mut e, CONTROLS_GET, 0x2222);
        // The primary button was pressed and is held.
        e.register(CONTROLS_QUERY_00A23A50, |_, a| Ret {
            eax: (a[2] == 1 || a[2] == 0) as u32,
            ..Ret::default()
        });
        interface_manager_idle(&mut e, m);
        // The tile is already entered: the press goes to the menu's slot 8
        // with the menu entry id (42) and the tile, becoming the drag-over
        // tile; the held button reaches slots 0x20 and 0x24.
        assert_eq!(e.get(m, InterfaceManager::pDragOverTileTarget), 0x7100);
        assert_eq!(e.get(m, InterfaceManager::pDragOverTileMenu), menu);
        let click = menu_logs.remove(0);
        let _drag_over = menu_logs.remove(0);
        let held = menu_logs.remove(0);
        let drag_held = menu_logs.remove(0);
        assert_eq!(click.borrow().as_slice(), &[vec![menu, 42, 0x7100]]);
        assert_eq!(held.borrow().as_slice(), &[vec![menu, 42, 0x7100]]);
        assert_eq!(drag_held.borrow().as_slice(), &[vec![menu, 42, 0x7100]]);
        assert!(calls(&e, DO_ENTER).is_empty(), "same tile: nothing entered");
    }

    #[test]
    fn idle_scrolls_the_menu_with_the_wheel() {
        let (mut e, m) = idle_world(2);
        let (menu, _) = virtual_object(&mut e, &[], 0);
        e.set(m, InterfaceManager::pOverTileTarget, 0x7200);
        e.set(m, InterfaceManager::pOverTileMenu, menu);
        returns(&mut e, CONTROLS_GET, 0x2222);
        // The wheel query answers 240; the tile wants the wheel.
        e.register(CONTROLS_QUERY_00A239E0, |_, a| Ret {
            eax: if a[1] == 3 { 240 } else { 0 },
            ..Ret::default()
        });
        returns(&mut e, GET_WHEEL_STEPS, 240);
        returns(&mut e, TILE_IS_TRUE, 1);
        returns_st0(&mut e, TILE_GET_VALUE, 9.0);
        interface_manager_idle(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::fMouseWheel), 240.0);
        // 240 / -120: the wheel trait is pulsed with -2 and cleared.
        let sets = calls(&e, TILE_SET_INT);
        assert!(sets.contains(&vec![0x7200, 0xff3, (-2i32) as u32]));
        assert!(sets.contains(&vec![0x7200, 0xff3, 0]));
        assert_eq!(
            calls(&e, DO_WHEEL_MOVE),
            vec![vec![m.addr(), menu, 9, 0x7200]]
        );
    }

    #[test]
    fn idle_opens_the_pipboy_page_for_a_hot_key() {
        let (mut e, m) = idle_world(1);
        returns(&mut e, TILE_GET_MENU_BY_CLASS, 1);
        returns(&mut e, CONTROLS_GET, 0x2222);
        key_query(&mut e, CONTROLS_QUERY_00A24180, 0x3c, 1);
        returns(&mut e, GET_MENUS_ROOT, 0x9999);
        // Closed pipboy: the request goes to `0070f4e0` (refused here).
        returns(&mut e, CONDITION_CHECK_005A03F0, 1);
        interface_manager_idle(&mut e, m);
        let player = e.global::<u32>(PLAYER_SINGLETON);
        assert_eq!(calls(&e, CONDITION_CHECK_005A03F0), vec![vec![player, 4]]);
        // Open pipboy on that page already: the request lowers it.
        let (mut e, m) = idle_world(1);
        returns(&mut e, TILE_GET_MENU_BY_CLASS, 1);
        returns(&mut e, CONTROLS_GET, 0x2222);
        key_query(&mut e, CONTROLS_QUERY_00A24180, 0x3c, 1);
        returns(&mut e, GET_MENUS_ROOT, 0x9999);
        e.set(m, InterfaceManager::field_4bc, 3);
        returns_st0(&mut e, TILE_GET_VALUE, 1002.0);
        returns(&mut e, POP_FROM_ENTER_STACK, 0xffff_ffff);
        interface_manager_idle(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::field_4bc), 4);
        // On another page: the page is switched instead.
        let (mut e, m) = idle_world(1);
        returns(&mut e, TILE_GET_MENU_BY_CLASS, 1);
        returns(&mut e, CONTROLS_GET, 0x2222);
        key_query(&mut e, CONTROLS_QUERY_00A24180, 0x3c, 1);
        returns(&mut e, GET_MENUS_ROOT, 0x9999);
        e.set(m, InterfaceManager::field_4bc, 3);
        returns_st0(&mut e, TILE_GET_VALUE, 1.0);
        interface_manager_idle(&mut e, m);
        assert_eq!(calls(&e, SET_INVENTORY_MENU_VISIBLE), vec![vec![1, 0, 1]]);
    }

    #[test]
    fn idle_toggles_the_console_with_its_key() {
        let (mut e, m) = idle_world(1);
        returns(&mut e, TILE_GET_MENU_BY_CLASS, 1);
        returns(&mut e, CONTROLS_GET, 0x2222);
        key_query(&mut e, CONTROLS_QUERY_00A24660, 0x1d, 1);
        returns(&mut e, MENU_CONSOLE_INSTANCE, 0x1111);
        returns(&mut e, MENU_CONSOLE_TOGGLE_VISIBLE, 1);
        interface_manager_idle(&mut e, m);
        assert_eq!(calls(&e, CLEAR_KEYSTROKES), vec![vec![0x2222]]);
        assert_eq!(calls(&e, MENU_CONSOLE_TOGGLE_VISIBLE), vec![vec![0x1111]]);
        assert_eq!(calls(&e, ADD_TO_ENTER_STACK), vec![vec![m.addr(), 3]]);
        returns(&mut e, MENU_CONSOLE_TOGGLE_VISIBLE, 0);
        interface_manager_idle(&mut e, m);
        assert_eq!(calls(&e, POP_FROM_ENTER_STACK), vec![vec![m.addr(), 3, 0]]);
    }

    #[test]
    fn idle_runs_each_root_menus_per_frame_virtual() {
        let (mut e, m) = idle_world(1);
        returns(&mut e, TILE_GET_MENU_BY_CLASS, 1);
        let (menu, mut logs) = virtual_object(&mut e, &[(0x2c, 0x0900_00b0)], 0);
        e.set(m, InterfaceManager::pMenusRoot, 0x2000_0f00);
        // The children list: one node holding the child tile.
        let node = e.mem.alloc(0xc);
        e.mem.set_u32(node + 8, 0x7300);
        returns(&mut e, NI_POINTER_GET, node);
        e.register(LIST_NEXT_ELEMENT, |e, a| {
            let node = e.mem.u32(a[1]);
            e.mem.set_u32(a[1], 0);
            Ret {
                eax: node + 8,
                ..Ret::default()
            }
        });
        returns(&mut e, TILE_GET_MENU, menu);
        interface_manager_idle(&mut e, m);
        assert_eq!(logs.remove(0).borrow().as_slice(), &[vec![menu]]);
        assert_eq!(calls(&e, LIST_NEXT_ELEMENT).len(), 1);
    }

    /// `CheckMenuButton(button, 1)` answers from `states`; any other button
    /// reports 3 (neither pressed, held nor released).
    fn button_states(e: &mut Engine, states: &[(u32, u32)]) {
        let table: Vec<(u32, u32)> = states.to_vec();
        e.register_double(CHECK_MENU_BUTTON, move |_, a| Ret {
            eax: table
                .iter()
                .find(|(button, _)| *button == a[0])
                .map(|(_, state)| *state)
                .unwrap_or(3),
            ..Ret::default()
        });
    }

    fn gamepad_world() -> (Engine, Ptr<InterfaceManager>, Ptr) {
        let mut e = arithmetic_world();
        let m = manager(&mut e);
        returns(&mut e, HAS_360_CONTROLLER_GETTER, 1);
        returns(&mut e, FADE_CLOCK_READ, 1000);
        let out = Ptr::<()>::new(e.mem.alloc(4));
        (e, m, out)
    }

    #[test]
    fn without_a_controller_no_button_acts() {
        let (mut e, m, out) = gamepad_world();
        returns(&mut e, HAS_360_CONTROLLER_GETTER, 0);
        assert_eq!(
            interface_manager_get_top_gamepad_button(&mut e, m, 0, 0, out),
            0
        );
        assert!(calls(&e, CHECK_MENU_BUTTON).is_empty());
        returns(&mut e, HAS_360_CONTROLLER_GETTER, 1);
        e.set(m, InterfaceManager::bShowMouse, 1);
        assert_eq!(
            interface_manager_get_top_gamepad_button(&mut e, m, 0, 0, out),
            0
        );
    }

    #[test]
    fn a_pressed_or_held_face_button_acts_at_once() {
        let (mut e, m, out) = gamepad_world();
        e.set(m, InterfaceManager::iLastGamepadEvent, 4);
        button_states(&mut e, &[(9, 2)]);
        assert_eq!(
            interface_manager_get_top_gamepad_button(&mut e, m, 0, 0, out),
            9
        );
        assert_eq!(e.mem.u8(out.addr()), 0, "pressed, not held");
        assert_eq!(e.get(m, InterfaceManager::iLastGamepadEvent), 0);
        button_states(&mut e, &[(10, 1)]);
        assert_eq!(
            interface_manager_get_top_gamepad_button(&mut e, m, 0, 0, out),
            10
        );
        assert_eq!(e.mem.u8(out.addr()), 1, "held");
        // A press of button 6 wins over the shoulder buttons.
        button_states(&mut e, &[(6, 2), (0xf, 2)]);
        assert_eq!(
            interface_manager_get_top_gamepad_button(&mut e, m, 0, 0, out),
            6
        );
    }

    #[test]
    fn a_direction_repeats_after_its_delay() {
        let (mut e, m, out) = gamepad_world();
        let slow = float_cell(&mut e, 250.0);
        let fast = float_cell(&mut e, 50.0);
        e.register_double(FLOAT_HOLDER_GET, move |_, a| Ret {
            eax: if a[0] == REPEAT_DELAY_HOLDER_FIRST {
                slow
            } else {
                fast
            },
            ..Ret::default()
        });
        // Up is pressed: the first press, with the repeat wanted by tile.
        button_states(&mut e, &[(1, 2)]);
        assert_eq!(
            interface_manager_get_top_gamepad_button(&mut e, m, 1, 0, out),
            1
        );
        assert_eq!(e.get(m, InterfaceManager::iLastGamepadEvent), 1);
        assert_eq!(e.get(m, InterfaceManager::uGamepadRepeatStartTime), 1250);
        // Without a repeating tile the delay is twice the tick count.
        e.set(m, InterfaceManager::iLastGamepadEvent, 0);
        assert_eq!(
            interface_manager_get_top_gamepad_button(&mut e, m, 0, 0, out),
            1
        );
        assert_eq!(e.get(m, InterfaceManager::uGamepadRepeatStartTime), 2000);
        // Released: the previous event repeats once its time has come.
        button_states(&mut e, &[(1, 0)]);
        e.set(m, InterfaceManager::iLastGamepadEvent, 1);
        e.set(m, InterfaceManager::uGamepadRepeatStartTime, 1500);
        assert_eq!(
            interface_manager_get_top_gamepad_button(&mut e, m, 1, 0, out),
            0,
            "not yet"
        );
        e.set(m, InterfaceManager::uGamepadRepeatStartTime, 900);
        assert_eq!(
            interface_manager_get_top_gamepad_button(&mut e, m, 1, 0, out),
            1
        );
        assert_eq!(e.get(m, InterfaceManager::uGamepadRepeatStartTime), 1050);
    }

    #[test]
    fn idle_gamepad_resets_the_repeat_state() {
        let (mut e, m, out) = gamepad_world();
        e.set(m, InterfaceManager::iLastGamepadEvent, 3);
        button_states(&mut e, &[]);
        assert_eq!(
            interface_manager_get_top_gamepad_button(&mut e, m, 0, 0, out),
            0
        );
        assert_eq!(e.get(m, InterfaceManager::iLastGamepadEvent), 0);
        assert_eq!(e.get(m, InterfaceManager::uGamepadRepeatStartTime), 2000);
        // A shoulder button pair: only 0xf pressed.
        button_states(&mut e, &[(0xf, 2)]);
        assert_eq!(
            interface_manager_get_top_gamepad_button(&mut e, m, 0, 0, out),
            0xf
        );
        button_states(&mut e, &[(0xd, 2), (5, 2)]);
        assert_eq!(
            interface_manager_get_top_gamepad_button(&mut e, m, 0, 0, out),
            0xd
        );
        button_states(&mut e, &[(5, 2)]);
        assert_eq!(
            interface_manager_get_top_gamepad_button(&mut e, m, 0, 0, out),
            5
        );
    }

    /// A root menu list with the given children (tile addresses), iterated
    /// from the last: the previous-element double walks `children`.
    fn menu_list(e: &mut Engine, m: Ptr<InterfaceManager>, children: &[u32]) {
        e.set(m, InterfaceManager::pMenusRoot, 0x2000_1000);
        returns(e, GET_MENUS_ROOT, 0x2000_1000);
        let tiles: Vec<u32> = children.to_vec();
        let cursor = Rc::new(RefCell::new(tiles.len()));
        let tail = if tiles.is_empty() { 0 } else { 1 };
        e.register_double(GET_FIELD_AT_4, move |_, a| Ret {
            eax: if a[0] == 0x2000_1004 { tail } else { 0x7f00 },
            ..Ret::default()
        });
        let cells: Vec<u32> = tiles
            .iter()
            .map(|t| {
                let cell = e.mem.alloc(4);
                e.mem.set_u32(cell, *t);
                cell
            })
            .collect();
        e.register_double(LIST_PREVIOUS_ELEMENT, move |e, a| {
            let mut index = cursor.borrow_mut();
            *index -= 1;
            let cell = cells[*index];
            e.mem.set_u32(a[1], *index as u32);
            Ret {
                eax: cell,
                ..Ret::default()
            }
        });
    }

    #[test]
    fn a_root_menu_can_take_the_event_before_anything_else() {
        let (mut e, m, _) = gamepad_world();
        returns(&mut e, HAS_360_CONTROLLER_GETTER, 0);
        menu_list(&mut e, m, &[0x7a00, 0x7b00]);
        returns(&mut e, TILE_GET_MENU, 0x7c00);
        returns(&mut e, TILE_IS_VISIBLE, 1);
        returns(&mut e, DO_GAMEPAD, 1);
        interface_manager_do_gamepad_event(&mut e, m, 5);
        assert_eq!(
            calls(&e, DO_GAMEPAD),
            vec![vec![m.addr(), 0x7c00, 0xffff_ffff, 0.0f32.to_bits()]]
        );
        assert!(calls(&e, MENU_MANAGER_INSTANCE).is_empty());
    }

    #[test]
    fn the_frontmost_menu_gets_the_event_with_full_strength() {
        let (mut e, m, _) = gamepad_world();
        returns(&mut e, HAS_360_CONTROLLER_GETTER, 0);
        menu_list(&mut e, m, &[]);
        returns(&mut e, MENU_MANAGER_INSTANCE, 0x2000_2000);
        returns(&mut e, GET_FRONTMOST_MENU, 0x7d00);
        returns(&mut e, DO_GAMEPAD, 1);
        interface_manager_do_gamepad_event(&mut e, m, 5);
        assert_eq!(
            calls(&e, DO_GAMEPAD),
            vec![vec![m.addr(), 0x7d00, 5, 1.0f32.to_bits()]]
        );
        assert_eq!(calls(&e, GET_FRONTMOST_MENU), vec![vec![0x2000_2000]]);
    }

    #[test]
    fn a_released_button_goes_to_the_frontmost_menu_slot_0x3c() {
        let (mut e, m, _) = gamepad_world();
        menu_list(&mut e, m, &[]);
        let (menu, mut logs) = virtual_object(&mut e, &[(0x3c, 0x0900_00d0)], 0);
        returns(&mut e, MENU_MANAGER_INSTANCE, 0x2000_2000);
        returns(&mut e, GET_FRONTMOST_MENU, menu);
        returns(&mut e, GET_MOUSE_OVER_TARGET, 0);
        button_states(&mut e, &[(9, 1)]);
        interface_manager_do_gamepad_event(&mut e, m, 0);
        // Button 9 is held: the key repeat is reset only for other events.
        assert_eq!(
            logs.remove(0).borrow().as_slice(),
            &[vec![menu, 9, 1.0f32.to_bits()]]
        );
        assert!(calls(&e, KEY_REPEAT_00705B10).len() == 1);
        assert!(calls(&e, DO_GAMEPAD).is_empty());
    }

    #[test]
    fn with_no_menu_the_console_pick_list_steps() {
        let (mut e, m, _) = gamepad_world();
        returns(&mut e, HAS_360_CONTROLLER_GETTER, 0);
        menu_list(&mut e, m, &[]);
        returns(&mut e, MENU_CONSOLE_INSTANCE, 0x1111);
        returns(&mut e, CONSOLE_PRIMARY_DOWN, 1);
        returns(&mut e, PICK_LIST_COUNT, 3);
        e.set(m, InterfaceManager::iCurrentPickIndex, 1);
        interface_manager_do_gamepad_event(&mut e, m, 1);
        assert_eq!(e.get(m, InterfaceManager::iCurrentPickIndex), 2);
        interface_manager_do_gamepad_event(&mut e, m, 1);
        assert_eq!(
            e.get(m, InterfaceManager::iCurrentPickIndex),
            -1,
            "wraps past the end"
        );
        interface_manager_do_gamepad_event(&mut e, m, 2);
        assert_eq!(
            e.get(m, InterfaceManager::iCurrentPickIndex),
            2,
            "wraps to the last"
        );
    }

    fn menu_world() -> (Engine, Ptr<InterfaceManager>, u32) {
        let (mut e, m, _) = gamepad_world();
        returns(&mut e, HAS_360_CONTROLLER_GETTER, 0);
        menu_list(&mut e, m, &[]);
        let (frontmost, _) = virtual_object(&mut e, &[], 0);
        returns(&mut e, MENU_MANAGER_INSTANCE, 0x2000_2000);
        returns(&mut e, GET_FRONTMOST_MENU, frontmost);
        (e, m, frontmost)
    }

    #[test]
    fn a_direction_with_no_focus_clicks_the_first_reacting_tile() {
        let (mut e, m, _) = menu_world();
        let (menu, mut logs) = virtual_object(&mut e, &[(0xc, 0x0900_00e0)], 0);
        e.set_global(MANAGER_SINGLETON, m.addr());
        // Re-register the frontmost menu with a slot 0xc.
        returns(&mut e, GET_FRONTMOST_MENU, menu);
        returns(&mut e, GET_MOUSE_OVER_TARGET, 0);
        returns(&mut e, SCAN_FOR_MAX_FOCUS, 0);
        returns(&mut e, TILE_GET_FIRST_REF_COPY, 0x9300);
        returns_st0(&mut e, TILE_GET_VALUE, 11.0);
        interface_manager_do_gamepad_event(&mut e, m, 3);
        assert_eq!(e.get(m, InterfaceManager::bMouseInMotion), 0);
        // Direction 3 uses trait 0xfda; the first tile found is clicked.
        assert_eq!(calls(&e, TILE_GET_FIRST_REF_COPY)[0][0..2], [0x7f00, 0xfda]);
        assert_eq!(
            calls(&e, SET_CURRENT_FOCUS_TARGET),
            vec![vec![m.addr(), 0, 0xfc3, 1]]
        );
        assert_eq!(calls(&e, TILE_PLAY_TILE_SOUND), vec![vec![0x9300, 0xfcb]]);
        assert_eq!(
            logs.remove(0).borrow().as_slice(),
            &[vec![menu, 11, 0x9300]]
        );
    }

    #[test]
    fn a_direction_moves_the_focus_to_the_tile_that_reacts() {
        let (mut e, m, _) = menu_world();
        returns(&mut e, GET_MOUSE_OVER_TARGET, 0x9400);
        // The first reacting tile is usable (traits 0xfa3 and 0xfaf true)
        // and was found under trait 0xfc3.
        e.register(TILE_GET_FIRST_REF_COPY, |e, a| {
            e.mem.set_u32(a[2], 0xfc3);
            Ret {
                eax: 0x9500,
                ..Ret::default()
            }
        });
        returns(&mut e, TILE_IS_TRUE, 1);
        interface_manager_do_gamepad_event(&mut e, m, 1);
        assert_eq!(calls(&e, TILE_PLAY_TILE_SOUND), vec![vec![0x9500, 0xfe8]]);
        assert_eq!(
            calls(&e, SET_CURRENT_FOCUS_TARGET),
            vec![vec![m.addr(), 0x9500, 0xfc3, 1]]
        );
        // The tried tiles are kept in a list that is built and torn down.
        assert_eq!(calls(&e, SIMPLE_LIST_CONSTRUCT).len(), 1);
        assert_eq!(calls(&e, SIMPLE_LIST_DESTROY).len(), 1);
        assert_eq!(calls(&e, LIST_PUSH_FRONT_005AE3D0).len(), 1);
        assert_eq!(calls(&e, LIST_CLEAR).len(), 1);
    }

    #[test]
    fn a_direction_with_nowhere_to_go_is_offered_to_the_menu() {
        let (mut e, m, _) = menu_world();
        returns(&mut e, GET_MOUSE_OVER_TARGET, 0x9400);
        returns(&mut e, TILE_GET_FIRST_REF_COPY, 0);
        e.set(m, InterfaceManager::pMouseOverTarget, 0);
        interface_manager_do_gamepad_event(&mut e, m, 2);
        // The frontmost menu is offered the event twice (before and after);
        // nobody takes it, so the focus is set to the current tile.
        assert_eq!(calls(&e, DO_GAMEPAD).len(), 2);
        assert_eq!(
            calls(&e, SET_CURRENT_FOCUS_TARGET),
            vec![vec![m.addr(), 0x9400, 0xfc3, 1]]
        );
        assert_eq!(calls(&e, TILE_PLAY_TILE_SOUND), vec![vec![0x9400, 0xfe8]]);
    }

    #[test]
    fn a_button_event_clicks_the_tile_bound_to_it() {
        let (mut e, m, _) = menu_world();
        let (menu, mut logs) = virtual_object(&mut e, &[(0xc, 0x0900_00f0)], 0);
        returns(&mut e, GET_MOUSE_OVER_TARGET, 0x9600);
        returns(&mut e, TILE_IS_VISIBLE, 1);
        returns(&mut e, TILE_IS_TRUE, 1);
        returns(&mut e, TILE_GET_MENU, menu);
        returns_st0(&mut e, TILE_GET_VALUE, 7.0);
        e.register(TILE_GET_FIRST_REF_COPY, |e, a| {
            e.mem.set_u32(a[2], 0xfc5);
            Ret {
                eax: 0x9700,
                ..Ret::default()
            }
        });
        interface_manager_do_gamepad_event(&mut e, m, 9);
        // Trait 0xfdd belongs to button 9; the modifier bit is raised only
        // while the click runs.
        assert_eq!(calls(&e, TILE_GET_FIRST_REF_COPY)[0][0..2], [0x9600, 0xfdd]);
        assert_eq!(calls(&e, TILE_PLAY_TILE_SOUND), vec![vec![0x9700, 0xfcb]]);
        assert_eq!(logs.remove(0).borrow().as_slice(), &[vec![menu, 7, 0x9700]]);
        assert_eq!(e.get(m, InterfaceManager::iModifierKeys), 0);
        assert_eq!(calls(&e, CLEAR_MENU_BUTTON), vec![vec![9], vec![6]]);
    }

    #[test]
    fn a_button_with_a_disabled_tile_plays_the_refusal_sound() {
        let (mut e, m, _) = menu_world();
        returns(&mut e, GET_MOUSE_OVER_TARGET, 0x9600);
        returns(&mut e, TILE_IS_VISIBLE, 1);
        e.register(TILE_IS_TRUE, |_, _| Ret::default());
        e.register(TILE_GET_FIRST_REF_COPY, |e, a| {
            e.mem.set_u32(a[2], 0xfc7);
            Ret {
                eax: 0x9700,
                ..Ret::default()
            }
        });
        interface_manager_do_gamepad_event(&mut e, m, 10);
        assert_eq!(calls(&e, PLAY_MENU_SOUND), vec![vec![2]]);
        assert!(calls(&e, CLEAR_MENU_BUTTON).is_empty());
    }

    /// A pick list with the given references hit by the ray; the added list
    /// really stores them.
    fn pick_world(references: &[u32]) -> (Engine, Ptr<InterfaceManager>, u32) {
        let mut e = arithmetic_world();
        let m = manager(&mut e);
        let player = e.mem.alloc(0x1000);
        e.mem.set_f32(player + 0x670, 1.5);
        e.set_global(PLAYER_SINGLETON, player);
        e.set(m, InterfaceManager::iCurrentPickIndex, 4);
        e.mem.set_f32(m.addr() + 0x38, 640.0);
        returns(&mut e, NI_PICK_PICK_OBJECTS, 1);
        returns(&mut e, NI_PICK_GET_RESULTS, 0x2000_3000);
        returns(&mut e, RESULTS_COUNT, references.len() as u32);
        let objects: Vec<u32> = references.iter().map(|r| r + 0x1000).collect();
        e.register_double(RESULT_OBJECT, move |_, a| Ret {
            eax: a[0],
            ..Ret::default()
        });
        e.register_double(RESULTS_GET, move |_, a| Ret {
            eax: objects[a[1] as usize],
            ..Ret::default()
        });
        let references: Vec<u32> = references.to_vec();
        e.register_double(FIND_REFERENCE_FOR_3D, move |_, a| Ret {
            eax: a[0] - 0x1000,
            ..Ret::default()
        });
        let _ = references;
        // The list stores what is added to it (the head holds the first).
        e.register(LIST_ADD, |e, a| {
            let head = a[0];
            if e.mem.u32(head) == 0 {
                let slot = e.mem.u32(a[1]);
                e.mem.set_u32(head, slot);
            }
            Ret::default()
        });
        (e, m, player)
    }

    #[test]
    fn the_console_pick_list_is_rebuilt_from_the_ray() {
        let (mut e, m, _) = pick_world(&[0x500, 0x600]);
        fn_00710880(&mut e, m);
        // The ray goes through the cursor position and the camera.
        let ray = calls(&e, CAMERA_BUILD_PICK_RAY);
        assert_eq!(ray.len(), 1);
        assert_eq!(ray[0][1..3], [640, 0]);
        // The field of view comes from the player.
        assert_eq!(
            calls(&e, SCENE_GRAPH_SET_CAMERA_FOV)[0][1..],
            [1.5f32.to_bits(), 0, 0, 0]
        );
        // Every reference hit is added to the new list, then copied in
        // (the old list is cleared) and the index goes back to 0.
        assert_eq!(calls(&e, LIST_ADD).len(), 3);
        assert_eq!(calls(&e, LIST_CLEAR)[0], vec![m.addr() + 0x70]);
        assert_eq!(e.get(m, InterfaceManager::iCurrentPickIndex), 0);
        assert_eq!(e.mem.u32(m.addr() + 0x70), 0x500);
        assert_eq!(calls(&e, NI_PICK_CONSTRUCT)[0][1..], [0, 8]);
        assert_eq!(calls(&e, NI_PICK_DESTRUCT).len(), 1);
        assert_eq!(calls(&e, SIMPLE_LIST_DESTROY).len(), 1);
    }

    #[test]
    fn an_unchanged_pick_list_is_not_copied_again() {
        let (mut e, m, _) = pick_world(&[0x500]);
        // The manager's list already starts with that reference.
        e.mem.set_u32(m.addr() + 0x70, 0x500);
        fn_00710880(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::iCurrentPickIndex), 0);
        // Only the one add into the new list: nothing copied into the
        // manager's list.
        assert_eq!(calls(&e, LIST_ADD).len(), 1);
        assert_eq!(calls(&e, LIST_CLEAR).len(), 2);
    }

    fn f64_words(value: f64) -> [u32; 2] {
        let bits = value.to_bits();
        [bits as u32, (bits >> 32) as u32]
    }

    /// The debug text world: line height 17 (font height 14 + 3), text top
    /// 100, chars pointer 0xc0c0 for every string, print position 600.
    fn debug_text_world() -> (Engine, Ptr<InterfaceManager>) {
        let mut e = arithmetic_world();
        let m = manager(&mut e);
        let setting = e.mem.alloc(4);
        e.mem.set_u32(setting, 100);
        returns(&mut e, GET_SETTING_VALUE, setting);
        returns_st0(&mut e, FONT_HEIGHT, 14.0);
        returns(&mut e, DEBUG_TEXT_INSTANCE, 0xd0d0);
        returns(&mut e, NI_POINTER_GET, 0xc0c0);
        e.mem.set_f32(DEBUG_TEXT_X, 600.0);
        e.mem.set_f32(CURSOR_DIRECTION_Y, -1.0);
        (e, m)
    }

    fn printed(e: &Engine) -> Vec<(u32, u32)> {
        calls(e, DEBUG_TEXT_PRINT)
            .iter()
            .map(|c| (c[0], f32::from_bits(c[3]) as u32))
            .collect()
    }

    #[test]
    fn with_no_pick_the_debug_text_is_cleared() {
        let (mut e, m) = debug_text_world();
        e.set_global(DEBUG_TEXT_BOTTOM, 200i32);
        e.set(m, InterfaceManager::pPickRef, 0x9000);
        interface_manager_display_current_pick_ref(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::pPickRef), 0);
        assert_eq!(calls(&e, SET_PICK_REF_DISPLAY), vec![vec![m.addr(), 0]]);
        // One empty line at the top, then the old lines are blanked down to
        // the previous bottom, a line apart.
        let rows: Vec<u32> = printed(&e).iter().map(|p| p.1).collect();
        assert_eq!(rows, vec![100, 130, 147, 164, 181, 198]);
        assert_eq!(
            calls(&e, DEBUG_TEXT_PRINT)[0],
            vec![
                0xd0d0,
                EMPTY_TEXT,
                600.0f32.to_bits(),
                100.0f32.to_bits(),
                2,
                0xffff_ffff,
                (-1.0f32).to_bits(),
                0,
                0
            ]
        );
        assert_eq!(e.global::<i32>(DEBUG_TEXT_BOTTOM), 130);
    }

    #[test]
    fn the_picked_reference_is_printed_with_its_name_and_form_id() {
        let (mut e, m) = debug_text_world();
        e.mem.set_u32(m.addr() + 0x70, 0x9000);
        returns(&mut e, FORM_ID_GETTER, 0x1234);
        returns(&mut e, REFR_NAME, 0x5555);
        interface_manager_display_current_pick_ref(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::pPickRef), 0x9000);
        assert_eq!(
            calls(&e, SET_PICK_REF_DISPLAY),
            vec![vec![m.addr(), 0x9000]]
        );
        let format = calls(&e, FORMAT_INTO_STRING);
        assert_eq!(format.len(), 1);
        assert_eq!(format[0][1..], [FORMAT_NAME_AND_ID, 0x5555, 0x1234]);
        assert_eq!(calls(&e, DEBUG_TEXT_PRINT)[0][1], 0xc0c0);
        assert_eq!(printed(&e), vec![(0xd0d0, 100)]);
        assert_eq!(calls(&e, STRING_CONSTRUCT).len(), 1);
        assert_eq!(calls(&e, STRING_DESTROY).len(), 1);
    }

    /// A picked reference object (virtual: slot 0x100 is the actor test)
    /// in a world where help is shown.
    fn help_world(actor: u32) -> (Engine, Ptr<InterfaceManager>, u32) {
        let (mut e, m) = debug_text_world();
        let (pick, _) = virtual_object(&mut e, &[(0x100, 0x0900_0100)], actor);
        e.mem.set_u32(m.addr() + 0x70, pick);
        e.set(m, InterfaceManager::bFullHelp, 1);
        returns(&mut e, FORM_ID_GETTER, 0x1234);
        returns(&mut e, REFR_NAME, 0x5555);
        returns(&mut e, REFR_EXTRA_DATA_LIST, 0x7a7a);
        returns(&mut e, EXTRA_GET_COUNT, 1);
        (e, m, pick)
    }

    #[test]
    fn the_help_lines_list_the_owner_and_the_stack_count() {
        let (mut e, m, pick) = help_world(0);
        let (owner, _) = virtual_object(&mut e, &[(0x130, 0x0900_0101)], 0x7777);
        returns(&mut e, REFR_GET_OWNER, owner);
        returns(&mut e, EXTRA_GET_COUNT, 3);
        interface_manager_display_current_pick_ref(&mut e, m);
        let formats = calls(&e, FORMAT_INTO_STRING);
        assert_eq!(formats.len(), 3);
        assert_eq!(formats[0][1..], [FORMAT_NAME_AND_ID, 0x5555, 0x1234]);
        assert_eq!(formats[1][1..], [OWNER_FORMAT, 0x7777, 0x1234]);
        assert_eq!(formats[2][1..], [COUNT_FORMAT, 3]);
        // The first line is at the top; the help lines start 30 below and
        // are a line (17) apart.
        let rows: Vec<u32> = printed(&e).iter().map(|p| p.1).collect();
        assert_eq!(rows, vec![100, 130, 147]);
        assert_eq!(e.global::<i32>(DEBUG_TEXT_BOTTOM), 164);
        let _ = pick;
    }

    #[test]
    fn the_help_lines_show_the_lock_and_the_teleport_door() {
        let (mut e, m, _) = help_world(0);
        let lock = e.mem.alloc(0x40);
        e.mem.set_u32(lock + 4, 0x4000_0000);
        returns(&mut e, REFR_GET_LOCK, lock);
        returns(&mut e, LOCK_IS_LOCKED, 1);
        returns(&mut e, LOCK_GET_LEVEL, 2);
        e.mem.set_u32(LOCK_LEVEL_TABLE + 8, 0x0102_0000);
        returns(&mut e, STRING_HOLDER_GET, 0x4545);
        returns(&mut e, MAP_MARKER_GET_LOCATION_NAME, 0x1111);
        let door = e.mem.alloc(0x20);
        returns(&mut e, REFR_GET_TELEPORT_DATA, door);
        returns(&mut e, DOOR_TELEPORT_GET_CELL, 0x2000_5000);
        interface_manager_display_current_pick_ref(&mut e, m);
        let formats = calls(&e, FORMAT_INTO_STRING);
        assert!(formats
            .iter()
            .any(|f| f[1..] == [LOCK_FORMAT, 0x4545, LOCKED_TEXT]));
        assert!(formats.iter().any(|f| f[1..] == [KEY_FORMAT, 0x1111]));
        assert!(formats
            .iter()
            .any(|f| f[1..] == [TELEPORT_FORMAT, 0x1111, 0x5555]));
        assert_eq!(calls(&e, LOCK_GET_LEVEL).len(), 1);
        assert_eq!(calls(&e, STRING_HOLDER_GET), vec![vec![0x0102_0000]]);
    }

    #[test]
    fn the_help_lines_spell_out_the_flags_and_the_furniture_markers() {
        let (mut e, m, pick) = help_world(0);
        returns(&mut e, REFR_FLAGS, 5);
        returns(&mut e, TRIM_TEXT, 0x6a6a);
        // The furniture: heading and two markers (enabled, disabled).
        returns(&mut e, REFR_IS_FURNITURE, 1);
        let rotation = e.mem.alloc(0xc);
        e.mem.set_f32(rotation + 8, 1.0);
        returns(&mut e, REFR_GET_ROTATION, rotation);
        e.mem.set_f64(RADIANS_TO_DEGREES, 57.0);
        e.register(REFR_GET_MARKER_AT_INDEX, |_, a| Ret {
            eax: (a[1] < 2) as u32,
            ..Ret::default()
        });
        e.register(FURNITURE_GET_MARKER_ENABLED, |_, a| Ret {
            eax: (a[1] == 0) as u32,
            ..Ret::default()
        });
        returns(&mut e, MARKER_KIND, 5);
        returns(&mut e, FURNITURE_IS_SIT_MARKER, 1);
        returns(&mut e, REFR_GET_MARKER_USED, 1);
        returns_st0(&mut e, FURNITURE_GET_MARKER_ANGLE, 2.0);
        returns_st0(&mut e, REFR_GET_SCALE, 1.0);
        e.register(FURNITURE_GET_MARKER_TARGET_OFFSET, |e, a| {
            for (i, v) in [1.5f32, 2.5, 3.5].iter().enumerate() {
                e.mem.set_f32(a[1] + 4 * i as u32, *v);
            }
            Ret::default()
        });
        interface_manager_display_current_pick_ref(&mut e, m);
        let formats = calls(&e, FORMAT_INTO_STRING);
        let words = |v: f64| f64_words(v);
        // Heading: 1.0 * 57 degrees.
        assert!(formats.iter().any(|f| f[1..] == [HEADING_FORMAT, 57]));
        // The enabled marker, with its three offsets as doubles.
        let mut enabled = vec![MARKER_FORMAT, SIT_TEXT, 5];
        for v in [1.5, 2.5, 3.5] {
            enabled.extend(words(v));
        }
        enabled.extend([114, USED_TEXT]);
        assert!(formats.iter().any(|f| f[1..] == enabled[..]));
        assert!(formats
            .iter()
            .any(|f| f[1..] == [DISABLED_MARKER_FORMAT, SIT_TEXT, 5]));
        // Flags 1 and 4 were appended to the prefix, in order.
        assert_eq!(calls(&e, STRING_COPY)[0][1..], [0x100, FLAGS_PREFIX]);
        let appended: Vec<u32> = calls(&e, STRING_APPEND).iter().map(|c| c[2]).collect();
        assert_eq!(appended, vec![FLAG_TEXT_1, FLAG_TEXT_4]);
        assert!(formats.iter().any(|f| f[1..] == [0x6a6a]));
        let _ = pick;
    }

    fn cursor_world() -> (Engine, Ptr<InterfaceManager>) {
        let (mut e, m) = idle_world(2);
        e.set(m, InterfaceManager::pCursor, 0x7000);
        returns(&mut e, CONTROLS_GET, 0x2222);
        returns(&mut e, TILE_IMAGE_NODE, 0x5555);
        let translation = e.mem.alloc(0xc);
        returns(&mut e, NODE_TRANSLATION, translation);
        returns_st0(&mut e, GET_SCREEN_WIDTH, 1280.0);
        returns_st0(&mut e, GET_SCREEN_HEIGHT, 960.0);
        returns_st0(&mut e, GET_DESKTOP_WIDTH, 1920.0);
        returns_st0(&mut e, GET_DESKTOP_HEIGHT, 1080.0);
        returns_st0(&mut e, TILE_GET_VALUE, 2.0);
        e.mem.set_f64(CURSOR_TILT_SCALE, -0.008);
        // The mouse moved 10 right and 5 up.
        e.register(CONTROLS_QUERY_00A239E0, |_, a| Ret {
            eax: match a[1] {
                1 => 10,
                2 => (-5i32) as u32,
                _ => 0,
            },
            ..Ret::default()
        });
        (e, m)
    }

    /// Records the translation vectors written to the node.
    fn vector_recorder(e: &mut Engine) -> Rc<RefCell<Vec<[f32; 3]>>> {
        let seen = Rc::new(RefCell::new(vec![]));
        let log = seen.clone();
        e.register_double(NODE_SET_TRANSLATE_VECTOR, move |e, a| {
            log.borrow_mut()
                .push([e.mem.f32(a[1]), e.mem.f32(a[1] + 4), e.mem.f32(a[1] + 8)]);
            Ret::default()
        });
        seen
    }

    #[test]
    fn the_cursor_follows_the_mouse() {
        let (mut e, m) = cursor_world();
        let vectors = vector_recorder(&mut e);
        fn_007118d0(&mut e, m);
        // x = 1280 * 10 / 1280 and z = -(960 * -5 / 960), kept on screen.
        assert_eq!(vectors.borrow().len(), 1);
        assert_eq!(vectors.borrow()[0], [10.0, (2.0f64 * -0.008) as f32, 5.0]);
        assert_eq!(calls(&e, NODE_SET_TRANSLATE_VECTOR)[0][0], 0x5555);
        // The manager remembers the position and moves the real cursor.
        assert_eq!(e.mem.f32(m.addr() + 0x2c), 10.0);
        assert_eq!(e.mem.f32(m.addr() + 0x34), 5.0);
        assert_eq!(e.get(m, InterfaceManager::bMouseInMotion), 1);
        assert_eq!(e.mem.f32(m.addr() + 0x38), 975.0);
        assert_eq!(e.mem.f32(m.addr() + 0x40), 534.375);
        // The cursor node is shown through trait 0xfa3 and node updated.
        assert!(calls(&e, TILE_SET_INT).contains(&vec![0x7000, 0xfa3, 1]));
        assert_eq!(calls(&e, NODE_SET_FLAG), vec![vec![0x5555, 0]]);
        assert_eq!(calls(&e, NODE_UPDATE).len(), 1);
        // Same input again, from the same place: nothing moved.
        e.set(m, InterfaceManager::bMouseInMotion, 0);
        fn_007118d0(&mut e, m);
        assert_eq!(e.get(m, InterfaceManager::bMouseInMotion), 0);
    }

    #[test]
    fn the_cursor_stays_inside_the_screen() {
        let (mut e, m) = cursor_world();
        let vectors = vector_recorder(&mut e);
        e.register(CONTROLS_QUERY_00A239E0, |_, a| Ret {
            eax: if a[1] == 1 { 100_000 } else { 0 },
            ..Ret::default()
        });
        fn_007118d0(&mut e, m);
        // Half the 1280 screen minus 2.
        assert_eq!(vectors.borrow()[0][0], 638.0);
    }

    #[test]
    fn with_a_controller_and_no_console_the_cursor_does_not_move() {
        let (mut e, m) = cursor_world();
        returns(&mut e, HAS_360_CONTROLLER_GETTER, 1);
        returns(&mut e, CONSOLE_PRIMARY_DOWN, 0);
        fn_007118d0(&mut e, m);
        assert!(calls(&e, SCOPE_GUARD_BEGIN).is_empty());
        assert!(calls(&e, NODE_SET_TRANSLATE_VECTOR).is_empty());
        // With the mouse shown it moves even so.
        e.set(m, InterfaceManager::bShowMouse, 1);
        fn_007118d0(&mut e, m);
        assert_eq!(calls(&e, NODE_SET_TRANSLATE_VECTOR).len(), 1);
    }
    // @@TESTS-END@@
}
