//! `fallout shared/tes.cpp` (Xbox PDB source unit), part 2: its functions from `004579e0` up to
//! (not including) `004fd3e0` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::tes`]; anything public there may be used here.
//!
//! Session 1 of this part (the first 40 functions, `004579e0` to `00458ad0`)
//! holds three groups:
//!
//! - the geometry copy helpers (`MakeNewGeomData`, `CreateDeepCopySameTextures`)
//!   and `TES::InitModelsToLoad`, which loads the cells around the player and
//!   shows the loading menu;
//! - the loading-menu glue (`00457d70` and the small accessors on the loading
//!   menu singleton at `011da0c0`, whose word at `+0x222` holds menu state
//!   bits), `TES::SetWorldSpace`, and `TES::Pick`, `TES::Pick_ov2` and
//!   `TES::PickNI` (the Havok ray pick that `TES::Pick_ov2` falls back on);
//! - the `bhkPickData` vector helpers: SSE `hkVector4` operations on 16-byte
//!   vectors (copy of one lane, add, subtract, absolute value, compare, sign
//!   mask) that the pick code uses. They are translated lane by lane in `f32`,
//!   which is what the SSE instructions compute.
//!
//! Not translated: the compiler's exception-unwinding frames, the stack
//! cookie checks and the 16-byte stack realignment prologues.
//!
//! The next session continues at `00458b20`.

#[allow(unused_imports)]
use super::tes::*;
#[allow(unused_imports)]
use crate::prelude::*;

/// The loading menu singleton (written by the menu's `Create`, `00788daa`).
const LOADING_MENU: u32 = 0x011d_a0c0;
/// The HUD main menu singleton (written by the `Create` at `0076c0eb`).
const HUD_MAIN_MENU: u32 = 0x011d_96c0;
/// Tick count (ms) at which the loading menu may be closed even if a hide
/// request did not come (0 = none set).
const LOADING_MENU_DEADLINE: u32 = 0x011c_3efc;
/// `bShowLoadingAreaMessage:General` (an `INISetting`).
const SETTING_SHOW_LOADING_AREA_MESSAGE: u32 = 0x011c_3dbc;
/// `bPreemptivelyUnloadCells:General`.
const SETTING_PREEMPTIVELY_UNLOAD_CELLS: u32 = 0x011c_3d40;
/// `uGridsToLoad:General`.
const SETTING_GRIDS_TO_LOAD: u32 = 0x011c_63cc;
/// Returns a pointer to an unsigned setting's value.
const SETTING_VALUE_ADDRESS_INT: u32 = 0x0043_d4d0;
/// Returns a pointer to a boolean setting's value.
const SETTING_VALUE_ADDRESS_BYTE: u32 = 0x0040_8d60;
/// A `float` the pick code multiplies Havok coordinates by (set at start-up
/// by `00f38070` to the reciprocal of `01017820`).
const HAVOK_SCALE: u32 = 0x011c_3d18;
/// `0.001f`: how close a pick vector must be to zero to count as empty.
const PICK_EPSILON: u32 = 0x0101_7d00;
/// Four words `0x7fffffff`: the mask that clears the sign of a vector's lanes.
const ABS_MASK: u32 = 0x0101_7d10;
/// A byte `TES::SetWorldSpace` stores at its end (see `fn_004583f0`): set when
/// `005862e0` says the new world space is false.
const WORLD_SPACE_FLAG: u32 = 0x011a_d80c;
/// The `TES` singleton (`TES::this` of the game; its `+0x64` is the water system).
const TES_OBJECT: u32 = 0x011d_ea10;
/// The `TESDataHandler` singleton.
const DATA_HANDLER: u32 = 0x011c_3f2c;
/// Source path passed to the scope guard (`D:\_Fallout3\...\TES.cpp`).
const TES_CPP_PATH: u32 = 0x0101_7874;

/// `GetTickCount` (import slot).
const GET_TICK_COUNT: u32 = 0x00fd_f060;
/// `NiPointer<T>::NiPointer(T*)` (`ecx` = the pointer cell; takes a reference).
const NI_POINTER_CTOR: u32 = 0x0063_3c90;
/// `NiPointer<T>::operator T*` (returns the stored pointer).
const NI_POINTER_GET: u32 = 0x0055_9450;
/// `NiPointer<T>::~NiPointer` (releases the reference).
const NI_POINTER_DTOR: u32 = 0x0045_cec0;
/// `NiPointer<T>::operator=(T*)`.
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
/// The scope guard around allocations made under a memory tag.
const SCOPE_GUARD_CTOR: u32 = 0x0040_4eb0;
const SCOPE_GUARD_DTOR: u32 = 0x0040_4ee0;
/// A trivial constructor of an `hkVector4`/`NiPoint3`: returns `this`.
const VECTOR_CTOR: u32 = 0x0068_15c0;
/// Copies the 16 bytes at the argument into `this` (`hkVector4` assignment).
const VECTOR_COPY: u32 = 0x004a_3c90;
/// `TESForm::GetFormType` (the form-type byte at `+4`).
const FORM_GET_TYPE: u32 = 0x0040_1170;
/// `TESObjectCELL::IsInterior` (bit 0 of the cell flags at `+0x24`).
const CELL_IS_INTERIOR: u32 = 0x0042_5fd0;
/// `TESObjectCELL::GetWorldSpace` (Xbox PDB).
const CELL_GET_WORLD_SPACE: u32 = 0x0054_ddd0;
/// `TESObjectREFR::GetWorldSpace` (Xbox PDB).
const REFERENCE_GET_WORLD_SPACE: u32 = 0x0057_5d70;
/// Returns `this->pInteriorCell` (the map names this `ActorMover::GetPreferredMoveMode`).
const GET_INTERIOR_CELL: u32 = 0x005f_36f0;
/// `TES::GetWorldSpace` (`004fd3e0`, in the next part of this unit).
const GET_WORLD_SPACE: u32 = 0x004f_d3e0;
/// `GridCellArray::Get` (Xbox PDB): pointer to the cell slot at (x, y).
const GRID_CELL_ARRAY_GET: u32 = 0x004b_a490;
/// `GridCellArray::GetAttached` (Xbox PDB).
const GRID_CELL_ARRAY_GET_ATTACHED: u32 = 0x004b_a5a0;
/// `TESWorldSpace::LoadCell` (Xbox PDB).
const WORLD_SPACE_LOAD_CELL: u32 = 0x0058_5b30;
/// `TESDataHandler` function that creates the cell at (x, y) in a world space.
const DATA_HANDLER_CREATE_CELL: u32 = 0x0046_1c20;
/// `TESObjectCELL::Pick` (Xbox PDB).
const CELL_PICK: u32 = 0x0055_3ee0;

/// Reads `uGridsToLoad`, the way the code does: one call per use.
fn grids_to_load(e: &mut Engine) -> u32 {
    let value = e
        .call(SETTING_VALUE_ADDRESS_INT, &args![SETTING_GRIDS_TO_LOAD])
        .u32();
    e.mem.u32(value)
}

/// The stored pointer of the `NiPointer` at `cell`.
fn ni_pointer_get(e: &mut Engine, cell: Ptr) -> u32 {
    e.call(NI_POINTER_GET, &args![cell]).u32()
}

// Translated from 004579e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Passes its `float` on to `004019b0` (which hands it to `004019d0`, a
/// wrapper over the CRT routine `00ec6054` taking a `double`) and returns that
/// function's `float` result (x87 `ST0`).
pub fn fn_004579e0(e: &mut Engine, value: f32) -> f32 {
    e.call(0x0040_19b0, &args![value]).f32()
}

// Translated from 00457a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MakeNewGeomData` (Xbox PDB): gives a scene-graph node its own copy of its
/// geometry data. For a node whose slot `0x18` returns an object (`geometry`),
/// the `NiPointer` at `geometry + 0xB8` is read; when the pointed-to object's
/// word at `+0xE` has none of the bits `0x7000` and
/// `BGSSaveFormBuffer::GetForm` (`007af430`) says it has a form, the object is
/// deep-copied (`NiObject::CreateDeepCopy`) and the copy is installed through
/// `geometry`'s slot `0xE4`; otherwise the original is installed through the
/// same slot. For any other node the node's slot `0xC` returns a child list
/// and every child is processed the same way, last to first.
///
/// Not translated: the compiler's exception-unwinding frame.
pub fn make_new_geom_data(e: &mut Engine, node: Ptr) {
    e.with_stack(4, |e, copy| {
        e.call(NI_POINTER_CTOR, &args![copy, 0u32]);
        if !node.is_null() {
            let geometry = e.vcall(node.addr(), 0x18, &[]).u32();
            if geometry != 0 {
                let shape = e.call(0x0054_95f0, &args![geometry]).u32();
                e.with_stack(4, |e, held| {
                    e.call(NI_POINTER_CTOR, &args![held, shape]);
                    if ni_pointer_get(e, held) != 0 {
                        let object = ni_pointer_get(e, held);
                        let mut copied = false;
                        if fn_00457b80(e, Ptr::new(object)) == 0 {
                            let object = ni_pointer_get(e, held);
                            if e.call(0x007a_f430, &args![object]).u32() != 0 {
                                let object = ni_pointer_get(e, held);
                                e.call(0x00a5_d510, &args![object, copy]);
                                let new_object = ni_pointer_get(e, copy);
                                e.call(NI_POINTER_ASSIGN, &args![held, new_object]);
                                let installed = ni_pointer_get(e, held);
                                e.vcall(geometry, 0xe4, &args![installed]);
                                copied = true;
                            }
                        }
                        if !copied {
                            let installed = ni_pointer_get(e, held);
                            e.vcall(geometry, 0xe4, &args![installed]);
                        }
                    }
                    e.call(NI_POINTER_DTOR, &args![held]);
                });
            } else {
                let children = e.vcall(node.addr(), 0xc, &[]).u32();
                if children != 0 {
                    let mut count = e.call(0x0043_b480, &args![children]).u32();
                    while count != 0 {
                        count -= 1;
                        let child = e.call(0x0043_b4a0, &args![children, count]).u32();
                        make_new_geom_data(e, Ptr::new(child));
                    }
                }
            }
        }
        e.call(NI_POINTER_DTOR, &args![copy]);
    });
}

// Translated from 00457b80 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `+0xE` of the object, masked with `0x7000`.
pub fn fn_00457b80(e: &mut Engine, this: Ptr) -> u16 {
    e.mem.u16(this.addr() + 0xe) & 0x7000
}

// Translated from 00457ba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TES::CreateDeepCopySameTextures` (Xbox PDB): for a non-null `node`, clones
/// it (`NiObject::Clone`, `00a5d2c0`, with `clone_argument`) and runs
/// [`make_new_geom_data`] on the clone; returns the clone (null for a null
/// node). `this` is not used.
pub fn tes_create_deep_copy_same_textures(
    e: &mut Engine,
    _this: Ptr<TES>,
    node: Ptr,
    clone_argument: u32,
) -> Ptr {
    let mut copy = Ptr::NULL;
    if !node.is_null() {
        copy = e.call(0x00a5_d2c0, &args![node, clone_argument]).ptr();
        make_new_geom_data(e, copy);
    }
    copy
}

