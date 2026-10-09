//! `fallout shared/tesobjectcell.cpp` (Xbox PDB source unit), part 3: its functions from `00552470` up to
//! (not including) `00558e20` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::tesobjectcell`]; anything public there may be used here.
//!
//! # Where this file stops
//!
//! The first batch holds the 40 functions from `00552470` to `00553ee0`: a
//! few flag and field accessors the compiler emitted here, the scene-graph
//! walks that update morph controllers (`UpdateMorphControllersOnly`), the
//! pending-node queue and its worker, `AttachToWorld`, `Detach`, the creation
//! of an interior cell's Havok world, the small wrappers of that world
//! (`005533c0` to `005534f0`), the world cinfo destructors, `DetachHavok`,
//! `UpdateRefSounds` and the ray-test helpers (`Pick`). The next session
//! continues with the first function after `00553ee0` that is not done
//! (`00553f70`).
//!
//! # Conventions of the exe worth knowing
//!
//! - A word pushed before a call to a function that pops fewer words
//!   belongs to a later call (`PUSH x; PUSH 0; CALL 00450b80; ADD ESP,4;
//!   MOV ECX,EAX; CALL 00b5ddf0` passes `x` to `00b5ddf0`); every call here
//!   was matched against the callee's `RET n`.
//! - The 12-byte update record `0043d410` builds on the stack is passed by
//!   address to the scene-graph update virtuals (`+0x94`, `+0xc0`) and to
//!   `00a59c60`.
//! - `NiTMap` walk: `004b9ba0` gives the first position, `006b7f20` takes
//!   the addresses of the position and of two out words (key, value) and
//!   advances the position.
//! - Scope guards (`00404eb0`, 4 bytes on the stack, with the source path
//!   and the line of the allocation), C++ exception unwinding and the stack
//!   cookies are modelled only as far as the guard's constructor and
//!   destructor calls.
//! - The `float` results of `005471e0` and `005723b0` come in `ST0`; the
//!   x87 `FISTP` with truncation of `UpdateRefSounds` is reproduced by
//!   [`truncated_low_word`].

#[allow(unused_imports)]
use super::tesobjectcell::*;
#[allow(unused_imports)]
use crate::prelude::*;

// ---------------------------------------------------------------------------
// Callees outside this file, by exe address.

/// `(word at +0x30) & mask != 0` (`00456630`) and the setter of the same
/// bits `(this, set, mask)` (`0043b370`): the flag word of the scene-graph
/// objects `fn_00552470`, `fn_005526b0` and `fn_005526d0` read.
const TEST_FLAG_BITS: u32 = 0x0045_6630;
const SET_FLAG_BITS: u32 = 0x0043_b370;
/// Returns a pointer to a byte (`this + 4`, or a static zero byte for null).
const BYTE_HOLDER_GET: u32 = 0x0040_8d60;
/// The object whose address `fn_005524d0` hands to `00408d60`.
const BYTE_HOLDER: u32 = 0x0126_7c24;

/// The lock (`011ca1a0`) and the list (`011ca174`) of nodes waiting for the
/// worker `fn_00552570`, and the calls on them.
const PENDING_LOCK: u32 = 0x011c_a1a0;
const PENDING_LIST: u32 = 0x011c_a174;
const LOCK_ENTER: u32 = 0x0040_fbf0;
const LOCK_LEAVE: u32 = 0x0040_fba0;
/// Appends a smart pointer slot (by address) to the pending list.
const PENDING_LIST_ADD: u32 = 0x0063_1540;
/// The list is empty (`004a4460`) and `RemoveHead` (`004ee8a0`); the address
/// of a list node's item is `006815c0`.
const PENDING_LIST_IS_EMPTY: u32 = 0x004a_4460;
const PENDING_LIST_REMOVE_HEAD: u32 = 0x004e_e8a0;
const LIST_NODE_ITEM_ADDRESS: u32 = 0x0068_15c0;
const LIST_NODE_NEXT: u32 = 0x0072_6070;
const LIST_NODE_IS_END: u32 = 0x0082_56d0;
/// `NiPointer` slot: construct from a raw pointer, copy-construct from the
/// address of another slot, read the pointer, release.
const SLOT_FROM_RAW: u32 = 0x0063_3c90;
const SLOT_COPY: u32 = 0x0055_9a40;
const SLOT_GET_POINTER: u32 = 0x0055_9450;
const SLOT_RELEASE: u32 = 0x0045_cec0;
/// `TESObjectREFR::FindReferenceFor3D` (Xbox PDB), cdecl.
const FIND_REFERENCE_FOR_3D: u32 = 0x0056_f930;
/// The parent cell of a reference (`008d6f30`) and the cell function the
/// worker calls on it with `(reference, 1, 0)`.
const REFERENCE_GET_PARENT_CELL: u32 = 0x008d_6f30;
const CELL_UPDATE_FOR_REFERENCE: u32 = 0x0054_a070;

/// The 12-byte update record: constructor `(this, time, flag, flag)`.
const UPDATE_RECORD_CONSTRUCT: u32 = 0x0043_d410;
const UPDATE_RECORD_SIZE: u32 = 12;
/// The child array of a node (virtual `+0x0c` gives it), its count and its
/// element at an index.
const NODE_CHILD_COUNT: u32 = 0x0043_b480;
const NODE_CHILD_AT: u32 = 0x0043_b4a0;
/// Applies an update record to a controller (`00a59c60`).
const CONTROLLER_UPDATE: u32 = 0x00a5_9c60;
/// `NiObjectNET::GetController(this, type)` (`00a5c570`), the type the
/// morph walk asks for, and the update time it passes.
const NODE_GET_CONTROLLER: u32 = 0x00a5_c570;
const MORPH_CONTROLLER_TYPE: u32 = 0x011f_3728;
const UPDATE_TIME: u32 = 0x011c_3c08;

/// `NiTMap` walk (see the header) and the node of an animated reference
/// (`009611e0`).
const MAP_FIRST_POSITION: u32 = 0x004b_9ba0;
const MAP_NEXT: u32 = 0x006b_7f20;
const ANIMATED_NODE_OWNER: u32 = 0x0096_11e0;

/// The cell's node accessors: `00545cf0` is `Load3D`; `005497a0`,
/// `0054a050`, `0054aa60` and `00524cf0` return nodes of the cell;
/// `00456fc0(cell, index)` returns the child of the cell node at `index`.
const CELL_LOAD_3D: u32 = 0x0054_5cf0;
const CELL_NODE_A: u32 = 0x0054_97a0;
const CELL_NODE_B: u32 = 0x0054_a050;
const CELL_NODE_C: u32 = 0x0054_aa60;
const CELL_NODE_D: u32 = 0x0052_4cf0;
const CELL_CHILD_NODE: u32 = 0x0045_6fc0;
/// Cell helpers: the Havok world of the cell (`004543c0`), the cell's
/// collision world (`00537b30`), "is interior" (`00425fd0`), the cell state
/// setter and getter (`004512a0`, `00450fd0`), "is loaded" (`00450ff0`), the
/// cell lock (`00541ac0`, `00541ae0`) and the list of references (`009604f0`).
const CELL_HAVOK_WORLD: u32 = 0x0045_43c0;
const CELL_COLLISION_WORLD: u32 = 0x0053_7b30;
const CELL_IS_INTERIOR: u32 = 0x0042_5fd0;
const CELL_SET_STATE: u32 = 0x0045_12a0;
const CELL_GET_STATE: u32 = 0x0045_0fd0;
const CELL_IS_LOADED: u32 = 0x0045_0ff0;
const CELL_LOCK_ENTER: u32 = 0x0054_1ac0;
const CELL_LOCK_LEAVE: u32 = 0x0054_1ae0;
const CELL_REFERENCES: u32 = 0x0096_04f0;
const CELL_GET_WORLD_SPACE: u32 = 0x0054_ddd0;
const CELL_GET_DATA_X: u32 = 0x0054_4c30;
const CELL_GET_DATA_Y: u32 = 0x0054_4c60;
const CELL_GET_WATER_HEIGHT: u32 = 0x0054_71e0;
const CELL_REFERENCE_CENTER: u32 = 0x0054_c030;
const CELL_SET_LIGHTS_ATTACHED: u32 = 0x0054_ba80;
const CELL_SET_ADDON_NODES_ATTACHED: u32 = 0x0055_77b0;
const CELL_SET_ATTACHED_FLAG: u32 = 0x0054_b950;
const CELL_AFTER_ATTACH_A: u32 = 0x0055_7470;
const CELL_AFTER_ATTACH_B: u32 = 0x0055_75d0;
const CELL_BEFORE_DETACH: u32 = 0x0055_7c40;
const CELL_DETACH_STEP_A: u32 = 0x0055_74d0;
const CELL_DETACH_STEP_B: u32 = 0x0055_76c0;
const CELL_SET_TERRAIN_UPDATE: u32 = 0x0054_af40;
const CELL_CLEAR_LOADED_STATE: u32 = 0x0054_6c70;
const CELL_PORTAL_GRAPH: u32 = 0x0054_6930;
const CELL_OPTIONS: u32 = 0x009d_9f20;
const CELL_REFERENCE_NODES: u32 = 0x0054_5710;
const CELL_UPDATE_REFERENCE_FLAGS: u32 = 0x0048_4730;
const FORM_FLAG_4000: u32 = 0x0040_77c0;
const FORM_FLAG_2: u32 = 0x0046_0340;
const FORM_ID: u32 = 0x0084_e3a0;
/// The same getter (`mov eax,[ecx+0x0c]`) used on the scene singleton, where
/// the word is the node `Detach` detaches the cell's 3D from.
const WORD_AT_0C: u32 = 0x0084_e3a0;
/// `TESObjectREFR::GetBaseObject` (`007af430`) and the form type byte
/// (`00401170`).
const REFERENCE_BASE_OBJECT: u32 = 0x007a_f430;
const FORM_TYPE: u32 = 0x0040_1170;
/// `fn_00552900` / `Detach` call this on the world space of an exterior
/// cell: virtual `+0xc8`.
const WORLD_SPACE_SLOT: u32 = 0xc8;

/// Singletons: the scene (`011dea10`, the object of `tes.cpp`'s methods), the
/// loader (`011ddf38`), the loader state (`011c3f2c`), the model loader
/// (`011c3b3c`), the object `011de134` and the render pointer `011d6e2c`.
const SCENE_SINGLETON: u32 = 0x011d_ea10;
const LOADER_SINGLETON: u32 = 0x011d_df38;
const LOADER_STATE_SINGLETON: u32 = 0x011c_3f2c;
const MODEL_LOADER_SINGLETON: u32 = 0x011c_3b3c;
const STATE_OBJECT_SINGLETON: u32 = 0x011d_e134;
const RENDER_SINGLETON: u32 = 0x011d_6e2c;
/// Scene (`tes.cpp`) calls: hide the cell's world, attach interior and
/// exterior cells.
const SCENE_REFRESH: u32 = 0x0045_2e50;
const SCENE_ATTACH_INTERIOR: u32 = 0x0045_4c20;
const SCENE_ATTACH_EXTERIOR: u32 = 0x0045_5130;
/// Model loader: `CancelReferencesForCell` (Xbox PDB `ModelLoader`).
const MODEL_LOADER_CANCEL_FOR_CELL: u32 = 0x0044_5670;
/// Render calls on `011d6e2c` (`navmeshrender.cpp`): add and remove the
/// draw-only nav meshes of a cell, and the flag it tests.
const NAV_MESH_RENDER_ADD: u32 = 0x006a_37a0;
const NAV_MESH_RENDER_REMOVE: u32 = 0x006a_5bb0;
/// Save/load of the whole game: "the form has changes" test and the call
/// that drops them.
const FORM_HAS_CHANGES: u32 = 0x0046_9860;
const REMOVE_CHANGES: u32 = 0x0084_a7c0;
/// `tes.cpp`'s table of nodes (`00450b80(index)`), the setter of the shadow
/// scene node (`00b5ddf0`) and the child accessor `0045bc00(entry, 5)`.
const TABLE_ENTRY: u32 = 0x0045_0b80;
const SCENE_NODE_ATTACH: u32 = 0x00b5_ddf0;
const SCENE_NODE_CHILD: u32 = 0x0045_bc00;
/// The navmesh obstacle manager: `006c0720` returns it, `006c1170` runs.
const OBSTACLE_MANAGER_GET: u32 = 0x006c_0720;
const OBSTACLE_MANAGER_RUN: u32 = 0x006c_1170;
/// The portal graph (`bsportalgraph.obj`): count of the always-render array
/// and its add, replace and remove operations.
const PORTAL_ALWAYS_RENDER_COUNT: u32 = 0x00c5_b5d0;
const PORTAL_ALWAYS_RENDER_ADD: u32 = 0x00c5_b1c0;
const PORTAL_ALWAYS_RENDER_SET: u32 = 0x00c5_b9a0;
const PORTAL_ALWAYS_RENDER_REMOVE: u32 = 0x00c5_ba50;

/// Havok world (`bhkWorld`, 0xA0 bytes) calls: the visual debugger flag
/// functions, the world's own setters and the collision world accessors.
const WORLD_SET_VISUAL_DEBUGGER_ENABLED: u32 = 0x0045_4380;
const WORLD_SET_VISUAL_DEBUGGER: u32 = 0x00c6_a640;
const VISUAL_DEBUGGER_SETTING: u32 = 0x0045_4ae0;
const VISUAL_DEBUGGER_SUPPRESSED: u32 = 0x0045_6c70;
const SCENE_HAVOK_SETTING: u32 = 0x0045_3720;
const WORLD_SET_GRAVITY_SCALE: u32 = 0x00c6_a620;
const WORLD_SET_CENTER: u32 = 0x00c6_8750;
const WORLD_CONSTRUCT: u32 = 0x00c6_9010;
const WORLD_UPDATE_RENDERER: u32 = 0x00c6_8a00;
const WORLD_COLLISION_OBJECT: u32 = 0x0045_cd60;
const COLLISION_OBJECT_SET_ACTIVE: u32 = 0x0061_f8e0;
const COLLISION_OBJECT_RELEASE: u32 = 0x0048_93c0;
const COLLISION_WORLD_UNLOAD: u32 = 0x0062_1e80;
const COLLISION_WORLD_CONSTRUCT: u32 = 0x0062_1d10;
/// The `011ca088` object, its constructor and the registration call.
const LISTENER_SINGLETON: u32 = 0x011c_a088;
const LISTENER_CONSTRUCT: u32 = 0x0062_4150;
const LISTENER_REGISTER: u32 = 0x0062_42a0;
/// Cell extra data (`this + 0x28`): store the Havok world and the collision
/// world.
const EXTRA_LIST_OFFSET: u32 = 0x28;
const EXTRA_LIST_SET_HAVOK_WORLD: u32 = 0x0041_b8d0;
const EXTRA_LIST_SET_COLLISION_WORLD: u32 = 0x0041_ba60;
/// The object a world holder gives (`00541c80`); `00483710`, a function whose body is empty (the wrappers call it around their setters, and `UpdateRefSounds` on its sound handle at the end); the world setters.
const HOLDER_GET_WORLD: u32 = 0x0054_1c80;
const EMPTY_FUNCTION: u32 = 0x0048_3710;
const WORLD_SET_GRAVITY_VECTOR: u32 = 0x00c9_3820;
const WORLD_SET_FIELD_A: u32 = 0x00c9_3a20;
const WORLD_SET_FIELD_B: u32 = 0x00c9_3860;
const HOLDER_FIELD_70_ADD: u32 = 0x0062_99c0;
/// Objects the interior world is made of: allocation size and constructor.
const BROAD_PHASE_CONSTRUCT: u32 = 0x0062_d5c0;
const FILTER_CONSTRUCT: u32 = 0x0061_f460;
const WATER_CONSTRUCT: u32 = 0x0062_e630;
const LISTENER_OBJECT_CONSTRUCT: u32 = 0x0062_a460;
const HAS_WATER_TEST: u32 = 0x0045_18e0;
/// The world cinfo (`hkpWorldCinfo`): constructor, setters, the 16-byte
/// vector constructor and the stack block it needs.
const CINFO_CONSTRUCT: u32 = 0x00c6_81c0;
const CINFO_SET_GRAVITY: u32 = 0x004a_3f10;
const CINFO_SET_BROAD_PHASE_SIZE: u32 = 0x00c9_0db0;
const CINFO_SIZE: u32 = 0xf0;
const GRAVITY_Z: u32 = 0x011c_a158;
const BROAD_PHASE_SIZE: u32 = 0x0102_efc0;
/// The cinfo destructor chain.
const CINFO_VTABLE: u32 = 0x0102_f0ec;
const CINFO_MEMBER_DESTRUCT: u32 = 0x0055_8e80;
const CINFO_BASE_DESTRUCT: u32 = 0x0053_8e10;
const CINFO_FREE: u32 = 0x0053_8eb0;
/// Terrain transform of an exterior cell: the transform constructor, the
/// identity rotation, the flags word, `2048.0` as a double and the call.
const TRANSFORM_CONSTRUCT: u32 = 0x0047_6a80;
const IDENTITY_ROTATION: u32 = 0x011a_9448;
const TERRAIN_FLAGS: u32 = 0x011a_9ea4;
const HALF_CELL_SIZE: u32 = 0x0101_6968;
const WORLD_ADD_TERRAIN: u32 = 0x0062_2590;

/// Memory manager: the object (`00401020`), `Allocate` and `Deallocate`;
/// `operator new(size)` (`00401000`) and the allocator `00aa13e0` (cdecl).
const MEMORY_MANAGER_GET: u32 = 0x0040_1020;
const MEMORY_MANAGER_ALLOCATE: u32 = 0x00aa_3e40;
const MEMORY_MANAGER_DEALLOCATE: u32 = 0x00aa_4060;
const OPERATOR_NEW: u32 = 0x0040_1000;
const ALLOCATE_CDECL: u32 = 0x00aa_13e0;

/// The allocation scope guard (4 bytes on the stack) and the path of
/// `TESObjectCELL.cpp` it records.
const SCOPE_GUARD_CONSTRUCT: u32 = 0x0040_4eb0;
const SCOPE_GUARD_DESTRUCT: u32 = 0x0040_4ee0;
const SCOPE_GUARD_SIZE: u32 = 4;
const SOURCE_FILE_NAME: u32 = 0x0102_ed68;

/// Tests `UpdateRefSounds` makes on a reference: disabled (`00440da0`),
/// destroyed (`00477ba0`), its base form (`00441110`), the chance test
/// (`004dff00`), the sound name of the form (`00511840`), the distance
/// (`005723b0`, from the object at `011dea3c`) and the divisor of the chance.
const REFERENCE_IS_DISABLED: u32 = 0x0044_0da0;
const REFERENCE_IS_DESTROYED: u32 = 0x0047_7ba0;
const REFERENCE_BASE_FORM: u32 = 0x0044_1110;
const SOUND_CHANCE_TEST: u32 = 0x004d_ff00;
const FORM_SOUND_NAME: u32 = 0x0051_1840;
const REFERENCE_DISTANCE: u32 = 0x0057_23b0;
const DISTANCE_SOURCE_SINGLETON: u32 = 0x011d_ea3c;
const SOUND_CHANCE_DIVISOR: u32 = 0x0101_79e0;
/// The audio singleton getter (`00453a70`) and `GetSoundHandleByNumericID`
/// `(this, handle, form id, flags)`, then the sound handle calls.
const AUDIO_GET: u32 = 0x0045_3a70;
const AUDIO_GET_SOUND_HANDLE: u32 = 0x00ad_73b0;
const SOUND_HANDLE_FOLLOW: u32 = 0x00ad_8f20;
const SOUND_HANDLE_SET_POSITION: u32 = 0x00ad_8b60;
const SOUND_HANDLE_SET_PRIORITY: u32 = 0x00ad_9030;
const SOUND_HANDLE_PLAY: u32 = 0x00ad_8830;
const SOUND_HANDLE_SIZE: u32 = 12;
const SOUND_FLAGS: u32 = 0x2102;
const SOUND_PRIORITY: u32 = 0xc0;
/// `4e75d0(this, out)`: the lookup `fn_00553bb0` makes on the object at
/// `+0x1c`.
const SOUND_LOOKUP: u32 = 0x004e_75d0;