// Translated from 00457be0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TES::InitModelsToLoad` (Xbox PDB): walks the `uGridsToLoad` x
/// `uGridsToLoad` square of cells around the current grid position. A cell is
/// visited when `mode` is 0 and the grid array has no attached cell there,
/// when `mode` is 1 and the grid array has no cell there, or for any other
/// `mode`. For each visited cell, with a world space present, the world
/// space's `LoadCell` (or, when it gives none, the data handler's cell
/// creation) gives a cell; if `00450fb0` is false for that cell, `00454400`
/// is run on it and its result counted. When anything was counted, the
/// loading menu code [`fn_00457d70`] runs with (show, world space, 0).
pub fn tes_init_models_to_load(e: &mut Engine, this: Ptr<TES>, mode: u32) {
    let mut counted = 0u32;
    let half = grids_to_load(e) >> 1;
    let first_x = e.get(this, TES::iCurrentGridX).wrapping_sub(half as i32);
    let half = grids_to_load(e) >> 1;
    let first_y = e.get(this, TES::iCurrentGridY).wrapping_sub(half as i32);
    let grid_array = e.get(this, TES::pGridCellA);

    let mut x = 0u32;
    while x < grids_to_load(e) {
        let mut y = 0u32;
        while y < grids_to_load(e) {
            let skip = match mode {
                0 => e
                    .call(GRID_CELL_ARRAY_GET_ATTACHED, &args![grid_array, x, y])
                    .bool(),
                1 => e.call(GRID_CELL_ARRAY_GET, &args![grid_array, x, y]).u32() != 0,
                _ => false,
            };
            if !skip && e.call(GET_WORLD_SPACE, &args![this]).u32() != 0 {
                let cell_x = first_x.wrapping_add(x as i32);
                let cell_y = first_y.wrapping_add(y as i32);
                let world_space = e.get(this, TES::pWorldSpace);
                let mut cell = e
                    .call(WORLD_SPACE_LOAD_CELL, &args![world_space, cell_x, cell_y])
                    .u32();
                if cell == 0 {
                    let world_space = e.call(GET_WORLD_SPACE, &args![this]).u32();
                    let data_handler: u32 = e.global(DATA_HANDLER);
                    cell = e
                        .call(
                            DATA_HANDLER_CREATE_CELL,
                            &args![data_handler, cell_x, cell_y, world_space, 1u32],
                        )
                        .u32();
                }
                if cell != 0 && !e.call(0x0045_0fb0, &args![cell]).bool() {
                    counted = counted.wrapping_add(e.call(0x0045_4400, &args![cell]).u32());
                }
            }
            y += 1;
        }
        x += 1;
    }
    if counted != 0 {
        let world_space = e.get(this, TES::pWorldSpace);
        fn_00457d70(e, this, 1, world_space, 0);
    }
}

// Translated from 00457d70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Shows (`show` != 0) or hides the loading menu. Returns true only when it
/// started showing it.
///
/// Hiding: the menu is closed (deadline cleared, menu bit 10 cleared,
/// `Interface::CloseLoadingMenu`, and the HUD main menu mode set to 2 unless
/// it is already `0x18`) when bit 10 is clear, or when it is set and
/// `force_close` is; otherwise a deadline five seconds from now is set the
/// first time, and the menu is closed once the tick count passes it.
///
/// Showing: if the loading menu is visible and bit 3 is clear, only
/// [`fn_00458180`] runs (with `form`). Otherwise, unless `00451530` says no,
/// the sky is told (`0063e010`), all sounds of type `0x14` end, and for a
/// `form` the game looks up a world space through its type (`0x39`: a cell,
/// replaced by its world space unless it is an interior; `0x3a` to `0x40` and
/// `0x69`: a reference, whose cell's or own world space is looked up, a
/// result the game then never uses), `Interface::CreateLoadingMenu(form,
/// force_close)` runs, and the state words of the `00aff100` object are
/// switched ([`fn_00458010`], its slot `0x30`, [`fn_00457ff0`]).
pub fn fn_00457d70(e: &mut Engine, this: Ptr<TES>, show: u8, form: Ptr, force_close: u8) -> bool {
    if show == 0 {
        if !fn_004580e0(e) || (fn_004580e0(e) && force_close != 0) {
            close_loading_menu(e);
        } else {
            if e.global::<u32>(LOADING_MENU_DEADLINE) == 0 {
                let now = fn_00457fe0(e);
                e.set_global(LOADING_MENU_DEADLINE, now.wrapping_add(5000));
            }
            let now = fn_00457fe0(e);
            if now > e.global::<u32>(LOADING_MENU_DEADLINE) {
                close_loading_menu(e);
            }
        }
        return false;
    }

    if e.call(0x0070_5e80, &[]).bool() && !fn_00458140(e) {
        fn_00458180(e, form);
        return false;
    }
    if e.call(0x0045_1530, &args![this]).bool() {
        return false;
    }
    let sky = e.get(this, TES::pSky);
    e.call(0x0063_e010, &args![sky]);
    // `BSAudio::QInstance` takes no argument; the 0x14 is the sound type that
    // `BSAudio::EndAllSoundsOfType` (`00ad8780`) takes.
    let audio = e.call(0x0045_3a70, &[]).u32();
    e.call(0x00ad_8780, &args![audio, 0x14u32]);

    let mut form = form;
    if !form.is_null() {
        let form_type = e.call(FORM_GET_TYPE, &args![form]).u32();
        match form_type.wrapping_sub(0x39) {
            0 => {
                if !e.call(CELL_IS_INTERIOR, &args![form]).bool() {
                    form = e.call(CELL_GET_WORLD_SPACE, &args![form]).ptr();
                }
            }
            1..=7 | 0x30 => {
                let cell = e.call(0x008d_6f30, &args![form]).u32();
                if cell != 0 {
                    if !e.call(CELL_IS_INTERIOR, &args![cell]).bool() {
                        e.call(CELL_GET_WORLD_SPACE, &args![cell]);
                    }
                } else {
                    e.call(REFERENCE_GET_WORLD_SPACE, &args![form]);
                }
            }
            _ => {}
        }
    }
    e.call(0x0070_5e00, &args![form, force_close as u32]);
    let menu_state = e.call(0x00af_f100, &[]).ptr();
    fn_00458010(e, menu_state);
    let object = e.call(0x00af_f100, &[]).u32();
    e.vcall(object, 0x30, &[]);
    let menu_state = e.call(0x00af_f100, &[]).ptr();
    fn_00457ff0(e, menu_state);
    true
}

/// The closing sequence of the loading menu, shared by both hide paths of
/// [`fn_00457d70`].
fn close_loading_menu(e: &mut Engine) {
    e.set_global(LOADING_MENU_DEADLINE, 0u32);
    fn_00458060(e, 0);
    e.call(0x0070_5e30, &[]);
    if fn_00458030(e) != 0x18 {
        e.call(0x0077_1700, &args![2u32]);
    }
}

// Translated from 00457fe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `GetTickCount` (through its import slot).
pub fn fn_00457fe0(e: &mut Engine) -> u32 {
    e.call(GET_TICK_COUNT, &[]).u32()
}

// Translated from 00457ff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Changes the state word at `+0x5C` from 1 to 2 (any other value stays).
pub fn fn_00457ff0(e: &mut Engine, this: Ptr) {
    if e.mem.u32(this.addr() + 0x5c) == 1 {
        e.mem.set_u32(this.addr() + 0x5c, 2);
    }
}

// Translated from 00458010 (decompiled, FalloutNV.exe 1.4.0.525)
/// Changes the state word at `+0x5C` from 2 to 1 (any other value stays).
pub fn fn_00458010(e: &mut Engine, this: Ptr) {
    if e.mem.u32(this.addr() + 0x5c) == 2 {
        e.mem.set_u32(this.addr() + 0x5c, 1);
    }
}

// Translated from 00458030 (decompiled, FalloutNV.exe 1.4.0.525)
/// The HUD main menu's mode (`+0x1C4`), 0 when there is no HUD main menu.
pub fn fn_00458030(e: &mut Engine) -> u32 {
    let menu: u32 = e.global(HUD_MAIN_MENU);
    if menu == 0 {
        0
    } else {
        e.mem.u32(menu + 0x1c4)
    }
}

// Translated from 00458060 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears bit 10 of the loading menu's state word (if there is a
/// loading menu).
pub fn fn_00458060(e: &mut Engine, set: u8) {
    let menu: u32 = e.global(LOADING_MENU);
    if menu != 0 {
        fn_00458080(e, Ptr::new(menu), 10, set);
    }
}

// Translated from 00458080 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`set` != 0) or clears bit `bit` (mod 32, as the shift does) of the
/// 16-bit state word at `+0x222` of the loading menu.
pub fn fn_00458080(e: &mut Engine, this: Ptr, bit: u32, set: u8) {
    let mask = 1u32 << (bit & 0x1f);
    let state = e.mem.u16(this.addr() + 0x222) as u32;
    let state = if set != 0 {
        state | mask
    } else {
        state & !mask
    };
    e.mem.set_u16(this.addr() + 0x222, state as u16);
}

// Translated from 004580e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit 10 of the loading menu's state word; false without a loading menu.
pub fn fn_004580e0(e: &mut Engine) -> bool {
    let menu: u32 = e.global(LOADING_MENU);
    menu != 0 && fn_00458110(e, Ptr::new(menu), 10)
}

// Translated from 00458110 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `bit` (mod 32) of the 16-bit state word at `+0x222` is set.
pub fn fn_00458110(e: &mut Engine, this: Ptr, bit: u32) -> bool {
    let state = e.mem.u16(this.addr() + 0x222) as u32;
    state & (1u32 << (bit & 0x1f)) != 0
}

// Translated from 00458140 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit 3 of the loading menu's state word; false without a loading menu.
pub fn fn_00458140(e: &mut Engine) -> bool {
    let menu: u32 = e.global(LOADING_MENU);
    menu != 0 && fn_00458110(e, Ptr::new(menu), 3)
}

// Translated from 00458180 (decompiled, FalloutNV.exe 1.4.0.525)
/// With a loading menu whose word at `+0x1E8` is 0 and a non-null `form`:
/// sets bit 3 of the state word and calls `00788ab0` on the menu with `form`.
pub fn fn_00458180(e: &mut Engine, form: Ptr) {
    let menu: u32 = e.global(LOADING_MENU);
    if menu != 0 && !form.is_null() && e.mem.u32(menu + 0x1e8) == 0 {
        fn_004581c0(e, 1);
        e.call(0x0078_8ab0, &args![menu, form]);
    }
}

// Translated from 004581c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears bit 3 of the loading menu's state word (if there is a
/// loading menu).
pub fn fn_004581c0(e: &mut Engine, set: u8) {
    let menu: u32 = e.global(LOADING_MENU);
    if menu != 0 {
        fn_00458080(e, Ptr::new(menu), 3, set);
    }
}

// Translated from 004581e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of the value of the INI setting
/// `bShowLoadingAreaMessage:General`. `this` and the one stack word are not
/// used.
pub fn fn_004581e0(e: &mut Engine, _this: u32, _unused_1: u32) -> u32 {
    e.call(
        SETTING_VALUE_ADDRESS_BYTE,
        &args![SETTING_SHOW_LOADING_AREA_MESSAGE],
    )
    .u32()
}