/// Ray tests of `Pick`: against a node (`004b2800` with a bound, `004b2780`
/// with a child node), node position helpers, vector subtraction and length.
const RAY_TEST_BOUNDED: u32 = 0x004b_2800;
const RAY_TEST_NODE: u32 = 0x004b_2780;
const NODE_POSITION_A: u32 = 0x0050_0940;
const NODE_POSITION_B: u32 = 0x0096_8670;
const NODE_POSITION_C: u32 = 0x0041_3f40;
const VECTOR_SUBTRACT: u32 = 0x0043_9ef0;
const VECTOR_LENGTH: u32 = 0x0045_7990;
const PICK_WORLD_SHAPE: u32 = 0x00c6_6fd0;
const PICK_FIND_ENTRY: u32 = 0x004b_5a80;
const PICK_ENTRY_SOURCE: u32 = 0x0043_b4f0;
const PICK_ENTRY_TYPE: u32 = 0x0043_b4d0;
const PICK_FALLBACK_NODE: u32 = 0x00c6_6fb0;
const CELL_NODE_OF_CELL: u32 = 0x0054_5cb0;
/// The value `PICK_ENTRY_TYPE` gives for entries whose answer is the cell's
/// own node.
const PICK_OWN_NODE_TYPE: u32 = 0x11;

/// Helper: runs `body` between the construction and the destruction of an
/// allocation scope guard (the guard the game keeps on its stack).
fn with_scope_guard<R>(
    e: &mut Engine,
    tag: u32,
    line: u32,
    body: impl FnOnce(&mut Engine) -> R,
) -> R {
    e.with_stack(SCOPE_GUARD_SIZE, |e, guard| {
        e.call(
            SCOPE_GUARD_CONSTRUCT,
            &args![guard, tag, 1u32, SOURCE_FILE_NAME, line],
        );
        let result = body(e);
        e.call(SCOPE_GUARD_DESTRUCT, &args![guard]);
        result
    })
}

/// Helper: the item of a list node: the word at the address `006815c0`
/// returns.
fn node_item(e: &mut Engine, node: Ptr) -> Ptr {
    let address = e.call(LIST_NODE_ITEM_ADDRESS, &args![node]).u32();
    Ptr::new(e.mem.u32(address))
}

/// Helper: the `NiTMap` walk the map loops share. `visit` gets the key and
/// the value of every entry, in the order `006b7f20` gives them.
fn for_each_map_entry(e: &mut Engine, map: Ptr, mut visit: impl FnMut(&mut Engine, Ptr, Ptr)) {
    e.with_stack(12, |e, scratch| {
        let position = scratch.addr();
        let key = scratch.addr() + 4;
        let value = scratch.addr() + 8;
        let first = e.call(MAP_FIRST_POSITION, &args![map]).u32();
        e.mem.set_u32(position, first);
        while e.mem.u32(position) != 0 {
            e.mem.set_u32(key, 0);
            e.mem.set_u32(value, 0);
            e.call(MAP_NEXT, &args![map, position, key, value]);
            let (k, v) = (Ptr::new(e.mem.u32(key)), Ptr::new(e.mem.u32(value)));
            visit(e, k, v);
        }
    });
}

/// Helper: builds an update record `(time, a, b)` on the stack and runs
/// `body` with its address.
fn with_update_record<R>(
    e: &mut Engine,
    time: f32,
    a: u32,
    b: u32,
    body: impl FnOnce(&mut Engine, Ptr) -> R,
) -> R {
    e.with_stack(UPDATE_RECORD_SIZE, |e, record| {
        e.call(UPDATE_RECORD_CONSTRUCT, &args![record, time, a, b]);
        body(e, record)
    })
}

/// Helper: what the x87 `FISTP` with truncation leaves in the low word of a
/// `float` (the indefinite value `0x8000000000000000` for NaN and values out
/// of the 64-bit range).
fn truncated_low_word(value: f32) -> u32 {
    let value = value as f64;
    let limit = (1u64 << 63) as f64;
    if value.is_nan() || !(-limit..limit).contains(&value) {
        0
    } else {
        (value.trunc() as i64) as u32
    }
}

// Translated from 00552470 (decompiled, FalloutNV.exe 1.4.0.525)
/// Tests flag `0x100000` of the flag word at `+0x30` of an object (through
/// `00456630`). The map has no name for it.
pub fn fn_00552470(e: &mut Engine, this: Ptr) -> bool {
    e.call(TEST_FLAG_BITS, &args![this, 0x0010_0000u32]).bool()
}

// Translated from 00552490 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the byte at `+0x41` of the object.
pub fn fn_00552490(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x41)
}

// Translated from 005524b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the byte at `+0x8c` of the object.
pub fn fn_005524b0(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x8c)
}

// Translated from 005524d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the byte `00408d60` finds for the object at `01267c24`.
pub fn fn_005524d0(e: &mut Engine) -> u8 {
    let holder = e.call(BYTE_HOLDER_GET, &args![BYTE_HOLDER]).u32();
    e.mem.u8(holder)
}

// Translated from 005524f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Queues a node for the worker `fn_00552570` (cdecl, one word): under the
/// pending lock, wraps `node` in a smart pointer and appends it to the
/// pending list. The exception frame is not translated.
pub fn fn_005524f0(e: &mut Engine, node: u32) {
    e.call(LOCK_ENTER, &args![PENDING_LOCK, 0u32]);
    e.with_stack(4, |e, slot| {
        e.call(SLOT_FROM_RAW, &args![slot, node]);
        e.call(PENDING_LIST_ADD, &args![PENDING_LIST, slot]);
        e.call(SLOT_RELEASE, &args![slot]);
    });
    e.call(LOCK_LEAVE, &args![PENDING_LOCK]);
}

// Translated from 00552570 (decompiled, FalloutNV.exe 1.4.0.525)
/// The worker of the pending list: under the lock, takes each queued node
/// off the list, updates its morph controllers (`fn_005526f0`) and, when the
/// node still has a parent (virtual `+0x10`) and carries flag `0x400000`,
/// finds its reference (`FindReferenceFor3D`), tells the reference's parent
/// cell about it `(reference, 1, 0)` and clears the flag. The exception frame
/// is not translated.
pub fn fn_00552570(e: &mut Engine) {
    e.call(LOCK_ENTER, &args![PENDING_LOCK, 0u32]);
    while !e.call(PENDING_LIST_IS_EMPTY, &args![PENDING_LIST]).bool() {
        e.with_stack(4, |e, slot| {
            let head = e.call(LIST_NODE_ITEM_ADDRESS, &args![PENDING_LIST]).u32();
            e.call(SLOT_COPY, &args![slot, head]);
            e.call(PENDING_LIST_REMOVE_HEAD, &args![PENDING_LIST]);
            let node = e.call(SLOT_GET_POINTER, &args![slot]).ptr();
            fn_005526f0(e, node);
            if !e.call(SLOT_GET_POINTER, &args![slot]).ptr::<()>().is_null() {
                let node = e.call(SLOT_GET_POINTER, &args![slot]).ptr::<()>();
                if e.vcall(node.addr(), 0x10, &[]).u32() != 0 {
                    let node = e.call(SLOT_GET_POINTER, &args![slot]).ptr();
                    if fn_005526b0(e, node) {
                        let node = e.call(SLOT_GET_POINTER, &args![slot]).ptr::<()>();
                        let reference = e.call(FIND_REFERENCE_FOR_3D, &args![node]).ptr::<()>();
                        if !reference.is_null()
                            && e.call(REFERENCE_GET_PARENT_CELL, &args![reference]).u32() != 0
                        {
                            let cell = e.call(REFERENCE_GET_PARENT_CELL, &args![reference]).u32();
                            e.call(
                                CELL_UPDATE_FOR_REFERENCE,
                                &args![cell, reference, 1u32, 0u32],
                            );
                            let node = e.call(SLOT_GET_POINTER, &args![slot]).ptr();
                            fn_005526d0(e, node, false);
                        }
                    }
                }
            }
            e.call(SLOT_RELEASE, &args![slot]);
        });
    }
    e.call(LOCK_LEAVE, &args![PENDING_LOCK]);
}

// Translated from 005526b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Tests flag `0x400000` of the flag word at `+0x30` of an object.
pub fn fn_005526b0(e: &mut Engine, this: Ptr) -> bool {
    e.call(TEST_FLAG_BITS, &args![this, 0x0040_0000u32]).bool()
}

// Translated from 005526d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears flag `0x400000` of the flag word at `+0x30` of an object.
pub fn fn_005526d0(e: &mut Engine, this: Ptr, set: bool) {
    e.call(SET_FLAG_BITS, &args![this, set, 0x0040_0000u32]);
}

// Translated from 005526f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Walks a scene-graph node (cdecl, one word): a node with a child array
/// (virtual `+0x0c`) recurses into every child; a node without one asks for
/// its controller (virtual `+0x30`) and, if it has one, applies an update
/// record `(UPDATE_TIME, 1, 0)` to it.
pub fn fn_005526f0(e: &mut Engine, node: Ptr) {
    if node.is_null() {
        return;
    }
    let children = e.vcall(node.addr(), 0x0c, &[]).ptr::<()>();
    if !children.is_null() {
        let count = e.call(NODE_CHILD_COUNT, &args![children]).u32();
        for index in 0..count {
            let child = e.call(NODE_CHILD_AT, &args![children, index]).ptr();
            fn_005526f0(e, child);
        }
    } else {
        let controller = e.vcall(node.addr(), 0x30, &[]).ptr::<()>();
        if !controller.is_null() {
            let time: f32 = e.global(UPDATE_TIME);
            with_update_record(e, time, 1, 0, |e, record| {
                e.call(CONTROLLER_UPDATE, &args![controller, record]);
            });
        }
    }
}

// Translated from 00552790 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::UpdateMorphControllersOnly` (Xbox PDB), a walk of the
/// node it is given (cdecl, one word): updates the morph controller of the
/// node (`GetController` with the type at `011f3728`, then virtual `+0x94`
/// with an update record `(UPDATE_TIME, 1, 0)`) and recurses into the
/// children of the child array (virtual `+0x0c`).
pub fn tes_object_cell_update_morph_controllers_only(e: &mut Engine, node: Ptr) {
    if node.is_null() {
        return;
    }
    let controller = e
        .call(NODE_GET_CONTROLLER, &args![node, MORPH_CONTROLLER_TYPE])
        .ptr::<()>();
    if !controller.is_null() {
        let time: f32 = e.global(UPDATE_TIME);
        with_update_record(e, time, 1, 0, |e, record| {
            e.vcall(controller.addr(), 0x94, &args![record]);
        });
    }
    let children = e.vcall(node.addr(), 0x0c, &[]).ptr::<()>();
    if !children.is_null() {
        let count = e.call(NODE_CHILD_COUNT, &args![children]).u32();
        for index in 0..count {
            let child = e.call(NODE_CHILD_AT, &args![children, index]).ptr();
            tes_object_cell_update_morph_controllers_only(e, child);
        }
    }
}

// Translated from 00552840 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sends an update record `(0.0, 0, 0)` to the cell's node (`005497a0`,
/// virtual `+0xc0`) and then to every value of the loaded data's animated
/// reference map (`+0x0c` of `pLoadedData`) whose owner (`009611e0`) is not
/// the cell's own node.
pub fn fn_00552840(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    with_update_record(e, 0.0, 0, 0, |e, record| {
        let own = e.call(CELL_NODE_A, &args![this]).ptr::<()>();
        if !own.is_null() {
            e.vcall(own.addr(), 0xc0, &args![record]);
        }
        let loaded = e.get(this, TESObjectCELL::pLoadedData);
        if !loaded.is_null() {
            let map = loaded.byte_add(LoadedCellData::AnimatedRefMap.off);
            for_each_map_entry(e, map, |e, _key, value| {
                if !value.is_null() && e.call(ANIMATED_NODE_OWNER, &args![value]).ptr::<()>() != own
                {
                    e.vcall(value.addr(), 0xc0, &args![record]);
                }
            });
        }
    });
}

// Translated from 00552900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00484730(this, flag)` and, for a form without flag `0x4000`
/// that has flag `0x2` and is an exterior cell with a world space, calls the
/// world space's virtual `+0xc8` with `1`.
pub fn fn_00552900(e: &mut Engine, this: Ptr<TESObjectCELL>, flag: u8) {
    e.call(CELL_UPDATE_REFERENCE_FLAGS, &args![this, flag]);
    if !e.call(FORM_FLAG_4000, &args![this]).bool()
        && e.call(FORM_FLAG_2, &args![this]).bool()
        && !e.call(CELL_IS_INTERIOR, &args![this]).bool()
    {
        let world_space = e.call(CELL_GET_WORLD_SPACE, &args![this]).ptr::<()>();
        if !world_space.is_null() {
            e.vcall(world_space.addr(), WORLD_SPACE_SLOT, &args![1u32]);
        }
    }
}

// Translated from 00552970 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::AttachToWorld` (Xbox PDB): loads the cell's 3D, sets the
/// cell state to 5 and attaches the 3D to `parent` (virtual `+0xdc`). With
/// a Havok world: builds the exterior terrain objects for an exterior cell
/// and switches the visual debugger. Then attaches the lights, the add-on
/// nodes and the loaded flag, adds the draw-only nav meshes when the render
/// singleton asks for them, runs the attach follow-ups, registers the cell's
/// nodes with the portal graph's always-render array (only when it is empty)
/// and with the cell's option set, and clears `bCellDetached`.
pub fn tes_object_cell_attach_to_world(e: &mut Engine, this: Ptr<TESObjectCELL>, parent: Ptr) {
    with_scope_guard(e, 0x1a, 0x395d, |e| {
        let node = e.call(CELL_LOAD_3D, &args![this]).u32();
        e.call(CELL_SET_STATE, &args![this, 5u32]);
        e.vcall(parent.addr(), 0xdc, &args![node, 1u32]);
        let world = e.call(CELL_HAVOK_WORLD, &args![this]).ptr::<()>();
        if !world.is_null() {
            if !e.call(CELL_IS_INTERIOR, &args![this]).bool() {
                fn_005535f0(e, this);
            }
            let suppressed = e.call(VISUAL_DEBUGGER_SUPPRESSED, &[]).bool();
            e.call(
                WORLD_SET_VISUAL_DEBUGGER_ENABLED,
                &args![world, !suppressed],
            );
            let setting = e.call(VISUAL_DEBUGGER_SETTING, &[]).u8();
            e.call(WORLD_SET_VISUAL_DEBUGGER, &args![world, setting]);
        }
        e.call(CELL_SET_LIGHTS_ATTACHED, &args![this, 1u32]);
        e.call(CELL_SET_ADDON_NODES_ATTACHED, &args![this, 1u32]);
        e.call(CELL_SET_ATTACHED_FLAG, &args![this, 1u32]);
        let render = fn_00552ba0(e);
        if fn_00552bb0(e, Ptr::new(render)) {
            let render = fn_00552ba0(e);
            e.call(NAV_MESH_RENDER_ADD, &args![render, this, 1u32]);
        }
        e.call(CELL_AFTER_ATTACH_A, &args![this]);
        e.call(CELL_AFTER_ATTACH_B, &args![this]);
        let obstacles = e.call(OBSTACLE_MANAGER_GET, &[]).u32();
        e.call(OBSTACLE_MANAGER_RUN, &args![obstacles]);
        let options = e.call(CELL_OPTIONS, &args![this]).u32();
        let table = e.call(TABLE_ENTRY, &args![0u32]).u32();
        e.call(SCENE_NODE_ATTACH, &args![table, options]);
        if e.call(PORTAL_ALWAYS_RENDER_COUNT, &args![options]).u32() == 0 {
            let node_a = e.call(CELL_NODE_A, &args![this]).u32();
            e.call(PORTAL_ALWAYS_RENDER_ADD, &args![options, node_a]);
            let node_b = e.call(CELL_NODE_B, &args![this]).u32();
            e.call(PORTAL_ALWAYS_RENDER_ADD, &args![options, node_b]);
            let table = e.call(TABLE_ENTRY, &args![0u32]).u32();
            let child = e.call(SCENE_NODE_CHILD, &args![table, 5u32]).u32();
            e.call(PORTAL_ALWAYS_RENDER_ADD, &args![options, child]);
        }
        let child = e.call(CELL_CHILD_NODE, &args![this, 6u32]).ptr::<()>();
        let graph = e.call(CELL_PORTAL_GRAPH, &args![options]).u32();
        e.vcall(child.addr(), 0xdc, &args![graph, 1u32]);
        for node in [CELL_NODE_C, CELL_NODE_B] {
            let node = e.call(node, &args![this]).u32();
            e.call(PORTAL_ALWAYS_RENDER_SET, &args![options, node]);
        }
        let child = e.call(CELL_CHILD_NODE, &args![this, 2u32]).u32();
        e.call(PORTAL_ALWAYS_RENDER_SET, &args![options, child]);
        let node = e.call(CELL_NODE_D, &args![this]).u32();
        e.call(PORTAL_ALWAYS_RENDER_SET, &args![options, node]);
        e.set(this, TESObjectCELL::bCellDetached, false);
    });
}

// Translated from 00552ba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the render singleton pointer (`011d6e2c`).
pub fn fn_00552ba0(e: &mut Engine) -> u32 {
    e.global(RENDER_SINGLETON)
}

// Translated from 00552bb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the byte at `+0x210` of the render singleton: whether the
/// draw-only nav meshes are wanted.
pub fn fn_00552bb0(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 0x210) != 0
}

// Translated from 00552bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::Detach` (Xbox PDB): under the cell lock, for a cell whose
/// state is 5 or 6: sets the state to 4, cancels the model loader's
/// references for the cell, detaches the cell from the scene (interior and
/// exterior differ), detaches the Havok world when `detach_havok`, detaches
/// the 3D from the scene node, sets the state to 3 and `bCellDetached`,
/// removes the draw-only nav meshes, drops the saved changes when the form
/// has them and removes the cell's nodes from the portal graph's option set.
pub fn tes_object_cell_detach(e: &mut Engine, this: Ptr<TESObjectCELL>, detach_havok: bool) {
    e.call(CELL_LOCK_ENTER, &args![this]);
    let state = e.call(CELL_GET_STATE, &args![this]).u32();
    if state == 6 || state == 5 {
        e.call(CELL_SET_STATE, &args![this, 4u32]);
        e.call(CELL_BEFORE_DETACH, &args![this]);
        let model_loader: u32 = e.global(MODEL_LOADER_SINGLETON);
        e.call(MODEL_LOADER_CANCEL_FOR_CELL, &args![model_loader, this]);
        let scene: u32 = e.global(SCENE_SINGLETON);
        e.call(SCENE_REFRESH, &args![scene]);
        e.call(CELL_SET_LIGHTS_ATTACHED, &args![this, 0u32]);
        let scene: u32 = e.global(SCENE_SINGLETON);
        if e.call(CELL_IS_INTERIOR, &args![this]).bool() {
            e.call(SCENE_ATTACH_INTERIOR, &args![scene, this]);
        } else {
            e.call(SCENE_ATTACH_EXTERIOR, &args![scene, this]);
        }
        if detach_havok {
            fn_00553700(e, this);
        }
        e.call(CELL_SET_ADDON_NODES_ATTACHED, &args![this, 0u32]);
        let node = e.call(CELL_LOAD_3D, &args![this]).u32();
        let scene: u32 = e.global(SCENE_SINGLETON);
        let parent = e.call(WORD_AT_0C, &args![scene]).ptr::<()>();
        e.vcall(parent.addr(), 0xe8, &args![node]);
        e.call(CELL_DETACH_STEP_A, &args![this]);
        e.call(CELL_DETACH_STEP_B, &args![this]);
        e.call(CELL_SET_STATE, &args![this, 3u32]);
        e.set(this, TESObjectCELL::bCellDetached, true);
        let state_object: u32 = e.global(STATE_OBJECT_SINGLETON);
        if fn_00552da0(e, Ptr::new(state_object)) == 0 {
            e.call(CELL_SET_TERRAIN_UPDATE, &args![this, 1u32]);
        }
        e.call(CELL_CLEAR_LOADED_STATE, &args![this]);
        e.call(CELL_SET_ATTACHED_FLAG, &args![this, 0u32]);
        let render = fn_00552ba0(e);
        e.call(NAV_MESH_RENDER_REMOVE, &args![render, this]);
        let form_id = e.call(FORM_ID, &args![this]).u32();
        let loader_state: u32 = e.global(LOADER_STATE_SINGLETON);
        if e.call(FORM_HAS_CHANGES, &args![loader_state, form_id])
            .bool()
        {
            let loader: u32 = e.global(LOADER_SINGLETON);
            e.call(REMOVE_CHANGES, &args![loader, this]);
        }
        let scene: u32 = e.global(SCENE_SINGLETON);
        e.call(SCENE_REFRESH, &args![scene]);
        let options = e.call(CELL_OPTIONS, &args![this]).u32();
        let node_c = e.call(CELL_NODE_C, &args![this]).u32();
        e.call(PORTAL_ALWAYS_RENDER_REMOVE, &args![options, node_c]);
        let node_b = e.call(CELL_NODE_B, &args![this]).u32();
        e.call(PORTAL_ALWAYS_RENDER_REMOVE, &args![options, node_b]);
        let child = e.call(CELL_CHILD_NODE, &args![this, 2u32]).u32();
        e.call(PORTAL_ALWAYS_RENDER_REMOVE, &args![options, child]);
        let node_d = e.call(CELL_NODE_D, &args![this]).u32();
        e.call(PORTAL_ALWAYS_RENDER_REMOVE, &args![options, node_d]);
    }
    e.call(CELL_LOCK_LEAVE, &args![this]);
}

// Translated from 00552da0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the byte at `+0x26` of the object (`Detach` calls it on the object
/// at `011de134`).
pub fn fn_00552da0(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x26)
}

// Translated from 00552dc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Creates the Havok world of an interior cell that has none (the map has no
/// name for it): builds a world cinfo (gravity `(0, 0, g, 0)`, broad phase
/// size, scale `1.0`), then the world with its broad phase, filter, optional
/// water, a listener object and the `011ca088` singleton, positions it at the
/// cell's reference centre, enables the visual debugger setting and stores the
/// world in the cell's extra data. Then, when the cell has no collision world,
/// creates one (`00621d10`) and stores it in the extra data too. The exception
/// frames and the stack cookie are not translated.
pub fn fn_00552dc0(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    let world = e.call(CELL_HAVOK_WORLD, &args![this]).ptr::<()>();
    if e.call(CELL_IS_INTERIOR, &args![this]).bool() && world.is_null() {
        with_scope_guard(e, 0x10, 0x3a3f, |e| {
            e.with_stack(CINFO_SIZE, |e, cinfo| {
                e.call(CINFO_CONSTRUCT, &args![cinfo]);
                let gravity: f32 = e.global(GRAVITY_Z);
                e.with_stack(16, |e, vector| {
                    let vector = fn_005532a0(e, vector, 0.0, 0.0, gravity, 0.0);
                    e.call(CINFO_SET_GRAVITY, &args![cinfo.byte_add(0x10), vector]);
                });
                let size: f32 = e.global(BROAD_PHASE_SIZE);
                e.call(CINFO_SET_BROAD_PHASE_SIZE, &args![cinfo, size]);
                e.mem.set_f32(cinfo.addr() + 0x54, 1.0);
                let raw = fn_00553300(e, 0xa0);
                let world = if raw == 0 {
                    Ptr::NULL
                } else {
                    e.call(WORLD_CONSTRUCT, &args![raw, cinfo]).ptr()
                };
                fn_005533c0(e, world);
                let block = e.call(OPERATOR_NEW, &args![0x24u32]).u32();
                let broad_phase = if block == 0 {
                    0
                } else {
                    e.call(BROAD_PHASE_CONSTRUCT, &args![block]).u32()
                };
                fn_00553480(e, world, if broad_phase == 0 { 0 } else { broad_phase + 8 });
                let block = e.call(OPERATOR_NEW, &args![0x28u32]).u32();
                let filter = if block == 0 {
                    0
                } else {
                    e.call(FILTER_CONSTRUCT, &args![block]).u32()
                };
                fn_005534f0(e, world, filter);
                if e.call(HAS_WATER_TEST, &args![this]).bool() {
                    let block = e.call(OPERATOR_NEW, &args![0x40u32]).u32();
                    let water = if block == 0 {
                        0
                    } else {
                        let height = e.call(CELL_GET_WATER_HEIGHT, &args![this]).f32();
                        e.call(WATER_CONSTRUCT, &args![block, height]).u32()
                    };
                    e.call(WORLD_SET_GRAVITY_SCALE, &args![world, water]);
                }
                let setting = e.call(SCENE_HAVOK_SETTING, &[]).u32();
                fn_00553440(e, world, setting);
                let block = e.call(OPERATOR_NEW, &args![0x14u32]).u32();
                let listener = if block == 0 {
                    0
                } else {
                    e.call(LISTENER_OBJECT_CONSTRUCT, &args![block]).u32()
                };
                fn_005533f0(e, world, listener);
                if e.global::<u32>(LISTENER_SINGLETON) == 0 {
                    let block = e.call(OPERATOR_NEW, &args![8u32]).u32();
                    let singleton = if block == 0 {
                        0
                    } else {
                        e.call(LISTENER_CONSTRUCT, &args![block]).u32()
                    };
                    e.set_global(LISTENER_SINGLETON, singleton);
                }
                let held = e.call(HOLDER_GET_WORLD, &args![world]).u32();
                let singleton: u32 = e.global(LISTENER_SINGLETON);
                e.call(LISTENER_REGISTER, &args![singleton, held]);
                e.with_stack(12, |e, center| {
                    e.call(LIST_NODE_ITEM_ADDRESS, &args![center]);
                    e.call(CELL_REFERENCE_CENTER, &args![this, center]);
                    e.call(WORLD_SET_CENTER, &args![world, center]);
                });
                let setting = e.call(VISUAL_DEBUGGER_SETTING, &[]).u8();
                e.call(WORLD_SET_VISUAL_DEBUGGER, &args![world, setting]);
                e.call(
                    EXTRA_LIST_SET_HAVOK_WORLD,
                    &args![this.byte_add(EXTRA_LIST_OFFSET), world],
                );
                fn_005533c0(e, world);
                e.vcall(world.addr(), 0xc4, &[]);
                let suppressed = e.call(VISUAL_DEBUGGER_SUPPRESSED, &[]).bool();
                e.call(
                    WORLD_SET_VISUAL_DEBUGGER_ENABLED,
                    &args![world, !suppressed],
                );
                fn_00553520(e, cinfo);
            });
        });
    }
    if e.call(CELL_COLLISION_WORLD, &args![this]).u32() == 0 {
        with_scope_guard(e, 0x10, 0x3a91, |e| {
            let block = e.call(ALLOCATE_CDECL, &args![0x18u32]).u32();
            let collision = if block == 0 {
                0
            } else {
                e.call(COLLISION_WORLD_CONSTRUCT, &args![block]).u32()
            };
            e.call(
                EXTRA_LIST_SET_COLLISION_WORLD,
                &args![this.byte_add(EXTRA_LIST_OFFSET), collision],
            );
        });
    }
}

// Translated from 005532a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores four floats at `this` (a 16-byte vector constructor, SSE
/// `UNPCKLPS`): `(x, y, z, w)` in order. Returns `this`.
pub fn fn_005532a0(e: &mut Engine, this: Ptr, x: f32, y: f32, z: f32, w: f32) -> Ptr {
    for (index, value) in [x, y, z, w].into_iter().enumerate() {
        e.mem.set_f32(this.addr() + 4 * index as u32, value);
    }
    this
}

// Translated from 00553300 (decompiled, FalloutNV.exe 1.4.0.525)
/// Allocates `size` bytes aligned to 16 (cdecl): `fn_00553320(16, size)`.
pub fn fn_00553300(e: &mut Engine, size: u32) -> u32 {
    fn_00553320(e, 0x10, size)
}

// Translated from 00553320 (decompiled, FalloutNV.exe 1.4.0.525)
/// Aligned allocation (cdecl): allocates `align + size` bytes from the memory
/// manager, moves the pointer up to the next multiple of `align` (a whole
/// `align` when it already is one) and stores the distance moved in the byte
/// before the returned pointer, where `fn_00553380` finds it.
pub fn fn_00553320(e: &mut Engine, align: u8, size: u32) -> u32 {
    let manager = e.call(MEMORY_MANAGER_GET, &[]).u32();
    let total = (align as u32).wrapping_add(size);
    let block = e
        .call(MEMORY_MANAGER_ALLOCATE, &args![manager, total])
        .u32();
    let misalignment = (align.wrapping_sub(1) as u32 & block) as u8;
    let distance = align.wrapping_sub(misalignment);
    let aligned = (distance as u32).wrapping_add(block);
    e.mem.set_u8(aligned.wrapping_sub(1), distance);
    aligned
}

// Translated from 00553380 (decompiled, FalloutNV.exe 1.4.0.525)
/// Frees a block of [`fn_00553320`] (cdecl): moves the pointer back by the
/// distance stored before it and gives it to the memory manager. A null
/// pointer is ignored.
pub fn fn_00553380(e: &mut Engine, pointer: u32) {
    if pointer != 0 {
        let distance = e.mem.u8(pointer.wrapping_sub(1)) as u32;
        let manager = e.call(MEMORY_MANAGER_GET, &[]).u32();
        e.call(
            MEMORY_MANAGER_DEALLOCATE,
            &args![manager, pointer.wrapping_sub(distance)],
        );
    }
}

// Translated from 005533c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Havok world wrapper: takes the world's object (`00541c80`) and, if there is
/// one, calls `00483710` on it.
pub fn fn_005533c0(e: &mut Engine, this: Ptr) {
    let world = e.call(HOLDER_GET_WORLD, &args![this]).u32();
    if world != 0 {
        e.call(EMPTY_FUNCTION, &args![world]);
    }
}

// Translated from 005533f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Havok world wrapper: with the world's object (`00541c80`), if there is
/// one: takes the lock around `00c93820(world, value)`, and adds the address
/// of `value` to the holder's list at `+0x70` (`006299c0`).
pub fn fn_005533f0(e: &mut Engine, this: Ptr, value: u32) {
    let world = e.call(HOLDER_GET_WORLD, &args![this]).u32();
    if world != 0 {
        fn_005533c0(e, this);
        e.call(WORLD_SET_GRAVITY_VECTOR, &args![world, value]);
        fn_005533c0(e, this);
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), value);
            e.call(HOLDER_FIELD_70_ADD, &args![this.byte_add(0x70), slot]);
        });
    }
}

// Translated from 00553440 (decompiled, FalloutNV.exe 1.4.0.525)
/// Havok world wrapper: with the world's object (`00541c80`), if there is
/// one: `00c93a20(world, value)` between two calls of `fn_005533c0`.
pub fn fn_00553440(e: &mut Engine, this: Ptr, value: u32) {
    let world = e.call(HOLDER_GET_WORLD, &args![this]).u32();
    if world != 0 {
        fn_005533c0(e, this);
        e.call(WORLD_SET_FIELD_A, &args![world, value]);
        fn_005533c0(e, this);
    }
}

// Translated from 00553480 (decompiled, FalloutNV.exe 1.4.0.525)
/// Havok world wrapper: runs `fn_005533f0(this, value)`, then, with the world's
/// object, sets `00c93860(world, value + 0x14)` (0 for a null `value`)
/// between two calls of `fn_005533c0`; stores `value` at `+0x20`.
pub fn fn_00553480(e: &mut Engine, this: Ptr, value: u32) {
    fn_005533f0(e, this, value);
    let world = e.call(HOLDER_GET_WORLD, &args![this]).u32();
    if world != 0 {
        fn_005533c0(e, this);
        let inner = if value == 0 { 0 } else { value + 0x14 };
        e.call(WORLD_SET_FIELD_B, &args![world, inner]);
        fn_005533c0(e, this);
    }
    e.mem.set_u32(this.addr() + 0x20, value);
}

// Translated from 005534f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Havok world wrapper: runs `fn_005533f0(this, value)` and stores `value` at
/// `+0x28`.
pub fn fn_005534f0(e: &mut Engine, this: Ptr, value: u32) {
    fn_005533f0(e, this, value);
    e.mem.set_u32(this.addr() + 0x28, value);
}

// Translated from 00553520 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the destructor [`hkp_world_cinfo_destructor`] of the world cinfo.
pub fn fn_00553520(e: &mut Engine, this: Ptr) {
    hkp_world_cinfo_destructor(e, this);
}

// Translated from 00553540 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorldCinfo::~hkpWorldCinfo` (Xbox PDB): stores its vtable, runs
/// `00558e80` on the members at `+0x6c`, `+0x5c` and `+0x58`, then the base
/// destructor `00538e10`. The exception frame is not translated.
pub fn hkp_world_cinfo_destructor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), CINFO_VTABLE);
    for offset in [0x6c, 0x5c, 0x58] {
        e.call(CINFO_MEMBER_DESTRUCT, &args![this.byte_add(offset)]);
    }
    e.call(CINFO_BASE_DESTRUCT, &args![this]);
}

// Translated from 005535c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorldCinfo::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor, then frees the block (`00538eb0`) when bit 0 of `flags` is
/// set. Returns `this`.
pub fn hkp_world_cinfo_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    hkp_world_cinfo_destructor(e, this);
    if flags & 1 != 0 {
        e.call(CINFO_FREE, &args![this]);
    }
    this
}

// Translated from 005535f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// For an exterior cell with a Havok world and a collision world: builds a
/// transform (identity rotation from `011a9448`, translation
/// `(cellX * 4096 + 2048, cellY * 4096 + 2048, 0)`, scale `1.0`) and gives its
/// translation, with the flags at `011a9ea4`, to the collision world
/// (`00622590`). The exception frame is not translated.
pub fn fn_005535f0(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    let world = e.call(CELL_HAVOK_WORLD, &args![this]).u32();
    let cell_x = e.call(CELL_GET_DATA_X, &args![this]).i32();
    let cell_y = e.call(CELL_GET_DATA_Y, &args![this]).i32();
    let collision = e.call(CELL_COLLISION_WORLD, &args![this]).u32();
    if world != 0 && collision != 0 {
        with_scope_guard(e, 0x10, 0x3aa6, |e| {
            e.with_stack(0x34, |e, transform| {
                let at = transform.addr();
                e.call(TRANSFORM_CONSTRUCT, &args![transform]);
                e.mem.set_f32(at + 0x30, 1.0);
                for index in 0..9 {
                    let word = e.mem.u32(IDENTITY_ROTATION + 4 * index);
                    e.mem.set_u32(at + 4 * index, word);
                }
                let half: f64 = e.global(HALF_CELL_SIZE);
                e.mem
                    .set_f32(at + 0x24, ((cell_x << 12) as f64 + half) as f32);
                e.mem
                    .set_f32(at + 0x28, ((cell_y << 12) as f64 + half) as f32);
                e.mem.set_f32(at + 0x2c, 0.0);
                e.call(
                    WORLD_ADD_TERRAIN,
                    &args![collision, world, at + 0x24, TERRAIN_FLAGS],
                );
            });
        });
    }
}

// Translated from 00553700 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::DetachHavok` (Xbox PDB). Interior cell with a Havok
/// world: turns the visual debugger flag and setting off, and if the world
/// has a collision object, activates it with `1` and releases it with `0`.
/// Exterior cell: unloads the cell's collision world (`00621e80`), then under
/// the cell lock calls virtual `+0x1c0` of every reference; releases the Havok
/// world's collision object (`004893c0(.., 0)`) and, if the object at
/// `011de134` says so (`fn_00552da0`), calls `00c68a00(world, 0)`.
pub fn fn_00553700(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    if e.call(CELL_IS_INTERIOR, &args![this]).bool() {
        let world = e.call(CELL_HAVOK_WORLD, &args![this]).u32();
        if world != 0 {
            e.call(WORLD_SET_VISUAL_DEBUGGER_ENABLED, &args![world, false]);
            e.call(WORLD_SET_VISUAL_DEBUGGER, &args![world, 0u32]);
            if e.call(WORLD_COLLISION_OBJECT, &args![world]).u32() != 0 {
                let object = e.call(WORLD_COLLISION_OBJECT, &args![world]).u32();
                e.call(COLLISION_OBJECT_SET_ACTIVE, &args![object, 1u32]);
                e.call(COLLISION_OBJECT_RELEASE, &args![object, 0u32]);
            }
        }
        return;
    }
    let collision = e.call(CELL_COLLISION_WORLD, &args![this]).u32();
    if collision != 0 {
        e.call(COLLISION_WORLD_UNLOAD, &args![collision]);
    }
    e.call(CELL_LOCK_ENTER, &args![this]);
    let mut node = e.call(CELL_REFERENCES, &args![this]).ptr::<()>();
    while !node.is_null() {
        let reference = node_item(e, node);
        node = e.call(LIST_NODE_NEXT, &args![node]).ptr();
        if !reference.is_null() {
            e.vcall(reference.addr(), 0x1c0, &[]);
        }
    }
    let world = e.call(CELL_HAVOK_WORLD, &args![this]).u32();
    if world != 0 {
        let object = e.call(WORLD_COLLISION_OBJECT, &args![world]).u32();
        e.call(COLLISION_OBJECT_RELEASE, &args![object, 0u32]);
    }
    let state_object: u32 = e.global(STATE_OBJECT_SINGLETON);
    if fn_00552da0(e, Ptr::new(state_object)) != 0 {
        e.call(WORLD_UPDATE_RENDERER, &args![world, 0u32]);
    }
    e.call(CELL_LOCK_LEAVE, &args![this]);
}