// Translated from 00458200 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TES::SetWorldSpace` (Xbox PDB): switches the world space (nothing for a
/// null one or the current one).
///
/// With `bPreemptivelyUnloadCells` set: if an interior cell is loaded
/// `00453940` runs first, the cells of the old world space are unloaded
/// (`00455200` with (1, old world space), plus `00454d50(1)` unless an
/// interior cell is loaded) and, if that unloaded anything, the unused
/// textures are cleaned up (`00452490(0)`). Then `pWorldSpace` is stored; the
/// word at `+0xD0` of the new world space ([`fn_00458400`]) is run through
/// `00526270` when set; `00453940`, `00450d80`, `007043c0` and
/// `Interface::SetRootMapSpace` (`00709d20`) run; and the water system's
/// `0052d3b0` is called with the new world space's texture (loaded through a
/// `NiPointer` and `004568c0` when the byte `00454b50` points at is set, else
/// with 0). The last step stores `005862e0(new) == 0` ([`fn_004583f0`]).
///
/// Not translated: the compiler's exception-unwinding frame.
pub fn tes_set_world_space(e: &mut Engine, this: Ptr<TES>, world_space: Ptr) {
    if world_space.is_null() || e.get(this, TES::pWorldSpace) == world_space {
        return;
    }
    let setting = e
        .call(
            SETTING_VALUE_ADDRESS_BYTE,
            &args![SETTING_PREEMPTIVELY_UNLOAD_CELLS],
        )
        .u32();
    if e.mem.u8(setting) != 0 {
        if e.call(GET_INTERIOR_CELL, &args![this]).u32() != 0 {
            e.call(0x0045_3940, &args![this]);
        }
        let old = e.get(this, TES::pWorldSpace);
        let mut unloaded = e.call(0x0045_5200, &args![this, 1u32, old]).u32();
        if e.call(GET_INTERIOR_CELL, &args![this]).u32() == 0 {
            unloaded = unloaded.wrapping_add(e.call(0x0045_4d50, &args![this, 1u32]).u32());
        }
        if unloaded != 0 {
            e.call(0x0045_2490, &args![this, 0u32]);
        }
    }
    e.set(this, TES::pWorldSpace, world_space);
    let extra = fn_00458400(e, world_space);
    if extra != 0 {
        e.call(0x0052_6270, &args![extra]);
    }
    let unknown = e.call(0x0067_33e0, &args![world_space]).u32();
    e.call(0x0058_eeb0, &args![unknown]);
    e.call(0x0045_3940, &args![this]);
    e.call(0x0045_0d80, &args![this]);
    e.call(0x0070_43c0, &[]);
    e.call(0x0070_9d20, &args![world_space]);

    let flag = e.call(0x0045_4b50, &args![world_space]).u32();
    let tes_object: u32 = e.global(TES_OBJECT);
    if e.mem.i8(flag) != 0 {
        e.with_stack(4, |e, guard| {
            e.call(
                SCOPE_GUARD_CTOR,
                &args![guard, 0x1du32, 1u32, TES_CPP_PATH, 0x1751u32],
            );
            e.with_stack(4, |e, texture| {
                e.call(NI_POINTER_CTOR, &args![texture, 0u32]);
                let name = e.call(0x0045_4b50, &args![world_space]).u32();
                e.call(0x0045_68c0, &args![tes_object, name, texture, 1u32, 1u32]);
                let loaded = ni_pointer_get(e, texture);
                let water = e.call(0x0070_ec90, &args![tes_object]).u32();
                e.call(0x0052_d3b0, &args![water, loaded]);
                e.call(NI_POINTER_DTOR, &args![texture]);
            });
            e.call(SCOPE_GUARD_DTOR, &args![guard]);
        });
    } else {
        let water = e.call(0x0070_ec90, &args![tes_object]).u32();
        e.call(0x0052_d3b0, &args![water, 0u32]);
    }
    let known = e.call(0x0058_62e0, &args![world_space]).bool();
    fn_004583f0(e, !known as u8);
}

// Translated from 004583f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores a byte in the global `011ad80c`.
pub fn fn_004583f0(e: &mut Engine, value: u8) {
    e.set_global(WORLD_SPACE_FLAG, value);
}

// Translated from 00458400 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `+0xD0` of a `TESWorldSpace` (what it holds is not confirmed;
/// [`tes_set_world_space`] passes it to `00526270` when it is not null).
pub fn fn_00458400(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0xd0)
}

// Translated from 00458420 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TES::Pick` (Xbox PDB): [`tes_pick_ov2`] with the cell-based pick enabled.
pub fn tes_pick(e: &mut Engine, this: Ptr<TES>, pick: Ptr) -> u32 {
    tes_pick_ov2(e, this, pick, 1)
}

// Translated from 00458440 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TES::Pick_ov2` (Xbox PDB). With `by_cell` == 0: [`tes_pick_ni`]. Otherwise
/// the pick runs on the interior cell or, without one, on the centre cell of
/// the grid (`uGridsToLoad / 2` in x and y, read from the grid array), through
/// `TESObjectCELL::Pick` (`00553ee0`); 0 when there is no such cell.
pub fn tes_pick_ov2(e: &mut Engine, this: Ptr<TES>, pick: Ptr, by_cell: u8) -> u32 {
    if by_cell == 0 {
        return tes_pick_ni(e, this, pick);
    }
    let mut cell = e.call(GET_INTERIOR_CELL, &args![this]).u32();
    if cell == 0 {
        let first = grids_to_load(e) >> 1;
        let second = grids_to_load(e) >> 1;
        let grid_array = e.get(this, TES::pGridCellA);
        let slot = e
            .call(GRID_CELL_ARRAY_GET, &args![grid_array, second, first])
            .u32();
        cell = e.mem.u32(slot);
    }
    if cell == 0 {
        0
    } else {
        e.call(CELL_PICK, &args![cell, pick]).u32()
    }
}

// Translated from 004584d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TES::PickNI` (Xbox PDB): the Havok ray described by a `bhkPickData`
/// (`pick`) run as a `NiPick` over the scene-graph root. The ray starts at the
/// pick data's `m_from` and ends at its `m_to` (`bhkPickData::CalcTo` first
/// completes it from `hkLength`), both scaled by the `float` at `011c3d18`
/// into game units; the direction is end minus start (`00439ef0`). A
/// `NiPick` (0x34 bytes, constructor `00e98f20` with (0, 8)) targets the root
/// (`0084e3a0`), gets its flag byte at `+0x11` set (`00458b30`), and picks
/// (`NiPick::PickObjects`). Returns the first result's object (through the
/// `NiPointer` getter `00458b50`), or 0 when nothing was hit.
///
/// Not translated: the compiler's exception-unwinding frame.
pub fn tes_pick_ni(e: &mut Engine, this: Ptr<TES>, pick: Ptr) -> u32 {
    // Locals: origin, direction, end and difference (12 bytes each), the NiPick.
    e.with_stack(0x70, |e, block| {
        let origin = block;
        let direction = block.byte_add(12);
        let end = block.byte_add(24);
        let difference = block.byte_add(36);
        let ni_pick = block.byte_add(48);
        e.call(VECTOR_CTOR, &args![direction]);
        e.call(VECTOR_CTOR, &args![origin]);
        e.call(VECTOR_CTOR, &args![end]);
        fn_004585f0(e, pick, origin);
        fn_004587f0(e, pick, end);
        let delta = e.call(0x0043_9ef0, &args![end, difference, origin]).u32();
        for word in 0..3 {
            let value = e.mem.u32(delta + 4 * word);
            e.mem.set_u32(direction.addr() + 4 * word, value);
        }
        e.call(0x00e9_8f20, &args![ni_pick, 0u32, 8u32]);
        let root = e.call(0x0084_e3a0, &args![this]).u32();
        e.call(0x0070_5fc0, &args![ni_pick, root]);
        e.call(0x0045_8b30, &args![ni_pick, 1u32]);
        let mut result = 0;
        if e.call(0x00e9_8e20, &args![ni_pick, origin, direction, 0u32])
            .bool()
        {
            let results = e.call(0x0050_0940, &args![ni_pick]).u32();
            let first = e.call(0x0096_8670, &args![results, 0u32]).u32();
            result = e.call(0x0045_8b50, &args![first]).u32();
        }
        e.call(0x00e9_8fa0, &args![ni_pick]);
        result
    })
}

// Translated from 004585f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the first three lanes of the vector at `this` (`m_from` of a
/// `bhkPickData`), scaled ([`fn_00458620`]), to the three floats at `out`.
/// Returns `out`.
pub fn fn_004585f0(e: &mut Engine, this: Ptr, out: Ptr) -> Ptr {
    let vector = e.call(VECTOR_CTOR, &args![this]).ptr();
    fn_00458620(e, out, vector)
}

// Translated from 00458620 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes lanes 0 to 2 of the vector at `vector`, each multiplied by the
/// `float` at `011c3d18` ([`fn_004587d0`]), to the three floats at `out`.
/// Returns `out`. (cdecl; the compiler realigns the stack to 16 bytes.)
pub fn fn_00458620(e: &mut Engine, out: Ptr, vector: Ptr) -> Ptr {
    e.with_stack(0x30, |e, temporaries| {
        for lane in 0..3u32 {
            let lane_copy = temporaries.byte_add(16 * lane);
            let broadcast = fn_00458700(e, vector, lane_copy, lane as i32);
            let value = fn_004586d0(e, broadcast);
            let scaled = fn_004587d0(e, value);
            e.mem.set_f32(out.addr() + 4 * lane, scaled);
        }
    });
    out
}

// Translated from 004586d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Lane 0 of the vector at `this`, as a `float`.
pub fn fn_004586d0(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr())
}

// Translated from 00458700 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores into the 16-byte vector at `out` (through the vector copy
/// `004a3c90`) lane `index` of the vector at `this` repeated in all four
/// lanes (`SHUFPS`); for `index` 0 the vector at `this` is copied as it is.
/// Any `index` above 2 gives lane 3. Returns `out`.
pub fn fn_00458700(e: &mut Engine, this: Ptr, out: Ptr, index: i32) -> Ptr {
    if index == 0 {
        e.call(VECTOR_COPY, &args![out, this]);
    } else {
        let lane = if index > 2 { 3 } else { index as u32 };
        let value = e.mem.u32(this.addr() + 4 * lane);
        e.with_stack(16, |e, shuffled| {
            for word in 0..4 {
                e.mem.set_u32(shuffled.addr() + 4 * word, value);
            }
            e.call(VECTOR_COPY, &args![out, shuffled]);
        });
    }
    out
}

// Translated from 004587d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `value` times the `float` at `011c3d18` (Havok to game scale), as a `float`.
pub fn fn_004587d0(e: &mut Engine, value: f32) -> f32 {
    let scale: f32 = e.global(HAVOK_SCALE);
    (scale as f64 * value as f64) as f32
}

// Translated from 004587f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Like [`fn_004585f0`] for `m_to` (`this + 0x10`, after [`fn_00458820`] has
/// completed it from the length). Returns `out`.
pub fn fn_004587f0(e: &mut Engine, this: Ptr, out: Ptr) -> Ptr {
    let to = fn_00458820(e, this);
    fn_00458620(e, out, to)
}

// Translated from 00458820 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs `bhkPickData::CalcTo` on the pick data and returns the address of its
/// `m_to` vector (`+0x10`).
pub fn fn_00458820(e: &mut Engine, this: Ptr) -> Ptr {
    bhk_pick_data_calc_to(e, this);
    this.byte_add(0x10)
}

// Translated from 00458840 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkPickData::CalcTo` (Xbox PDB): when the length vector `hkLength`
/// (`+0x90`) is not (nearly) zero ([`fn_004588d0`]), sets `m_to` (`+0x10`) to
/// `m_from` (`+0x00`) plus `hkLength`.
pub fn bhk_pick_data_calc_to(e: &mut Engine, this: Ptr) {
    if fn_004588d0(e, this) {
        fn_00458880(e, this.byte_add(0x10), this, this.byte_add(0x90));
    }
}

// Translated from 00458880 (decompiled, FalloutNV.exe 1.4.0.525)
/// `this = a + b`, lane by lane (`ADDPS`, `float`) on 16-byte vectors.
pub fn fn_00458880(e: &mut Engine, this: Ptr, a: Ptr, b: Ptr) {
    let lanes: Vec<f32> = (0..4)
        .map(|lane| e.mem.f32(a.addr() + 4 * lane) + e.mem.f32(b.addr() + 4 * lane))
        .collect();
    for (lane, value) in lanes.into_iter().enumerate() {
        e.mem.set_f32(this.addr() + 4 * lane as u32, value);
    }
}

// Translated from 004588d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when the vector at `this + 0x90` (`hkLength`) is NOT within the
/// `float` at `01017d00` (0.001) of the zero vector that `00458b20` returns,
/// in lanes x, y and z ([`fn_00458900`]).
pub fn fn_004588d0(e: &mut Engine, this: Ptr) -> bool {
    let epsilon: f32 = e.global(PICK_EPSILON);
    let zero = e.call(0x0045_8b20, &[]).ptr();
    !fn_00458900(e, this.byte_add(0x90), zero, epsilon)
}

// Translated from 00458900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether `|this - other| <= tolerance` in lanes x, y and z (lane w is not
/// tested): the difference ([`fn_00458a10`]), its absolute value
/// ([`fn_00458ad0`]), the tolerance repeated in four lanes ([`fn_004589c0`]),
/// the lane compare ([`fn_00458a60`]) and the sign-bit test with mask 7
/// ([`fn_00458990`]).
pub fn fn_00458900(e: &mut Engine, this: Ptr, other: Ptr, tolerance: f32) -> bool {
    e.with_stack(0x30, |e, block| {
        let difference = block;
        let limit = block.byte_add(16);
        let flags = block.byte_add(32);
        e.call(VECTOR_CTOR, &args![difference]);
        fn_00458a10(e, difference, this, other);
        fn_00458ad0(e, difference, difference);
        e.call(VECTOR_CTOR, &args![limit]);
        fn_004589c0(e, limit, tolerance);
        let flags = fn_00458a60(e, difference, flags, limit);
        fn_00458990(e, flags, 7)
    })
}