// Translated from 00553820 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the cell is loaded (`00450ff0`) and has loaded data: for every
/// reference of the animated reference map (`+0x0c` of `pLoadedData`) that
/// answers false to virtual `+0x100`, has a base object (`007af430`) whose
/// form type (`00401170`) is not `0x25`, calls virtual `+0x1e0`.
pub fn fn_00553820(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    if !e.call(CELL_IS_LOADED, &args![this]).bool() {
        return;
    }
    let loaded = e.get(this, TESObjectCELL::pLoadedData);
    if loaded.is_null() {
        return;
    }
    let map = loaded.byte_add(LoadedCellData::AnimatedRefMap.off);
    for_each_map_entry(e, map, |e, reference, _node| {
        if !e.vcall(reference.addr(), 0x100, &[]).bool() {
            let base = e.call(REFERENCE_BASE_OBJECT, &args![reference]).u32();
            if base != 0 {
                let base = e.call(REFERENCE_BASE_OBJECT, &args![reference]).u32();
                if e.call(FORM_TYPE, &args![base]).u32() != 0x25 {
                    e.vcall(reference.addr(), 0x1e0, &[]);
                }
            }
        }
    });
}

// Translated from 005538e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::UpdateRefSounds` (Xbox PDB): under the cell lock, for a
/// loaded cell, walks its references until the first empty list node. A
/// reference that is not disabled (`00440da0`) or destroyed (`00477ba0`),
/// passes `fn_00553bb0`, and (if virtual `+0x21c` is true) is not excluded by
/// virtual `+0x22c(0)`, whose base form (`00441110`) passes the chance test
/// (`fn_00553b60 / divisor`) and has a sound name, and whose distance
/// (truncated to its low word) is below `fn_00553b90 * 100` starts the
/// form's sound: a sound handle for form id and flags `0x2102`, following
/// the reference's 3D (virtual `+0x1d0`), at the position of virtual `+0x1f4`,
/// priority `0xc0`, played.
pub fn tes_object_cell_update_ref_sounds(e: &mut Engine, this: Ptr<TESObjectCELL>) {
    e.call(CELL_LOCK_ENTER, &args![this]);
    if e.call(CELL_IS_LOADED, &args![this]).bool() {
        let mut node = e.call(CELL_REFERENCES, &args![this]).ptr::<()>();
        while !node.is_null() {
            let address = e.call(LIST_NODE_ITEM_ADDRESS, &args![node]).u32();
            if e.mem.u32(address) == 0 {
                break;
            }
            let reference = node_item(e, node);
            if !reference.is_null() {
                update_reference_sound(e, reference);
            }
            node = e.call(LIST_NODE_NEXT, &args![node]).ptr();
        }
    }
    e.call(CELL_LOCK_LEAVE, &args![this]);
}

/// The body of the loop of [`tes_object_cell_update_ref_sounds`] for one
/// reference.
fn update_reference_sound(e: &mut Engine, reference: Ptr) {
    if e.call(REFERENCE_IS_DISABLED, &args![reference]).bool()
        || e.call(REFERENCE_IS_DESTROYED, &args![reference]).bool()
        || !fn_00553bb0(e, reference)
    {
        return;
    }
    if e.vcall(reference.addr(), 0x21c, &[]).bool()
        && e.vcall(reference.addr(), 0x22c, &args![0u32]).bool()
    {
        return;
    }
    let form = e.call(REFERENCE_BASE_FORM, &args![reference]).ptr::<()>();
    if form.is_null() {
        return;
    }
    let chance = fn_00553b60(e, form) as i32;
    let divisor: f64 = e.global(SOUND_CHANCE_DIVISOR);
    let chance = (chance as f64 / divisor) as f32;
    if !e.call(SOUND_CHANCE_TEST, &args![chance]).bool() {
        return;
    }
    let name = e.call(FORM_SOUND_NAME, &args![form]).u32();
    if e.mem.i8(name) == 0 {
        return;
    }
    let range = fn_00553b90(e, form) as u32 * 100;
    let source: u32 = e.global(DISTANCE_SOURCE_SINGLETON);
    let distance = e
        .call(REFERENCE_DISTANCE, &args![source, reference, 0u32, 0u32])
        .f32();
    if range <= truncated_low_word(distance) {
        return;
    }
    e.with_stack(SOUND_HANDLE_SIZE, |e, handle| {
        let audio = e.call(AUDIO_GET, &[]).u32();
        let form_id = e.call(FORM_ID, &args![form]).u32();
        e.call(
            AUDIO_GET_SOUND_HANDLE,
            &args![audio, handle, form_id, SOUND_FLAGS],
        );
        if e.vcall(reference.addr(), 0x1d0, &[]).u32() != 0 {
            let follow = e.vcall(reference.addr(), 0x1d0, &[]).u32();
            e.call(SOUND_HANDLE_FOLLOW, &args![handle, follow]);
        }
        let position = e.vcall(reference.addr(), 0x1f4, &[]).u32();
        let (x, y, z) = (
            e.mem.f32(position),
            e.mem.f32(position + 4),
            e.mem.f32(position + 8),
        );
        e.call(SOUND_HANDLE_SET_POSITION, &args![handle, x, y, z]);
        e.call(SOUND_HANDLE_SET_PRIORITY, &args![handle, SOUND_PRIORITY]);
        e.call(SOUND_HANDLE_PLAY, &args![handle, 0u32]);
        e.call(EMPTY_FUNCTION, &args![handle]);
    });
}

// Translated from 00553b60 (decompiled, FalloutNV.exe 1.4.0.525)
/// The sound chance of a form: the signed byte at `+0x68` when bit 1 of the
/// word at `+0x48` is set, else 0.
pub fn fn_00553b60(e: &mut Engine, this: Ptr) -> i8 {
    if e.mem.u32(this.addr() + 0x48) & 2 != 0 {
        e.mem.i8(this.addr() + 0x68)
    } else {
        0
    }
}

// Translated from 00553b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the byte at `+0x45` of the form (the sound range in hundreds).
pub fn fn_00553b90(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x45)
}

// Translated from 00553bb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the object at `+0x1c` exists and its lookup (`004e75d0`) gives a
/// record whose word at `+4` has bit 1 set.
pub fn fn_00553bb0(e: &mut Engine, this: Ptr) -> bool {
    let holder = e.mem.u32(this.addr() + 0x1c);
    if holder == 0 {
        return false;
    }
    e.with_stack(0x20, |e, out| {
        let record = e.call(SOUND_LOOKUP, &args![holder, out]).u32();
        e.mem.u32(record + 4) & 2 != 0
    })
}

// Translated from 00553bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Ray test against the cell's reference nodes (`00545710`): `true` when a
/// reference with a 3D (virtual `+0x1d0`) passes the bounded test
/// `004b2800(object, a, b, limit, 3D)` and the distance from the node's
/// position (`00500940`/`00968670`/`00413f40`, minus `b`'s vector with
/// `00439ef0`) is below `limit`. Stops at an empty list node.
pub fn fn_00553bf0(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    object: Ptr,
    a: u32,
    b: u32,
    limit: f32,
) -> bool {
    if object.is_null() {
        return false;
    }
    let mut node = e.call(CELL_REFERENCE_NODES, &args![this]).ptr::<()>();
    while !node.is_null() {
        if e.call(LIST_NODE_IS_END, &args![node]).bool() {
            return false;
        }
        let reference = node_item(e, node);
        if e.vcall(reference.addr(), 0x1d0, &[]).u32() != 0 {
            let node_3d = e.vcall(reference.addr(), 0x1d0, &[]).u32();
            if e.call(RAY_TEST_BOUNDED, &args![object, a, b, limit, node_3d])
                .bool()
            {
                let position = e.call(NODE_POSITION_A, &args![object]).u32();
                let position = e.call(NODE_POSITION_B, &args![position, 0u32]).u32();
                let position = e.call(NODE_POSITION_C, &args![position]).u32();
                let (x, y, z) = (
                    e.mem.u32(position),
                    e.mem.u32(position + 4),
                    e.mem.u32(position + 8),
                );
                let length = e.with_stack(0x20, |e, scratch| {
                    e.mem.set_u32(scratch.addr() + 0x10, x);
                    e.mem.set_u32(scratch.addr() + 0x14, y);
                    e.mem.set_u32(scratch.addr() + 0x18, z);
                    let difference = e
                        .call(VECTOR_SUBTRACT, &args![scratch.addr() + 0x10, scratch, a])
                        .u32();
                    e.call(VECTOR_LENGTH, &args![difference]).f32()
                });
                if limit > length {
                    return true;
                }
            }
        }
        node = e.call(LIST_NODE_NEXT, &args![node]).ptr();
    }
    false
}

// Translated from 00553d00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Ray test against the cell's child nodes selected by `mask` (cdecl-like
/// with five words): for each set bit (1, 2, 4, 8, 16, and 8 or 16 again for
/// the last test) runs `004b2780(object, a, b, limit, child, 0)` on the
/// child node `00456fc0` gives for index 0, 1, 2, 3, 4 and 7. Returns whether
/// any test succeeded; false for a null `object` or an empty mask.
pub fn fn_00553d00(
    e: &mut Engine,
    this: Ptr<TESObjectCELL>,
    object: Ptr,
    a: u32,
    b: u32,
    limit: f32,
    mask: i8,
) -> bool {
    let mut hit = false;
    if object.is_null() || mask == 0 {
        return false;
    }
    let low = mask & 0x08 != 0;
    let high = mask & 0x10 != 0;
    let tests = [
        (mask & 0x01 != 0, 0u32),
        (mask & 0x02 != 0, 1),
        (mask & 0x04 != 0, 2),
        (low, 3),
        (high, 4),
        (low || high, 7),
    ];
    for (enabled, index) in tests {
        if enabled {
            let child = e.call(CELL_CHILD_NODE, &args![this, index]).u32();
            if e.call(RAY_TEST_NODE, &args![object, a, b, limit, child, 0u32])
                .bool()
            {
                hit = true;
            }
        }
    }
    hit
}