// Translated from 00458990 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MOVMSKPS`: collects the sign bits of the four lanes (lane 0 is bit 0);
/// true when all bits of `mask` are set.
pub fn fn_00458990(e: &mut Engine, this: Ptr, mask: u32) -> bool {
    let mut signs = 0u32;
    for lane in 0..4 {
        signs |= (e.mem.u32(this.addr() + 4 * lane) >> 31) << lane;
    }
    signs & mask == mask
}

// Translated from 004589c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in all four lanes of the vector at `this` (`SHUFPS`).
pub fn fn_004589c0(e: &mut Engine, this: Ptr, value: f32) {
    for lane in 0..4 {
        e.mem.set_u32(this.addr() + 4 * lane, value.to_bits());
    }
}

// Translated from 00458a10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `this = a - b`, lane by lane (`SUBPS`, `float`).
pub fn fn_00458a10(e: &mut Engine, this: Ptr, a: Ptr, b: Ptr) {
    let lanes: Vec<f32> = (0..4)
        .map(|lane| e.mem.f32(a.addr() + 4 * lane) - e.mem.f32(b.addr() + 4 * lane))
        .collect();
    for (lane, value) in lanes.into_iter().enumerate() {
        e.mem.set_f32(this.addr() + 4 * lane as u32, value);
    }
}

// Translated from 00458a60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `CMPLEPS`: for each lane, `out` gets all ones where `this <= b` (false for
/// NaN) and zero elsewhere. Returns `out`.
pub fn fn_00458a60(e: &mut Engine, this: Ptr, out: Ptr, b: Ptr) -> Ptr {
    let lanes: Vec<u32> = (0..4)
        .map(|lane| {
            if e.mem.f32(this.addr() + 4 * lane) <= e.mem.f32(b.addr() + 4 * lane) {
                u32::MAX
            } else {
                0
            }
        })
        .collect();
    for (lane, value) in lanes.into_iter().enumerate() {
        e.mem.set_u32(out.addr() + 4 * lane as u32, value);
    }
    out
}