// Translated from 00553ee0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectCELL::Pick` (Xbox PDB): with a Havok world that accepts the
/// query (virtual `+0xc8(query)`) finds the pick entry of the query's world
/// shape (`00c66fd0`, `004b5a80`) and, when its type (`0043b4d0` of the
/// source `0043b4f0` gives) is `0x11`, returns the cell's own node
/// (`00545cb0`); otherwise the query's fallback node (`00c66fb0`). Without a
/// world, or when the world refuses, returns 0.
pub fn tes_object_cell_pick(e: &mut Engine, this: Ptr<TESObjectCELL>, query: Ptr) -> u32 {
    let world = e.call(CELL_HAVOK_WORLD, &args![this]).ptr::<()>();
    if world.is_null() || !e.vcall(world.addr(), 0xc8, &args![query]).bool() {
        return 0;
    }
    let shape = e.call(PICK_WORLD_SHAPE, &args![query]).u32();
    let entry = e.call(PICK_FIND_ENTRY, &args![shape]).u32();
    if entry != 0 {
        let source = e.with_stack(4, |e, out| {
            e.call(PICK_ENTRY_SOURCE, &args![entry, out]).u32()
        });
        if e.call(PICK_ENTRY_TYPE, &args![source]).u32() == PICK_OWN_NODE_TYPE {
            return e.call(CELL_NODE_OF_CELL, &args![this]).u32();
        }
    }
    e.call(PICK_FALLBACK_NODE, &args![query]).u32()
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00552470, fn_00552470(Ptr) -> bool),
        entry!(0x00552490, fn_00552490(Ptr) -> u8),
        entry!(0x005524b0, fn_005524b0(Ptr) -> u8),
        entry!(0x005524d0, fn_005524d0() -> u8),
        entry!(0x005524f0, fn_005524f0(u32)),
        entry!(0x00552570, fn_00552570()),
        entry!(0x005526b0, fn_005526b0(Ptr) -> bool),
        entry!(0x005526d0, fn_005526d0(Ptr, bool)),
        entry!(0x005526f0, fn_005526f0(Ptr)),
        entry!(
            0x00552790,
            tes_object_cell_update_morph_controllers_only(Ptr)
        ),
        entry!(0x00552840, fn_00552840(Ptr<TESObjectCELL>)),
        entry!(0x00552900, fn_00552900(Ptr<TESObjectCELL>, u8)),
        entry!(
            0x00552970,
            tes_object_cell_attach_to_world(Ptr<TESObjectCELL>, Ptr)
        ),
        entry!(0x00552ba0, fn_00552ba0() -> u32),
        entry!(0x00552bb0, fn_00552bb0(Ptr) -> bool),
        entry!(0x00552bd0, tes_object_cell_detach(Ptr<TESObjectCELL>, bool)),
        entry!(0x00552da0, fn_00552da0(Ptr) -> u8),
        entry!(0x00552dc0, fn_00552dc0(Ptr<TESObjectCELL>)),
        entry!(0x005532a0, fn_005532a0(Ptr, f32, f32, f32, f32) -> Ptr),
        entry!(0x00553300, fn_00553300(u32) -> u32),
        entry!(0x00553320, fn_00553320(u8, u32) -> u32),
        entry!(0x00553380, fn_00553380(u32)),
        entry!(0x005533c0, fn_005533c0(Ptr)),
        entry!(0x005533f0, fn_005533f0(Ptr, u32)),
        entry!(0x00553440, fn_00553440(Ptr, u32)),
        entry!(0x00553480, fn_00553480(Ptr, u32)),
        entry!(0x005534f0, fn_005534f0(Ptr, u32)),
        entry!(0x00553520, fn_00553520(Ptr)),
        entry!(0x00553540, hkp_world_cinfo_destructor(Ptr)),
        entry!(
            0x005535c0,
            hkp_world_cinfo_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(0x005535f0, fn_005535f0(Ptr<TESObjectCELL>)),
        entry!(0x00553700, fn_00553700(Ptr<TESObjectCELL>)),
        entry!(0x00553820, fn_00553820(Ptr<TESObjectCELL>)),
        entry!(
            0x005538e0,
            tes_object_cell_update_ref_sounds(Ptr<TESObjectCELL>)
        ),
        entry!(0x00553b60, fn_00553b60(Ptr) -> i8),
        entry!(0x00553b90, fn_00553b90(Ptr) -> u8),
        entry!(0x00553bb0, fn_00553bb0(Ptr) -> bool),
        entry!(
            0x00553bf0,
            fn_00553bf0(Ptr<TESObjectCELL>, Ptr, u32, u32, f32) -> bool
        ),
        entry!(
            0x00553d00,
            fn_00553d00(Ptr<TESObjectCELL>, Ptr, u32, u32, f32, i8) -> bool
        ),
        entry!(
            0x00553ee0,
            tes_object_cell_pick(Ptr<TESObjectCELL>, Ptr) -> u32
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// The vtable of every test object: slot `n` leads to the address
    /// `fake(n)`, where a test registers the double it needs.
    const TEST_VTABLE: u32 = 0x7f00_0000;

    fn fake(slot: u32) -> u32 {
        0x7e00_0000 + slot
    }

    /// An engine with the exe's data pages mapped (zero filled) and the test
    /// vtable.
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.map(0x0100_0000, 0x0030_0000);
        e.map(TEST_VTABLE, 0x1000);
        for slot in (0..0x300).step_by(4) {
            e.mem.set_u32(TEST_VTABLE + slot, fake(slot));
        }
        e
    }

    /// A zeroed object of `size` bytes whose vtable is the test vtable.
    fn object(e: &mut Engine, size: u32) -> u32 {
        let at = e.mem.alloc(size);
        e.mem.set_u32(at, TEST_VTABLE);
        at
    }

    /// Registers do-nothing doubles returning 0.
    fn quiet(e: &mut Engine, addresses: &[u32]) {
        for address in addresses {
            e.register(*address, |_, _| Ret::default());
        }
    }

    /// A double that returns `value`.
    fn returns(e: &mut Engine, address: u32, value: u32) {
        e.register_double(address, move |_, _| value.into_ret());
    }

    fn start_log(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    /// The calls made after the one under test, as `(address, words)`.
    fn calls(e: &Engine) -> Vec<(u32, Vec<u32>)> {
        e.call_log.clone().unwrap().into_iter().skip(1).collect()
    }

    fn addresses(log: &[(u32, Vec<u32>)]) -> Vec<u32> {
        log.iter().map(|(address, _)| *address).collect()
    }

    fn calls_to(log: &[(u32, Vec<u32>)], address: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// Every function of this file is registered under its address.
    #[test]
    fn functions_are_registered() {
        let table = funcs();
        assert_eq!(table.len(), 40);
        let mut seen: Vec<u32> = table.iter().map(|(a, _)| *a).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 40);
        assert!(seen.iter().all(|a| (0x0055_2470..0x0055_8e20).contains(a)));
    }

    #[test]
    fn flag_100000_is_tested_through_the_flag_word_helper() {
        let mut e = engine();
        returns(&mut e, TEST_FLAG_BITS, 1);
        start_log(&mut e);
        assert!(e.call(0x0055_2470, &args![0x1234u32]).bool());
        assert_eq!(calls(&e), vec![(TEST_FLAG_BITS, vec![0x1234, 0x0010_0000])]);
    }

    #[test]
    fn byte_accessors_read_their_offsets() {
        let mut e = engine();
        let at = e.mem.alloc(0x100);
        e.mem.set_u8(at + 0x41, 7);
        e.mem.set_u8(at + 0x8c, 9);
        assert_eq!(e.call(0x0055_2490, &args![at]).u8(), 7);
        assert_eq!(e.call(0x0055_24b0, &args![at]).u8(), 9);
        assert_eq!(e.call(0x0055_2da0, &args![at - 0x41 + 0x26]).u8(), 0);
        e.mem.set_u8(at + 0x26, 3);
        assert_eq!(e.call(0x0055_2da0, &args![at]).u8(), 3);
        e.mem.set_u8(at + 0x210, 1);
        assert!(e.call(0x0055_2bb0, &args![at]).bool());
        assert!(!e.call(0x0055_2bb0, &args![at + 8]).bool());
    }

    #[test]
    fn byte_holder_is_read_through_its_getter() {
        let mut e = engine();
        let byte = e.mem.alloc(4);
        e.mem.set_u8(byte, 0x5a);
        returns(&mut e, BYTE_HOLDER_GET, byte);
        start_log(&mut e);
        assert_eq!(e.call(0x0055_24d0, &[]).u8(), 0x5a);
        assert_eq!(calls(&e), vec![(BYTE_HOLDER_GET, vec![BYTE_HOLDER])]);
    }

    #[test]
    fn queueing_a_node_wraps_it_in_a_slot_under_the_lock() {
        let mut e = engine();
        quiet(
            &mut e,
            &[LOCK_ENTER, LOCK_LEAVE, PENDING_LIST_ADD, SLOT_RELEASE],
        );
        e.register(SLOT_FROM_RAW, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        let slot_seen = Rc::new(RefCell::new(0));
        let seen = slot_seen.clone();
        e.register_double(PENDING_LIST_ADD, move |e, a| {
            *seen.borrow_mut() = e.mem.u32(a[1]);
            Ret::default()
        });
        start_log(&mut e);
        e.call(0x0055_24f0, &args![0x4455u32]);
        let log = calls(&e);
        assert_eq!(
            addresses(&log),
            vec![
                LOCK_ENTER,
                SLOT_FROM_RAW,
                PENDING_LIST_ADD,
                SLOT_RELEASE,
                LOCK_LEAVE
            ]
        );
        assert_eq!(log[0].1, vec![PENDING_LOCK, 0]);
        assert_eq!(calls_to(&log, PENDING_LIST_ADD)[0][0], PENDING_LIST);
        assert_eq!(*slot_seen.borrow(), 0x4455);
        assert_eq!(log[4].1, vec![PENDING_LOCK]);
    }

    /// Two queued nodes: the first has a parent, the flag, a reference and a
    /// parent cell; the second has no parent.
    #[test]
    fn worker_updates_the_cell_of_a_flagged_node_and_clears_the_flag() {
        let mut e = engine();
        let complete = object(&mut e, 0x100);
        let orphan = object(&mut e, 0x100);
        e.mem.set_u32(complete + 0x30, 0x0040_0000);
        e.mem.set_u32(complete + 0x80, 1);
        e.mem.set_u32(orphan + 0x30, 0x0040_0000);
        quiet(
            &mut e,
            &[
                LOCK_ENTER,
                LOCK_LEAVE,
                PENDING_LIST_REMOVE_HEAD,
                SLOT_RELEASE,
                CELL_UPDATE_FOR_REFERENCE,
                SET_FLAG_BITS,
            ],
        );
        // The children and controller virtuals answer nothing; the parent
        // virtual reads a word of the node.
        quiet(&mut e, &[fake(0x0c), fake(0x30)]);
        e.register(fake(0x10), |e, a| e.mem.u32(a[0] + 0x80).into_ret());
        let queue = Rc::new(RefCell::new(vec![orphan, complete]));
        let remaining = queue.clone();
        e.register_double(PENDING_LIST_IS_EMPTY, move |_, _| {
            u32::from(remaining.borrow().is_empty()).into_ret()
        });
        returns(&mut e, LIST_NODE_ITEM_ADDRESS, 0x1000);
        let source = queue.clone();
        e.register_double(SLOT_COPY, move |e, a| {
            let node = source.borrow_mut().pop().unwrap();
            e.mem.set_u32(a[0], node);
            Ret::default()
        });
        e.register(SLOT_GET_POINTER, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(TEST_FLAG_BITS, |e, a| {
            u32::from(e.mem.u32(a[0] + 0x30) & a[1] != 0).into_ret()
        });
        returns(&mut e, FIND_REFERENCE_FOR_3D, 0x9100);
        returns(&mut e, REFERENCE_GET_PARENT_CELL, 0x9200);
        start_log(&mut e);
        e.call(0x0055_2570, &[]);
        let log = calls(&e);
        assert_eq!(calls_to(&log, FIND_REFERENCE_FOR_3D), vec![vec![complete]]);
        assert_eq!(
            calls_to(&log, CELL_UPDATE_FOR_REFERENCE),
            vec![vec![0x9200, 0x9100, 1, 0]]
        );
        assert_eq!(
            calls_to(&log, SET_FLAG_BITS),
            vec![vec![complete, 0, 0x0040_0000]]
        );
        assert_eq!(calls_to(&log, SLOT_RELEASE).len(), 2);
        assert_eq!(calls_to(&log, LOCK_LEAVE).len(), 1);
    }

    #[test]
    fn flag_400000_test_and_set() {
        let mut e = engine();
        returns(&mut e, TEST_FLAG_BITS, 1);
        quiet(&mut e, &[SET_FLAG_BITS]);
        start_log(&mut e);
        assert!(e.call(0x0055_26b0, &args![0x77u32]).bool());
        assert_eq!(calls(&e), vec![(TEST_FLAG_BITS, vec![0x77, 0x0040_0000])]);
        start_log(&mut e);
        e.call(0x0055_26d0, &args![0x77u32, 1u32]);
        assert_eq!(calls(&e), vec![(SET_FLAG_BITS, vec![0x77, 1, 0x0040_0000])]);
        start_log(&mut e);
        e.call(0x0055_26d0, &args![0x77u32, 0u32]);
        assert_eq!(calls(&e), vec![(SET_FLAG_BITS, vec![0x77, 0, 0x0040_0000])]);
    }

    /// Doubles for the update record and the child array (`0x7000_0000 + n`
    /// is child number `n`).
    fn morph_doubles(e: &mut Engine) {
        e.register(UPDATE_RECORD_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        quiet(e, &[CONTROLLER_UPDATE, fake(0x94)]);
        e.register(NODE_CHILD_COUNT, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(NODE_CHILD_AT, |e, a| {
            e.mem.u32(a[0] + 4 + 4 * a[1]).into_ret()
        });
    }

    #[test]
    fn controller_walk_updates_leaf_controllers_only() {
        let mut e = engine();
        morph_doubles(&mut e);
        e.set_global(UPDATE_TIME, 0.25f32);
        // The child array of the root: two children; the first has a
        // controller, the second is a node without controller; the root has
        // no controller of its own.
        let array = e.mem.alloc(16);
        let root = object(&mut e, 0x40);
        let with_controller = object(&mut e, 0x40);
        let without = object(&mut e, 0x40);
        let controller = object(&mut e, 0x40);
        e.mem.set_u32(array, 2);
        e.mem.set_u32(array + 4, with_controller);
        e.mem.set_u32(array + 8, without);
        e.mem.set_u32(root + 0x38, array);
        e.mem.set_u32(with_controller + 0x3c, controller);
        e.register(fake(0x0c), |e, a| e.mem.u32(a[0] + 0x38).into_ret());
        e.register(fake(0x30), |e, a| e.mem.u32(a[0] + 0x3c).into_ret());
        start_log(&mut e);
        e.call(0x0055_26f0, &args![root]);
        let log = calls(&e);
        let updates = calls_to(&log, CONTROLLER_UPDATE);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0][0], controller);
        let record = calls_to(&log, UPDATE_RECORD_CONSTRUCT);
        assert_eq!(record.len(), 1);
        assert_eq!(&record[0][1..], &[0.25f32.to_bits(), 1, 0]);
        // A null node does nothing.
        start_log(&mut e);
        e.call(0x0055_26f0, &args![0u32]);
        assert!(calls(&e).is_empty());
    }

    #[test]
    fn morph_walk_updates_every_node_with_a_controller() {
        let mut e = engine();
        morph_doubles(&mut e);
        e.set_global(UPDATE_TIME, 2.0f32);
        let array = e.mem.alloc(16);
        let root = object(&mut e, 0x40);
        let child = object(&mut e, 0x40);
        let controller = object(&mut e, 0x40);
        e.mem.set_u32(array, 1);
        e.mem.set_u32(array + 4, child);
        e.mem.set_u32(root + 0x38, array);
        e.mem.set_u32(child + 0x3c, controller);
        e.register(fake(0x0c), |e, a| e.mem.u32(a[0] + 0x38).into_ret());
        // GetController(node, type): the root has none, the child has one.
        e.register(NODE_GET_CONTROLLER, |e, a| {
            e.mem.u32(a[0] + 0x3c).into_ret()
        });
        start_log(&mut e);
        e.call(0x0055_2790, &args![root]);
        let log = calls(&e);
        assert_eq!(
            calls_to(&log, NODE_GET_CONTROLLER),
            vec![
                vec![root, MORPH_CONTROLLER_TYPE],
                vec![child, MORPH_CONTROLLER_TYPE]
            ]
        );
        // The controller's virtual +0x94 got the record.
        assert_eq!(calls_to(&log, fake(0x94)).len(), 1);
        assert_eq!(calls_to(&log, fake(0x94))[0][0], controller);
        assert_eq!(
            calls_to(&log, UPDATE_RECORD_CONSTRUCT)[0][1],
            2.0f32.to_bits()
        );
        start_log(&mut e);
        e.call(0x0055_2790, &args![0u32]);
        assert!(calls(&e).is_empty());
    }

    #[test]
    fn cell_update_sends_a_record_to_the_cell_node_and_the_animated_nodes() {
        let mut e = engine();
        e.register(UPDATE_RECORD_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], 0xfeed);
            Ret::default()
        });
        quiet(&mut e, &[fake(0xc0)]);
        let cell = e.new_object::<TESObjectCELL>();
        let own = object(&mut e, 0x40);
        let other = object(&mut e, 0x40);
        let loaded = e.new_object::<LoadedCellData>();
        e.set(cell, TESObjectCELL::pLoadedData, loaded.cast());
        returns(&mut e, CELL_NODE_A, own);
        // The map holds three entries: a null value, the cell's own node and
        // another node.
        returns(&mut e, MAP_FIRST_POSITION, 1);
        let entries = Rc::new(RefCell::new(vec![other, own, 0]));
        let queue = entries.clone();
        e.register_double(MAP_NEXT, move |e, a| {
            let value = queue.borrow_mut().pop().unwrap();
            e.mem.set_u32(a[2], 0x1111);
            e.mem.set_u32(a[3], value);
            if queue.borrow().is_empty() {
                e.mem.set_u32(a[1], 0);
            }
            Ret::default()
        });
        e.register(ANIMATED_NODE_OWNER, |_, a| a[0].into_ret());
        // For the second entry the owner is made to differ from `own`.
        e.register_double(ANIMATED_NODE_OWNER, move |_, a| {
            if a[0] == own { own } else { 0x1 }.into_ret()
        });
        start_log(&mut e);
        e.call(0x0055_2840, &args![cell]);
        let log = calls(&e);
        let sent = calls_to(&log, fake(0xc0));
        // The cell node and `other` (whose owner differs) got the record.
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0][0], own);
        assert_eq!(sent[1][0], other);
        let map = loaded.addr() + LoadedCellData::AnimatedRefMap.off;
        assert_eq!(calls_to(&log, MAP_FIRST_POSITION), vec![vec![map]]);
        let record = calls_to(&log, UPDATE_RECORD_CONSTRUCT);
        assert_eq!(&record[0][1..], &[0, 0, 0]);
    }

    #[test]
    fn cell_update_without_loaded_data_only_updates_the_cell_node() {
        let mut e = engine();
        quiet(&mut e, &[UPDATE_RECORD_CONSTRUCT, fake(0xc0)]);
        let cell = e.new_object::<TESObjectCELL>();
        returns(&mut e, CELL_NODE_A, 0);
        start_log(&mut e);
        e.call(0x0055_2840, &args![cell]);
        assert_eq!(
            addresses(&calls(&e)),
            vec![UPDATE_RECORD_CONSTRUCT, CELL_NODE_A]
        );
    }

    #[test]
    fn reference_flags_are_updated_and_exterior_world_space_notified() {
        let mut e = engine();
        quiet(&mut e, &[CELL_UPDATE_REFERENCE_FLAGS, fake(0xc8)]);
        let cell = e.new_object::<TESObjectCELL>();
        let world_space = object(&mut e, 0x40);
        // Flag 0x2 set, no flag 0x4000, exterior, with a world space.
        e.register(FORM_FLAG_4000, |_, _| Ret::default());
        returns(&mut e, FORM_FLAG_2, 1);
        returns(&mut e, CELL_IS_INTERIOR, 0);
        returns(&mut e, CELL_GET_WORLD_SPACE, world_space);
        start_log(&mut e);
        e.call(0x0055_2900, &args![cell, 5u32]);
        let log = calls(&e);
        assert_eq!(
            calls_to(&log, CELL_UPDATE_REFERENCE_FLAGS),
            vec![vec![cell.addr(), 5]]
        );
        assert_eq!(calls_to(&log, fake(0xc8)), vec![vec![world_space, 1]]);
        // An interior cell, a cell without flag 2 and one with flag 0x4000
        // leave the world space alone.
        returns(&mut e, CELL_IS_INTERIOR, 1);
        start_log(&mut e);
        e.call(0x0055_2900, &args![cell, 0u32]);
        assert!(calls_to(&calls(&e), fake(0xc8)).is_empty());
        returns(&mut e, CELL_IS_INTERIOR, 0);
        returns(&mut e, FORM_FLAG_2, 0);
        start_log(&mut e);
        e.call(0x0055_2900, &args![cell, 0u32]);
        assert!(calls_to(&calls(&e), fake(0xc8)).is_empty());
        returns(&mut e, FORM_FLAG_2, 1);
        returns(&mut e, FORM_FLAG_4000, 1);
        start_log(&mut e);
        e.call(0x0055_2900, &args![cell, 0u32]);
        assert!(calls_to(&calls(&e), fake(0xc8)).is_empty());
        // No world space.
        returns(&mut e, FORM_FLAG_4000, 0);
        returns(&mut e, CELL_GET_WORLD_SPACE, 0);
        start_log(&mut e);
        e.call(0x0055_2900, &args![cell, 0u32]);
        assert!(calls_to(&calls(&e), fake(0xc8)).is_empty());
    }

    #[test]
    fn render_singleton_is_read_from_its_global() {
        let mut e = engine();
        e.set_global(RENDER_SINGLETON, 0x1357u32);
        assert_eq!(e.call(0x0055_2ba0, &[]).u32(), 0x1357);
    }

    /// Builds a list of `{ item, next }` nodes, returns the first node.
    fn list_of(e: &mut Engine, items: &[u32]) -> u32 {
        let mut next = 0;
        for item in items.iter().rev() {
            let node = e.mem.alloc(8);
            e.mem.set_u32(node, *item);
            e.mem.set_u32(node + 4, next);
            next = node;
        }
        next
    }

    /// The list accessors read nodes of that shape.
    fn list_doubles(e: &mut Engine) {
        e.register(LIST_NODE_ITEM_ADDRESS, |_, a| a[0].into_ret());
        e.register(LIST_NODE_NEXT, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.register(LIST_NODE_IS_END, |e, a| {
            u32::from(e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0).into_ret()
        });
    }

    /// Doubles for the guard calls and the word-field getters.
    fn guard_doubles(e: &mut Engine) {
        quiet(e, &[SCOPE_GUARD_CONSTRUCT, SCOPE_GUARD_DESTRUCT]);
    }

    /// Doubles for everything `AttachToWorld` calls; returns the cell, the
    /// parent node, the render singleton and the child node of the portal
    /// graph slot. The cell is interior with a Havok world; the render
    /// singleton wants no nav meshes; the always-render array is empty.
    fn attach_setup(e: &mut Engine) -> (Ptr<TESObjectCELL>, u32, u32, u32) {
        guard_doubles(e);
        let cell = e.new_object::<TESObjectCELL>();
        e.set(cell, TESObjectCELL::bCellDetached, true);
        let parent = object(e, 0x40);
        let world = object(e, 0x40);
        let render = e.mem.alloc(0x300);
        e.set_global(RENDER_SINGLETON, render);
        quiet(
            e,
            &[
                CELL_SET_STATE,
                CELL_SET_LIGHTS_ATTACHED,
                CELL_SET_ADDON_NODES_ATTACHED,
                CELL_SET_ATTACHED_FLAG,
                NAV_MESH_RENDER_ADD,
                CELL_AFTER_ATTACH_A,
                CELL_AFTER_ATTACH_B,
                OBSTACLE_MANAGER_RUN,
                SCENE_NODE_ATTACH,
                PORTAL_ALWAYS_RENDER_ADD,
                PORTAL_ALWAYS_RENDER_SET,
                WORLD_SET_VISUAL_DEBUGGER_ENABLED,
                WORLD_SET_VISUAL_DEBUGGER,
                fake(0xdc),
            ],
        );
        returns(e, CELL_LOAD_3D, 0xa3d);
        returns(e, CELL_HAVOK_WORLD, world);
        returns(e, CELL_IS_INTERIOR, 1);
        returns(e, VISUAL_DEBUGGER_SUPPRESSED, 0);
        returns(e, VISUAL_DEBUGGER_SETTING, 1);
        returns(e, OBSTACLE_MANAGER_GET, 0x0b57);
        returns(e, CELL_OPTIONS, 0x0905);
        returns(e, TABLE_ENTRY, 0x7ab1);
        returns(e, PORTAL_ALWAYS_RENDER_COUNT, 0);
        returns(e, CELL_NODE_A, 0xa);
        returns(e, CELL_NODE_B, 0xb);
        returns(e, CELL_NODE_C, 0xc);
        returns(e, CELL_NODE_D, 0xd);
        returns(e, SCENE_NODE_CHILD, 0x5c);
        returns(e, CELL_PORTAL_GRAPH, 0x96);
        let child = object(e, 0x40);
        returns(e, CELL_CHILD_NODE, child);
        (cell, parent, render, child)
    }

    #[test]
    fn attach_to_world_attaches_the_3d_and_registers_the_nodes() {
        let mut e = engine();
        let (cell, parent, render, child) = attach_setup(&mut e);
        e.mem.set_u8(render + 0x210, 1);
        let world = e.call(CELL_HAVOK_WORLD, &args![cell]).u32();
        start_log(&mut e);
        e.call(0x0055_2970, &args![cell, parent]);
        let log = calls(&e);
        assert_eq!(calls_to(&log, CELL_SET_STATE), vec![vec![cell.addr(), 5]]);
        // The 3D goes under the parent (virtual +0xdc), the portal graph slot
        // under the cell's child node.
        assert_eq!(
            calls_to(&log, fake(0xdc)),
            vec![vec![parent, 0xa3d, 1], vec![child, 0x96, 1]]
        );
        // The interior cell skips the exterior terrain, the world gets the
        // visual debugger flags.
        assert!(calls_to(&log, CELL_COLLISION_WORLD).is_empty());
        assert_eq!(
            calls_to(&log, WORLD_SET_VISUAL_DEBUGGER_ENABLED),
            vec![vec![world, 1]]
        );
        assert_eq!(
            calls_to(&log, WORLD_SET_VISUAL_DEBUGGER),
            vec![vec![world, 1]]
        );
        // The render singleton wants the nav meshes.
        assert_eq!(
            calls_to(&log, NAV_MESH_RENDER_ADD),
            vec![vec![render, cell.addr(), 1]]
        );
        assert_eq!(
            calls_to(&log, SCENE_NODE_ATTACH),
            vec![vec![0x7ab1, 0x0905]]
        );
        // The always-render array is empty: the cell's two nodes and the
        // table child are added.
        assert_eq!(
            calls_to(&log, PORTAL_ALWAYS_RENDER_ADD),
            vec![vec![0x0905, 0xa], vec![0x0905, 0xb], vec![0x0905, 0x5c]]
        );
        assert_eq!(
            calls_to(&log, PORTAL_ALWAYS_RENDER_SET),
            vec![
                vec![0x0905, 0xc],
                vec![0x0905, 0xb],
                vec![0x0905, child],
                vec![0x0905, 0xd]
            ]
        );
        assert!(!e.get(cell, TESObjectCELL::bCellDetached));
        let guard = calls_to(&log, SCOPE_GUARD_CONSTRUCT);
        assert_eq!(&guard[0][1..], &[0x1a, 1, SOURCE_FILE_NAME, 0x395d]);
        assert_eq!(calls_to(&log, SCOPE_GUARD_DESTRUCT).len(), 1);
    }

    #[test]
    fn attach_to_world_without_world_or_nav_meshes_and_with_a_filled_array() {
        let mut e = engine();
        let (cell, parent, _render, _child) = attach_setup(&mut e);
        returns(&mut e, CELL_HAVOK_WORLD, 0);
        returns(&mut e, PORTAL_ALWAYS_RENDER_COUNT, 4);
        start_log(&mut e);
        e.call(0x0055_2970, &args![cell, parent]);
        let seen = addresses(&calls(&e));
        assert!(!seen.contains(&NAV_MESH_RENDER_ADD));
        assert!(!seen.contains(&PORTAL_ALWAYS_RENDER_ADD));
        assert!(!seen.contains(&WORLD_SET_VISUAL_DEBUGGER));
    }

    #[test]
    fn attach_to_world_builds_the_terrain_of_an_exterior_cell_with_a_world() {
        let mut e = engine();
        let (cell, parent, _render, _child) = attach_setup(&mut e);
        returns(&mut e, CELL_HAVOK_WORLD, 0x5511);
        returns(&mut e, CELL_IS_INTERIOR, 0);
        returns(&mut e, VISUAL_DEBUGGER_SUPPRESSED, 1);
        // No collision world: the terrain call is skipped, but the data
        // accessors are asked.
        returns(&mut e, CELL_GET_DATA_X, 0);
        returns(&mut e, CELL_GET_DATA_Y, 0);
        returns(&mut e, CELL_COLLISION_WORLD, 0);
        start_log(&mut e);
        e.call(0x0055_2970, &args![cell, parent]);
        let log = calls(&e);
        assert_eq!(calls_to(&log, CELL_COLLISION_WORLD).len(), 1);
        assert_eq!(
            calls_to(&log, WORLD_SET_VISUAL_DEBUGGER_ENABLED),
            vec![vec![0x5511, 0]]
        );
    }

    /// The doubles of `Detach` that stand for the scene, the loader and the
    /// render singleton; returns the cell.
    fn detach_setup(e: &mut Engine, state: u32) -> Ptr<TESObjectCELL> {
        guard_doubles(e);
        let cell = e.new_object::<TESObjectCELL>();
        let render = e.mem.alloc(0x300);
        e.set_global(RENDER_SINGLETON, render);
        e.set_global(SCENE_SINGLETON, 0x5c00u32);
        e.set_global(LOADER_SINGLETON, 0x1d00u32);
        e.set_global(LOADER_STATE_SINGLETON, 0x1500u32);
        e.set_global(MODEL_LOADER_SINGLETON, 0x3d00u32);
        let state_object = e.mem.alloc(0x40);
        e.set_global(STATE_OBJECT_SINGLETON, state_object);
        quiet(
            e,
            &[
                CELL_LOCK_ENTER,
                CELL_LOCK_LEAVE,
                CELL_SET_STATE,
                CELL_BEFORE_DETACH,
                MODEL_LOADER_CANCEL_FOR_CELL,
                SCENE_REFRESH,
                CELL_SET_LIGHTS_ATTACHED,
                SCENE_ATTACH_EXTERIOR,
                SCENE_ATTACH_INTERIOR,
                CELL_SET_ADDON_NODES_ATTACHED,
                CELL_DETACH_STEP_A,
                CELL_DETACH_STEP_B,
                CELL_SET_TERRAIN_UPDATE,
                CELL_CLEAR_LOADED_STATE,
                CELL_SET_ATTACHED_FLAG,
                NAV_MESH_RENDER_REMOVE,
                REMOVE_CHANGES,
                PORTAL_ALWAYS_RENDER_REMOVE,
                fake(0xe8),
            ],
        );
        returns(e, CELL_GET_STATE, state);
        returns(e, CELL_IS_INTERIOR, 0);
        returns(e, CELL_LOAD_3D, 0x3d);
        let parent = object(e, 0x40);
        // `WORD_AT_0C` and `FORM_ID` are the same getter: the form id of the
        // cell (0xf0f0) and the word at +0x0c of the scene singleton (the
        // parent node).
        let cell_address = cell.addr();
        e.register_double(WORD_AT_0C, move |_, a| {
            if a[0] == cell_address { 0xf0f0 } else { parent }.into_ret()
        });
        returns(e, FORM_HAS_CHANGES, 1);
        returns(e, CELL_OPTIONS, 0x0905);
        returns(e, CELL_NODE_B, 0xb);
        returns(e, CELL_NODE_C, 0xc);
        returns(e, CELL_NODE_D, 0xd);
        returns(e, CELL_CHILD_NODE, 0x2c);
        cell
    }

    #[test]
    fn detach_takes_the_cell_out_of_the_scene_in_order() {
        let mut e = engine();
        let cell = detach_setup(&mut e, 5);
        start_log(&mut e);
        e.call(0x0055_2bd0, &args![cell, 0u32]);
        let log = calls(&e);
        let expected = [
            CELL_LOCK_ENTER,
            CELL_GET_STATE,
            CELL_SET_STATE,
            CELL_BEFORE_DETACH,
            MODEL_LOADER_CANCEL_FOR_CELL,
            SCENE_REFRESH,
            CELL_SET_LIGHTS_ATTACHED,
            CELL_IS_INTERIOR,
            SCENE_ATTACH_EXTERIOR,
            CELL_SET_ADDON_NODES_ATTACHED,
            CELL_LOAD_3D,
            WORD_AT_0C,
            fake(0xe8),
            CELL_DETACH_STEP_A,
            CELL_DETACH_STEP_B,
            CELL_SET_STATE,
            CELL_SET_TERRAIN_UPDATE,
            CELL_CLEAR_LOADED_STATE,
            CELL_SET_ATTACHED_FLAG,
            NAV_MESH_RENDER_REMOVE,
            FORM_ID,
            FORM_HAS_CHANGES,
            REMOVE_CHANGES,
            SCENE_REFRESH,
            CELL_OPTIONS,
            CELL_NODE_C,
            PORTAL_ALWAYS_RENDER_REMOVE,
            CELL_NODE_B,
            PORTAL_ALWAYS_RENDER_REMOVE,
            CELL_CHILD_NODE,
            PORTAL_ALWAYS_RENDER_REMOVE,
            CELL_NODE_D,
            PORTAL_ALWAYS_RENDER_REMOVE,
            CELL_LOCK_LEAVE,
        ];
        assert_eq!(addresses(&log), expected);
        assert_eq!(
            calls_to(&log, CELL_SET_STATE),
            vec![vec![cell.addr(), 4], vec![cell.addr(), 3]]
        );
        assert_eq!(
            calls_to(&log, MODEL_LOADER_CANCEL_FOR_CELL),
            vec![vec![0x3d00, cell.addr()]]
        );
        assert_eq!(
            calls_to(&log, SCENE_ATTACH_EXTERIOR),
            vec![vec![0x5c00, cell.addr()]]
        );
        assert_eq!(calls_to(&log, FORM_HAS_CHANGES), vec![vec![0x1500, 0xf0f0]]);
        assert_eq!(
            calls_to(&log, REMOVE_CHANGES),
            vec![vec![0x1d00, cell.addr()]]
        );
        assert_eq!(calls_to(&log, fake(0xe8))[0][1], 0x3d);
        assert!(e.get(cell, TESObjectCELL::bCellDetached));
    }

    #[test]
    fn detach_skips_terrain_update_and_changes_when_the_state_object_and_form_say_so() {
        let mut e = engine();
        let cell = detach_setup(&mut e, 6);
        let state_object: u32 = e.global(STATE_OBJECT_SINGLETON);
        e.mem.set_u8(state_object + 0x26, 1);
        returns(&mut e, FORM_HAS_CHANGES, 0);
        returns(&mut e, CELL_IS_INTERIOR, 1);
        start_log(&mut e);
        e.call(0x0055_2bd0, &args![cell, 0u32]);
        let seen = addresses(&calls(&e));
        assert!(seen.contains(&SCENE_ATTACH_INTERIOR));
        assert!(!seen.contains(&SCENE_ATTACH_EXTERIOR));
        assert!(!seen.contains(&CELL_SET_TERRAIN_UPDATE));
        assert!(!seen.contains(&REMOVE_CHANGES));
    }

    #[test]
    fn detach_does_nothing_for_other_states_and_detaches_havok_on_request() {
        let mut e = engine();
        let cell = detach_setup(&mut e, 4);
        start_log(&mut e);
        e.call(0x0055_2bd0, &args![cell, 1u32]);
        assert_eq!(
            addresses(&calls(&e)),
            vec![CELL_LOCK_ENTER, CELL_GET_STATE, CELL_LOCK_LEAVE]
        );
        // State 5 with `detach_havok`: the interior test of DetachHavok shows
        // up between the scene detach and the add-on nodes.
        returns(&mut e, CELL_GET_STATE, 5);
        returns(&mut e, CELL_HAVOK_WORLD, 0);
        returns(&mut e, CELL_COLLISION_WORLD, 0);
        quiet(&mut e, &[CELL_REFERENCES]);
        let state_object = e.mem.alloc(0x40);
        e.set_global(STATE_OBJECT_SINGLETON, state_object);
        start_log(&mut e);
        e.call(0x0055_2bd0, &args![cell, 1u32]);
        let seen = addresses(&calls(&e));
        let at = seen
            .iter()
            .position(|a| *a == CELL_COLLISION_WORLD)
            .unwrap();
        assert_eq!(seen[at - 1], CELL_IS_INTERIOR);
        assert_eq!(seen[at + 1], CELL_LOCK_ENTER);
    }

    #[test]
    fn vector4_constructor_stores_the_floats_in_order() {
        let mut e = engine();
        let at = e.mem.alloc(16);
        let result = e.call(0x0055_32a0, &args![at, 1.0f32, 2.0f32, -3.5f32, 4.25f32]);
        assert_eq!(result.u32(), at);
        let words: Vec<f32> = (0..4).map(|i| e.mem.f32(at + 4 * i)).collect();
        assert_eq!(words, vec![1.0, 2.0, -3.5, 4.25]);
    }

    fn manager_doubles(e: &mut Engine) {
        returns(e, MEMORY_MANAGER_GET, 0x44);
        e.register(MEMORY_MANAGER_ALLOCATE, |e, a| {
            assert_eq!(a[0], 0x44);
            e.mem.alloc(a[1]).into_ret()
        });
        e.register(MEMORY_MANAGER_DEALLOCATE, |e, a| {
            assert_eq!(a[0], 0x44);
            e.mem.free(a[1]);
            Ret::default()
        });
    }

    #[test]
    fn aligned_allocation_aligns_and_remembers_the_distance() {
        let mut e = engine();
        manager_doubles(&mut e);
        start_log(&mut e);
        let aligned = e.call(0x0055_3320, &args![16u32, 40u32]).u32();
        assert_eq!(aligned % 16, 0);
        let requested = calls_to(&calls(&e), MEMORY_MANAGER_ALLOCATE);
        assert_eq!(requested, vec![vec![0x44, 56]]);
        let distance = e.mem.u8(aligned - 1) as u32;
        assert!(distance == 8 || distance == 16);
        // Another call: a block that is already aligned moves up by a whole
        // alignment.
        let second = e.call(0x0055_3300, &args![0xa0u32]).u32();
        assert_eq!(second % 16, 0);
        assert!(matches!(e.mem.u8(second - 1), 8 | 16));
        // An alignment of 4 with an 8-aligned block moves by 4.
        let third = e.call(0x0055_3320, &args![4u32, 8u32]).u32();
        assert_eq!(third % 4, 0);
        assert_eq!(e.mem.u8(third - 1), 4);
    }

    #[test]
    fn aligned_free_gives_back_the_original_block() {
        let mut e = engine();
        manager_doubles(&mut e);
        let aligned = e.call(0x0055_3320, &args![16u32, 24u32]).u32();
        let block = aligned - e.mem.u8(aligned - 1) as u32;
        start_log(&mut e);
        e.call(0x0055_3380, &args![aligned]);
        assert_eq!(
            calls_to(&calls(&e), MEMORY_MANAGER_DEALLOCATE),
            vec![vec![0x44, block]]
        );
        start_log(&mut e);
        e.call(0x0055_3380, &args![0u32]);
        assert!(calls(&e).is_empty());
    }

    /// A world holder whose object (the word at `+0`) is the first word.
    fn holder_doubles(e: &mut Engine) {
        e.register(HOLDER_GET_WORLD, |e, a| e.mem.u32(a[0] + 0x90).into_ret());
        quiet(
            e,
            &[
                EMPTY_FUNCTION,
                WORLD_SET_GRAVITY_VECTOR,
                WORLD_SET_FIELD_A,
                WORLD_SET_FIELD_B,
                HOLDER_FIELD_70_ADD,
            ],
        );
    }

    #[test]
    fn world_wrapper_calls_the_empty_function_on_the_world_object() {
        let mut e = engine();
        holder_doubles(&mut e);
        let holder = e.mem.alloc(0xb0);
        start_log(&mut e);
        e.call(0x0055_33c0, &args![holder]);
        assert_eq!(addresses(&calls(&e)), vec![HOLDER_GET_WORLD]);
        e.mem.set_u32(holder + 0x90, 0x7700);
        start_log(&mut e);
        e.call(0x0055_33c0, &args![holder]);
        assert_eq!(
            calls(&e),
            vec![
                (HOLDER_GET_WORLD, vec![holder]),
                (EMPTY_FUNCTION, vec![0x7700])
            ]
        );
    }

    #[test]
    fn world_wrapper_005533f0_sets_the_value_and_queues_its_address() {
        let mut e = engine();
        holder_doubles(&mut e);
        let holder = e.mem.alloc(0xb0);
        let seen = Rc::new(RefCell::new(0));
        let slot_value = seen.clone();
        e.register_double(HOLDER_FIELD_70_ADD, move |e, a| {
            *slot_value.borrow_mut() = e.mem.u32(a[1]);
            Ret::default()
        });
        start_log(&mut e);
        e.call(0x0055_33f0, &args![holder, 0x1234u32]);
        assert_eq!(addresses(&calls(&e)), vec![HOLDER_GET_WORLD]);
        e.mem.set_u32(holder + 0x90, 0x7700);
        start_log(&mut e);
        e.call(0x0055_33f0, &args![holder, 0x1234u32]);
        let log = calls(&e);
        assert_eq!(
            addresses(&log),
            vec![
                HOLDER_GET_WORLD,
                HOLDER_GET_WORLD,
                EMPTY_FUNCTION,
                WORLD_SET_GRAVITY_VECTOR,
                HOLDER_GET_WORLD,
                EMPTY_FUNCTION,
                HOLDER_FIELD_70_ADD
            ]
        );
        assert_eq!(
            calls_to(&log, WORLD_SET_GRAVITY_VECTOR),
            vec![vec![0x7700, 0x1234]]
        );
        assert_eq!(calls_to(&log, HOLDER_FIELD_70_ADD)[0][0], holder + 0x70);
        assert_eq!(*seen.borrow(), 0x1234);
    }

    #[test]
    fn world_wrapper_00553440_sets_a_field_between_two_empty_calls() {
        let mut e = engine();
        holder_doubles(&mut e);
        let holder = e.mem.alloc(0xb0);
        e.mem.set_u32(holder + 0x90, 0x7700);
        start_log(&mut e);
        e.call(0x0055_3440, &args![holder, 9u32]);
        let log = calls(&e);
        assert_eq!(
            addresses(&log),
            vec![
                HOLDER_GET_WORLD,
                HOLDER_GET_WORLD,
                EMPTY_FUNCTION,
                WORLD_SET_FIELD_A,
                HOLDER_GET_WORLD,
                EMPTY_FUNCTION
            ]
        );
        assert_eq!(calls_to(&log, WORLD_SET_FIELD_A), vec![vec![0x7700, 9]]);
        // Without a world object nothing is set.
        e.mem.set_u32(holder + 0x90, 0);
        start_log(&mut e);
        e.call(0x0055_3440, &args![holder, 9u32]);
        assert_eq!(addresses(&calls(&e)), vec![HOLDER_GET_WORLD]);
    }

    #[test]
    fn world_wrapper_00553480_sets_the_inner_value_and_stores_it() {
        let mut e = engine();
        holder_doubles(&mut e);
        let holder = e.mem.alloc(0xb0);
        e.mem.set_u32(holder + 0x90, 0x7700);
        start_log(&mut e);
        e.call(0x0055_3480, &args![holder, 0x1000u32]);
        let log = calls(&e);
        assert_eq!(
            calls_to(&log, WORLD_SET_GRAVITY_VECTOR),
            vec![vec![0x7700, 0x1000]]
        );
        assert_eq!(
            calls_to(&log, WORLD_SET_FIELD_B),
            vec![vec![0x7700, 0x1014]]
        );
        assert_eq!(e.mem.u32(holder + 0x20), 0x1000);
        // A null value gives a null inner value.
        start_log(&mut e);
        e.call(0x0055_3480, &args![holder, 0u32]);
        assert_eq!(
            calls_to(&calls(&e), WORLD_SET_FIELD_B),
            vec![vec![0x7700, 0]]
        );
        assert_eq!(e.mem.u32(holder + 0x20), 0);
    }

    #[test]
    fn world_wrapper_005534f0_stores_the_value_at_0x28() {
        let mut e = engine();
        holder_doubles(&mut e);
        let holder = e.mem.alloc(0xb0);
        e.call(0x0055_34f0, &args![holder, 0x4321u32]);
        assert_eq!(e.mem.u32(holder + 0x28), 0x4321);
    }

    #[test]
    fn cinfo_destructor_runs_the_member_and_base_destructors() {
        let mut e = engine();
        quiet(
            &mut e,
            &[CINFO_MEMBER_DESTRUCT, CINFO_BASE_DESTRUCT, CINFO_FREE],
        );
        let cinfo = e.mem.alloc(0xf0);
        start_log(&mut e);
        e.call(0x0055_3540, &args![cinfo]);
        let log = calls(&e);
        assert_eq!(
            log,
            vec![
                (CINFO_MEMBER_DESTRUCT, vec![cinfo + 0x6c]),
                (CINFO_MEMBER_DESTRUCT, vec![cinfo + 0x5c]),
                (CINFO_MEMBER_DESTRUCT, vec![cinfo + 0x58]),
                (CINFO_BASE_DESTRUCT, vec![cinfo]),
            ]
        );
        assert_eq!(e.mem.u32(cinfo), CINFO_VTABLE);
        // The 00553520 wrapper runs the same destructor.
        start_log(&mut e);
        e.call(0x0055_3520, &args![cinfo]);
        assert_eq!(calls(&e).len(), 4);
    }

    #[test]
    fn cinfo_deleting_destructor_frees_only_when_asked() {
        let mut e = engine();
        quiet(
            &mut e,
            &[CINFO_MEMBER_DESTRUCT, CINFO_BASE_DESTRUCT, CINFO_FREE],
        );
        let cinfo = e.mem.alloc(0xf0);
        start_log(&mut e);
        assert_eq!(e.call(0x0055_35c0, &args![cinfo, 0u32]).u32(), cinfo);
        assert!(calls_to(&calls(&e), CINFO_FREE).is_empty());
        start_log(&mut e);
        assert_eq!(e.call(0x0055_35c0, &args![cinfo, 3u32]).u32(), cinfo);
        assert_eq!(calls_to(&calls(&e), CINFO_FREE), vec![vec![cinfo]]);
    }

    /// What the interior-world creation builds, noted by the doubles.
    #[derive(Default)]
    struct WorldTrace {
        /// `(size, address)` of every `operator new`.
        allocations: Vec<(u32, u32)>,
        /// The gravity vector given to the cinfo.
        gravity: Vec<f32>,
        /// The scale stored in the cinfo when the world was built.
        scale: f32,
        cinfo: u32,
    }

    /// Doubles for `fn_00552dc0`: a cell that is interior and has no Havok
    /// world and no collision world; the world is made of fresh blocks.
    fn world_creation_setup(e: &mut Engine) -> (Ptr<TESObjectCELL>, Rc<RefCell<WorldTrace>>) {
        guard_doubles(e);
        manager_doubles(e);
        let trace = Rc::new(RefCell::new(WorldTrace::default()));
        let cell = e.new_object::<TESObjectCELL>();
        quiet(
            e,
            &[
                CINFO_CONSTRUCT,
                CINFO_MEMBER_DESTRUCT,
                CINFO_BASE_DESTRUCT,
                EMPTY_FUNCTION,
                WORLD_SET_GRAVITY_VECTOR,
                WORLD_SET_FIELD_A,
                WORLD_SET_FIELD_B,
                HOLDER_FIELD_70_ADD,
                WORLD_SET_GRAVITY_SCALE,
                LISTENER_REGISTER,
                LIST_NODE_ITEM_ADDRESS,
                CELL_REFERENCE_CENTER,
                WORLD_SET_CENTER,
                WORLD_SET_VISUAL_DEBUGGER,
                WORLD_SET_VISUAL_DEBUGGER_ENABLED,
                EXTRA_LIST_SET_HAVOK_WORLD,
                EXTRA_LIST_SET_COLLISION_WORLD,
                CINFO_SET_BROAD_PHASE_SIZE,
                fake(0xc4),
            ],
        );
        returns(e, CELL_HAVOK_WORLD, 0);
        returns(e, CELL_IS_INTERIOR, 1);
        returns(e, CELL_COLLISION_WORLD, 0);
        returns(e, HAS_WATER_TEST, 0);
        returns(e, SCENE_HAVOK_SETTING, 0x42);
        returns(e, VISUAL_DEBUGGER_SETTING, 1);
        returns(e, VISUAL_DEBUGGER_SUPPRESSED, 1);
        e.register(HOLDER_GET_WORLD, |_, a| a[0].into_ret());
        let noted = trace.clone();
        e.register_double(OPERATOR_NEW, move |e, a| {
            let block = e.mem.alloc(a[0]);
            noted.borrow_mut().allocations.push((a[0], block));
            block.into_ret()
        });
        e.register(ALLOCATE_CDECL, |e, a| e.mem.alloc(a[0]).into_ret());
        for constructor in [
            BROAD_PHASE_CONSTRUCT,
            FILTER_CONSTRUCT,
            LISTENER_OBJECT_CONSTRUCT,
            LISTENER_CONSTRUCT,
            COLLISION_WORLD_CONSTRUCT,
        ] {
            e.register(constructor, |_, a| a[0].into_ret());
        }
        e.register(WATER_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        let noted = trace.clone();
        e.register_double(CINFO_SET_GRAVITY, move |e, a| {
            noted.borrow_mut().gravity = (0..4).map(|i| e.mem.f32(a[1] + 4 * i)).collect();
            Ret::default()
        });
        let noted = trace.clone();
        e.register_double(WORLD_CONSTRUCT, move |e, a| {
            let mut trace = noted.borrow_mut();
            trace.scale = e.mem.f32(a[1] + 0x54);
            trace.cinfo = a[1];
            // The world object is the raw block; give it a vtable for the
            // virtual call at +0xc4.
            e.mem.set_u32(a[0], TEST_VTABLE);
            a[0].into_ret()
        });
        e.set_global(GRAVITY_Z, -9.5f32);
        e.set_global(BROAD_PHASE_SIZE, 3250.0f32);
        (cell, trace)
    }

    #[test]
    fn interior_world_is_built_from_its_parts_and_stored_in_the_extra_data() {
        let mut e = engine();
        let (cell, trace) = world_creation_setup(&mut e);
        returns(&mut e, HAS_WATER_TEST, 1);
        e.register(CELL_GET_WATER_HEIGHT, |_, _| 12.5f32.into_ret());
        start_log(&mut e);
        e.call(0x0055_2dc0, &args![cell]);
        let log = calls(&e);
        let trace = trace.borrow();
        // The cinfo: gravity (0, 0, g, 0) and scale 1.0 when the world is
        // constructed.
        assert_eq!(trace.gravity, vec![0.0, 0.0, -9.5, 0.0]);
        assert_eq!(trace.scale, 1.0);
        assert_eq!(
            calls_to(&log, CINFO_SET_BROAD_PHASE_SIZE),
            vec![vec![trace.cinfo, 3250.0f32.to_bits()]]
        );
        // The world was allocated as 0xa0 aligned bytes.
        let world = calls_to(&log, WORLD_CONSTRUCT)[0][0];
        assert_eq!(world % 16, 0);
        assert_eq!(
            calls_to(&log, MEMORY_MANAGER_ALLOCATE),
            vec![vec![0x44, 0xa0 + 0x10]]
        );
        assert_eq!(
            calls_to(&log, EXTRA_LIST_SET_HAVOK_WORLD),
            vec![vec![cell.addr() + 0x28, world]]
        );
        // Broad phase (0x24 bytes, stored at +8 inside), filter (0x28), water
        // (0x40, built from the cell's water height), listener object (0x14).
        let sizes: Vec<u32> = trace.allocations.iter().map(|(size, _)| *size).collect();
        assert_eq!(sizes, vec![0x24, 0x28, 0x40, 0x14, 8]);
        let block = |size: u32| {
            trace
                .allocations
                .iter()
                .find(|(s, _)| *s == size)
                .unwrap()
                .1
        };
        assert_eq!(e.mem.u32(world + 0x20), block(0x24) + 8);
        assert_eq!(e.mem.u32(world + 0x28), block(0x28));
        assert_eq!(
            calls_to(&log, WATER_CONSTRUCT),
            vec![vec![block(0x40), 12.5f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&log, WORLD_SET_GRAVITY_SCALE),
            vec![vec![world, block(0x40)]]
        );
        assert_eq!(calls_to(&log, WORLD_SET_FIELD_A), vec![vec![world, 0x42]]);
        // The listener singleton is created once and registered with the
        // world holder's object.
        let singleton: u32 = e.global(LISTENER_SINGLETON);
        assert_eq!(singleton, block(8));
        assert_eq!(
            calls_to(&log, LISTENER_REGISTER),
            vec![vec![singleton, world]]
        );
        // Visual debugger, virtual +0xc4, and the destruction of the cinfo.
        assert_eq!(
            calls_to(&log, WORLD_SET_VISUAL_DEBUGGER),
            vec![vec![world, 1]]
        );
        assert_eq!(
            calls_to(&log, WORLD_SET_VISUAL_DEBUGGER_ENABLED),
            vec![vec![world, 0]]
        );
        assert_eq!(calls_to(&log, fake(0xc4)).len(), 1);
        assert_eq!(calls_to(&log, CINFO_BASE_DESTRUCT), vec![vec![trace.cinfo]]);
        // The collision world (0x18 bytes) is created in a second guard.
        let guards = calls_to(&log, SCOPE_GUARD_CONSTRUCT);
        assert_eq!(guards.len(), 2);
        assert_eq!(&guards[0][1..], &[0x10, 1, SOURCE_FILE_NAME, 0x3a3f]);
        assert_eq!(&guards[1][1..], &[0x10, 1, SOURCE_FILE_NAME, 0x3a91]);
        let collision = calls_to(&log, COLLISION_WORLD_CONSTRUCT)[0][0];
        assert_eq!(
            calls_to(&log, EXTRA_LIST_SET_COLLISION_WORLD),
            vec![vec![cell.addr() + 0x28, collision]]
        );
    }

    #[test]
    fn interior_world_without_water_and_with_an_existing_listener_singleton() {
        let mut e = engine();
        let (cell, trace) = world_creation_setup(&mut e);
        e.set_global(LISTENER_SINGLETON, 0x1357u32);
        returns(&mut e, CELL_COLLISION_WORLD, 0x777);
        start_log(&mut e);
        e.call(0x0055_2dc0, &args![cell]);
        let log = calls(&e);
        assert!(calls_to(&log, WATER_CONSTRUCT).is_empty());
        assert!(calls_to(&log, WORLD_SET_GRAVITY_SCALE).is_empty());
        let sizes: Vec<u32> = trace
            .borrow()
            .allocations
            .iter()
            .map(|(size, _)| *size)
            .collect();
        assert_eq!(sizes, vec![0x24, 0x28, 0x14]);
        assert_eq!(e.global::<u32>(LISTENER_SINGLETON), 0x1357);
        // A collision world exists: only one guard.
        assert_eq!(calls_to(&log, SCOPE_GUARD_CONSTRUCT).len(), 1);
        assert!(calls_to(&log, EXTRA_LIST_SET_COLLISION_WORLD).is_empty());
    }

    #[test]
    fn world_creation_is_skipped_for_exterior_cells_and_cells_with_a_world() {
        let mut e = engine();
        let (cell, _trace) = world_creation_setup(&mut e);
        returns(&mut e, CELL_COLLISION_WORLD, 0x777);
        returns(&mut e, CELL_IS_INTERIOR, 0);
        start_log(&mut e);
        e.call(0x0055_2dc0, &args![cell]);
        assert_eq!(
            addresses(&calls(&e)),
            vec![CELL_HAVOK_WORLD, CELL_IS_INTERIOR, CELL_COLLISION_WORLD]
        );
        returns(&mut e, CELL_IS_INTERIOR, 1);
        returns(&mut e, CELL_HAVOK_WORLD, 0x555);
        start_log(&mut e);
        e.call(0x0055_2dc0, &args![cell]);
        assert_eq!(
            addresses(&calls(&e)),
            vec![CELL_HAVOK_WORLD, CELL_IS_INTERIOR, CELL_COLLISION_WORLD]
        );
    }

    #[test]
    fn exterior_terrain_transform_is_placed_at_the_cell_centre() {
        let mut e = engine();
        guard_doubles(&mut e);
        let cell = e.new_object::<TESObjectCELL>();
        returns(&mut e, CELL_HAVOK_WORLD, 0x5511);
        returns(&mut e, CELL_COLLISION_WORLD, 0x6622);
        returns(&mut e, CELL_GET_DATA_X, 3);
        returns(&mut e, CELL_GET_DATA_Y, -2i32 as u32);
        quiet(&mut e, &[TRANSFORM_CONSTRUCT]);
        e.set_global(HALF_CELL_SIZE, 2048.0f64);
        for i in 0..9u32 {
            e.set_global(IDENTITY_ROTATION + 4 * i, 0x3f80_0000 + i);
        }
        // The double reads the transform at the call.
        let seen = Rc::new(RefCell::new(Vec::new()));
        let noted = seen.clone();
        e.register_double(WORLD_ADD_TERRAIN, move |e, a| {
            let translation = a[2];
            let mut words: Vec<u32> = (0..9)
                .map(|i| e.mem.u32(translation - 0x24 + 4 * i))
                .collect();
            words.extend((0..3).map(|i| e.mem.f32(translation + 4 * i).to_bits()));
            words.push(e.mem.f32(translation + 12).to_bits());
            *noted.borrow_mut() = words;
            Ret::default()
        });
        start_log(&mut e);
        e.call(0x0055_35f0, &args![cell]);
        let log = calls(&e);
        let terrain = calls_to(&log, WORLD_ADD_TERRAIN);
        assert_eq!(terrain.len(), 1);
        assert_eq!(terrain[0][0], 0x6622);
        assert_eq!(terrain[0][1], 0x5511);
        assert_eq!(terrain[0][3], TERRAIN_FLAGS);
        let words = seen.borrow().clone();
        assert_eq!(
            &words[..9],
            &(0..9).map(|i| 0x3f80_0000 + i).collect::<Vec<u32>>()[..]
        );
        assert_eq!(f32::from_bits(words[9]), 14336.0);
        assert_eq!(f32::from_bits(words[10]), -6144.0);
        assert_eq!(f32::from_bits(words[11]), 0.0);
        assert_eq!(f32::from_bits(words[12]), 1.0);
        let guard = calls_to(&log, SCOPE_GUARD_CONSTRUCT);
        assert_eq!(&guard[0][1..], &[0x10, 1, SOURCE_FILE_NAME, 0x3aa6]);
        // Without a collision world (or a Havok world) nothing is built.
        returns(&mut e, CELL_COLLISION_WORLD, 0);
        start_log(&mut e);
        e.call(0x0055_35f0, &args![cell]);
        assert!(calls_to(&calls(&e), TRANSFORM_CONSTRUCT).is_empty());
        returns(&mut e, CELL_COLLISION_WORLD, 0x6622);
        returns(&mut e, CELL_HAVOK_WORLD, 0);
        start_log(&mut e);
        e.call(0x0055_35f0, &args![cell]);
        assert!(calls_to(&calls(&e), TRANSFORM_CONSTRUCT).is_empty());
    }

    #[test]
    fn detach_havok_of_an_interior_cell_switches_the_debugger_off_and_releases_the_object() {
        let mut e = engine();
        let cell = e.new_object::<TESObjectCELL>();
        returns(&mut e, CELL_IS_INTERIOR, 1);
        returns(&mut e, CELL_HAVOK_WORLD, 0x5511);
        returns(&mut e, WORLD_COLLISION_OBJECT, 0x8800);
        quiet(
            &mut e,
            &[
                WORLD_SET_VISUAL_DEBUGGER_ENABLED,
                WORLD_SET_VISUAL_DEBUGGER,
                COLLISION_OBJECT_SET_ACTIVE,
                COLLISION_OBJECT_RELEASE,
            ],
        );
        start_log(&mut e);
        e.call(0x0055_3700, &args![cell]);
        let log = calls(&e);
        assert_eq!(
            addresses(&log),
            vec![
                CELL_IS_INTERIOR,
                CELL_HAVOK_WORLD,
                WORLD_SET_VISUAL_DEBUGGER_ENABLED,
                WORLD_SET_VISUAL_DEBUGGER,
                WORLD_COLLISION_OBJECT,
                WORLD_COLLISION_OBJECT,
                COLLISION_OBJECT_SET_ACTIVE,
                COLLISION_OBJECT_RELEASE,
            ]
        );
        assert_eq!(
            calls_to(&log, COLLISION_OBJECT_SET_ACTIVE),
            vec![vec![0x8800, 1]]
        );
        assert_eq!(
            calls_to(&log, COLLISION_OBJECT_RELEASE),
            vec![vec![0x8800, 0]]
        );
        assert_eq!(
            calls_to(&log, WORLD_SET_VISUAL_DEBUGGER_ENABLED),
            vec![vec![0x5511, 0]]
        );
        // No world: nothing else happens.
        returns(&mut e, CELL_HAVOK_WORLD, 0);
        start_log(&mut e);
        e.call(0x0055_3700, &args![cell]);
        assert_eq!(
            addresses(&calls(&e)),
            vec![CELL_IS_INTERIOR, CELL_HAVOK_WORLD]
        );
    }

    #[test]
    fn detach_havok_of_an_exterior_cell_detaches_every_reference() {
        let mut e = engine();
        list_doubles(&mut e);
        let cell = e.new_object::<TESObjectCELL>();
        let first = object(&mut e, 0x40);
        let second = object(&mut e, 0x40);
        let list = list_of(&mut e, &[first, 0, second]);
        let state_object = e.mem.alloc(0x40);
        e.mem.set_u8(state_object + 0x26, 1);
        e.set_global(STATE_OBJECT_SINGLETON, state_object);
        returns(&mut e, CELL_IS_INTERIOR, 0);
        returns(&mut e, CELL_COLLISION_WORLD, 0x6622);
        returns(&mut e, CELL_REFERENCES, list);
        returns(&mut e, CELL_HAVOK_WORLD, 0x5511);
        returns(&mut e, WORLD_COLLISION_OBJECT, 0x8800);
        quiet(
            &mut e,
            &[
                COLLISION_WORLD_UNLOAD,
                CELL_LOCK_ENTER,
                CELL_LOCK_LEAVE,
                COLLISION_OBJECT_RELEASE,
                WORLD_UPDATE_RENDERER,
                fake(0x1c0),
            ],
        );
        start_log(&mut e);
        e.call(0x0055_3700, &args![cell]);
        let log = calls(&e);
        assert_eq!(calls_to(&log, COLLISION_WORLD_UNLOAD), vec![vec![0x6622]]);
        // The empty item in the middle is skipped, the others are detached.
        let detached: Vec<u32> = calls_to(&log, fake(0x1c0)).iter().map(|c| c[0]).collect();
        assert_eq!(detached, vec![first, second]);
        assert_eq!(
            calls_to(&log, COLLISION_OBJECT_RELEASE),
            vec![vec![0x8800, 0]]
        );
        assert_eq!(calls_to(&log, WORLD_UPDATE_RENDERER), vec![vec![0x5511, 0]]);
        assert_eq!(*addresses(&log).first().unwrap(), CELL_IS_INTERIOR);
        assert_eq!(*addresses(&log).last().unwrap(), CELL_LOCK_LEAVE);
        // Without collision world, Havok world and renderer flag.
        returns(&mut e, CELL_COLLISION_WORLD, 0);
        returns(&mut e, CELL_HAVOK_WORLD, 0);
        e.mem.set_u8(state_object + 0x26, 0);
        start_log(&mut e);
        e.call(0x0055_3700, &args![cell]);
        let seen = addresses(&calls(&e));
        assert!(!seen.contains(&COLLISION_WORLD_UNLOAD));
        assert!(!seen.contains(&COLLISION_OBJECT_RELEASE));
        assert!(!seen.contains(&WORLD_UPDATE_RENDERER));
    }

    #[test]
    fn animated_references_are_asked_and_the_not_excluded_ones_notified() {
        let mut e = engine();
        let cell = e.new_object::<TESObjectCELL>();
        let loaded = e.new_object::<LoadedCellData>();
        e.set(cell, TESObjectCELL::pLoadedData, loaded.cast());
        // Three references: one answers true to +0x100; one has no base
        // object; one has form type 0x25; the last one is notified.
        let refs: Vec<u32> = (0..4).map(|_| object(&mut e, 0x40)).collect();
        e.mem.set_u32(refs[0] + 0x30, 1);
        e.mem.set_u32(refs[1] + 0x20, 0);
        let excluded_form = e.mem.alloc(0x10);
        let notified_form = e.mem.alloc(0x10);
        e.mem.set_u8(excluded_form + 4, 0x25);
        e.mem.set_u8(notified_form + 4, 0x10);
        e.mem.set_u32(refs[2] + 0x20, excluded_form);
        e.mem.set_u32(refs[3] + 0x20, notified_form);
        returns(&mut e, CELL_IS_LOADED, 1);
        returns(&mut e, MAP_FIRST_POSITION, 1);
        e.register(fake(0x100), |e, a| e.mem.u32(a[0] + 0x30).into_ret());
        quiet(&mut e, &[fake(0x1e0)]);
        e.register(REFERENCE_BASE_OBJECT, |e, a| {
            e.mem.u32(a[0] + 0x20).into_ret()
        });
        // Form types: 0x25 excludes the reference.
        e.register(FORM_TYPE, |e, a| u32::from(e.mem.u8(a[0] + 4)).into_ret());
        let queue = Rc::new(RefCell::new(
            refs.iter().rev().copied().collect::<Vec<u32>>(),
        ));
        let source = queue.clone();
        e.register_double(MAP_NEXT, move |e, a| {
            let reference = source.borrow_mut().pop().unwrap();
            e.mem.set_u32(a[2], reference);
            e.mem.set_u32(a[3], 0x3333);
            if source.borrow().is_empty() {
                e.mem.set_u32(a[1], 0);
            }
            Ret::default()
        });
        start_log(&mut e);
        e.call(0x0055_3820, &args![cell]);
        let log = calls(&e);
        let notified: Vec<u32> = calls_to(&log, fake(0x1e0)).iter().map(|c| c[0]).collect();
        assert_eq!(notified, vec![refs[3]]);
        // Not loaded or without loaded data: nothing is walked.
        returns(&mut e, CELL_IS_LOADED, 0);
        start_log(&mut e);
        e.call(0x0055_3820, &args![cell]);
        assert_eq!(addresses(&calls(&e)), vec![CELL_IS_LOADED]);
        returns(&mut e, CELL_IS_LOADED, 1);
        e.set(cell, TESObjectCELL::pLoadedData, Ptr::NULL);
        start_log(&mut e);
        e.call(0x0055_3820, &args![cell]);
        assert_eq!(addresses(&calls(&e)), vec![CELL_IS_LOADED]);
    }

    #[test]
    fn sound_chance_and_range_accessors() {
        let mut e = engine();
        let form = e.mem.alloc(0x80);
        e.mem.set_i8(form + 0x68, -5);
        assert_eq!(e.call(0x0055_3b60, &args![form]).u8() as i8, 0);
        e.mem.set_u32(form + 0x48, 2);
        assert_eq!(e.call(0x0055_3b60, &args![form]).u8() as i8, -5);
        e.mem.set_u32(form + 0x48, 5);
        assert_eq!(e.call(0x0055_3b60, &args![form]).u8() as i8, 0);
        e.mem.set_u8(form + 0x45, 9);
        assert_eq!(e.call(0x0055_3b90, &args![form]).u8(), 9);
    }

    #[test]
    fn sound_lookup_flag_needs_the_holder_and_bit_1() {
        let mut e = engine();
        let object = e.mem.alloc(0x40);
        let record = e.mem.alloc(0x10);
        e.register_double(SOUND_LOOKUP, move |_, a| {
            assert_eq!(a[0], record);
            record.into_ret()
        });
        assert!(!e.call(0x0055_3bb0, &args![object]).bool());
        e.mem.set_u32(object + 0x1c, record);
        assert!(!e.call(0x0055_3bb0, &args![object]).bool());
        e.mem.set_u32(record + 4, 3);
        assert!(e.call(0x0055_3bb0, &args![object]).bool());
        e.mem.set_u32(record + 4, 1);
        assert!(!e.call(0x0055_3bb0, &args![object]).bool());
    }

    /// A form with the sound fields the walk reads.
    fn sound_form(e: &mut Engine, chance: i8, range: u8) -> u32 {
        let form = e.mem.alloc(0x80);
        let name = e.mem.alloc(8);
        e.mem.set_u8(name, b'x');
        e.mem.set_u32(form + 0x48, 2);
        e.mem.set_i8(form + 0x68, chance);
        e.mem.set_u8(form + 0x45, range);
        e.mem.set_u32(form + 0x70, name);
        e.mem.set_u32(form + 0x0c, 0xf0f0);
        form
    }

    /// A reference that qualifies for a sound at distance `distance`.
    fn sound_reference(e: &mut Engine, form: u32, distance: f32) -> u32 {
        let reference = object(e, 0x80);
        let record = e.mem.alloc(0x10);
        e.mem.set_u32(record + 4, 2);
        e.mem.set_u32(reference + 0x1c, record);
        e.mem.set_u32(reference + 0x30, form);
        e.mem.set_f32(reference + 0x34, distance);
        e.mem.set_u32(reference + 0x38, 0x3d3d);
        e.mem.set_f32(reference + 0x40, 1.0);
        e.mem.set_f32(reference + 0x44, 2.0);
        e.mem.set_f32(reference + 0x48, 3.0);
        reference
    }

    /// Doubles for `UpdateRefSounds`; the reference words: `+0x20` disabled,
    /// `+0x24` destroyed, `+0x28`/`+0x2c` the virtual answers.
    fn sound_setup(e: &mut Engine, items: &[u32]) -> Ptr<TESObjectCELL> {
        list_doubles(e);
        let cell = e.new_object::<TESObjectCELL>();
        let list = list_of(e, items);
        quiet(
            e,
            &[
                CELL_LOCK_ENTER,
                CELL_LOCK_LEAVE,
                AUDIO_GET_SOUND_HANDLE,
                SOUND_HANDLE_FOLLOW,
                SOUND_HANDLE_SET_POSITION,
                SOUND_HANDLE_SET_PRIORITY,
                SOUND_HANDLE_PLAY,
                EMPTY_FUNCTION,
            ],
        );
        returns(e, CELL_IS_LOADED, 1);
        returns(e, CELL_REFERENCES, list);
        returns(e, AUDIO_GET, 0xa0d0);
        e.register(REFERENCE_IS_DISABLED, |e, a| {
            e.mem.u32(a[0] + 0x20).into_ret()
        });
        e.register(REFERENCE_IS_DESTROYED, |e, a| {
            e.mem.u32(a[0] + 0x24).into_ret()
        });
        e.register(fake(0x21c), |e, a| e.mem.u32(a[0] + 0x28).into_ret());
        e.register(fake(0x22c), |e, a| e.mem.u32(a[0] + 0x2c).into_ret());
        e.register(fake(0x1d0), |e, a| e.mem.u32(a[0] + 0x38).into_ret());
        e.register(fake(0x1f4), |_, a| (a[0] + 0x40).into_ret());
        e.register(REFERENCE_BASE_FORM, |e, a| {
            e.mem.u32(a[0] + 0x30).into_ret()
        });
        e.register(FORM_SOUND_NAME, |e, a| e.mem.u32(a[0] + 0x70).into_ret());
        e.register(FORM_ID, |e, a| e.mem.u32(a[0] + 0x0c).into_ret());
        e.register(SOUND_LOOKUP, |_, a| a[0].into_ret());
        e.register(SOUND_CHANCE_TEST, |_, a| {
            u32::from(f32::from_bits(a[0]) > 0.5).into_ret()
        });
        e.register(REFERENCE_DISTANCE, |e, a| e.mem.f32(a[1] + 0x34).into_ret());
        e.set_global(SOUND_CHANCE_DIVISOR, 100.0f64);
        e.set_global(DISTANCE_SOURCE_SINGLETON, 0xd157u32);
        cell
    }

    #[test]
    fn ref_sounds_start_the_sound_of_a_close_reference_with_a_chance() {
        let mut e = engine();
        let form = sound_form(&mut e, 100, 3);
        let close = sound_reference(&mut e, form, 299.9);
        let disabled = sound_reference(&mut e, form, 1.0);
        e.mem.set_u32(disabled + 0x20, 1);
        let destroyed = sound_reference(&mut e, form, 1.0);
        e.mem.set_u32(destroyed + 0x24, 1);
        let unlikely_form = sound_form(&mut e, 10, 3);
        let unlikely = sound_reference(&mut e, unlikely_form, 1.0);
        let far = sound_reference(&mut e, form, 350.5);
        let after_the_end = sound_reference(&mut e, form, 1.0);
        let cell = sound_setup(
            &mut e,
            &[disabled, destroyed, unlikely, far, close, 0, after_the_end],
        );
        start_log(&mut e);
        e.call(0x0055_38e0, &args![cell]);
        let log = calls(&e);
        // Only the close reference starts a sound: form id and flags 0x2102.
        let started = calls_to(&log, AUDIO_GET_SOUND_HANDLE);
        assert_eq!(started.len(), 1);
        assert_eq!(started[0][0], 0xa0d0);
        assert_eq!(&started[0][2..], &[0xf0f0, 0x2102]);
        let handle = started[0][1];
        // It follows the reference's 3D, is placed at the reference's
        // position, gets priority 0xc0, is played and released.
        assert_eq!(
            calls_to(&log, SOUND_HANDLE_FOLLOW),
            vec![vec![handle, 0x3d3d]]
        );
        assert_eq!(
            calls_to(&log, SOUND_HANDLE_SET_POSITION),
            vec![vec![
                handle,
                1.0f32.to_bits(),
                2.0f32.to_bits(),
                3.0f32.to_bits()
            ]]
        );
        assert_eq!(
            calls_to(&log, SOUND_HANDLE_SET_PRIORITY),
            vec![vec![handle, 0xc0]]
        );
        assert_eq!(calls_to(&log, SOUND_HANDLE_PLAY), vec![vec![handle, 0]]);
        assert_eq!(calls_to(&log, EMPTY_FUNCTION), vec![vec![handle]]);
        // The chance given to the test is chance / divisor; the references
        // after the empty node are never looked at.
        let chances: Vec<f32> = calls_to(&log, SOUND_CHANCE_TEST)
            .iter()
            .map(|c| f32::from_bits(c[0]))
            .collect();
        assert_eq!(chances, vec![0.1, 1.0, 1.0]);
        assert_eq!(*addresses(&log).first().unwrap(), CELL_LOCK_ENTER);
        assert_eq!(*addresses(&log).last().unwrap(), CELL_LOCK_LEAVE);
        assert!(calls_to(&log, REFERENCE_IS_DISABLED)
            .iter()
            .all(|c| c[0] != after_the_end));
    }

    #[test]
    fn ref_sounds_skip_references_the_virtuals_exclude() {
        let mut e = engine();
        let form = sound_form(&mut e, 100, 3);
        let excluded = sound_reference(&mut e, form, 1.0);
        e.mem.set_u32(excluded + 0x28, 1);
        e.mem.set_u32(excluded + 0x2c, 1);
        let allowed = sound_reference(&mut e, form, 1.0);
        e.mem.set_u32(allowed + 0x28, 1);
        let unflagged = sound_reference(&mut e, form, 1.0);
        e.mem.set_u32(unflagged + 0x1c, 0);
        let nameless_form = sound_form(&mut e, 100, 3);
        let name = e.mem.u32(nameless_form + 0x70);
        e.mem.set_u8(name, 0);
        let nameless = sound_reference(&mut e, nameless_form, 1.0);
        let formless = sound_reference(&mut e, 0, 1.0);
        let cell = sound_setup(&mut e, &[excluded, unflagged, nameless, formless, allowed]);
        start_log(&mut e);
        e.call(0x0055_38e0, &args![cell]);
        let log = calls(&e);
        assert_eq!(calls_to(&log, AUDIO_GET_SOUND_HANDLE).len(), 1);
        // The excluded one was asked with argument 0 and no further.
        assert_eq!(calls_to(&log, fake(0x22c))[0], vec![excluded, 0]);
        // An unloaded cell does nothing but the lock.
        returns(&mut e, CELL_IS_LOADED, 0);
        start_log(&mut e);
        e.call(0x0055_38e0, &args![cell]);
        assert_eq!(
            addresses(&calls(&e)),
            vec![CELL_LOCK_ENTER, CELL_IS_LOADED, CELL_LOCK_LEAVE]
        );
    }

    #[test]
    fn truncation_to_the_low_word_follows_the_x87_rules() {
        assert_eq!(truncated_low_word(299.9), 299);
        assert_eq!(truncated_low_word(-1.5), 0xffff_ffff);
        assert_eq!(truncated_low_word(f32::NAN), 0);
        assert_eq!(truncated_low_word(1.0e30), 0);
        assert_eq!(truncated_low_word(4.0e9), 4_000_000_000);
    }

    /// Doubles for the ray tests: two reference nodes, the first without 3D.
    fn ray_setup(e: &mut Engine) -> (Ptr<TESObjectCELL>, u32) {
        list_doubles(e);
        let cell = e.new_object::<TESObjectCELL>();
        let without = object(e, 0x40);
        let with = object(e, 0x40);
        e.mem.set_u32(with + 0x20, 0x3d3d);
        let list = list_of(e, &[without, with]);
        returns(e, CELL_REFERENCE_NODES, list);
        e.register(fake(0x1d0), |e, a| e.mem.u32(a[0] + 0x20).into_ret());
        e.register(RAY_TEST_BOUNDED, |_, a| {
            u32::from(a[4] == 0x3d3d).into_ret()
        });
        returns(e, NODE_POSITION_A, 0x11);
        returns(e, NODE_POSITION_B, 0x22);
        let position = e.mem.alloc(16);
        e.mem.set_f32(position, 1.0);
        e.mem.set_f32(position + 4, 2.0);
        e.mem.set_f32(position + 8, 3.0);
        returns(e, NODE_POSITION_C, position);
        e.register(VECTOR_SUBTRACT, |e, a| {
            for i in 0..3 {
                let word = e.mem.u32(a[0] + 4 * i);
                e.mem.set_u32(a[1] + 4 * i, word);
            }
            a[1].into_ret()
        });
        e.register(VECTOR_LENGTH, |e, a| e.mem.f32(a[0] + 8).into_ret());
        (cell, with)
    }

    #[test]
    fn ray_test_over_reference_nodes_checks_the_distance_to_the_node() {
        let mut e = engine();
        let (cell, with) = ray_setup(&mut e);
        let object_pointer = e.mem.alloc(0x40);
        start_log(&mut e);
        assert!(e
            .call(
                0x0055_3bf0,
                &args![cell, object_pointer, 7u32, 8u32, 10.0f32]
            )
            .bool());
        let log = calls(&e);
        // The bounded test got (object, a, b, limit, 3D) for the node with
        // a 3D only; the position chain starts at the object.
        assert_eq!(
            calls_to(&log, RAY_TEST_BOUNDED),
            vec![vec![object_pointer, 7, 8, 10.0f32.to_bits(), 0x3d3d]]
        );
        assert_eq!(calls_to(&log, NODE_POSITION_A), vec![vec![object_pointer]]);
        assert_eq!(calls_to(&log, NODE_POSITION_B), vec![vec![0x11, 0]]);
        assert_eq!(calls_to(&log, NODE_POSITION_C), vec![vec![0x22]]);
        // The subtraction takes `a` (the first word after the object).
        assert_eq!(calls_to(&log, VECTOR_SUBTRACT)[0][2], 7);
        // A limit below the length (3.0) fails; the list ends.
        start_log(&mut e);
        assert!(!e
            .call(
                0x0055_3bf0,
                &args![cell, object_pointer, 7u32, 8u32, 3.0f32]
            )
            .bool());
        // A null object and an empty list fail without a test.
        start_log(&mut e);
        assert!(!e
            .call(0x0055_3bf0, &args![cell, 0u32, 7u32, 8u32, 10.0f32])
            .bool());
        assert!(calls_to(&calls(&e), RAY_TEST_BOUNDED).is_empty());
        let empty = e.mem.alloc(8);
        returns(&mut e, CELL_REFERENCE_NODES, empty);
        start_log(&mut e);
        assert!(!e
            .call(
                0x0055_3bf0,
                &args![cell, object_pointer, 7u32, 8u32, 10.0f32]
            )
            .bool());
        assert!(calls_to(&calls(&e), RAY_TEST_BOUNDED).is_empty());
        let _ = with;
    }

    #[test]
    fn ray_test_over_reference_nodes_rejects_nodes_the_bounded_test_rejects() {
        let mut e = engine();
        let (cell, _with) = ray_setup(&mut e);
        e.register(RAY_TEST_BOUNDED, |_, _| Ret::default());
        let object_pointer = e.mem.alloc(0x40);
        start_log(&mut e);
        assert!(!e
            .call(
                0x0055_3bf0,
                &args![cell, object_pointer, 7u32, 8u32, 10.0f32]
            )
            .bool());
        assert!(calls_to(&calls(&e), NODE_POSITION_A).is_empty());
    }

    #[test]
    fn mask_selects_the_child_nodes_that_are_ray_tested() {
        let mut e = engine();
        let cell = e.new_object::<TESObjectCELL>();
        e.register(CELL_CHILD_NODE, |_, a| (0x1000 + a[1]).into_ret());
        e.register(RAY_TEST_NODE, |_, a| u32::from(a[4] == 0x1007).into_ret());
        let object_pointer = e.mem.alloc(0x40);
        let run = |e: &mut Engine, mask: u32| {
            start_log(e);
            let hit = e
                .call(
                    0x0055_3d00,
                    &args![cell, object_pointer, 7u32, 8u32, 10.0f32, mask],
                )
                .bool();
            let tested: Vec<u32> = calls_to(&calls(e), RAY_TEST_NODE)
                .iter()
                .map(|c| c[4] - 0x1000)
                .collect();
            (hit, tested)
        };
        assert_eq!(run(&mut e, 0x09), (true, vec![0, 3, 7]));
        assert_eq!(run(&mut e, 0x02), (false, vec![1]));
        assert_eq!(run(&mut e, 0x04), (false, vec![2]));
        assert_eq!(run(&mut e, 0x10), (true, vec![4, 7]));
        assert_eq!(run(&mut e, 0x01), (false, vec![0]));
        assert_eq!(run(&mut e, 0x1f), (true, vec![0, 1, 2, 3, 4, 7]));
        assert_eq!(run(&mut e, 0x00), (false, vec![]));
        // The arguments of a test: (object, a, b, limit, node, 0).
        start_log(&mut e);
        e.call(
            0x0055_3d00,
            &args![cell, object_pointer, 7u32, 8u32, 10.0f32, 2u32],
        );
        assert_eq!(
            calls_to(&calls(&e), RAY_TEST_NODE),
            vec![vec![object_pointer, 7, 8, 10.0f32.to_bits(), 0x1001, 0]]
        );
        // A null object never tests.
        start_log(&mut e);
        assert!(!e
            .call(0x0055_3d00, &args![cell, 0u32, 7u32, 8u32, 10.0f32, 1u32])
            .bool());
        assert!(calls_to(&calls(&e), RAY_TEST_NODE).is_empty());
    }

    #[test]
    fn pick_answers_the_cell_node_for_type_0x11_and_the_fallback_otherwise() {
        let mut e = engine();
        let cell = e.new_object::<TESObjectCELL>();
        let world = object(&mut e, 0x40);
        e.register(fake(0xc8), |e, a| e.mem.u32(a[0] + 0x30).into_ret());
        returns(&mut e, CELL_HAVOK_WORLD, 0);
        returns(&mut e, PICK_WORLD_SHAPE, 0x51);
        returns(&mut e, CELL_NODE_OF_CELL, 0xc311);
        returns(&mut e, PICK_FALLBACK_NODE, 0xfa11);
        e.register(PICK_FIND_ENTRY, |e, a| {
            assert_eq!(a[0], 0x51);
            e.mem.u32(0x7f00_0800).into_ret()
        });
        e.register(PICK_ENTRY_SOURCE, |_, a| a[1].into_ret());
        e.register(PICK_ENTRY_TYPE, |e, _| e.mem.u32(0x7f00_0804).into_ret());
        // No world.
        assert_eq!(e.call(0x0055_3ee0, &args![cell, 0x9999u32]).u32(), 0);
        // The world refuses the query.
        returns(&mut e, CELL_HAVOK_WORLD, world);
        assert_eq!(e.call(0x0055_3ee0, &args![cell, 0x9999u32]).u32(), 0);
        // The world accepts, the entry has the own-node type.
        e.mem.set_u32(world + 0x30, 1);
        e.mem.set_u32(0x7f00_0800, 0x77);
        e.mem.set_u32(0x7f00_0804, 0x11);
        assert_eq!(e.call(0x0055_3ee0, &args![cell, 0x9999u32]).u32(), 0xc311);
        // Another type, and no entry at all.
        e.mem.set_u32(0x7f00_0804, 0x12);
        assert_eq!(e.call(0x0055_3ee0, &args![cell, 0x9999u32]).u32(), 0xfa11);
        e.mem.set_u32(0x7f00_0800, 0);
        e.mem.set_u32(0x7f00_0804, 0x11);
        assert_eq!(e.call(0x0055_3ee0, &args![cell, 0x9999u32]).u32(), 0xfa11);
    }
}