// Translated from 00458ad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `this = source & mask` for the four words, with the mask (all
/// `0x7fffffff`) read from `01017d10`: the absolute value of each lane.
pub fn fn_00458ad0(e: &mut Engine, this: Ptr, source: Ptr) {
    let words: Vec<u32> = (0..4)
        .map(|word| e.mem.u32(source.addr() + 4 * word) & e.mem.u32(ABS_MASK + 4 * word))
        .collect();
    for (word, value) in words.into_iter().enumerate() {
        e.mem.set_u32(this.addr() + 4 * word as u32, value);
    }
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x004579e0, fn_004579e0(f32) -> f32),
        entry!(0x00457a00, make_new_geom_data(Ptr)),
        entry!(0x00457b80, fn_00457b80(Ptr) -> u16),
        entry!(
            0x00457ba0,
            tes_create_deep_copy_same_textures(Ptr<TES>, Ptr, u32) -> Ptr
        ),
        entry!(0x00457be0, tes_init_models_to_load(Ptr<TES>, u32)),
        entry!(0x00457d70, fn_00457d70(Ptr<TES>, u8, Ptr, u8) -> bool),
        entry!(0x00457fe0, fn_00457fe0() -> u32),
        entry!(0x00457ff0, fn_00457ff0(Ptr)),
        entry!(0x00458010, fn_00458010(Ptr)),
        entry!(0x00458030, fn_00458030() -> u32),
        entry!(0x00458060, fn_00458060(u8)),
        entry!(0x00458080, fn_00458080(Ptr, u32, u8)),
        entry!(0x004580e0, fn_004580e0() -> bool),
        entry!(0x00458110, fn_00458110(Ptr, u32) -> bool),
        entry!(0x00458140, fn_00458140() -> bool),
        entry!(0x00458180, fn_00458180(Ptr)),
        entry!(0x004581c0, fn_004581c0(u8)),
        entry!(0x004581e0, fn_004581e0(u32, u32) -> u32),
        entry!(0x00458200, tes_set_world_space(Ptr<TES>, Ptr)),
        entry!(0x004583f0, fn_004583f0(u8)),
        entry!(0x00458400, fn_00458400(Ptr) -> u32),
        entry!(0x00458420, tes_pick(Ptr<TES>, Ptr) -> u32),
        entry!(0x00458440, tes_pick_ov2(Ptr<TES>, Ptr, u8) -> u32),
        entry!(0x004584d0, tes_pick_ni(Ptr<TES>, Ptr) -> u32),
        entry!(0x004585f0, fn_004585f0(Ptr, Ptr) -> Ptr),
        entry!(0x00458620, fn_00458620(Ptr, Ptr) -> Ptr),
        entry!(0x004586d0, fn_004586d0(Ptr) -> f32),
        entry!(0x00458700, fn_00458700(Ptr, Ptr, i32) -> Ptr),
        entry!(0x004587d0, fn_004587d0(f32) -> f32),
        entry!(0x004587f0, fn_004587f0(Ptr, Ptr) -> Ptr),
        entry!(0x00458820, fn_00458820(Ptr) -> Ptr),
        entry!(0x00458840, bhk_pick_data_calc_to(Ptr)),
        entry!(0x00458880, fn_00458880(Ptr, Ptr, Ptr)),
        entry!(0x004588d0, fn_004588d0(Ptr) -> bool),
        entry!(0x00458900, fn_00458900(Ptr, Ptr, f32) -> bool),
        entry!(0x00458990, fn_00458990(Ptr, u32) -> bool),
        entry!(0x004589c0, fn_004589c0(Ptr, f32)),
        entry!(0x00458a10, fn_00458a10(Ptr, Ptr, Ptr)),
        entry!(0x00458a60, fn_00458a60(Ptr, Ptr, Ptr) -> Ptr),
        entry!(0x00458ad0, fn_00458ad0(Ptr, Ptr)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    type Log = Rc<RefCell<Vec<Vec<u32>>>>;

    fn ret(eax: u32) -> Ret {
        Ret {
            eax,
            ..Ret::default()
        }
    }

    fn returns(e: &mut Engine, addr: u32, value: u32) {
        e.register_double(addr, move |_, _| ret(value));
    }

    fn noop(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
    }

    /// A double that records the argument words of each call and returns
    /// `value`.
    fn recording(e: &mut Engine, addr: u32, value: u32) -> Log {
        let log: Log = Rc::new(RefCell::new(vec![]));
        let inner = log.clone();
        e.register_double(addr, move |_, a| {
            inner.borrow_mut().push(a.to_vec());
            ret(value)
        });
        log
    }

    /// The pages holding the globals this file reads.
    fn map_globals(e: &mut Engine) {
        for page in [
            0x0101_7000u32,
            0x011a_d000,
            0x011c_3000,
            0x011c_6000,
            0x011d_9000,
            0x011d_a000,
            0x011d_e000,
        ] {
            e.map(page, 0x1000);
        }
    }

    /// A setting object (value at +4) at `addr`, and the value-pointer
    /// functions for it.
    fn put_setting(e: &mut Engine, addr: u32, value: u32) {
        e.mem.set_u32(addr + 4, value);
        for function in [SETTING_VALUE_ADDRESS_INT, SETTING_VALUE_ADDRESS_BYTE] {
            e.register(function, |_, a| ret(a[0] + 4));
        }
    }

    /// A vtable with the given (byte offset, function address) slots.
    fn vtable(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
        let table = e.mem.alloc(0x100);
        for (offset, function) in slots {
            e.mem.set_u32(table + offset, *function);
        }
        table
    }

    fn object_with(e: &mut Engine, table: u32) -> Ptr {
        let object = e.mem.alloc(0x100);
        e.mem.set_u32(object, table);
        Ptr::new(object)
    }

    fn ni_pointer_doubles(e: &mut Engine) {
        e.register(NI_POINTER_CTOR, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(NI_POINTER_GET, |e, a| ret(e.mem.u32(a[0])));
        e.register(NI_POINTER_DTOR, |e, a| {
            e.mem.set_u32(a[0], 0);
            Ret::default()
        });
        e.register(NI_POINTER_ASSIGN, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
    }

    fn vector(e: &mut Engine, lanes: [f32; 4]) -> Ptr {
        let vector = Ptr::<()>::new(e.mem.alloc(16));
        for (lane, value) in lanes.iter().enumerate() {
            e.mem.set_f32(vector.addr() + 4 * lane as u32, *value);
        }
        vector
    }

    fn lanes(e: &Engine, vector: Ptr) -> [f32; 4] {
        [0, 1, 2, 3].map(|lane| e.mem.f32(vector.addr() + 4 * lane))
    }

    /// The vector constructor (returns `this`), the vector copy, the constants
    /// the vector code reads and the zero vector `00458b20` returns.
    fn vector_engine() -> Engine {
        let mut e = Engine::new();
        map_globals(&mut e);
        e.register(VECTOR_CTOR, |_, a| ret(a[0]));
        e.register(VECTOR_COPY, |e, a| {
            for word in 0..4 {
                let value = e.mem.u32(a[1] + 4 * word);
                e.mem.set_u32(a[0] + 4 * word, value);
            }
            ret(a[0])
        });
        for word in 0..4 {
            e.set_global(ABS_MASK + 4 * word, 0x7fff_ffffu32);
        }
        e.set_global(PICK_EPSILON, 0.001f32);
        e.set_global(HAVOK_SCALE, 2.0f32);
        let zero = Ptr::<()>::new(e.mem.alloc(16));
        e.register_double(0x0045_8b20, move |_, _| ret(zero.addr()));
        e
    }

    // ----- 004579e0 -----

    #[test]
    fn float_wrapper_returns_the_callee_result() {
        let mut e = Engine::new();
        e.register(0x0040_19b0, |_, a| Ret {
            st0: f32::from_bits(a[0]) as f64 * 2.0,
            ..Ret::default()
        });
        assert_eq!(e.call(0x0045_79e0, &args![1.5f32]).f32(), 3.0);
    }

    // ----- 00457a00, 00457b80, 00457ba0 -----

    struct Geometry {
        e: Engine,
        node: Ptr,
        installs: Log,
        copy_object: u32,
        shape: u32,
    }

    /// A node whose slot 0x18 gives a geometry object holding `shape` (whose
    /// word at +0xE is `flags`); `has_form` is what `007af430` says.
    fn geometry_world(flags: u16, has_form: bool) -> Geometry {
        let mut e = Engine::new();
        ni_pointer_doubles(&mut e);
        let geometry_vtable = vtable(&mut e, &[(0xe4, 0x7000_00e4)]);
        let geometry = object_with(&mut e, geometry_vtable).addr();
        let installs = recording(&mut e, 0x7000_00e4, 0);
        let node_vtable = vtable(&mut e, &[(0x18, 0x7000_0018), (0xc, 0x7000_000c)]);
        let node = object_with(&mut e, node_vtable);
        returns(&mut e, 0x7000_0018, geometry);
        returns(&mut e, 0x7000_000c, 0);
        let shape = e.mem.alloc(0x20);
        e.mem.set_u16(shape + 0xe, flags);
        returns(&mut e, 0x0054_95f0, shape);
        returns(&mut e, 0x007a_f430, has_form as u32);
        let copy_object = e.mem.alloc(0x20);
        e.register_double(0x00a5_d510, move |e, a| {
            e.mem.set_u32(a[1], copy_object);
            Ret::default()
        });
        Geometry {
            e,
            node,
            installs,
            copy_object,
            shape,
        }
    }

    #[test]
    fn geom_data_is_copied_when_the_shape_has_a_form_and_no_flags() {
        let mut world = geometry_world(0x0fff, true);
        world.e.call(0x0045_7a00, &args![world.node]);
        let installs = world.installs.borrow();
        assert_eq!(installs.len(), 1);
        assert_eq!(installs[0][1], world.copy_object);
    }

    #[test]
    fn geom_data_keeps_the_original_without_a_form() {
        let mut world = geometry_world(0, false);
        world.e.call(0x0045_7a00, &args![world.node]);
        let installs = world.installs.borrow();
        assert_eq!(installs.len(), 1);
        assert_eq!(installs[0][1], world.shape);
    }

    #[test]
    fn geom_data_keeps_the_original_when_a_flag_bit_is_set() {
        let mut world = geometry_world(0x1000, true);
        let form_checks = recording(&mut world.e, 0x007a_f430, 1);
        world.e.call(0x0045_7a00, &args![world.node]);
        assert!(form_checks.borrow().is_empty());
        assert_eq!(world.installs.borrow()[0][1], world.shape);
    }

    #[test]
    fn geom_data_walks_the_children_last_to_first() {
        let mut e = Engine::new();
        ni_pointer_doubles(&mut e);
        // Leaves have neither geometry nor children; the parent's slot 0xC
        // gives a child list.
        let leaf_vtable = vtable(&mut e, &[(0x18, 0x7000_0018), (0xc, 0x7000_000c)]);
        let parent_vtable = vtable(&mut e, &[(0x18, 0x7000_0018), (0xc, 0x7000_0010)]);
        let parent = object_with(&mut e, parent_vtable);
        returns(&mut e, 0x7000_0010, 0x0aaa_0000);
        returns(&mut e, 0x7000_000c, 0);
        let geometry_asks = recording(&mut e, 0x7000_0018, 0);
        returns(&mut e, 0x0043_b480, 2);
        let children = [
            object_with(&mut e, leaf_vtable).addr(),
            object_with(&mut e, leaf_vtable).addr(),
        ];
        let order: Log = Rc::new(RefCell::new(vec![]));
        let inner = order.clone();
        e.register_double(0x0043_b4a0, move |_, a| {
            inner.borrow_mut().push(a.to_vec());
            ret(children[a[1] as usize])
        });
        e.call(0x0045_7a00, &args![parent]);
        // The child at index 1 comes before the child at index 0.
        let indexes: Vec<u32> = order.borrow().iter().map(|call| call[1]).collect();
        assert_eq!(indexes, vec![1, 0]);
        // The parent and its two children each asked for their geometry.
        let askers: Vec<u32> = geometry_asks.borrow().iter().map(|call| call[0]).collect();
        assert_eq!(askers, vec![parent.addr(), children[1], children[0]]);
    }

    #[test]
    fn geom_data_of_a_null_node_only_builds_and_drops_the_pointer() {
        let mut e = Engine::new();
        ni_pointer_doubles(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0045_7a00, &args![0u32]);
        let log = e.call_log.take().unwrap();
        let addrs: Vec<u32> = log.iter().map(|call| call.0).collect();
        assert_eq!(addrs, vec![0x0045_7a00, NI_POINTER_CTOR, NI_POINTER_DTOR]);
    }

    #[test]
    fn flag_word_is_masked_with_7000() {
        let mut e = Engine::new();
        let object = Ptr::<()>::new(e.mem.alloc(0x20));
        e.mem.set_u16(object.addr() + 0xe, 0xffff);
        assert_eq!(e.call(0x0045_7b80, &args![object]).u32(), 0x7000);
        e.mem.set_u16(object.addr() + 0xe, 0x0fff);
        assert_eq!(e.call(0x0045_7b80, &args![object]).u32(), 0);
    }

    #[test]
    fn deep_copy_clones_the_node_and_rebuilds_its_geometry() {
        let mut e = Engine::new();
        ni_pointer_doubles(&mut e);
        let leaf_vtable = vtable(&mut e, &[(0x18, 0x7000_0018), (0xc, 0x7000_000c)]);
        let clone = object_with(&mut e, leaf_vtable);
        returns(&mut e, 0x7000_0018, 0);
        returns(&mut e, 0x7000_000c, 0);
        let clones = recording(&mut e, 0x00a5_d2c0, clone.addr());
        let tes: Ptr<TES> = e.new_object();
        let result = e.call(0x0045_7ba0, &args![tes, 0x1234u32, 7u32]).u32();
        assert_eq!(result, clone.addr());
        assert_eq!(clones.borrow()[0], vec![0x1234, 7]);
        // A null node gives null without cloning.
        assert_eq!(e.call(0x0045_7ba0, &args![tes, 0u32, 7u32]).u32(), 0);
        assert_eq!(clones.borrow().len(), 1);
    }

    // ----- 00457be0 -----

    struct Grid {
        e: Engine,
        tes: Ptr<TES>,
        loads: Log,
        counted: Log,
    }

    /// A 3 x 3 grid around (10, 20) in world space 0x9000 where every
    /// `LoadCell` succeeds except (9, 19); `00454400` counts `count` per cell.
    fn grid_world(count: u32) -> Grid {
        let mut e = Engine::new();
        map_globals(&mut e);
        put_setting(&mut e, SETTING_GRIDS_TO_LOAD, 3);
        let tes: Ptr<TES> = e.new_object();
        e.set(tes, TES::iCurrentGridX, 10);
        e.set(tes, TES::iCurrentGridY, 20);
        e.set(tes, TES::pGridCellA, Ptr::new(0x0bad_0000));
        e.set(tes, TES::pWorldSpace, Ptr::new(0x9000));
        returns(&mut e, GET_WORLD_SPACE, 0x9000);
        e.set_global(DATA_HANDLER, 0x6000u32);
        e.register_double(WORLD_SPACE_LOAD_CELL, |_, a| {
            ret(if (a[1], a[2]) == (9, 19) {
                0
            } else {
                0x5000 + a[1] * 16 + a[2]
            })
        });
        let loads = recording(&mut e, DATA_HANDLER_CREATE_CELL, 0x7777);
        returns(&mut e, 0x0045_0fb0, 0);
        let counted = recording(&mut e, 0x0045_4400, count);
        // The loading menu code stops at "a load is pending".
        returns(&mut e, 0x0070_5e80, 0);
        returns(&mut e, 0x0045_1530, 1);
        Grid {
            e,
            tes,
            loads,
            counted,
        }
    }

    #[test]
    fn models_load_every_cell_of_the_square_for_other_modes() {
        let mut grid = grid_world(1);
        let pending = recording(&mut grid.e, 0x0045_1530, 1);
        let loaded = recording(&mut grid.e, WORLD_SPACE_LOAD_CELL, 0x5001);
        grid.e.call(0x0045_7be0, &args![grid.tes, 2u32]);
        let coordinates: Vec<(u32, u32)> = loaded.borrow().iter().map(|a| (a[1], a[2])).collect();
        // First cell is 9, 19: the centre (10, 20) minus uGridsToLoad / 2;
        // x is the outer loop.
        assert_eq!(coordinates.len(), 9);
        assert_eq!(coordinates[0], (9, 19));
        assert_eq!(coordinates[1], (9, 20));
        assert_eq!(coordinates[3], (10, 19));
        assert_eq!(coordinates[8], (11, 21));
        assert_eq!(grid.counted.borrow().len(), 9);
        // Something was counted: the loading menu code ran (show = 1).
        assert_eq!(pending.borrow().len(), 1);
    }

    #[test]
    fn models_fall_back_to_creating_the_cell() {
        let mut grid = grid_world(1);
        grid.e.call(0x0045_7be0, &args![grid.tes, 2u32]);
        // LoadCell failed at (9, 19): the data handler created that cell in
        // the world space `004fd3e0` gives, at the same coordinates.
        let created = grid.loads.borrow();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0], vec![0x6000, 9, 19, 0x9000, 1]);
        // 0x7777 is among the cells counted.
        assert!(grid.counted.borrow().iter().any(|a| a[0] == 0x7777));
    }

    #[test]
    fn models_mode_zero_skips_attached_cells() {
        let mut grid = grid_world(1);
        // Attached cells are the ones in row y = 1.
        grid.e
            .register(GRID_CELL_ARRAY_GET_ATTACHED, |_, a| ret((a[2] == 1) as u32));
        grid.e.call(0x0045_7be0, &args![grid.tes, 0u32]);
        assert_eq!(grid.counted.borrow().len(), 6);
    }

    #[test]
    fn models_mode_one_skips_cells_the_array_has() {
        let mut grid = grid_world(1);
        // The array has the cells of column x = 0.
        grid.e
            .register(GRID_CELL_ARRAY_GET, |_, a| ret((a[1] == 0) as u32));
        grid.e.call(0x0045_7be0, &args![grid.tes, 1u32]);
        assert_eq!(grid.counted.borrow().len(), 6);
    }

    #[test]
    fn models_do_not_call_the_loading_menu_when_nothing_was_counted() {
        let mut grid = grid_world(0);
        let pending = recording(&mut grid.e, 0x0045_1530, 1);
        grid.e.call(0x0045_7be0, &args![grid.tes, 2u32]);
        assert!(pending.borrow().is_empty());
    }

    #[test]
    fn models_load_nothing_without_a_world_space() {
        let mut grid = grid_world(1);
        returns(&mut grid.e, GET_WORLD_SPACE, 0);
        grid.e.call(0x0045_7be0, &args![grid.tes, 2u32]);
        assert!(grid.counted.borrow().is_empty());
    }

    // ----- 00457d70 -----

    struct Menu {
        e: Engine,
        tes: Ptr<TES>,
        menu: u32,
        ticks: Rc<std::cell::Cell<u32>>,
        modes: Log,
        closes: Log,
        created: Log,
    }

    /// A loading menu with the given state word, a HUD main menu in mode
    /// `hud_mode`, a settable tick count and recording doubles for the menu
    /// calls.
    fn menu_world(state: u16, hud_mode: u32) -> Menu {
        let mut e = Engine::new();
        map_globals(&mut e);
        let menu = e.mem.alloc(0x300);
        e.mem.set_u16(menu + 0x222, state);
        e.set_global(LOADING_MENU, menu);
        let hud = e.mem.alloc(0x200);
        e.mem.set_u32(hud + 0x1c4, hud_mode);
        e.set_global(HUD_MAIN_MENU, hud);
        let ticks = Rc::new(std::cell::Cell::new(1000u32));
        let inner = ticks.clone();
        e.register_double(GET_TICK_COUNT, move |_, _| ret(inner.get()));
        let modes = recording(&mut e, 0x0077_1700, 0);
        let closes = recording(&mut e, 0x0070_5e30, 0);
        let created = recording(&mut e, 0x0070_5e00, 0);
        let tes: Ptr<TES> = e.new_object();
        Menu {
            e,
            tes,
            menu,
            ticks,
            modes,
            closes,
            created,
        }
    }

    #[test]
    fn hiding_a_menu_that_is_not_open_closes_it() {
        let mut world = menu_world(0, 0);
        let shown = world.e.call(0x0045_7d70, &args![world.tes, 0u8, 0u32, 0u8]);
        assert!(!shown.bool());
        assert_eq!(world.closes.borrow().len(), 1);
        // The HUD main menu is switched to mode 2 unless it is in mode 0x18.
        assert_eq!(world.modes.borrow()[0], vec![2]);
        assert_eq!(world.e.global::<u32>(LOADING_MENU_DEADLINE), 0);
    }

    #[test]
    fn hiding_in_hud_mode_0x18_leaves_the_mode_alone() {
        let mut world = menu_world(0, 0x18);
        world.e.call(0x0045_7d70, &args![world.tes, 0u8, 0u32, 0u8]);
        assert_eq!(world.closes.borrow().len(), 1);
        assert!(world.modes.borrow().is_empty());
    }

    #[test]
    fn hiding_an_open_menu_waits_five_seconds_then_closes() {
        let mut world = menu_world(1 << 10, 0);
        world.e.call(0x0045_7d70, &args![world.tes, 0u8, 0u32, 0u8]);
        assert!(world.closes.borrow().is_empty());
        assert_eq!(world.e.global::<u32>(LOADING_MENU_DEADLINE), 6000);
        // Still open before the deadline (the deadline stays).
        world.ticks.set(6000);
        world.e.call(0x0045_7d70, &args![world.tes, 0u8, 0u32, 0u8]);
        assert!(world.closes.borrow().is_empty());
        world.ticks.set(6001);
        world.e.call(0x0045_7d70, &args![world.tes, 0u8, 0u32, 0u8]);
        assert_eq!(world.closes.borrow().len(), 1);
        assert_eq!(world.e.global::<u32>(LOADING_MENU_DEADLINE), 0);
        // Bit 10 was cleared by the close.
        assert_eq!(world.e.mem.u16(world.menu + 0x222), 0);
    }

    #[test]
    fn hiding_an_open_menu_with_the_force_flag_closes_at_once() {
        let mut world = menu_world(1 << 10, 0);
        world.e.call(0x0045_7d70, &args![world.tes, 0u8, 0u32, 1u8]);
        assert_eq!(world.closes.borrow().len(), 1);
    }

    #[test]
    fn showing_a_visible_menu_without_bit_3_only_hands_over_the_form() {
        let mut world = menu_world(0, 0);
        returns(&mut world.e, 0x0070_5e80, 1);
        let handed = recording(&mut world.e, 0x0078_8ab0, 0);
        let shown = world
            .e
            .call(0x0045_7d70, &args![world.tes, 1u8, 0x4444u32, 0u8]);
        assert!(!shown.bool());
        assert_eq!(handed.borrow()[0], vec![world.menu, 0x4444]);
        // Bit 3 is now set, and nothing else ran.
        assert_eq!(world.e.mem.u16(world.menu + 0x222), 1 << 3);
        assert!(world.created.borrow().is_empty());
    }

    #[test]
    fn showing_does_nothing_while_a_load_is_pending() {
        let mut world = menu_world(0, 0);
        returns(&mut world.e, 0x0070_5e80, 0);
        returns(&mut world.e, 0x0045_1530, 1);
        let shown = world
            .e
            .call(0x0045_7d70, &args![world.tes, 1u8, 0x4444u32, 0u8]);
        assert!(!shown.bool());
        assert!(world.created.borrow().is_empty());
    }

    /// Everything the show path calls, with the `00aff100` object's state
    /// word at +0x5C set to 2 and its slot 0x30 recording that word.
    fn show_world() -> (Menu, Log) {
        let mut world = menu_world(0, 0);
        returns(&mut world.e, 0x0070_5e80, 0);
        returns(&mut world.e, 0x0045_1530, 0);
        noop(&mut world.e, &[0x0063_e010, 0x0045_3a70, 0x00ad_8780]);
        let object_vtable = vtable(&mut world.e, &[(0x30, 0x7000_0030)]);
        let object = object_with(&mut world.e, object_vtable);
        world.e.mem.set_u32(object.addr() + 0x5c, 2);
        returns(&mut world.e, 0x00af_f100, object.addr());
        let seen: Log = Rc::new(RefCell::new(vec![]));
        let inner = seen.clone();
        world.e.register_double(0x7000_0030, move |e, a| {
            inner.borrow_mut().push(vec![e.mem.u32(a[0] + 0x5c)]);
            Ret::default()
        });
        (world, seen)
    }

    #[test]
    fn showing_for_a_non_interior_cell_creates_the_menu_for_its_world_space() {
        let (mut world, seen) = show_world();
        returns(&mut world.e, FORM_GET_TYPE, 0x39);
        returns(&mut world.e, CELL_IS_INTERIOR, 0);
        returns(&mut world.e, CELL_GET_WORLD_SPACE, 0x8888);
        let shown = world
            .e
            .call(0x0045_7d70, &args![world.tes, 1u8, 0x4444u32, 1u8]);
        assert!(shown.bool());
        assert_eq!(world.created.borrow()[0], vec![0x8888, 1]);
        // The state word was 2 -> 1 before slot 0x30 ran and is 2 afterwards.
        assert_eq!(seen.borrow()[0], vec![1]);
        let object = world.e.call(0x00af_f100, &[]).u32();
        assert_eq!(world.e.mem.u32(object + 0x5c), 2);
    }

    #[test]
    fn showing_for_an_interior_cell_keeps_the_cell() {
        let (mut world, _) = show_world();
        returns(&mut world.e, FORM_GET_TYPE, 0x39);
        returns(&mut world.e, CELL_IS_INTERIOR, 1);
        world
            .e
            .call(0x0045_7d70, &args![world.tes, 1u8, 0x4444u32, 0u8]);
        assert_eq!(world.created.borrow()[0], vec![0x4444, 0]);
    }

    #[test]
    fn showing_for_a_reference_looks_the_world_space_up_but_passes_the_form() {
        let (mut world, _) = show_world();
        returns(&mut world.e, FORM_GET_TYPE, 0x3a);
        returns(&mut world.e, 0x008d_6f30, 0x5555);
        returns(&mut world.e, CELL_IS_INTERIOR, 0);
        let cell_lookup = recording(&mut world.e, CELL_GET_WORLD_SPACE, 0x8888);
        let reference_lookup = recording(&mut world.e, REFERENCE_GET_WORLD_SPACE, 0x8888);
        world
            .e
            .call(0x0045_7d70, &args![world.tes, 1u8, 0x4444u32, 0u8]);
        assert_eq!(cell_lookup.borrow()[0], vec![0x5555]);
        assert!(reference_lookup.borrow().is_empty());
        assert_eq!(world.created.borrow()[0], vec![0x4444, 0]);
        // Without a cell the reference's own world space is looked up.
        returns(&mut world.e, 0x008d_6f30, 0);
        world
            .e
            .call(0x0045_7d70, &args![world.tes, 1u8, 0x4444u32, 0u8]);
        assert_eq!(reference_lookup.borrow()[0], vec![0x4444]);
    }

    #[test]
    fn showing_for_other_form_types_does_no_lookup() {
        let (mut world, _) = show_world();
        let cell_lookup = recording(&mut world.e, CELL_GET_WORLD_SPACE, 0);
        let reference_lookup = recording(&mut world.e, REFERENCE_GET_WORLD_SPACE, 0);
        for form_type in [0x38u32, 0x41, 0x6a] {
            returns(&mut world.e, FORM_GET_TYPE, form_type);
            world
                .e
                .call(0x0045_7d70, &args![world.tes, 1u8, 0x4444u32, 0u8]);
        }
        assert!(cell_lookup.borrow().is_empty());
        assert!(reference_lookup.borrow().is_empty());
        // The reference type 0x69 does look up.
        returns(&mut world.e, FORM_GET_TYPE, 0x69);
        returns(&mut world.e, 0x008d_6f30, 0);
        world
            .e
            .call(0x0045_7d70, &args![world.tes, 1u8, 0x4444u32, 0u8]);
        assert_eq!(reference_lookup.borrow().len(), 1);
    }

    // ----- the loading menu accessors -----

    #[test]
    fn tick_count_comes_from_the_import() {
        let mut e = Engine::new();
        returns(&mut e, GET_TICK_COUNT, 4242);
        assert_eq!(e.call(0x0045_7fe0, &[]).u32(), 4242);
    }

    #[test]
    fn state_word_toggles_between_one_and_two() {
        let mut e = Engine::new();
        let object = Ptr::<()>::new(e.mem.alloc(0x80));
        e.mem.set_u32(object.addr() + 0x5c, 1);
        e.call(0x0045_7ff0, &args![object]);
        assert_eq!(e.mem.u32(object.addr() + 0x5c), 2);
        e.call(0x0045_7ff0, &args![object]);
        assert_eq!(e.mem.u32(object.addr() + 0x5c), 2);
        e.call(0x0045_8010, &args![object]);
        assert_eq!(e.mem.u32(object.addr() + 0x5c), 1);
        e.call(0x0045_8010, &args![object]);
        assert_eq!(e.mem.u32(object.addr() + 0x5c), 1);
        e.mem.set_u32(object.addr() + 0x5c, 7);
        e.call(0x0045_7ff0, &args![object]);
        e.call(0x0045_8010, &args![object]);
        assert_eq!(e.mem.u32(object.addr() + 0x5c), 7);
    }

    #[test]
    fn hud_mode_is_zero_without_a_hud() {
        let mut e = Engine::new();
        map_globals(&mut e);
        assert_eq!(e.call(0x0045_8030, &[]).u32(), 0);
        let hud = e.mem.alloc(0x200);
        e.mem.set_u32(hud + 0x1c4, 0x18);
        e.set_global(HUD_MAIN_MENU, hud);
        assert_eq!(e.call(0x0045_8030, &[]).u32(), 0x18);
    }

    #[test]
    fn menu_bits_are_set_and_cleared_in_the_state_word() {
        let mut e = Engine::new();
        let menu = Ptr::<()>::new(e.mem.alloc(0x300));
        e.call(0x0045_8080, &args![menu, 10u32, 1u8]);
        assert_eq!(e.mem.u16(menu.addr() + 0x222), 1 << 10);
        e.call(0x0045_8080, &args![menu, 3u32, 1u8]);
        assert_eq!(e.mem.u16(menu.addr() + 0x222), (1 << 10) | (1 << 3));
        e.call(0x0045_8080, &args![menu, 10u32, 0u8]);
        assert_eq!(e.mem.u16(menu.addr() + 0x222), 1 << 3);
        // The shift count wraps at 32: bit 35 is bit 3.
        e.call(0x0045_8080, &args![menu, 35u32, 0u8]);
        assert_eq!(e.mem.u16(menu.addr() + 0x222), 0);
        // Bits above 15 do not fit the word.
        e.call(0x0045_8080, &args![menu, 20u32, 1u8]);
        assert_eq!(e.mem.u16(menu.addr() + 0x222), 0);
    }

    #[test]
    fn menu_bits_are_read_from_the_state_word() {
        let mut e = Engine::new();
        let menu = Ptr::<()>::new(e.mem.alloc(0x300));
        e.mem.set_u16(menu.addr() + 0x222, 1 << 10);
        assert!(e.call(0x0045_8110, &args![menu, 10u32]).bool());
        assert!(!e.call(0x0045_8110, &args![menu, 3u32]).bool());
        assert!(e.call(0x0045_8110, &args![menu, 42u32]).bool());
    }

    #[test]
    fn menu_flag_accessors_need_a_loading_menu() {
        let mut e = Engine::new();
        map_globals(&mut e);
        // Without a loading menu: false, and the setters do nothing.
        assert!(!e.call(0x0045_80e0, &[]).bool());
        assert!(!e.call(0x0045_8140, &[]).bool());
        e.call(0x0045_8060, &args![1u8]);
        e.call(0x0045_81c0, &args![1u8]);
        let menu = e.mem.alloc(0x300);
        e.set_global(LOADING_MENU, menu);
        e.call(0x0045_8060, &args![1u8]);
        assert_eq!(e.mem.u16(menu + 0x222), 1 << 10);
        assert!(e.call(0x0045_80e0, &[]).bool());
        assert!(!e.call(0x0045_8140, &[]).bool());
        e.call(0x0045_81c0, &args![1u8]);
        assert!(e.call(0x0045_8140, &[]).bool());
        e.call(0x0045_8060, &args![0u8]);
        e.call(0x0045_81c0, &args![0u8]);
        assert_eq!(e.mem.u16(menu + 0x222), 0);
    }

    #[test]
    fn handing_a_form_to_the_menu_needs_a_menu_a_form_and_no_pending_word() {
        let mut e = Engine::new();
        map_globals(&mut e);
        let handed = recording(&mut e, 0x0078_8ab0, 0);
        e.call(0x0045_8180, &args![0x4444u32]);
        assert!(handed.borrow().is_empty());
        let menu = e.mem.alloc(0x300);
        e.set_global(LOADING_MENU, menu);
        e.call(0x0045_8180, &args![0u32]);
        assert!(handed.borrow().is_empty());
        e.mem.set_u32(menu + 0x1e8, 1);
        e.call(0x0045_8180, &args![0x4444u32]);
        assert!(handed.borrow().is_empty());
        e.mem.set_u32(menu + 0x1e8, 0);
        e.call(0x0045_8180, &args![0x4444u32]);
        assert_eq!(handed.borrow()[0], vec![menu, 0x4444]);
        assert_eq!(e.mem.u16(menu + 0x222), 1 << 3);
    }

    #[test]
    fn show_loading_area_message_setting_address_is_returned() {
        let mut e = Engine::new();
        let asked = recording(&mut e, SETTING_VALUE_ADDRESS_BYTE, 0x0123_4567);
        assert_eq!(e.call(0x0045_81e0, &args![0u32, 0u32]).u32(), 0x0123_4567);
        assert_eq!(asked.borrow()[0], vec![SETTING_SHOW_LOADING_AREA_MESSAGE]);
    }

    // ----- 00458200 -----

    struct WorldSpaceChange {
        e: Engine,
        tes: Ptr<TES>,
        new_space: Ptr,
        log: Vec<(u32, Vec<u32>)>,
    }

    /// Sets up the doubles for a switch to a new world space whose `+0xD0`
    /// word is `extra`, with `unload` the setting `bPreemptivelyUnloadCells`,
    /// `interior` the interior cell, `flag_byte` the byte `00454b50` points at
    /// and `known` what `005862e0` says.
    fn switch_world_space(
        unload: u32,
        interior: u32,
        extra: u32,
        flag_byte: u8,
        known: bool,
        unloaded: u32,
    ) -> WorldSpaceChange {
        let mut e = Engine::new();
        map_globals(&mut e);
        put_setting(&mut e, SETTING_PREEMPTIVELY_UNLOAD_CELLS, unload);
        let tes: Ptr<TES> = e.new_object();
        e.set(tes, TES::pWorldSpace, Ptr::new(0x1111));
        e.set_global(TES_OBJECT, 0x0bee_0000u32);
        returns(&mut e, GET_INTERIOR_CELL, interior);
        noop(
            &mut e,
            &[
                0x0045_3940,
                0x0045_2490,
                0x0052_6270,
                0x0058_eeb0,
                0x0045_0d80,
                0x0070_43c0,
                0x0070_9d20,
                0x0045_68c0,
                0x0052_d3b0,
                SCOPE_GUARD_CTOR,
                SCOPE_GUARD_DTOR,
            ],
        );
        ni_pointer_doubles(&mut e);
        returns(&mut e, 0x0045_5200, unloaded);
        returns(&mut e, 0x0045_4d50, 0);
        let new_space = Ptr::<()>::new(e.mem.alloc(0x100));
        e.mem.set_u32(new_space.addr() + 0xd0, extra);
        returns(&mut e, 0x0067_33e0, 0x3333);
        let byte = e.mem.alloc(8);
        e.mem.set_u8(byte, flag_byte);
        returns(&mut e, 0x0045_4b50, byte);
        returns(&mut e, 0x0070_ec90, 0x7a7a);
        returns(&mut e, 0x0058_62e0, known as u32);
        e.call_log = Some(vec![]);
        e.call(0x0045_8200, &args![tes, new_space]);
        let log = e.call_log.take().unwrap();
        WorldSpaceChange {
            e,
            tes,
            new_space,
            log,
        }
    }

    fn calls_to(log: &[(u32, Vec<u32>)], addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|call| call.0 == addr)
            .map(|call| call.1.clone())
            .collect()
    }

    #[test]
    fn switching_to_no_world_space_or_the_current_one_does_nothing() {
        let mut e = Engine::new();
        let tes: Ptr<TES> = e.new_object();
        e.set(tes, TES::pWorldSpace, Ptr::new(0x1111));
        e.call_log = Some(vec![]);
        e.call(0x0045_8200, &args![tes, 0u32]);
        e.call(0x0045_8200, &args![tes, 0x1111u32]);
        assert_eq!(e.call_log.take().unwrap().len(), 2);
    }

    #[test]
    fn switching_without_unloading_stores_the_space_and_updates_the_water() {
        let change = switch_world_space(0, 0, 0, 0, true, 0);
        assert_eq!(change.e.get(change.tes, TES::pWorldSpace), change.new_space);
        let log = &change.log;
        // No unloading, no extra data.
        assert!(calls_to(log, 0x0045_5200).is_empty());
        assert!(calls_to(log, 0x0052_6270).is_empty());
        // The sky/interface calls take the new world space.
        assert_eq!(calls_to(log, 0x0058_eeb0), vec![vec![0x3333]]);
        assert_eq!(
            calls_to(log, 0x0070_9d20),
            vec![vec![change.new_space.addr()]]
        );
        // The flag byte is 0: the water gets no texture.
        assert_eq!(calls_to(log, 0x0052_d3b0), vec![vec![0x7a7a, 0]]);
        assert!(calls_to(log, 0x0045_68c0).is_empty());
        // `005862e0` true: the final byte is 0.
        assert_eq!(change.e.global::<u8>(WORLD_SPACE_FLAG), 0);
    }

    #[test]
    fn switching_runs_the_extra_data_and_stores_one_when_the_space_is_unknown() {
        let change = switch_world_space(0, 0, 0xe0e0, 0, false, 0);
        assert_eq!(calls_to(&change.log, 0x0052_6270), vec![vec![0xe0e0]]);
        assert_eq!(change.e.global::<u8>(WORLD_SPACE_FLAG), 1);
    }

    #[test]
    fn switching_with_unloading_unloads_the_old_space_and_cleans_up() {
        let change = switch_world_space(1, 0, 0, 0, true, 5);
        let log = &change.log;
        assert_eq!(
            calls_to(log, 0x0045_5200),
            vec![vec![change.tes.addr(), 1, 0x1111]]
        );
        assert_eq!(calls_to(log, 0x0045_4d50), vec![vec![change.tes.addr(), 1]]);
        assert_eq!(calls_to(log, 0x0045_2490), vec![vec![change.tes.addr(), 0]]);
        // No interior cell: `00453940` runs only once, after the switch.
        assert_eq!(calls_to(log, 0x0045_3940).len(), 1);
    }

    #[test]
    fn switching_from_an_interior_cell_skips_the_second_unload() {
        let change = switch_world_space(1, 0x2222, 0, 0, true, 0);
        let log = &change.log;
        // `00453940` runs for the interior cell and again after the switch.
        assert_eq!(calls_to(log, 0x0045_3940).len(), 2);
        assert!(calls_to(log, 0x0045_4d50).is_empty());
        // Nothing was unloaded: no texture clean-up.
        assert!(calls_to(log, 0x0045_2490).is_empty());
    }

    #[test]
    fn switching_loads_the_texture_when_the_flag_byte_is_set() {
        let change = switch_world_space(0, 0, 0, 1, true, 0);
        let log = &change.log;
        let guard = calls_to(log, SCOPE_GUARD_CTOR);
        assert_eq!(guard.len(), 1);
        assert_eq!(&guard[0][1..], &[0x1d, 1, TES_CPP_PATH, 0x1751]);
        let texture = calls_to(log, 0x0045_68c0);
        assert_eq!(texture.len(), 1);
        assert_eq!(texture[0][0], 0x0bee_0000);
        assert_eq!(&texture[0][3..], &[1, 1]);
        // The water system gets whatever the `NiPointer` holds afterwards
        // (null here: the texture loader is a double).
        assert_eq!(calls_to(log, 0x0052_d3b0), vec![vec![0x7a7a, 0]]);
        assert_eq!(calls_to(log, SCOPE_GUARD_DTOR).len(), 1);
    }

    #[test]
    fn world_space_flag_byte_is_stored() {
        let mut e = Engine::new();
        map_globals(&mut e);
        e.call(0x0045_83f0, &args![1u8]);
        assert_eq!(e.global::<u8>(WORLD_SPACE_FLAG), 1);
        e.call(0x0045_83f0, &args![0u8]);
        assert_eq!(e.global::<u8>(WORLD_SPACE_FLAG), 0);
    }

    #[test]
    fn world_space_extra_word_is_read_at_0xd0() {
        let mut e = Engine::new();
        let space = Ptr::<()>::new(e.mem.alloc(0x100));
        e.mem.set_u32(space.addr() + 0xd0, 0xabcd);
        assert_eq!(e.call(0x0045_8400, &args![space]).u32(), 0xabcd);
    }

    // ----- picks -----

    #[test]
    fn pick_defaults_to_the_cell_pick() {
        let mut e = Engine::new();
        let tes: Ptr<TES> = e.new_object();
        returns(&mut e, GET_INTERIOR_CELL, 0x2222);
        let picked = recording(&mut e, CELL_PICK, 0x77);
        assert_eq!(e.call(0x0045_8420, &args![tes, 0x4444u32]).u32(), 0x77);
        assert_eq!(picked.borrow()[0], vec![0x2222, 0x4444]);
    }

    #[test]
    fn cell_pick_uses_the_centre_cell_without_an_interior_cell() {
        let mut e = Engine::new();
        map_globals(&mut e);
        put_setting(&mut e, SETTING_GRIDS_TO_LOAD, 5);
        let tes: Ptr<TES> = e.new_object();
        e.set(tes, TES::pGridCellA, Ptr::new(0x0bad_0000));
        returns(&mut e, GET_INTERIOR_CELL, 0);
        let slot = e.mem.alloc(8);
        e.mem.set_u32(slot, 0x3333);
        let looked_up = recording(&mut e, GRID_CELL_ARRAY_GET, slot);
        let picked = recording(&mut e, CELL_PICK, 0x77);
        assert_eq!(e.call(0x0045_8440, &args![tes, 0x4444u32, 1u8]).u32(), 0x77);
        // uGridsToLoad / 2 in both coordinates.
        assert_eq!(looked_up.borrow()[0], vec![0x0bad_0000, 2, 2]);
        assert_eq!(picked.borrow()[0], vec![0x3333, 0x4444]);
        // A null cell gives 0 without picking.
        e.mem.set_u32(slot, 0);
        assert_eq!(e.call(0x0045_8440, &args![tes, 0x4444u32, 1u8]).u32(), 0);
        assert_eq!(picked.borrow().len(), 1);
    }

    /// The doubles for the callees of `TES::PickNI`; the pick data at the
    /// returned pointer has `from`, `to` and `length` vectors and the scale is
    /// 2.0. `hit` is what `NiPick::PickObjects` says.
    struct PickWorld {
        e: Engine,
        tes: Ptr<TES>,
        pick: Ptr,
        picked: Log,
        calls: Log,
    }

    fn pick_world(from: [f32; 4], to: [f32; 4], length: [f32; 4], hit: bool) -> PickWorld {
        let mut e = vector_engine();
        let tes: Ptr<TES> = e.new_object();
        let pick = Ptr::<()>::new(e.mem.alloc(0xb0));
        for (lane, value) in from.iter().enumerate() {
            e.mem.set_f32(pick.addr() + 4 * lane as u32, *value);
        }
        for (lane, value) in to.iter().enumerate() {
            e.mem.set_f32(pick.addr() + 0x10 + 4 * lane as u32, *value);
        }
        for (lane, value) in length.iter().enumerate() {
            e.mem.set_f32(pick.addr() + 0x90 + 4 * lane as u32, *value);
        }
        // `end - origin`, as three floats, in the out parameter.
        e.register(0x0043_9ef0, |e, a| {
            for lane in 0..3 {
                let value = e.mem.f32(a[0] + 4 * lane) - e.mem.f32(a[2] + 4 * lane);
                e.mem.set_f32(a[1] + 4 * lane, value);
            }
            ret(a[1])
        });
        let calls: Log = Rc::new(RefCell::new(vec![]));
        let picked: Log = Rc::new(RefCell::new(vec![]));
        let inner = calls.clone();
        e.register_double(0x00e9_8f20, move |_, a| {
            inner.borrow_mut().push(a.to_vec());
            ret(a[0])
        });
        returns(&mut e, 0x0084_e3a0, 0x1234);
        noop(&mut e, &[0x0070_5fc0, 0x0045_8b30, 0x00e9_8fa0]);
        let seen = picked.clone();
        e.register_double(0x00e9_8e20, move |e, a| {
            let mut words = vec![a[0]];
            for lane in 0..3 {
                words.push(e.mem.f32(a[1] + 4 * lane).to_bits());
            }
            for lane in 0..3 {
                words.push(e.mem.f32(a[2] + 4 * lane).to_bits());
            }
            words.push(a[3]);
            seen.borrow_mut().push(words);
            ret(hit as u32)
        });
        returns(&mut e, 0x0050_0940, 0x6000);
        returns(&mut e, 0x0096_8670, 0x6100);
        returns(&mut e, 0x0045_8b50, 0x6200);
        PickWorld {
            e,
            tes,
            pick,
            picked,
            calls,
        }
    }

    fn floats(words: &[u32]) -> Vec<f32> {
        words.iter().map(|word| f32::from_bits(*word)).collect()
    }

    #[test]
    fn ni_pick_casts_the_scaled_ray_and_returns_the_hit_object() {
        let mut world = pick_world([1.0, 2.0, 3.0, 0.0], [4.0, 6.0, 8.0, 0.0], [0.0; 4], true);
        let hit = world.e.call(0x0045_84d0, &args![world.tes, world.pick]);
        assert_eq!(hit.u32(), 0x6200);
        // Origin (1,2,3) * 2 and direction (end - origin) = (8,12,16) - (2,4,6).
        let picked = world.picked.borrow();
        assert_eq!(picked.len(), 1);
        assert_eq!(floats(&picked[0][1..4]), vec![2.0, 4.0, 6.0]);
        assert_eq!(floats(&picked[0][4..7]), vec![6.0, 8.0, 10.0]);
        assert_eq!(picked[0][7], 0);
        // The NiPick is built with (0, 8).
        assert_eq!(&world.calls.borrow()[0][1..], &[0, 8]);
    }

    #[test]
    fn ni_pick_completes_the_end_from_the_length_first() {
        let mut world = pick_world(
            [1.0, 2.0, 3.0, 0.0],
            [100.0, 100.0, 100.0, 0.0],
            [1.0, 1.0, 1.0, 0.0],
            true,
        );
        world.e.call(0x0045_84d0, &args![world.tes, world.pick]);
        // `m_to` became `m_from + hkLength` = (2, 3, 4): end (4, 6, 8) scaled;
        // the direction is (4,6,8) - (2,4,6).
        let picked = world.picked.borrow();
        assert_eq!(floats(&picked[0][4..7]), vec![2.0, 2.0, 2.0]);
    }

    #[test]
    fn ni_pick_without_a_hit_returns_zero_and_still_drops_the_pick() {
        let mut world = pick_world([0.0; 4], [1.0, 1.0, 1.0, 0.0], [0.0; 4], false);
        let dropped = recording(&mut world.e, 0x00e9_8fa0, 0);
        let hit = world.e.call(0x0045_84d0, &args![world.tes, world.pick]);
        assert_eq!(hit.u32(), 0);
        assert_eq!(dropped.borrow().len(), 1);
    }

    #[test]
    fn pick_without_the_cell_flag_runs_the_ni_pick() {
        let mut world = pick_world([0.0; 4], [1.0, 1.0, 1.0, 0.0], [0.0; 4], true);
        let hit = world
            .e
            .call(0x0045_8440, &args![world.tes, world.pick, 0u8]);
        assert_eq!(hit.u32(), 0x6200);
        assert_eq!(world.picked.borrow().len(), 1);
    }

    // ----- the vector helpers -----

    #[test]
    fn first_lane_is_read_as_a_float() {
        let mut e = vector_engine();
        let v = vector(&mut e, [1.5, 2.0, 3.0, 4.0]);
        assert_eq!(e.call(0x0045_86d0, &args![v]).f32(), 1.5);
    }

    #[test]
    fn scale_multiplies_by_the_global() {
        let mut e = vector_engine();
        assert_eq!(e.call(0x0045_87d0, &args![1.5f32]).f32(), 3.0);
    }

    #[test]
    fn lane_broadcast_copies_the_vector_or_one_lane() {
        let mut e = vector_engine();
        let v = vector(&mut e, [1.0, 2.0, 3.0, 4.0]);
        let out = Ptr::<()>::new(e.mem.alloc(16));
        let back = e.call(0x0045_8700, &args![v, out, 0u32]).ptr::<()>();
        assert_eq!(back, out);
        assert_eq!(lanes(&e, out), [1.0, 2.0, 3.0, 4.0]);
        e.call(0x0045_8700, &args![v, out, 1u32]);
        assert_eq!(lanes(&e, out), [2.0; 4]);
        e.call(0x0045_8700, &args![v, out, 2u32]);
        assert_eq!(lanes(&e, out), [3.0; 4]);
        e.call(0x0045_8700, &args![v, out, 3u32]);
        assert_eq!(lanes(&e, out), [4.0; 4]);
        e.call(0x0045_8700, &args![v, out, 9u32]);
        assert_eq!(lanes(&e, out), [4.0; 4]);
    }

    #[test]
    fn three_lanes_are_scaled_into_an_out_vector() {
        let mut e = vector_engine();
        let v = vector(&mut e, [1.0, 2.0, 3.0, 4.0]);
        let out = Ptr::<()>::new(e.mem.alloc(16));
        let back = e.call(0x0045_8620, &args![out, v]).ptr::<()>();
        assert_eq!(back, out);
        assert_eq!(lanes(&e, out), [2.0, 4.0, 6.0, 0.0]);
    }

    #[test]
    fn pick_vectors_are_scaled_from_from_and_to() {
        let mut e = vector_engine();
        let pick = Ptr::<()>::new(e.mem.alloc(0xb0));
        for (lane, value) in [1.0f32, 2.0, 3.0, 0.0].iter().enumerate() {
            e.mem.set_f32(pick.addr() + 4 * lane as u32, *value);
        }
        for (lane, value) in [5.0f32, 6.0, 7.0, 0.0].iter().enumerate() {
            e.mem.set_f32(pick.addr() + 0x10 + 4 * lane as u32, *value);
        }
        let from = Ptr::<()>::new(e.mem.alloc(16));
        let to = Ptr::<()>::new(e.mem.alloc(16));
        e.call(0x0045_85f0, &args![pick, from]);
        e.call(0x0045_87f0, &args![pick, to]);
        assert_eq!(lanes(&e, from), [2.0, 4.0, 6.0, 0.0]);
        assert_eq!(lanes(&e, to), [10.0, 12.0, 14.0, 0.0]);
    }

    #[test]
    fn to_vector_address_is_returned_after_calc_to() {
        let mut e = vector_engine();
        let pick = Ptr::<()>::new(e.mem.alloc(0xb0));
        let to = e.call(0x0045_8820, &args![pick]).u32();
        assert_eq!(to, pick.addr() + 0x10);
    }

    #[test]
    fn calc_to_adds_a_non_zero_length_to_the_start() {
        let mut e = vector_engine();
        let pick = Ptr::<()>::new(e.mem.alloc(0xb0));
        for (lane, value) in [1.0f32, 2.0, 3.0, 4.0].iter().enumerate() {
            e.mem.set_f32(pick.addr() + 4 * lane as u32, *value);
            e.mem.set_f32(pick.addr() + 0x10 + 4 * lane as u32, 9.0);
        }
        // A zero length leaves `m_to` alone.
        e.call(0x0045_8840, &args![pick]);
        assert_eq!(lanes(&e, pick.byte_add(0x10)), [9.0; 4]);
        for (lane, value) in [0.5f32, 0.0, 0.0, 0.25].iter().enumerate() {
            e.mem.set_f32(pick.addr() + 0x90 + 4 * lane as u32, *value);
        }
        e.call(0x0045_8840, &args![pick]);
        assert_eq!(lanes(&e, pick.byte_add(0x10)), [1.5, 2.0, 3.0, 4.25]);
    }

    #[test]
    fn vectors_are_added_lane_by_lane() {
        let mut e = vector_engine();
        let a = vector(&mut e, [1.0, 2.0, 3.0, 4.0]);
        let b = vector(&mut e, [10.0, 20.0, 30.0, 40.0]);
        let out = Ptr::<()>::new(e.mem.alloc(16));
        e.call(0x0045_8880, &args![out, a, b]);
        assert_eq!(lanes(&e, out), [11.0, 22.0, 33.0, 44.0]);
    }

    #[test]
    fn vectors_are_subtracted_lane_by_lane() {
        let mut e = vector_engine();
        let a = vector(&mut e, [10.0, 20.0, 30.0, 40.0]);
        let b = vector(&mut e, [1.0, 2.0, 3.0, 4.0]);
        let out = Ptr::<()>::new(e.mem.alloc(16));
        e.call(0x0045_8a10, &args![out, a, b]);
        assert_eq!(lanes(&e, out), [9.0, 18.0, 27.0, 36.0]);
    }

    #[test]
    fn absolute_value_clears_the_sign_bits() {
        let mut e = vector_engine();
        let a = vector(&mut e, [-1.0, 2.0, -0.0, -4.5]);
        let out = Ptr::<()>::new(e.mem.alloc(16));
        e.call(0x0045_8ad0, &args![out, a]);
        assert_eq!(lanes(&e, out), [1.0, 2.0, 0.0, 4.5]);
        assert_eq!(e.mem.u32(out.addr() + 8), 0);
    }

    #[test]
    fn broadcast_stores_the_value_in_every_lane() {
        let mut e = vector_engine();
        let out = Ptr::<()>::new(e.mem.alloc(16));
        e.call(0x0045_89c0, &args![out, 0.25f32]);
        assert_eq!(lanes(&e, out), [0.25; 4]);
    }

    #[test]
    fn less_or_equal_gives_all_ones_masks_and_false_for_nan() {
        let mut e = vector_engine();
        let a = vector(&mut e, [1.0, 2.0, f32::NAN, 5.0]);
        let b = vector(&mut e, [1.0, 1.0, 3.0, 6.0]);
        let out = Ptr::<()>::new(e.mem.alloc(16));
        let back = e.call(0x0045_8a60, &args![a, out, b]).ptr::<()>();
        assert_eq!(back, out);
        let words: Vec<u32> = (0..4)
            .map(|lane| e.mem.u32(out.addr() + 4 * lane))
            .collect();
        assert_eq!(words, vec![u32::MAX, 0, 0, u32::MAX]);
    }

    #[test]
    fn sign_mask_test_needs_all_requested_bits() {
        let mut e = vector_engine();
        let v = Ptr::<()>::new(e.mem.alloc(16));
        for (lane, word) in [u32::MAX, 0, u32::MAX, 0].iter().enumerate() {
            e.mem.set_u32(v.addr() + 4 * lane as u32, *word);
        }
        assert!(e.call(0x0045_8990, &args![v, 5u32]).bool());
        assert!(!e.call(0x0045_8990, &args![v, 7u32]).bool());
        assert!(e.call(0x0045_8990, &args![v, 0u32]).bool());
        // Only the sign bit counts.
        e.mem.set_u32(v.addr() + 4, 0x7fff_ffff);
        assert!(!e.call(0x0045_8990, &args![v, 2u32]).bool());
    }

    #[test]
    fn closeness_test_compares_x_y_z_but_not_w() {
        let mut e = vector_engine();
        let a = vector(&mut e, [1.0, 2.0, 3.0, 4.0]);
        let near = vector(&mut e, [1.05, 1.95, 3.0, 400.0]);
        assert!(e.call(0x0045_8900, &args![a, near, 0.1f32]).bool());
        assert!(!e.call(0x0045_8900, &args![a, near, 0.01f32]).bool());
        let nan = vector(&mut e, [f32::NAN, 2.0, 3.0, 4.0]);
        assert!(!e.call(0x0045_8900, &args![a, nan, 1.0f32]).bool());
    }

    #[test]
    fn length_is_empty_only_within_the_epsilon_of_zero() {
        let mut e = vector_engine();
        let pick = Ptr::<()>::new(e.mem.alloc(0xb0));
        // All zero: not "non-empty".
        assert!(!e.call(0x0045_88d0, &args![pick]).bool());
        e.mem.set_f32(pick.addr() + 0x90 + 4, 0.0005);
        assert!(!e.call(0x0045_88d0, &args![pick]).bool());
        e.mem.set_f32(pick.addr() + 0x90 + 8, -0.5);
        assert!(e.call(0x0045_88d0, &args![pick]).bool());
        // The w lane is ignored.
        e.mem.set_f32(pick.addr() + 0x90 + 8, 0.0);
        e.mem.set_f32(pick.addr() + 0x90 + 12, 50.0);
        assert!(!e.call(0x0045_88d0, &args![pick]).bool());
    }
}
