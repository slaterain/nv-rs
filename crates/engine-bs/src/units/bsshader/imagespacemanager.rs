//! `BSShader/imagespacemanager.cpp` (Xbox PDB source unit), subsystem `BSShader`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// The thread block's word at +0x2b4 (the allocation context), 0x27 during the set-up.
const TLS_CONTEXT: u32 = 0x2b4;
const CONTEXT_IMAGE_SPACE: u32 = 0x27;
/// The manager's first-run byte (+0x1e4).
const INITIALIZED: u32 = 0x1e4;
/// The set-up steps, each `thiscall` on the manager without stack words: `00b94520` (engine
/// map unit `imagespacemanager.cpp`), `ImageSpaceManager::InitGeometry` (`00b8c170`),
/// `ImageSpaceManager::SetupEffects` (`00b8b360`) and `00b8ad70` (same unit).
const SET_UP_00B94520: u32 = 0x00b9_4520;
const INIT_GEOMETRY: u32 = 0x00b8_c170;
const SETUP_EFFECTS: u32 = 0x00b8_b360;
const SET_UP_00B8AD70: u32 = 0x00b8_ad70;
/// `ImageSpaceShaderParam::InitConstantMap` (engine map, `00b8ab10`, `thiscall` on the object
/// at `01202410`, `RET 8`).
const INIT_CONSTANT_MAP: u32 = 0x00b8_ab10;
const CONSTANT_MAP_01202410: u32 = 0x0120_2410;
/// The `float` stored at +0x304 (`01017870`).
const VALUE_01017870: u32 = 0x0101_7870;
/// `00b6e110` (unnamed, `BSShader`; `thiscall` on the object at `011f91a8`, `RET 0x14`), given
/// the word at `011f4748`, 3, 0, 0, 0; its answer goes to +0x318.
const MAKE_00B6E110: u32 = 0x00b6_e110;
const OBJECT_011F91A8: u32 = 0x011f_91a8;
const OBJECT_011F4748: u32 = 0x011f_4748;

// Translated from 00b977d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ImageSpaceManager::Initialize` (Xbox PDB). `thiscall`, no stack words. With the thread's
/// allocation context (+0x2b4 of the thread block) set to 0x27 for the duration (and put back
/// after): when the byte at +0x1e4 is clear, runs `00b94520` and `InitGeometry` (`00b8c170`),
/// sets the byte, runs `SetupEffects` (`00b8b360`), `ImageSpaceShaderParam::InitConstantMap`
/// on `01202410` with (0, 0) and `00b8ad70`; then stores, in the exe's order, 0.0 at +0x2f8,
/// +0x2fc, +0x308 and +0x300, 1.0 at +0x30c and +0x310, the `float` at `01017870` (through
/// the FPU, a signalling NaN quieted) at +0x304, clears +0, stores 0.0 at +0x314, and keeps
/// at +0x318 what `00b6e110` (on `011f91a8`, with the word at `011f4748`, 3, 0, 0, 0) answers.
// Translation incomplete: 00b977d0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the allocation context) is not translated.
pub fn image_space_manager_initialize(e: &mut Engine, this: Ptr) {
    let base = this.addr();
    let tls = e.tls();
    let saved_context = e.mem.u32(tls.wrapping_add(TLS_CONTEXT));
    e.mem
        .set_u32(tls.wrapping_add(TLS_CONTEXT), CONTEXT_IMAGE_SPACE);
    if e.mem.u8(base.wrapping_add(INITIALIZED)) == 0 {
        e.call(SET_UP_00B94520, &args![this]);
        e.call(INIT_GEOMETRY, &args![this]);
        e.mem.set_u8(base.wrapping_add(INITIALIZED), 1);
        e.call(SETUP_EFFECTS, &args![this]);
        e.call(INIT_CONSTANT_MAP, &args![CONSTANT_MAP_01202410, 0u32, 0u32]);
        e.call(SET_UP_00B8AD70, &args![this]);
        let zero = x87_store32(0.0);
        let one = x87_store32(1.0);
        let loaded = x87_quiet_bits(e.global::<u32>(VALUE_01017870));
        for offset in [0x2f8u32, 0x2fc, 0x308, 0x300] {
            e.mem.set_u32(base.wrapping_add(offset), zero);
        }
        e.mem.set_u32(base.wrapping_add(0x30c), one);
        e.mem.set_u32(base.wrapping_add(0x310), one);
        e.mem.set_u32(base.wrapping_add(0x304), loaded);
        e.mem.set_u32(base, 0);
        e.mem.set_u32(base.wrapping_add(0x314), zero);
        let object = e.global::<u32>(OBJECT_011F91A8);
        let word = e.global::<u32>(OBJECT_011F4748);
        let made = e
            .call(MAKE_00B6E110, &args![object, word, 3u32, 0u32, 0u32, 0u32])
            .u32();
        e.mem.set_u32(base.wrapping_add(0x318), made);
    }
    e.mem.set_u32(tls.wrapping_add(TLS_CONTEXT), saved_context);
}

// Translated from 00b8b360 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ImageSpaceManager::SetupEffects` (Xbox PDB), `thiscall`, no stack words: for each of the
/// 0x22 effect slots of the array at +0x8 (re-read for every slot), calls a set effect's
/// vtable slot +0x8 with the manager and 0.
pub fn image_space_manager_setup_effects(e: &mut Engine, this: Ptr) {
    for index in 0..0x22u32 {
        let effects = e.mem.u32(this.addr() + 8);
        let effect = e.mem.u32(effects.wrapping_add(4 * index));
        if effect != 0 {
            e.vcall(effect, 8, &args![this, 0u32]);
        }
    }
}

/// The `float` `00b8ad70` keeps on the x87 stack besides 1.0 and 0.0 (`01016248`).
const HALF_01016248: u32 = 0x0101_6248;

/// `FLD dword` of the `float` at `address` stored by `FSTP`: its bits, a signalling NaN
/// quieted.
fn loaded(e: &Engine, address: u32) -> u32 {
    x87_quiet_bits(e.global::<u32>(address))
}

// Translated from 00b8ad70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), no arguments: stores the image
/// space defaults in the `float`s from `011ff940` to `011ff9d0`, in the exe's order, from the
/// x87 stack (`FLD`/`FST`/`FXCH` tracked; each `FLD` quiets a signalling NaN): the `float` at
/// `01016248` in `011ff940`, `011ff988`, `011ff98c`, `011ff998` and `011ff9a8`; 1.0 in
/// `011ff94c`, `011ff950`, `011ff954`, `011ff960`, `011ff974`, `011ff948` and `011ff9a4` to
/// `011ff9cc` but `011ff9a8` and `011ff9c0`; 0.0 in `011ff964`, `011ff968`, `011ff9c0`,
/// `011ff9d0`; one constant each in the rest (`01018f98` in `011ff944`, `01022154` in
/// `011ff958`, `0102212c` in `011ff95c`, `01016088` in `011ff96c` and `011ff970`, `01021c9c`
/// in `011ff97c`, `0101df44` in `011ff984`, `01018238` in `011ff980`, `0102d934` in
/// `011ff990`, `010184f4` in `011ff9a0`, `0102d930` in `011ff994`, `0102d92c` in `011ff99c`).
pub fn fn_00b8ad70(e: &mut Engine) {
    let one = 1.0f32.to_bits();
    let zero = 0.0f32.to_bits();
    let half = loaded(e, HALF_01016248);
    e.set_global::<u32>(0x011f_f940, half);
    let value = loaded(e, 0x0101_8f98);
    e.set_global::<u32>(0x011f_f944, value);
    for target in [0x011f_f94cu32, 0x011f_f950, 0x011f_f954] {
        e.set_global::<u32>(target, one);
    }
    let value = loaded(e, 0x0102_2154);
    e.set_global::<u32>(0x011f_f958, value);
    let value = loaded(e, 0x0102_212c);
    e.set_global::<u32>(0x011f_f95c, value);
    e.set_global::<u32>(0x011f_f960, one);
    e.set_global::<u32>(0x011f_f964, zero);
    e.set_global::<u32>(0x011f_f968, zero);
    let value = loaded(e, 0x0101_6088);
    e.set_global::<u32>(0x011f_f96c, value);
    e.set_global::<u32>(0x011f_f970, value);
    e.set_global::<u32>(0x011f_f974, one);
    e.set_global::<u32>(0x011f_f948, one);
    let value = loaded(e, 0x0102_1c9c);
    e.set_global::<u32>(0x011f_f97c, value);
    let value = loaded(e, 0x0101_df44);
    e.set_global::<u32>(0x011f_f984, value);
    let value = loaded(e, 0x0101_8238);
    e.set_global::<u32>(0x011f_f980, value);
    e.set_global::<u32>(0x011f_f988, half);
    e.set_global::<u32>(0x011f_f98c, half);
    let value = loaded(e, 0x0102_d934);
    e.set_global::<u32>(0x011f_f990, value);
    let value = loaded(e, 0x0101_84f4);
    e.set_global::<u32>(0x011f_f9a0, value);
    let value = loaded(e, 0x0102_d930);
    e.set_global::<u32>(0x011f_f994, value);
    e.set_global::<u32>(0x011f_f998, half);
    let value = loaded(e, 0x0102_d92c);
    e.set_global::<u32>(0x011f_f99c, value);
    for target in [
        0x011f_f9b0u32,
        0x011f_f9ac,
        0x011f_f9a4,
        0x011f_f9b4,
        0x011f_f9b8,
        0x011f_f9bc,
        0x011f_f9c4,
        0x011f_f9c8,
        0x011f_f9cc,
    ] {
        e.set_global::<u32>(target, one);
    }
    e.set_global::<u32>(0x011f_f9a8, half);
    e.set_global::<u32>(0x011f_f9c0, zero);
    e.set_global::<u32>(0x011f_f9d0, zero);
}

/// The Ni allocation `00aa13e0` (`cdecl`, size).
const NI_ALLOCATE: u32 = 0x00aa_13e0;
/// Imported `InterlockedIncrement` / `InterlockedDecrement` (import slots).
const IMPORT_INTERLOCKED_INCREMENT: u32 = 0x00fd_f1c0;
const IMPORT_INTERLOCKED_DECREMENT: u32 = 0x00fd_f1c4;
/// `InitGeometry`'s callees (engine map names; all `thiscall`):
/// `NiScreenElementsData::NiScreenElementsData` (`00a881e0`, nine words),
/// `NiScreenElements::NiScreenElements` (`00a87810`, the data), `NiScreenElementsData::Insert`
/// (`00a87ae0`, three words), `::SetRectangle` (`00a88020`: index and four `float`s),
/// `::SetTextures` (`00a88130`: two words and four `float`s), `::UpdateBound` (`00a87900`),
/// the name setter `00a5b950` (unnamed: the address of an `NiFixedString`),
/// `NiAVObject::UpdateProperties` (`00a5a040`) and the update `00a59c60` (unnamed: the address
/// of an update block); and the name maker `00a5b690` (`cdecl`, a string).
const SCREEN_ELEMENTS_DATA_CONSTRUCT: u32 = 0x00a8_81e0;
const SCREEN_ELEMENTS_CONSTRUCT: u32 = 0x00a8_7810;
const SCREEN_ELEMENTS_DATA_INSERT: u32 = 0x00a8_7ae0;
const SCREEN_ELEMENTS_DATA_SET_RECTANGLE: u32 = 0x00a8_8020;
const SCREEN_ELEMENTS_DATA_SET_TEXTURES: u32 = 0x00a8_8130;
const SCREEN_ELEMENTS_DATA_UPDATE_BOUND: u32 = 0x00a8_7900;
const SET_NAME_00A5B950: u32 = 0x00a5_b950;
const UPDATE_PROPERTIES: u32 = 0x00a5_a040;
const UPDATE_00A59C60: u32 = 0x00a5_9c60;
const MAKE_FIXED_STRING_00A5B690: u32 = 0x00a5_b690;
/// `"Imagespace Effect Geometry"` (`010b04ac`) and the empty name word (`0109b220`).
const GEOMETRY_NAME_010B04AC: u32 = 0x010b_04ac;
const NULL_NAME_0109B220: u32 = 0x0109_b220;
/// The rectangle `float`s (`01012054`, `010162c0`, `0104a494`; each through `FLD`).
const RECTANGLE_X_01012054: u32 = 0x0101_2054;
const RECTANGLE_WIDTH_010162C0: u32 = 0x0101_62c0;
const RECTANGLE_HEIGHT_0104A494: u32 = 0x0104_a494;

/// `NiPointer<T>::operator=(T *)` expanded inline on a field of the manager.
fn assign_pointer(e: &mut Engine, slot: u32, object: u32) {
    let old = e.mem.u32(slot);
    if old == object {
        return;
    }
    if old != 0 {
        let left = e.call(IMPORT_INTERLOCKED_DECREMENT, &args![old + 4]).u32();
        if left == 0 {
            e.vcall(old, 4, &args![]);
        }
    }
    e.mem.set_u32(slot, object);
    if object != 0 {
        e.call(IMPORT_INTERLOCKED_INCREMENT, &args![object + 4]);
    }
}

// Translated from 00b8c170 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ImageSpaceManager::InitGeometry` (Xbox PDB), `thiscall`, no stack words. Makes an
/// `NiScreenElementsData` (0x70 bytes, constructed with 0, 0, 1, 1, 1, 4, 1, 2, 1; null when the
/// allocation fails) into the `NiPointer` at +0x1f4 and an `NiScreenElements` on it (0xc4
/// bytes) into the one at +0x1f0. On the elements' data (+0xb8 of the elements, read again
/// for each call): `Insert(4, 0, 0)`, `SetRectangle(0, x, 1.0, width, height)` with the
/// `float`s at `01012054`, `010162c0`, `0104a494`, `SetTextures(0, 0, 0.0, 0.0, 1.0, 1.0)` and
/// `UpdateBound`. Then names the elements `"Imagespace Effect Geometry"` (a name from
/// `00a5b690` passed by address to `00a5b950`, released afterwards unless it is the empty
/// word at `0109b220`), runs `UpdateProperties` and the update `00a59c60` with a stack block of
/// a 0.0 `float` and five clear bytes.
// Translation incomplete: 00b8c170 - the C++ exception frame (FS:[0] chain, unwind states 0 to 2 for the allocations and the name) is not translated.
pub fn image_space_manager_init_geometry(e: &mut Engine, this: Ptr) {
    let base = this.addr();
    let block = e.call(NI_ALLOCATE, &args![0x70u32]).u32();
    let data = if block != 0 {
        e.call(
            SCREEN_ELEMENTS_DATA_CONSTRUCT,
            &args![block, 0u32, 0u32, 1u32, 1u32, 1u32, 4u32, 1u32, 2u32, 1u32],
        )
        .u32()
    } else {
        0
    };
    assign_pointer(e, base + 0x1f4, data);
    let block = e.call(NI_ALLOCATE, &args![0xc4u32]).u32();
    let elements = if block != 0 {
        let data = e.mem.u32(base + 0x1f4);
        e.call(SCREEN_ELEMENTS_CONSTRUCT, &args![block, data]).u32()
    } else {
        0
    };
    assign_pointer(e, base + 0x1f0, elements);
    let elements_data = |e: &Engine| e.mem.u32(e.mem.u32(base + 0x1f0) + 0xb8);
    let data = elements_data(e);
    e.call(SCREEN_ELEMENTS_DATA_INSERT, &args![data, 4u32, 0u32, 0u32]);
    let height = x87_quiet_bits(e.global::<u32>(RECTANGLE_HEIGHT_0104A494));
    let data = elements_data(e);
    let width = x87_quiet_bits(e.global::<u32>(RECTANGLE_WIDTH_010162C0));
    let x = x87_quiet_bits(e.global::<u32>(RECTANGLE_X_01012054));
    e.call(
        SCREEN_ELEMENTS_DATA_SET_RECTANGLE,
        &args![data, 0u32, x, 1.0f32.to_bits(), width, height],
    );
    let data = elements_data(e);
    e.call(
        SCREEN_ELEMENTS_DATA_SET_TEXTURES,
        &args![
            data,
            0u32,
            0u32,
            0.0f32.to_bits(),
            0.0f32.to_bits(),
            1.0f32.to_bits(),
            1.0f32.to_bits()
        ],
    );
    let data = elements_data(e);
    e.call(SCREEN_ELEMENTS_DATA_UPDATE_BOUND, &args![data]);
    e.with_stack(0x10, |e, locals| {
        let name_slot = locals.addr();
        let update = name_slot + 4;
        let name = e
            .call(MAKE_FIXED_STRING_00A5B690, &args![GEOMETRY_NAME_010B04AC])
            .u32();
        e.mem.set_u32(name_slot, name);
        let elements = e.mem.u32(base + 0x1f0);
        e.call(SET_NAME_00A5B950, &args![elements, name_slot]);
        let name = e.mem.u32(name_slot);
        if name != e.global::<u32>(NULL_NAME_0109B220) {
            e.call(IMPORT_INTERLOCKED_DECREMENT, &args![name.wrapping_sub(8)]);
        }
        let elements = e.mem.u32(base + 0x1f0);
        e.call(UPDATE_PROPERTIES, &args![elements]);
        let elements = e.mem.u32(base + 0x1f0);
        e.mem.set_f32(update, 0.0);
        for offset in 4..9u32 {
            e.mem.set_u8(update + offset, 0);
        }
        e.call(UPDATE_00A59C60, &args![elements, update]);
    });
}

use crate::units::bsshader::geometrydecalshader::shader_inline::{MEMORY_ALLOCATE, MEMORY_MANAGER};

/// `00ba3aa0` (unnamed, same unit; `thiscall` on the manager's effect array at +0x4, `RET 8`):
/// stores the word the second argument points at under the index.
const ARRAY_SET_00BA3AA0: u32 = 0x00ba_3aa0;
/// The constructor of the 0x60-byte objects made in pairs by `00b94520` (`00ba4640`, unnamed,
/// `thiscall`).
const CONSTRUCT_00BA4640: u32 = 0x00ba_4640;

/// The first objects of `00b94520`, in the exe's order: the size given to the memory manager
/// (`00aa3e40` on `011f6238`), the constructor (`thiscall`) and the array index.
const SET_UP_MANAGED: [(u32, u32, u32); 20] = [
    (0x68, 0x00c0_5680, 0xd),
    (0x68, 0x00bd_93d0, 0x2),
    (0x7c, 0x00c0_3810, 0x8),
    (0x70, 0x00ba_d500, 0x1),
    (0x7c, 0x00c0_5420, 0x0),
    (0xc8, 0x00c0_4d20, 0x3),
    (0x58, 0x00bb_8900, 0xa),
    (0x58, 0x00bc_0600, 0xb),
    (0xbc, 0x00bd_7120, 0x4),
    (0x78, 0x00c0_49c0, 0xc),
    (0x88, 0x00ba_4880, 0x7),
    (0xd4, 0x00b5_a590, 0x5),
    (0x74, 0x00b5_a410, 0x6),
    (0x58, 0x00ba_a160, 0xe),
    (0x64, 0x00c0_4600, 0xf),
    (0xbc, 0x00ba_15e0, 0x1f),
    (0x74, 0x00ba_02f0, 0x20),
    (0x2bc, 0x00b9_b830, 0x21),
    (0x68, 0x00c0_33c0, 0x9),
    (0x58, 0x00bb_9180, 0x1e),
];

/// The `NiMemObject`s of `00b94520` stored through the inline `SetAt`, in the exe's order: the
/// size given to `00aa13e0`, the constructor (`thiscall`) and the array index; the element is
/// the object +0x6c.
const SET_UP_NI: [(u32, u32, u32); 78] = [
    (0xd0, 0x00b8_efa0, 0x22),
    (0xd0, 0x00b8_f010, 0x23),
    (0xd0, 0x00b8_f660, 0x24),
    (0xd0, 0x00b8_f520, 0x25),
    (0xd0, 0x00b8_efa0, 0x26),
    (0xd0, 0x00b8_f1b0, 0x27),
    (0xd0, 0x00b8_f360, 0x28),
    (0xd0, 0x00b9_2180, 0x29),
    (0xd0, 0x00b8_d330, 0x2a),
    (0xd0, 0x00b8_d850, 0x2b),
    (0xd0, 0x00b8_fea0, 0x2c),
    (0xd0, 0x00b8_d5b0, 0x2d),
    (0xd0, 0x00b8_d700, 0x2e),
    (0xd4, 0x00b9_22d0, 0x2f),
    (0xd0, 0x00b8_ee20, 0x32),
    (0xd0, 0x00b8_f7e0, 0x33),
    (0xd0, 0x00b8_fd40, 0x34),
    (0xd0, 0x00b9_17c0, 0x30),
    (0xd0, 0x00b9_1900, 0x31),
    (0xd0, 0x00b9_1b80, 0x35),
    (0xd0, 0x00b8_f930, 0x36),
    (0xd0, 0x00b8_fa80, 0x37),
    (0xd0, 0x00b8_fbe0, 0x38),
    (0xd0, 0x00b8_bea0, 0x39),
    (0xd0, 0x00b8_fff0, 0x3a),
    (0xd0, 0x00b9_0190, 0x3b),
    (0xd0, 0x00b9_0330, 0x3c),
    (0xd0, 0x00b9_0520, 0x3d),
    (0xd0, 0x00b9_06b0, 0x3e),
    (0xd0, 0x00b9_0830, 0x3f),
    (0xd0, 0x00b9_09c0, 0x40),
    (0xd0, 0x00b9_0b50, 0x41),
    (0xd0, 0x00b9_1000, 0x43),
    (0xd0, 0x00b9_11a0, 0x42),
    (0xd0, 0x00b9_1340, 0x45),
    (0xd0, 0x00b9_14e0, 0x44),
    (0xd0, 0x00b9_0ce0, 0x46),
    (0xd0, 0x00b9_0e70, 0x47),
    (0xd0, 0x00b9_1680, 0x48),
    (0xd0, 0x00b8_bce0, 0x49),
    (0xd0, 0x00b9_2500, 0x4b),
    (0xd0, 0x00b9_1a40, 0x4c),
    (0xd0, 0x00b8_d990, 0x4e),
    (0xd0, 0x00b8_dae0, 0x4f),
    (0xd0, 0x00b8_dc30, 0x50),
    (0xd0, 0x00b8_dd80, 0x51),
    (0xd0, 0x00b8_ded0, 0x52),
    (0xd0, 0x00b8_e020, 0x53),
    (0xd0, 0x00b8_e170, 0x54),
    (0xd0, 0x00b8_e2c0, 0x55),
    (0xd0, 0x00b8_e460, 0x56),
    (0xd0, 0x00b8_e600, 0x57),
    (0xd0, 0x00b8_e7a0, 0x58),
    (0xd0, 0x00b8_e940, 0x59),
    (0xd0, 0x00b8_eae0, 0x5a),
    (0xd0, 0x00b8_ec80, 0x5b),
    (0xd0, 0x00b9_32d0, 0x5d),
    (0xd0, 0x00b9_3600, 0x5e),
    (0xd0, 0x00b9_3950, 0x5f),
    (0xd0, 0x00b9_3aa0, 0x60),
    (0xd0, 0x00b9_3bf0, 0x61),
    (0xd0, 0x00b9_3d40, 0x62),
    (0xd0, 0x00b9_3e90, 0x63),
    (0xd0, 0x00b9_3fe0, 0x64),
    (0xd0, 0x00b9_4130, 0x65),
    (0xd0, 0x00b9_4280, 0x66),
    (0xd0, 0x00b9_32d0, 0x67),
    (0xd0, 0x00b9_43d0, 0x68),
    (0xd0, 0x00b9_2690, 0x69),
    (0xd0, 0x00b9_27e0, 0x6a),
    (0xd0, 0x00b9_2930, 0x6b),
    (0xd0, 0x00b9_2a80, 0x6c),
    (0xd0, 0x00b9_2bd0, 0x6d),
    (0xd0, 0x00b9_2d90, 0x6e),
    (0xd0, 0x00b9_2ee0, 0x6f),
    (0xd0, 0x00b9_3030, 0x70),
    (0xd0, 0x00b9_3180, 0x71),
    (0xd0, 0x00b9_1cf0, 0x72),
];

/// The last two `NiMemObject`s of `00b94520` (0xd0 bytes each), stored through `00ba3aa0`: the
/// constructor and the array index.
const SET_UP_NI_ARRAY: [(u32, u32); 2] = [(0x00b9_1e40, 0x73), (0x00b8_c070, 0x4d)];

/// The 16-bit size (+0xe of the manager) and element count (+0x10) of the effect array whose
/// element block pointer is at +0x8.
const EFFECTS: u32 = 0x8;
const EFFECTS_SIZE: u32 = 0xe;
const EFFECTS_COUNT: u32 = 0x10;

/// The element count of the effect array, moved by a wrapping 16-bit step.
fn effects_count_add(e: &mut Engine, base: u32, step: u16) {
    let count = e.mem.u16(base + EFFECTS_COUNT);
    e.mem
        .set_u16(base + EFFECTS_COUNT, count.wrapping_add(step));
}

// Translated from 00b94520 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`; `ImageSpaceManager::Initialize`
/// runs it first), `thiscall` on the manager, no stack words, no answer. Makes the image-space
/// effects and puts them in the effect array at +0x4 (element block +0x8, 16-bit size +0xe and
/// count +0x10):
///
/// - the 20 objects of `SET_UP_MANAGED`: each allocated from the memory manager, constructed
///   when the allocation is not null (else null), kept in a stack word and stored through
///   `00ba3aa0` with its index and the word's address;
/// - for k from 1 to 7, two 0x60-byte objects (constructor `00ba4640`, null when the
///   allocation fails, and called through the vtable even then): vtable +0x20 with k, then
///   +0x24 with 0 (the first) or 1 (the second), stored at index k + 0xf (the first) and
///   k + 0x16 (the second) by a short inline `SetAt` (an index at or past the size makes the
///   size index + 1, and that or an empty slot adds one to the count, without a null check);
/// - the 78 objects of `SET_UP_NI` (allocated through `00aa13e0`), each element (the object
///   +0x6c, or null) stored by the full inline `SetAt` (past the size: the size grows to the
///   index + 1 and a non-null element counts; else a non-null element into an empty slot adds
///   one and a null one over a full slot takes one away);
/// - the two of `SET_UP_NI_ARRAY`, their element (object +0x6c, or null) stored through
///   `00ba3aa0`;
///
/// then, for the elements at +0x88 to +0x1cc of the block (indices 0x22 to 0x73, the block
/// pointer read again for each), when set, calls vtable +0x120 of the object (element - 0x6c).
// Translation incomplete: 00b94520 - the C++ exception frame (FS:[0] chain, unwind states 0 to 0x65 for each allocation) and the stack cookie are not translated.
pub fn fn_00b94520(e: &mut Engine, this: Ptr) {
    let base = this.addr();
    let array = base + 4;
    e.with_stack(4, |e, local| {
        let local = local.addr();
        for (size, construct, index) in SET_UP_MANAGED {
            let object = e.call(MEMORY_ALLOCATE, &args![MEMORY_MANAGER, size]).u32();
            e.mem.set_u32(local, object);
            let made = if object != 0 {
                e.call(construct, &args![object]).u32()
            } else {
                0
            };
            e.mem.set_u32(local, made);
            e.call(ARRAY_SET_00BA3AA0, &args![array, index, local]);
        }
        for k in 1..=7u32 {
            for (flag, index) in [(0u32, k + 0xf), (1, k + 0x16)] {
                let object = e
                    .call(MEMORY_ALLOCATE, &args![MEMORY_MANAGER, 0x60u32])
                    .u32();
                let element = if object != 0 {
                    e.call(CONSTRUCT_00BA4640, &args![object]).u32()
                } else {
                    0
                };
                e.vcall(element, 0x20, &args![k]);
                e.vcall(element, 0x24, &args![flag]);
                let size = u32::from(e.mem.u16(base + EFFECTS_SIZE));
                if index >= size {
                    e.mem.set_u16(base + EFFECTS_SIZE, (index + 1) as u16);
                    effects_count_add(e, base, 1);
                } else {
                    let block = e.mem.u32(base + EFFECTS);
                    if e.mem.u32(block + 4 * index) == 0 {
                        effects_count_add(e, base, 1);
                    }
                }
                let block = e.mem.u32(base + EFFECTS);
                e.mem.set_u32(block + 4 * index, element);
            }
        }
        for (size, construct, index) in SET_UP_NI {
            let object = e.call(NI_ALLOCATE, &args![size]).u32();
            let made = if object != 0 {
                e.call(construct, &args![object]).u32()
            } else {
                0
            };
            let element = if made != 0 { made + 0x6c } else { 0 };
            if index as u16 >= e.mem.u16(base + EFFECTS_SIZE) {
                e.mem.set_u16(base + EFFECTS_SIZE, (index + 1) as u16);
                if element != 0 {
                    effects_count_add(e, base, 1);
                }
            } else if element != 0 {
                let block = e.mem.u32(base + EFFECTS);
                if e.mem.u32(block + 4 * index) == 0 {
                    effects_count_add(e, base, 1);
                }
            } else {
                let block = e.mem.u32(base + EFFECTS);
                if e.mem.u32(block + 4 * index) != 0 {
                    effects_count_add(e, base, 0xffff);
                }
            }
            let block = e.mem.u32(base + EFFECTS);
            e.mem.set_u32(block + 4 * index, element);
        }
        for (construct, index) in SET_UP_NI_ARRAY {
            let object = e.call(NI_ALLOCATE, &args![0xd0u32]).u32();
            let made = if object != 0 {
                e.call(construct, &args![object]).u32()
            } else {
                0
            };
            let element = if made != 0 { made + 0x6c } else { 0 };
            e.mem.set_u32(local, element);
            e.call(ARRAY_SET_00BA3AA0, &args![array, index, local]);
        }
    });
    for offset in (0x88..0x1d0u32).step_by(4) {
        let block = e.mem.u32(base + EFFECTS);
        let element = e.mem.u32(block + offset);
        if element != 0 {
            let effect = if element != 0 { element - 0x6c } else { 0 };
            e.vcall(effect, 0x120, &args![]);
        }
    }
}

/// The `ImageSpaceShader` base constructor of the image-space shaders (`00c04570`, unnamed in
/// the engine map; `thiscall`, no stack words).
const IMAGE_SPACE_SHADER_CONSTRUCT_00C04570: u32 = 0x00c0_4570;
/// The `NiTArray` resize `00a90180` (unnamed in the engine map; `thiscall` on the array,
/// `RET 4`: the new size).
const ARRAY_SET_SIZE_00A90180: u32 = 0x00a9_0180;
/// The texture array of an `ImageSpaceShader` (`pTextures`, Xbox PDB, at the
/// `ImageSpaceEffect` base +0x6c, +0x28 in it: +0x94; the same on PC).
const SHADER_TEXTURES: u32 = 0x94;
/// The `ImageSpaceEffect` base of an `ImageSpaceShader` (Xbox PDB, +0x6c).
const SHADER_EFFECT_BASE: u32 = 0x6c;

/// The body every plain image-space shader constructor below has, in the exe's order: the
/// base constructor `00c04570`, the class's vtable at +0, its `ImageSpaceEffect` vtable at
/// +0x6c, then the texture array (+0x94) resized to `textures` by `00a90180`. Answers `this`.
fn construct_image_space_shader(
    e: &mut Engine,
    this: Ptr,
    vtable: u32,
    effect_vtable: u32,
    textures: u32,
) -> Ptr {
    let base = this.addr();
    e.call(IMAGE_SPACE_SHADER_CONSTRUCT_00C04570, &args![this]);
    e.mem.set_u32(base, vtable);
    e.mem.set_u32(base + SHADER_EFFECT_BASE, effect_vtable);
    e.call(
        ARRAY_SET_SIZE_00A90180,
        &args![base + SHADER_TEXTURES, textures],
    );
    this
}

/// The object `00b922d0` keeps a reference to (`NiPointer` at +0xd0; the word at `011f9520`).
const OBJECT_011F9520: u32 = 0x011f_9520;

// Translated from 00b922d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of the 0xd4-byte `ImageSpaceShader` subclass whose object +0x6c `00b94520`
/// stores at index 0x2f of the effect array (`EFFECT_SHADER_TEXTUREMASK` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed). Runs the base
/// constructor `00c04570`, sets the vtables `010b5978` (+0) and `010b5954` (+0x6c), clears
/// the pointer at +0xd0, sizes the texture array (+0x94) to two (`00a90180`), then assigns
/// the object at `011f9520` to the `NiPointer` at +0xd0 (inline: nothing when it is already
/// there; else the old one's reference given up, deleted through its vtable slot +4 at zero,
/// and a reference taken on the new one when it is not null). Answers `this`.
// Translation incomplete: 00b922d0 - the C++ exception frame (FS:[0] chain, unwind states 0 and 1 for the base and the pointer) is not translated.
pub fn fn_00b922d0(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    e.call(IMAGE_SPACE_SHADER_CONSTRUCT_00C04570, &args![this]);
    e.mem.set_u32(base, 0x010b_5978);
    e.mem.set_u32(base + SHADER_EFFECT_BASE, 0x010b_5954);
    e.mem.set_u32(base + 0xd0, 0);
    e.call(
        ARRAY_SET_SIZE_00A90180,
        &args![base + SHADER_TEXTURES, 2u32],
    );
    let object = e.global::<u32>(OBJECT_011F9520);
    assign_pointer(e, base + 0xd0, object);
    this
}

// Translated from 00b8bce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x49 of the effect array (`EFFECT_SHADER_INTERFACE_VATS_SCANLINES_BLEND` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010affc0` (+0) and `010affa0` (+0x6c) and sizes the texture array (+0x94) to five
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8bce0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8bce0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010a_ffc0, 0x010a_ffa0, 5)
}

// Translated from 00b8bea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x39 of the effect array (`EFFECT_SHADER_SUNBEAMS` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b0198` (+0) and `010b0174` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8bea0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8bea0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_0198, 0x010b_0174, 2)
}

// Translated from 00b8c070 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x4d of the effect array (`EFFECT_SHADER_TV` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b0338` (+0) and `010b0318` (+0x6c) and sizes the texture array (+0x94) to four
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8c070 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8c070(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_0338, 0x010b_0318, 4)
}

// Translated from 00b8d330 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x2a of the effect array (`EFFECT_SHADER_VOLUMETRIC_FOG` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b0518` (+0) and `010b04f4` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8d330 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8d330(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_0518, 0x010b_04f4, 3)
}

// Translated from 00b8d5b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x2d of the effect array (`EFFECT_SHADER_ALPHAMULT` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b06c0` (+0) and `010b06a0` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8d5b0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8d5b0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_06c0, 0x010b_06a0, 2)
}

// Translated from 00b8d700 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x2e of the effect array (`EFFECT_SHADER_BLEND` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b0860` (+0) and `010b083c` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8d700 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8d700(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_0860, 0x010b_083c, 3)
}

// Translated from 00b8d850 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x2b of the effect array (`EFFECT_SHADER_BLUR` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b0a00` (+0) and `010b09dc` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8d850 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8d850(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_0a00, 0x010b_09dc, 2)
}

// Translated from 00b8d990 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x4e of the effect array (`EFFECT_SHADER_BLUR3` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b0b98` (+0) and `010b0b78` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8d990 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8d990(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_0b98, 0x010b_0b78, 2)
}

// Translated from 00b8dae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x4f of the effect array (`EFFECT_SHADER_BLUR5` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b0d40` (+0) and `010b0d1c` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8dae0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8dae0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_0d40, 0x010b_0d1c, 2)
}

// Translated from 00b8dc30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x50 of the effect array (`EFFECT_SHADER_BLUR7` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b0ec0` (+0) and `010b0e9c` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8dc30 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8dc30(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_0ec0, 0x010b_0e9c, 2)
}

// Translated from 00b8dd80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x51 of the effect array (`EFFECT_SHADER_BLUR9` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b1040` (+0) and `010b101c` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8dd80 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8dd80(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_1040, 0x010b_101c, 2)
}

// Translated from 00b8ded0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x52 of the effect array (`EFFECT_SHADER_BLUR11` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b11c0` (+0) and `010b119c` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8ded0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8ded0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_11c0, 0x010b_119c, 2)
}

// Translated from 00b8e020 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x53 of the effect array (`EFFECT_SHADER_BLUR13` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b1340` (+0) and `010b1320` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8e020 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8e020(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_1340, 0x010b_1320, 2)
}

// Translated from 00b8e170 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x54 of the effect array (`EFFECT_SHADER_BLUR15` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b14c0` (+0) and `010b14a0` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8e170 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8e170(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_14c0, 0x010b_14a0, 2)
}

// Translated from 00b8e2c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x55 of the effect array (`EFFECT_SHADER_BRIGHTPASS_BLUR3` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b1640` (+0) and `010b1620` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8e2c0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8e2c0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_1640, 0x010b_1620, 3)
}

// Translated from 00b8e460 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x56 of the effect array (`EFFECT_SHADER_BRIGHTPASS_BLUR5` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b17f8` (+0) and `010b17d8` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8e460 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8e460(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_17f8, 0x010b_17d8, 3)
}

// Translated from 00b8e600 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x57 of the effect array (`EFFECT_SHADER_BRIGHTPASS_BLUR7` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b1980` (+0) and `010b1960` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8e600 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8e600(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_1980, 0x010b_1960, 3)
}

// Translated from 00b8e7a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x58 of the effect array (`EFFECT_SHADER_BRIGHTPASS_BLUR9` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b1b08` (+0) and `010b1ae8` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8e7a0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8e7a0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_1b08, 0x010b_1ae8, 3)
}

// Translated from 00b8e940 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x59 of the effect array (`EFFECT_SHADER_BRIGHTPASS_BLUR11` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b1c90` (+0) and `010b1c70` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8e940 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8e940(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_1c90, 0x010b_1c70, 3)
}

// Translated from 00b8eae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x5a of the effect array (`EFFECT_SHADER_BRIGHTPASS_BLUR13` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b1e20` (+0) and `010b1dfc` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8eae0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8eae0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_1e20, 0x010b_1dfc, 3)
}

// Translated from 00b8ec80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x5b of the effect array (`EFFECT_SHADER_BRIGHTPASS_BLUR15` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b1fb0` (+0) and `010b1f8c` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8ec80 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8ec80(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_1fb0, 0x010b_1f8c, 3)
}

// Translated from 00b8ee20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x32 of the effect array (`EFFECT_SHADER_CINEMATIC` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b2140` (+0) and `010b211c` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8ee20 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8ee20(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_2140, 0x010b_211c, 2)
}

// Translated from 00b8efa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// indices 0x22 and 0x26 of the effect array (`EFFECT_SHADER_COPY` and `EFFECT_SHADER_COPYWITHALPHA` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b2a98` (+0) and `010b2a74` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8efa0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8efa0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_2a98, 0x010b_2a74, 2)
}

// Translated from 00b8f010 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x23 of the effect array (`EFFECT_SHADER_COPY_RENDER_TARGET1` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b2308` (+0) and `010b22e8` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8f010 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8f010(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_2308, 0x010b_22e8, 2)
}

// Translated from 00b8f1b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x27 of the effect array (`EFFECT_SHADER_STENCILCOPY` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b2490` (+0) and `010b2470` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8f1b0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8f1b0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_2490, 0x010b_2470, 2)
}

// Translated from 00b8f360 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x28 of the effect array (`EFFECT_SHADER_STENCILCOPY_NOTEX_NODEPTH` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b2610` (+0) and `010b25f0` (+0x6c) and sizes the texture array (+0x94) to one
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8f360 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8f360(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_2610, 0x010b_25f0, 1)
}

// Translated from 00b8f520 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x25 of the effect array (`EFFECT_SHADER_NULL` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b27a8` (+0) and `010b2784` (+0x6c) and sizes the texture array (+0x94) to one
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8f520 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8f520(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_27a8, 0x010b_2784, 1)
}

// Translated from 00b8f660 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x24 of the effect array (`EFFECT_SHADER_RESTORE_EDRAM` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b2920` (+0) and `010b28fc` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8f660 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8f660(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_2920, 0x010b_28fc, 3)
}

// Translated from 00b8f7e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x33 of the effect array (`EFFECT_SHADER_DEPTH_OF_FIELD` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b2c10` (+0) and `010b2bec` (+0x6c) and sizes the texture array (+0x94) to four
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8f7e0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8f7e0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_2c10, 0x010b_2bec, 4)
}

// Translated from 00b8f930 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x36 of the effect array (`EFFECT_SHADER_RADIAL_BLUR` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b2db0` (+0) and `010b2d8c` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8f930 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8f930(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_2db0, 0x010b_2d8c, 2)
}

// Translated from 00b8fa80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x37 of the effect array (`EFFECT_SHADER_RADIAL_BLUR_MED` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b2f50` (+0) and `010b2f30` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8fa80 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8fa80(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_2f50, 0x010b_2f30, 2)
}

// Translated from 00b8fbe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x38 of the effect array (`EFFECT_SHADER_RADIAL_BLUR_HIGH` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b30e0` (+0) and `010b30c0` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8fbe0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8fbe0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_30e0, 0x010b_30c0, 2)
}

// Translated from 00b8fd40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x34 of the effect array (`EFFECT_SHADER_DEPTH_OF_FIELD_MASK` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b3268` (+0) and `010b3244` (+0x6c) and sizes the texture array (+0x94) to five
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8fd40 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8fd40(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_3268, 0x010b_3244, 5)
}

// Translated from 00b8fea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x2c of the effect array (`EFFECT_SHADER_DOUBLEVIS` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b33f0` (+0) and `010b33cc` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8fea0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8fea0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_33f0, 0x010b_33cc, 3)
}

// Translated from 00b8fff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x3a of the effect array (`EFFECT_SHADER_HDR_BLENDINSHADER` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b3598` (+0) and `010b3574` (+0x6c) and sizes the texture array (+0x94) to four
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b8fff0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b8fff0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_3598, 0x010b_3574, 4)
}

// Translated from 00b90190 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x3b of the effect array (`EFFECT_SHADER_HDR_BLENDINSHADER_CINEMATIC` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b37a8` (+0) and `010b3788` (+0x6c) and sizes the texture array (+0x94) to four
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b90190 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b90190(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_37a8, 0x010b_3788, 4)
}

// Translated from 00b90330 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x3c of the effect array (`EFFECT_SHADER_HDR_BLENDINSHADER_CINEMATIC_ALPHAMASK` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b3958` (+0) and `010b3934` (+0x6c) and sizes the texture array (+0x94) to four
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b90330 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b90330(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_3958, 0x010b_3934, 4)
}

// Translated from 00b90520 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x3d of the effect array (`EFFECT_SHADER_HDR_BLUR` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b3af0` (+0) and `010b3ad0` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b90520 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b90520(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_3af0, 0x010b_3ad0, 2)
}

// Translated from 00b906b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x3e of the effect array (`EFFECT_SHADER_HDR_BRIGHTPASSFILTER` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b3c78` (+0) and `010b3c58` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b906b0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b906b0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_3c78, 0x010b_3c58, 2)
}

// Translated from 00b90830 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x3f of the effect array (`EFFECT_SHADER_HDR_DOWNSAMPLE16` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b3e18` (+0) and `010b3df4` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b90830 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b90830(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_3e18, 0x010b_3df4, 2)
}

// Translated from 00b909c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x40 of the effect array (`EFFECT_SHADER_HDR_DOWNSAMPLE9` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b3fa8` (+0) and `010b3f88` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b909c0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b909c0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_3fa8, 0x010b_3f88, 2)
}

// Translated from 00b90b50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x41 of the effect array (`EFFECT_SHADER_HDR_DOWNSAMPLE4` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b4138` (+0) and `010b4114` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b90b50 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b90b50(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_4138, 0x010b_4114, 2)
}

// Translated from 00b90ce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x46 of the effect array (`EFFECT_SHADER_HDR_LIGHTADAPT` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b42c8` (+0) and `010b42a4` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b90ce0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b90ce0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_42c8, 0x010b_42a4, 3)
}

// Translated from 00b90e70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x47 of the effect array (`EFFECT_SHADER_HDR_LUMCLAMP` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b4460` (+0) and `010b443c` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b90e70 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b90e70(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_4460, 0x010b_443c, 2)
}

// Translated from 00b91000 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x43 of the effect array (`EFFECT_SHADER_HDR_DOWNSAMPLE4_LIGHTADAPT` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b4600` (+0) and `010b45dc` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b91000 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b91000(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_4600, 0x010b_45dc, 3)
}

// Translated from 00b911a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x42 of the effect array (`EFFECT_SHADER_HDR_DOWNSAMPLE4_LUMCLAMP` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b47a0` (+0) and `010b477c` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b911a0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b911a0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_47a0, 0x010b_477c, 2)
}

// Translated from 00b91340 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x45 of the effect array (`EFFECT_SHADER_HDR_DOWNSAMPLE16_LIGHTADAPT` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b4938` (+0) and `010b4914` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b91340 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b91340(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_4938, 0x010b_4914, 3)
}

// Translated from 00b914e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x44 of the effect array (`EFFECT_SHADER_HDR_DOWNSAMPLE16_LUMCLAMP` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b4ad0` (+0) and `010b4aac` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b914e0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b914e0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_4ad0, 0x010b_4aac, 2)
}

// Translated from 00b91680 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x48 of the effect array (`EFFECT_SHADER_INTERFACE_SCANLINES_BLEND` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b4c68` (+0) and `010b4c48` (+0x6c) and sizes the texture array (+0x94) to five
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b91680 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b91680(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_4c68, 0x010b_4c48, 5)
}

// Translated from 00b917c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x30 of the effect array (`EFFECT_SHADER_MAP` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b4de8` (+0) and `010b4dc8` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b917c0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b917c0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_4de8, 0x010b_4dc8, 2)
}

// Translated from 00b91900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x31 of the effect array (`EFFECT_SHADER_MENUBG` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b4f80` (+0) and `010b4f60` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b91900 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b91900(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_4f80, 0x010b_4f60, 3)
}

// Translated from 00b91a40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x4c of the effect array (`EFFECT_SHADER_INTERFACE_MENUUNPACK` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b5140` (+0) and `010b511c` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b91a40 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b91a40(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_5140, 0x010b_511c, 2)
}

// Translated from 00b91b80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x35 of the effect array (`EFFECT_SHADER_MOTION_BLUR` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b52c0` (+0) and `010b52a0` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b91b80 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b91b80(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_52c0, 0x010b_52a0, 3)
}

// Translated from 00b91cf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x72 of the effect array (`EFFECT_SHADER_NOISE_SCROLL_AND_BLEND` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b5480` (+0) and `010b5460` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b91cf0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b91cf0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_5480, 0x010b_5460, 2)
}

// Translated from 00b91e40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x73 of the effect array (`EFFECT_SHADER_NOISE_NORMALMAP` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b5640` (+0) and `010b5620` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b91e40 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b91e40(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_5640, 0x010b_5620, 2)
}

// Translated from 00b92180 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x29 of the effect array (`EFFECT_SHADER_REFRACTION` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b57d0` (+0) and `010b57b0` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b92180 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b92180(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_57d0, 0x010b_57b0, 3)
}

// Translated from 00b92500 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x4b of the effect array (`EFFECT_SHADER_INTERFACE_VATS_SCAN` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b5b10` (+0) and `010b5af0` (+0x6c) and sizes the texture array (+0x94) to five
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b92500 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b92500(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_5b10, 0x010b_5af0, 5)
}

// Translated from 00b92690 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x69 of the effect array (`EFFECT_SHADER_WATER_DISPLACEMENT_CLEAR_SIMULATION` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b5cb8` (+0) and `010b5c94` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b92690 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b92690(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_5cb8, 0x010b_5c94, 2)
}

// Translated from 00b927e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x6a of the effect array (`EFFECT_SHADER_WATER_DISPLACEMENT_TEX_OFFSET` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b5ea8` (+0) and `010b5e84` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b927e0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b927e0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_5ea8, 0x010b_5e84, 2)
}

// Translated from 00b92930 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x6b of the effect array (`EFFECT_SHADER_WATER_DISPLACEMENT_WADING_RIPPLE` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b6048` (+0) and `010b6028` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b92930 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b92930(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_6048, 0x010b_6028, 2)
}

// Translated from 00b92a80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x6c of the effect array (`EFFECT_SHADER_WATER_DISPLACEMENT_RAIN_RIPPLE` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b61f0` (+0) and `010b61d0` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b92a80 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b92a80(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_61f0, 0x010b_61d0, 2)
}

// Translated from 00b92bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x6d of the effect array (`EFFECT_SHADER_WATER_DISPLACEMENT_WADING_HEIGHTMAP` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b6398` (+0) and `010b6374` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b92bd0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b92bd0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_6398, 0x010b_6374, 2)
}

// Translated from 00b92d90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x6e of the effect array (`EFFECT_SHADER_WATER_DISPLACEMENT_RAIN_HEIGHTMAP` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b6548` (+0) and `010b6524` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b92d90 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b92d90(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_6548, 0x010b_6524, 2)
}

// Translated from 00b92ee0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x6f of the effect array (`EFFECT_SHADER_WATER_DISPLACEMENT_BLEND_HEIGHTMAPS` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b66f0` (+0) and `010b66cc` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b92ee0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b92ee0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_66f0, 0x010b_66cc, 3)
}

// Translated from 00b93030 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x70 of the effect array (`EFFECT_SHADER_WATER_DISPLACEMENT_SMOOTH_HEIGHTMAP` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b68a0` (+0) and `010b687c` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b93030 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b93030(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_68a0, 0x010b_687c, 2)
}

// Translated from 00b93180 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x71 of the effect array (`EFFECT_SHADER_WATER_DISPLACEMENT_NORMALS` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b6a40` (+0) and `010b6a20` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b93180 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b93180(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_6a40, 0x010b_6a20, 2)
}

// Translated from 00b932d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// indices 0x5d and 0x67 of the effect array (`EFFECT_SHADER_WATER_FFT_WATER_SPECTRUM` and `EFFECT_SHADER_WATER_FFT_DISPLAY_WATER_SPECTRUM` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b6bd8` (+0) and `010b6bb8` (+0x6c) and sizes the texture array (+0x94) to three
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b932d0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b932d0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_6bd8, 0x010b_6bb8, 3)
}

// Translated from 00b93600 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x5e of the effect array (`EFFECT_SHADER_WATER_FFT_HORIZONTAL_BUTTERFLY` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b6d90` (+0) and `010b6d70` (+0x6c) and sizes the texture array (+0x94) to four
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b93600 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b93600(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_6d90, 0x010b_6d70, 4)
}

// Translated from 00b93950 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x5f of the effect array (`EFFECT_SHADER_WATER_FFT_VERTICAL_BUTTERFLY` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b6f38` (+0) and `010b6f14` (+0x6c) and sizes the texture array (+0x94) to four
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b93950 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b93950(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_6f38, 0x010b_6f14, 4)
}

// Translated from 00b93aa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x60 of the effect array (`EFFECT_SHADER_WATER_FFT_HORIZONTAL_SCRAMBLE` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b70d8` (+0) and `010b70b8` (+0x6c) and sizes the texture array (+0x94) to four
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b93aa0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b93aa0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_70d8, 0x010b_70b8, 4)
}

// Translated from 00b93bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x61 of the effect array (`EFFECT_SHADER_WATER_FFT_VERTICAL_SCRAMBLE` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b7278` (+0) and `010b7258` (+0x6c) and sizes the texture array (+0x94) to four
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b93bf0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b93bf0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_7278, 0x010b_7258, 4)
}

// Translated from 00b93d40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x62 of the effect array (`EFFECT_SHADER_WATER_FFT_NORMALS` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b7418` (+0) and `010b73f4` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b93d40 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b93d40(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_7418, 0x010b_73f4, 2)
}

// Translated from 00b93e90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x63 of the effect array (`EFFECT_SHADER_WATER_FFT_FILTER` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b7598` (+0) and `010b7578` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b93e90 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b93e90(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_7598, 0x010b_7578, 2)
}

// Translated from 00b93fe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x64 of the effect array (`EFFECT_SHADER_WATER_FFT_DISPLAY_NORMALS` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b7720` (+0) and `010b7700` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b93fe0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b93fe0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_7720, 0x010b_7700, 2)
}

// Translated from 00b94130 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x65 of the effect array (`EFFECT_SHADER_WATER_FFT_DISPLAY_HIGHW` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b78b8` (+0) and `010b7898` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b94130 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b94130(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_78b8, 0x010b_7898, 2)
}

// Translated from 00b94280 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x66 of the effect array (`EFFECT_SHADER_WATER_FFT_DISPLAY_AMPLITUDE` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b7a50` (+0) and `010b7a2c` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b94280 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b94280(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_7a50, 0x010b_7a2c, 2)
}

// Translated from 00b943d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// constructor of an `ImageSpaceShader` subclass whose object +0x6c `00b94520` stores at
/// index 0x68 of the effect array (`EFFECT_SHADER_WATER_FFT_DISPLAY_WATER_HEIGHT` in the Xbox PDB's
/// `ImageSpaceManager::ImageSpaceEffectEnum`; the class name is unconfirmed, it keeps
/// `ImageSpaceShader`'s `GetRTTI`). Runs the base constructor `00c04570`, sets the vtables
/// `010b7bf0` (+0) and `010b7bcc` (+0x6c) and sizes the texture array (+0x94) to two
/// (`00a90180`). Answers `this`.
// Translation incomplete: 00b943d0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b943d0(e: &mut Engine, this: Ptr) -> Ptr {
    construct_image_space_shader(e, this, 0x010b_7bf0, 0x010b_7bcc, 2)
}

/// The set-up `00b8b270` (this unit, unnamed; `thiscall` on the object at +0x58, two stack
/// words: an index and the address of a byte).
const SET_UP_00B8B270: u32 = 0x00b8_b270;

// Translated from 00b8bc20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: calls
/// `00b8b270` on +0x58 with the indices 0 to 3, each with the address of a stack byte (the high
/// byte of the word the function saved `ECX` in) set just before the call to 1, 1, 1 and 0.
pub fn fn_00b8bc20(e: &mut Engine, this: Ptr) {
    let target = this.addr() + 0x58;
    e.with_stack(4, |e, local| {
        let flag = local.addr() + 3;
        for (index, value) in [(0u32, 1u8), (1, 1), (2, 1), (3, 0)] {
            e.mem.set_u8(flag, value);
            e.call(SET_UP_00B8B270, &args![target, index, flag]);
        }
    });
}

// Translated from 00b8b270 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`): the set-at of an `NiTArray` of
/// bytes (base +4, size +0xa, element count +0xc), `thiscall` with two stack words: the index
/// and the address of the byte to store. At or past the size the size becomes the index plus one
/// and a set byte raises the element count. Below it a set byte replacing a clear one raises the
/// count and a clear byte replacing a set one lowers it. The byte is then stored at the index.
pub fn fn_00b8b270(e: &mut Engine, this: Ptr, index: u32, value: Ptr) {
    let base = this.addr();
    let byte = e.mem.u8(value.addr());
    let count = e.mem.u16(base + 0xc);
    if index >= u32::from(e.mem.u16(base + 0xa)) {
        e.mem.set_u16(base + 0xa, index.wrapping_add(1) as u16);
        if byte != 0 {
            e.mem.set_u16(base + 0xc, count.wrapping_add(1));
        }
    } else {
        let held = e.mem.u8(e.mem.u32(base + 4).wrapping_add(index));
        if byte != 0 {
            if held == 0 {
                e.mem.set_u16(base + 0xc, count.wrapping_add(1));
            }
        } else if held != 0 {
            e.mem.set_u16(base + 0xc, count.wrapping_add(0xffff));
        }
    }
    let elements = e.mem.u32(base + 4);
    let byte = e.mem.u8(value.addr());
    e.mem.set_u8(elements.wrapping_add(index), byte);
}

/// `ImageSpaceShader::~ImageSpaceShader` (engine map, `00c03a20`; `thiscall`, no stack words).
const IMAGE_SPACE_SHADER_DESTRUCT_00C03A20: u32 = 0x00c0_3a20;
/// The sized Ni free `00aa1460` (unnamed, `NiSystem`; `cdecl`: the block and its size).
const NI_SIZED_FREE_00AA1460: u32 = 0x00aa_1460;

/// The body of the image-space shaders' deleting destructors below, in the exe's order: the
/// class's vtable at +0, its `ImageSpaceEffect` vtable at +0x6c, `~ImageSpaceShader`
/// (`00c03a20`), then, when bit 0 of the flags is set, the 0xd0 bytes freed through `00aa1460`.
/// Answers `this`.
fn delete_image_space_shader(
    e: &mut Engine,
    this: Ptr,
    vtable: u32,
    effect_vtable: u32,
    flags: u32,
) -> Ptr {
    let base = this.addr();
    e.mem.set_u32(base, vtable);
    e.mem.set_u32(base + SHADER_EFFECT_BASE, effect_vtable);
    e.call(IMAGE_SPACE_SHADER_DESTRUCT_00C03A20, &args![this]);
    if flags & 1 != 0 {
        e.call(NI_SIZED_FREE_00AA1460, &args![this, 0xd0u32]);
    }
    this
}

// Translated from 00b91b50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): a deleting destructor that sets no vtable (slot 0 of the vtables `010b0338`, the one
/// `00b8c070` constructs with, `010b5140` and `010bc8f8`): `~ImageSpaceShader` (`00c03a20`),
/// then, when bit 0 of the flags is set, the 0xd0 bytes freed through `00aa1460`. Answers
/// `this`.
pub fn fn_00b91b50(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(IMAGE_SPACE_SHADER_DESTRUCT_00C03A20, &args![this]);
    if flags & 1 != 0 {
        e.call(NI_SIZED_FREE_00AA1460, &args![this, 0xd0u32]);
    }
    this
}

// Translated from 00b91b40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// `00b91b50` (slot 0 of the vtables `010b0318` and `010b511c`): `SUB ECX,0x6c`, then a jump to
/// it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b91b40(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b91b50(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b92450 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, no stack words: the
/// destructor of the shader `00b922d0` constructs. Sets its vtables `010b5978` (+0) and
/// `010b5954` (+0x6c), gives up the reference of the `NiPointer` at +0xd0 when it is set (the
/// imported `InterlockedDecrement` on its +4; at zero, its vtable slot +4), then runs
/// `~ImageSpaceShader` (`00c03a20`).
// Translation incomplete: 00b92450 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn fn_00b92450(e: &mut Engine, this: Ptr) {
    let base = this.addr();
    e.mem.set_u32(base, 0x010b_5978);
    e.mem.set_u32(base + SHADER_EFFECT_BASE, 0x010b_5954);
    let object = e.mem.u32(base + 0xd0);
    if object != 0 {
        let left = e
            .call(IMPORT_INTERLOCKED_DECREMENT, &args![object + 4])
            .u32();
        if left == 0 {
            e.vcall(object, 4, &args![]);
        }
    }
    e.call(IMAGE_SPACE_SHADER_DESTRUCT_00C03A20, &args![this]);
}

// Translated from 00b924d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b922d0` constructs (slot 0 of `010b5978`):
/// `00b92450`, then, when bit 0 of the flags is set, the 0xd4 bytes freed through `00aa1460`.
/// Answers `this`.
pub fn fn_00b924d0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b92450(e, this);
    if flags & 1 != 0 {
        e.call(NI_SIZED_FREE_00AA1460, &args![this, 0xd4u32]);
    }
    this
}

// Translated from 00b92440 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// `00b924d0` (slot 0 of the vtable `010b5954`): `SUB ECX,0x6c`, then a jump to it with the same
/// stack word (the flags). Answers what it answers.
pub fn fn_00b92440(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b924d0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

/// `ImageSpaceShaderVolumetricFog::ReturnVolumetricFogTexture` (engine map, `00b63d10`;
/// `thiscall`, no stack words), `ImageSpaceTexture::~ImageSpaceTexture` (engine map, `00ba3a00`;
/// `thiscall`), the memory manager's free `00aa4060` (`thiscall` on `011f6238`, the block) and
/// the effect array's set-at `00ba3aa0` (on the texture array +0x94: the index and the address
/// of the word to store).
const RETURN_VOLUMETRIC_FOG_TEXTURE_00B63D10: u32 = 0x00b6_3d10;
const IMAGE_SPACE_TEXTURE_DESTRUCT_00BA3A00: u32 = 0x00ba_3a00;
const MEMORY_FREE_00AA4060: u32 = 0x00aa_4060;

// Translated from 00b8d4d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ImageSpaceShaderVolumetricFog::~ImageSpaceShaderVolumetricFog` (Xbox PDB), `thiscall`, no
/// stack words. Sets the vtables `010b0518` (+0) and `010b04f4` (+0x6c), runs
/// `ReturnVolumetricFogTexture` (`00b63d10`); when the texture array's (+0x94) element block
/// (+0x98) holds a texture at index 2, destroys it (`~ImageSpaceTexture`, `00ba3a00`) and frees
/// it through the memory manager (`00aa4060` on `011f6238`) when it is not null, and stores null
/// at index 2 through `00ba3aa0`; then runs `~ImageSpaceShader` (`00c03a20`).
// Translation incomplete: 00b8d4d0 - the C++ exception frame (FS:[0] chain, unwind state 0 for the base) is not translated.
pub fn image_space_shader_volumetric_fog_destructor(e: &mut Engine, this: Ptr) {
    let base = this.addr();
    e.mem.set_u32(base, 0x010b_0518);
    e.mem.set_u32(base + SHADER_EFFECT_BASE, 0x010b_04f4);
    e.call(RETURN_VOLUMETRIC_FOG_TEXTURE_00B63D10, &args![this]);
    let block = e.mem.u32(base + 0x98);
    if e.mem.u32(block + 8) != 0 {
        let texture = e.mem.u32(block + 8);
        if texture != 0 {
            e.call(IMAGE_SPACE_TEXTURE_DESTRUCT_00BA3A00, &args![texture]);
            e.call(MEMORY_FREE_00AA4060, &args![MEMORY_MANAGER, texture]);
        }
        e.with_stack(4, |e, local| {
            e.mem.set_u32(local.addr(), 0);
            e.call(
                ARRAY_SET_00BA3AA0,
                &args![base + SHADER_TEXTURES, 2u32, local],
            );
        });
    }
    e.call(IMAGE_SPACE_SHADER_DESTRUCT_00C03A20, &args![this]);
}

// Translated from 00b8d580 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the volumetric fog shader (slot 0 of `010b0518`):
/// `~ImageSpaceShaderVolumetricFog` (`00b8d4d0`), then, when bit 0 of the flags is set, the
/// 0xd0 bytes freed through `00aa1460`. Answers `this`.
pub fn fn_00b8d580(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    image_space_shader_volumetric_fog_destructor(e, this);
    if flags & 1 != 0 {
        e.call(NI_SIZED_FREE_00AA1460, &args![this, 0xd0u32]);
    }
    this
}

// Translated from 00b8d430 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// `00b8d580` (slot 0 of the vtable `010b04f4`): `SUB ECX,0x6c`, then a jump to it with the same
/// stack word (the flags). Answers what it answers.
pub fn fn_00b8d430(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8d580(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8bda0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes `8a 41 04 c3`), `thiscall`, no stack words:
/// answers the byte at +4 (`MOV AL,[ECX+4]`). It is slot +0x18 of the `ImageSpaceEffect`
/// vtables of the image-space shaders (`010affb8`, `010b018c`, ...), the slot of
/// `ImageSpaceEffect::IsActive` in the Xbox PDB, whose `bIsActive` is +4 of the effect.
pub fn fn_00b8bda0(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 4)
}

// Translated from 00b8bd50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8bce0` constructs (vtables `010affc0` and
/// `010affa0`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8bd50(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010a_ffc0, 0x010a_ffa0, flags)
}

// Translated from 00b8bfa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8bea0` constructs (vtables `010b0198` and
/// `010b0174`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8bfa0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_0198, 0x010b_0174, flags)
}

// Translated from 00b8d6c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8d5b0` constructs (vtables `010b06c0` and
/// `010b06a0`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8d6c0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_06c0, 0x010b_06a0, flags)
}

// Translated from 00b8d810 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8d700` constructs (vtables `010b0860` and
/// `010b083c`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8d810(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_0860, 0x010b_083c, flags)
}

// Translated from 00b8d950 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8d850` constructs (vtables `010b0a00` and
/// `010b09dc`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8d950(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_0a00, 0x010b_09dc, flags)
}

// Translated from 00b8daa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8d990` constructs (vtables `010b0b98` and
/// `010b0b78`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8daa0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_0b98, 0x010b_0b78, flags)
}

// Translated from 00b8dbf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8dae0` constructs (vtables `010b0d40` and
/// `010b0d1c`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8dbf0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_0d40, 0x010b_0d1c, flags)
}

// Translated from 00b8dd40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8dc30` constructs (vtables `010b0ec0` and
/// `010b0e9c`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8dd40(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_0ec0, 0x010b_0e9c, flags)
}

// Translated from 00b8de90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8dd80` constructs (vtables `010b1040` and
/// `010b101c`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8de90(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_1040, 0x010b_101c, flags)
}

// Translated from 00b8dfe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8ded0` constructs (vtables `010b11c0` and
/// `010b119c`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8dfe0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_11c0, 0x010b_119c, flags)
}

// Translated from 00b8e130 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8e020` constructs (vtables `010b1340` and
/// `010b1320`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8e130(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_1340, 0x010b_1320, flags)
}

// Translated from 00b8e280 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8e170` constructs (vtables `010b14c0` and
/// `010b14a0`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8e280(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_14c0, 0x010b_14a0, flags)
}

// Translated from 00b8e420 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8e2c0` constructs (vtables `010b1640` and
/// `010b1620`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8e420(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_1640, 0x010b_1620, flags)
}

// Translated from 00b8e5c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8e460` constructs (vtables `010b17f8` and
/// `010b17d8`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8e5c0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_17f8, 0x010b_17d8, flags)
}

// Translated from 00b8e760 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8e600` constructs (vtables `010b1980` and
/// `010b1960`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8e760(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_1980, 0x010b_1960, flags)
}

// Translated from 00b8e900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8e7a0` constructs (vtables `010b1b08` and
/// `010b1ae8`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8e900(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_1b08, 0x010b_1ae8, flags)
}

// Translated from 00b8eaa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8e940` constructs (vtables `010b1c90` and
/// `010b1c70`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8eaa0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_1c90, 0x010b_1c70, flags)
}

// Translated from 00b8ec40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8eae0` constructs (vtables `010b1e20` and
/// `010b1dfc`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8ec40(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_1e20, 0x010b_1dfc, flags)
}

// Translated from 00b8ede0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8ec80` constructs (vtables `010b1fb0` and
/// `010b1f8c`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8ede0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_1fb0, 0x010b_1f8c, flags)
}

// Translated from 00b8ef60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8ee20` constructs (vtables `010b2140` and
/// `010b211c`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8ef60(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_2140, 0x010b_211c, flags)
}

// Translated from 00b8f120 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8f010` constructs (vtables `010b2308` and
/// `010b22e8`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8f120(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_2308, 0x010b_22e8, flags)
}

// Translated from 00b8f2b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8f1b0` constructs (vtables `010b2490` and
/// `010b2470`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8f2b0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_2490, 0x010b_2470, flags)
}

// Translated from 00b8f470 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8f360` constructs (vtables `010b2610` and
/// `010b25f0`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8f470(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_2610, 0x010b_25f0, flags)
}

// Translated from 00b8f620 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8f520` constructs (vtables `010b27a8` and
/// `010b2784`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8f620(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_27a8, 0x010b_2784, flags)
}

// Translated from 00b8f6d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8f660` constructs (vtables `010b2920` and
/// `010b28fc`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8f6d0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_2920, 0x010b_28fc, flags)
}

// Translated from 00b8f7a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8efa0` constructs (vtables `010b2a98` and
/// `010b2a74`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8f7a0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_2a98, 0x010b_2a74, flags)
}

// Translated from 00b8f8f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8f7e0` constructs (vtables `010b2c10` and
/// `010b2bec`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8f8f0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_2c10, 0x010b_2bec, flags)
}

// Translated from 00b8fa40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8f930` constructs (vtables `010b2db0` and
/// `010b2d8c`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8fa40(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_2db0, 0x010b_2d8c, flags)
}

// Translated from 00b8fba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8fa80` constructs (vtables `010b2f50` and
/// `010b2f30`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8fba0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_2f50, 0x010b_2f30, flags)
}

// Translated from 00b8fd00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8fbe0` constructs (vtables `010b30e0` and
/// `010b30c0`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8fd00(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_30e0, 0x010b_30c0, flags)
}

// Translated from 00b8fe60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8fd40` constructs (vtables `010b3268` and
/// `010b3244`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8fe60(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_3268, 0x010b_3244, flags)
}

// Translated from 00b8ffb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8fea0` constructs (vtables `010b33f0` and
/// `010b33cc`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b8ffb0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_33f0, 0x010b_33cc, flags)
}

// Translated from 00b90150 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b8fff0` constructs (vtables `010b3598` and
/// `010b3574`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b90150(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_3598, 0x010b_3574, flags)
}

// Translated from 00b902f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b90190` constructs (vtables `010b37a8` and
/// `010b3788`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b902f0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_37a8, 0x010b_3788, flags)
}

// Translated from 00b904e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b90330` constructs (vtables `010b3958` and
/// `010b3934`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b904e0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_3958, 0x010b_3934, flags)
}

// Translated from 00b90670 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b90520` constructs (vtables `010b3af0` and
/// `010b3ad0`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b90670(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_3af0, 0x010b_3ad0, flags)
}

// Translated from 00b907f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b906b0` constructs (vtables `010b3c78` and
/// `010b3c58`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b907f0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_3c78, 0x010b_3c58, flags)
}

// Translated from 00b90980 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b90830` constructs (vtables `010b3e18` and
/// `010b3df4`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b90980(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_3e18, 0x010b_3df4, flags)
}

// Translated from 00b90b10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b909c0` constructs (vtables `010b3fa8` and
/// `010b3f88`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b90b10(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_3fa8, 0x010b_3f88, flags)
}

// Translated from 00b90ca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b90b50` constructs (vtables `010b4138` and
/// `010b4114`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b90ca0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_4138, 0x010b_4114, flags)
}

// Translated from 00b90e30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b90ce0` constructs (vtables `010b42c8` and
/// `010b42a4`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b90e30(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_42c8, 0x010b_42a4, flags)
}

// Translated from 00b90fc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b90e70` constructs (vtables `010b4460` and
/// `010b443c`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b90fc0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_4460, 0x010b_443c, flags)
}

// Translated from 00b91160 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b91000` constructs (vtables `010b4600` and
/// `010b45dc`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b91160(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_4600, 0x010b_45dc, flags)
}

// Translated from 00b91300 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b911a0` constructs (vtables `010b47a0` and
/// `010b477c`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b91300(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_47a0, 0x010b_477c, flags)
}

// Translated from 00b914a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b91340` constructs (vtables `010b4938` and
/// `010b4914`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b914a0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_4938, 0x010b_4914, flags)
}

// Translated from 00b91640 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b914e0` constructs (vtables `010b4ad0` and
/// `010b4aac`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b91640(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_4ad0, 0x010b_4aac, flags)
}

// Translated from 00b91780 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b91680` constructs (vtables `010b4c68` and
/// `010b4c48`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b91780(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_4c68, 0x010b_4c48, flags)
}

// Translated from 00b918c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b917c0` constructs (vtables `010b4de8` and
/// `010b4dc8`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b918c0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_4de8, 0x010b_4dc8, flags)
}

// Translated from 00b91a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b91900` constructs (vtables `010b4f80` and
/// `010b4f60`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b91a00(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_4f80, 0x010b_4f60, flags)
}

// Translated from 00b91cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b91b80` constructs (vtables `010b52c0` and
/// `010b52a0`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b91cb0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_52c0, 0x010b_52a0, flags)
}

// Translated from 00b91e00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b91cf0` constructs (vtables `010b5480` and
/// `010b5460`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b91e00(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_5480, 0x010b_5460, flags)
}

// Translated from 00b91f50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b91e40` constructs (vtables `010b5640` and
/// `010b5620`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b91f50(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_5640, 0x010b_5620, flags)
}

// Translated from 00b92290 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b92180` constructs (vtables `010b57d0` and
/// `010b57b0`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b92290(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_57d0, 0x010b_57b0, flags)
}

// Translated from 00b92570 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b92500` constructs (vtables `010b5b10` and
/// `010b5af0`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b92570(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_5b10, 0x010b_5af0, flags)
}

// Translated from 00b927a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b92690` constructs (vtables `010b5cb8` and
/// `010b5c94`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b927a0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_5cb8, 0x010b_5c94, flags)
}

// Translated from 00b928f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b927e0` constructs (vtables `010b5ea8` and
/// `010b5e84`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b928f0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_5ea8, 0x010b_5e84, flags)
}

// Translated from 00b92a40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b92930` constructs (vtables `010b6048` and
/// `010b6028`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b92a40(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_6048, 0x010b_6028, flags)
}

// Translated from 00b92b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b92a80` constructs (vtables `010b61f0` and
/// `010b61d0`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b92b90(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_61f0, 0x010b_61d0, flags)
}

// Translated from 00b92ce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b92bd0` constructs (vtables `010b6398` and
/// `010b6374`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b92ce0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_6398, 0x010b_6374, flags)
}

// Translated from 00b92ea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b92d90` constructs (vtables `010b6548` and
/// `010b6524`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b92ea0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_6548, 0x010b_6524, flags)
}

// Translated from 00b92ff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b92ee0` constructs (vtables `010b66f0` and
/// `010b66cc`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b92ff0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_66f0, 0x010b_66cc, flags)
}

// Translated from 00b93140 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b93030` constructs (vtables `010b68a0` and
/// `010b687c`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b93140(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_68a0, 0x010b_687c, flags)
}

// Translated from 00b93290 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b93180` constructs (vtables `010b6a40` and
/// `010b6a20`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b93290(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_6a40, 0x010b_6a20, flags)
}

// Translated from 00b933e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b932d0` constructs (vtables `010b6bd8` and
/// `010b6bb8`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b933e0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_6bd8, 0x010b_6bb8, flags)
}

// Translated from 00b93710 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b93600` constructs (vtables `010b6d90` and
/// `010b6d70`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b93710(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_6d90, 0x010b_6d70, flags)
}

// Translated from 00b93a60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b93950` constructs (vtables `010b6f38` and
/// `010b6f14`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b93a60(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_6f38, 0x010b_6f14, flags)
}

// Translated from 00b93bb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b93aa0` constructs (vtables `010b70d8` and
/// `010b70b8`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b93bb0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_70d8, 0x010b_70b8, flags)
}

// Translated from 00b93d00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b93bf0` constructs (vtables `010b7278` and
/// `010b7258`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b93d00(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_7278, 0x010b_7258, flags)
}

// Translated from 00b93e50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b93d40` constructs (vtables `010b7418` and
/// `010b73f4`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b93e50(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_7418, 0x010b_73f4, flags)
}

// Translated from 00b93fa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b93e90` constructs (vtables `010b7598` and
/// `010b7578`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b93fa0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_7598, 0x010b_7578, flags)
}

// Translated from 00b940f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b93fe0` constructs (vtables `010b7720` and
/// `010b7700`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b940f0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_7720, 0x010b_7700, flags)
}

// Translated from 00b94240 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b94130` constructs (vtables `010b78b8` and
/// `010b7898`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b94240(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_78b8, 0x010b_7898, flags)
}

// Translated from 00b94390 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b94280` constructs (vtables `010b7a50` and
/// `010b7a2c`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b94390(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_7a50, 0x010b_7a2c, flags)
}

// Translated from 00b944e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (unit `imagespacemanager.cpp`), `thiscall`, one stack word (the
/// flags): the deleting destructor of the shader `00b943d0` constructs (vtables `010b7bf0` and
/// `010b7bcc`), see `delete_image_space_shader`. Answers `this`.
pub fn fn_00b944e0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    delete_image_space_shader(e, this, 0x010b_7bf0, 0x010b_7bcc, flags)
}

// Translated from 00b8bd80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8bd50` (slot 0 of the vtable `010affa0`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8bd80(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8bd50(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8bfd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8bfa0` (slot 0 of the vtable `010b0174`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8bfd0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8bfa0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8d6f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8d6c0` (slot 0 of the vtable `010b06a0`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8d6f0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8d6c0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8d840 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8d810` (slot 0 of the vtable `010b083c`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8d840(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8d810(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8d980 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8d950` (slot 0 of the vtable `010b09dc`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8d980(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8d950(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8dad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8daa0` (slot 0 of the vtable `010b0b78`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8dad0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8daa0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8dc20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8dbf0` (slot 0 of the vtable `010b0d1c`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8dc20(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8dbf0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8dd70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8dd40` (slot 0 of the vtable `010b0e9c`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8dd70(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8dd40(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8dec0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8de90` (slot 0 of the vtable `010b101c`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8dec0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8de90(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8e010 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8dfe0` (slot 0 of the vtable `010b119c`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8e010(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8dfe0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8e160 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8e130` (slot 0 of the vtable `010b1320`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8e160(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8e130(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8e2b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8e280` (slot 0 of the vtable `010b14a0`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8e2b0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8e280(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8e450 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8e420` (slot 0 of the vtable `010b1620`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8e450(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8e420(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8e5f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8e5c0` (slot 0 of the vtable `010b17d8`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8e5f0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8e5c0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8e790 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8e760` (slot 0 of the vtable `010b1960`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8e790(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8e760(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8e930 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8e900` (slot 0 of the vtable `010b1ae8`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8e930(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8e900(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8ead0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8eaa0` (slot 0 of the vtable `010b1c70`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8ead0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8eaa0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8ec70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8ec40` (slot 0 of the vtable `010b1dfc`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8ec70(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8ec40(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8ee10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8ede0` (slot 0 of the vtable `010b1f8c`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8ee10(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8ede0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8ef90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8ef60` (slot 0 of the vtable `010b211c`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8ef90(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8ef60(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8f150 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8f120` (slot 0 of the vtable `010b22e8`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8f150(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8f120(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8f2e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8f2b0` (slot 0 of the vtable `010b2470`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8f2e0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8f2b0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8f4a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8f470` (slot 0 of the vtable `010b25f0`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8f4a0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8f470(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8f650 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8f620` (slot 0 of the vtable `010b2784`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8f650(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8f620(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8f700 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8f6d0` (slot 0 of the vtable `010b28fc`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8f700(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8f6d0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8f7d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8f7a0` (slot 0 of the vtable `010b2a74`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8f7d0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8f7a0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8f920 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8f8f0` (slot 0 of the vtable `010b2bec`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8f920(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8f8f0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8fa70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8fa40` (slot 0 of the vtable `010b2d8c`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8fa70(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8fa40(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8fbd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8fba0` (slot 0 of the vtable `010b2f30`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8fbd0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8fba0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8fd30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8fd00` (slot 0 of the vtable `010b30c0`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8fd30(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8fd00(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8fe90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8fe60` (slot 0 of the vtable `010b3244`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8fe90(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8fe60(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b8ffe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b8ffb0` (slot 0 of the vtable `010b33cc`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b8ffe0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b8ffb0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b90180 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b90150` (slot 0 of the vtable `010b3574`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b90180(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b90150(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b90320 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b902f0` (slot 0 of the vtable `010b3788`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b90320(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b902f0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b90510 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b904e0` (slot 0 of the vtable `010b3934`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b90510(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b904e0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b906a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b90670` (slot 0 of the vtable `010b3ad0`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b906a0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b90670(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b90820 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b907f0` (slot 0 of the vtable `010b3c58`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b90820(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b907f0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b909b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b90980` (slot 0 of the vtable `010b3df4`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b909b0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b90980(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b90b40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b90b10` (slot 0 of the vtable `010b3f88`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b90b40(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b90b10(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b90cd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b90ca0` (slot 0 of the vtable `010b4114`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b90cd0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b90ca0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b90e60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b90e30` (slot 0 of the vtable `010b42a4`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b90e60(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b90e30(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b90ff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b90fc0` (slot 0 of the vtable `010b443c`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b90ff0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b90fc0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b91190 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b91160` (slot 0 of the vtable `010b45dc`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b91190(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b91160(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b91330 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b91300` (slot 0 of the vtable `010b477c`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b91330(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b91300(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b914d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b914a0` (slot 0 of the vtable `010b4914`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b914d0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b914a0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b91670 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b91640` (slot 0 of the vtable `010b4aac`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b91670(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b91640(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b917b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b91780` (slot 0 of the vtable `010b4c48`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b917b0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b91780(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b918f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b918c0` (slot 0 of the vtable `010b4dc8`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b918f0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b918c0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b91a30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b91a00` (slot 0 of the vtable `010b4f60`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b91a30(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b91a00(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b91ce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b91cb0` (slot 0 of the vtable `010b52a0`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b91ce0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b91cb0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b91e30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b91e00` (slot 0 of the vtable `010b5460`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b91e30(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b91e00(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b91f80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b91f50` (slot 0 of the vtable `010b5620`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b91f80(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b91f50(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b922c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b92290` (slot 0 of the vtable `010b57b0`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b922c0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b92290(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b925a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b92570` (slot 0 of the vtable `010b5af0`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b925a0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b92570(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b927d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b927a0` (slot 0 of the vtable `010b5c94`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b927d0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b927a0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b92920 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b928f0` (slot 0 of the vtable `010b5e84`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b92920(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b928f0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b92a70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b92a40` (slot 0 of the vtable `010b6028`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b92a70(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b92a40(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b92bc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b92b90` (slot 0 of the vtable `010b61d0`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b92bc0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b92b90(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b92d10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b92ce0` (slot 0 of the vtable `010b6374`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b92d10(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b92ce0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b92ed0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b92ea0` (slot 0 of the vtable `010b6524`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b92ed0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b92ea0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b93020 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b92ff0` (slot 0 of the vtable `010b66cc`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b93020(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b92ff0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b93170 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b93140` (slot 0 of the vtable `010b687c`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b93170(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b93140(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b932c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b93290` (slot 0 of the vtable `010b6a20`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b932c0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b93290(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b93410 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b933e0` (slot 0 of the vtable `010b6bb8`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b93410(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b933e0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b93740 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b93710` (slot 0 of the vtable `010b6d70`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b93740(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b93710(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b93a90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b93a60` (slot 0 of the vtable `010b6f14`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b93a90(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b93a60(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b93be0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b93bb0` (slot 0 of the vtable `010b70b8`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b93be0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b93bb0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b93d30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b93d00` (slot 0 of the vtable `010b7258`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b93d30(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b93d00(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b93e80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b93e50` (slot 0 of the vtable `010b73f4`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b93e80(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b93e50(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b93fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b93fa0` (slot 0 of the vtable `010b7578`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b93fd0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b93fa0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b94120 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b940f0` (slot 0 of the vtable `010b7700`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b94120(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b940f0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b94270 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b94240` (slot 0 of the vtable `010b7898`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b94270(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b94240(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b943c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b94390` (slot 0 of the vtable `010b7a2c`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b943c0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b94390(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

// Translated from 00b94510 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (no Ghidra function; read from its bytes): the `ImageSpaceEffect` (+0x6c) entry of
/// the deleting destructor `00b944e0` (slot 0 of the vtable `010b7bcc`): `SUB ECX,0x6c`, then a
/// jump to it with the same stack word (the flags). Answers what it answers.
pub fn fn_00b94510(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00b944e0(
        e,
        Ptr::new(this.addr().wrapping_sub(SHADER_EFFECT_BASE)),
        flags,
    )
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00b9_4520, fn_00b94520(Ptr)),
        entry!(0x00b8_c170, image_space_manager_init_geometry(Ptr)),
        entry!(0x00b8_ad70, fn_00b8ad70()),
        entry!(0x00b8_b360, image_space_manager_setup_effects(Ptr)),
        entry!(0x00b9_77d0, image_space_manager_initialize(Ptr)),
        entry!(0x00b9_22d0, fn_00b922d0(Ptr) -> Ptr),
        entry!(0x00b8_bce0, fn_00b8bce0(Ptr) -> Ptr),
        entry!(0x00b8_bea0, fn_00b8bea0(Ptr) -> Ptr),
        entry!(0x00b8_c070, fn_00b8c070(Ptr) -> Ptr),
        entry!(0x00b8_d330, fn_00b8d330(Ptr) -> Ptr),
        entry!(0x00b8_d5b0, fn_00b8d5b0(Ptr) -> Ptr),
        entry!(0x00b8_d700, fn_00b8d700(Ptr) -> Ptr),
        entry!(0x00b8_d850, fn_00b8d850(Ptr) -> Ptr),
        entry!(0x00b8_d990, fn_00b8d990(Ptr) -> Ptr),
        entry!(0x00b8_dae0, fn_00b8dae0(Ptr) -> Ptr),
        entry!(0x00b8_dc30, fn_00b8dc30(Ptr) -> Ptr),
        entry!(0x00b8_dd80, fn_00b8dd80(Ptr) -> Ptr),
        entry!(0x00b8_ded0, fn_00b8ded0(Ptr) -> Ptr),
        entry!(0x00b8_e020, fn_00b8e020(Ptr) -> Ptr),
        entry!(0x00b8_e170, fn_00b8e170(Ptr) -> Ptr),
        entry!(0x00b8_e2c0, fn_00b8e2c0(Ptr) -> Ptr),
        entry!(0x00b8_e460, fn_00b8e460(Ptr) -> Ptr),
        entry!(0x00b8_e600, fn_00b8e600(Ptr) -> Ptr),
        entry!(0x00b8_e7a0, fn_00b8e7a0(Ptr) -> Ptr),
        entry!(0x00b8_e940, fn_00b8e940(Ptr) -> Ptr),
        entry!(0x00b8_eae0, fn_00b8eae0(Ptr) -> Ptr),
        entry!(0x00b8_ec80, fn_00b8ec80(Ptr) -> Ptr),
        entry!(0x00b8_ee20, fn_00b8ee20(Ptr) -> Ptr),
        entry!(0x00b8_efa0, fn_00b8efa0(Ptr) -> Ptr),
        entry!(0x00b8_f010, fn_00b8f010(Ptr) -> Ptr),
        entry!(0x00b8_f1b0, fn_00b8f1b0(Ptr) -> Ptr),
        entry!(0x00b8_f360, fn_00b8f360(Ptr) -> Ptr),
        entry!(0x00b8_f520, fn_00b8f520(Ptr) -> Ptr),
        entry!(0x00b8_f660, fn_00b8f660(Ptr) -> Ptr),
        entry!(0x00b8_f7e0, fn_00b8f7e0(Ptr) -> Ptr),
        entry!(0x00b8_f930, fn_00b8f930(Ptr) -> Ptr),
        entry!(0x00b8_fa80, fn_00b8fa80(Ptr) -> Ptr),
        entry!(0x00b8_fbe0, fn_00b8fbe0(Ptr) -> Ptr),
        entry!(0x00b8_fd40, fn_00b8fd40(Ptr) -> Ptr),
        entry!(0x00b8_fea0, fn_00b8fea0(Ptr) -> Ptr),
        entry!(0x00b8_fff0, fn_00b8fff0(Ptr) -> Ptr),
        entry!(0x00b9_0190, fn_00b90190(Ptr) -> Ptr),
        entry!(0x00b9_0330, fn_00b90330(Ptr) -> Ptr),
        entry!(0x00b9_0520, fn_00b90520(Ptr) -> Ptr),
        entry!(0x00b9_06b0, fn_00b906b0(Ptr) -> Ptr),
        entry!(0x00b9_0830, fn_00b90830(Ptr) -> Ptr),
        entry!(0x00b9_09c0, fn_00b909c0(Ptr) -> Ptr),
        entry!(0x00b9_0b50, fn_00b90b50(Ptr) -> Ptr),
        entry!(0x00b9_0ce0, fn_00b90ce0(Ptr) -> Ptr),
        entry!(0x00b9_0e70, fn_00b90e70(Ptr) -> Ptr),
        entry!(0x00b9_1000, fn_00b91000(Ptr) -> Ptr),
        entry!(0x00b9_11a0, fn_00b911a0(Ptr) -> Ptr),
        entry!(0x00b9_1340, fn_00b91340(Ptr) -> Ptr),
        entry!(0x00b9_14e0, fn_00b914e0(Ptr) -> Ptr),
        entry!(0x00b9_1680, fn_00b91680(Ptr) -> Ptr),
        entry!(0x00b9_17c0, fn_00b917c0(Ptr) -> Ptr),
        entry!(0x00b9_1900, fn_00b91900(Ptr) -> Ptr),
        entry!(0x00b9_1a40, fn_00b91a40(Ptr) -> Ptr),
        entry!(0x00b9_1b80, fn_00b91b80(Ptr) -> Ptr),
        entry!(0x00b9_1cf0, fn_00b91cf0(Ptr) -> Ptr),
        entry!(0x00b9_1e40, fn_00b91e40(Ptr) -> Ptr),
        entry!(0x00b9_2180, fn_00b92180(Ptr) -> Ptr),
        entry!(0x00b9_2500, fn_00b92500(Ptr) -> Ptr),
        entry!(0x00b9_2690, fn_00b92690(Ptr) -> Ptr),
        entry!(0x00b9_27e0, fn_00b927e0(Ptr) -> Ptr),
        entry!(0x00b9_2930, fn_00b92930(Ptr) -> Ptr),
        entry!(0x00b9_2a80, fn_00b92a80(Ptr) -> Ptr),
        entry!(0x00b9_2bd0, fn_00b92bd0(Ptr) -> Ptr),
        entry!(0x00b9_2d90, fn_00b92d90(Ptr) -> Ptr),
        entry!(0x00b9_2ee0, fn_00b92ee0(Ptr) -> Ptr),
        entry!(0x00b9_3030, fn_00b93030(Ptr) -> Ptr),
        entry!(0x00b9_3180, fn_00b93180(Ptr) -> Ptr),
        entry!(0x00b9_32d0, fn_00b932d0(Ptr) -> Ptr),
        entry!(0x00b9_3600, fn_00b93600(Ptr) -> Ptr),
        entry!(0x00b9_3950, fn_00b93950(Ptr) -> Ptr),
        entry!(0x00b9_3aa0, fn_00b93aa0(Ptr) -> Ptr),
        entry!(0x00b9_3bf0, fn_00b93bf0(Ptr) -> Ptr),
        entry!(0x00b9_3d40, fn_00b93d40(Ptr) -> Ptr),
        entry!(0x00b9_3e90, fn_00b93e90(Ptr) -> Ptr),
        entry!(0x00b9_3fe0, fn_00b93fe0(Ptr) -> Ptr),
        entry!(0x00b9_4130, fn_00b94130(Ptr) -> Ptr),
        entry!(0x00b9_4280, fn_00b94280(Ptr) -> Ptr),
        entry!(0x00b9_43d0, fn_00b943d0(Ptr) -> Ptr),
        entry!(0x00b8_bc20, fn_00b8bc20(Ptr)),
        entry!(0x00b8_b270, fn_00b8b270(Ptr, u32, Ptr)),
        entry!(0x00b9_1b50, fn_00b91b50(Ptr, u32) -> Ptr),
        entry!(0x00b9_1b40, fn_00b91b40(Ptr, u32) -> Ptr),
        entry!(0x00b9_2450, fn_00b92450(Ptr)),
        entry!(0x00b9_24d0, fn_00b924d0(Ptr, u32) -> Ptr),
        entry!(0x00b9_2440, fn_00b92440(Ptr, u32) -> Ptr),
        entry!(
            0x00b8_d4d0,
            image_space_shader_volumetric_fog_destructor(Ptr)
        ),
        entry!(0x00b8_d580, fn_00b8d580(Ptr, u32) -> Ptr),
        entry!(0x00b8_d430, fn_00b8d430(Ptr, u32) -> Ptr),
        entry!(0x00b8_bda0, fn_00b8bda0(Ptr) -> u8),
        entry!(0x00b8_bd50, fn_00b8bd50(Ptr, u32) -> Ptr),
        entry!(0x00b8_bfa0, fn_00b8bfa0(Ptr, u32) -> Ptr),
        entry!(0x00b8_d6c0, fn_00b8d6c0(Ptr, u32) -> Ptr),
        entry!(0x00b8_d810, fn_00b8d810(Ptr, u32) -> Ptr),
        entry!(0x00b8_d950, fn_00b8d950(Ptr, u32) -> Ptr),
        entry!(0x00b8_daa0, fn_00b8daa0(Ptr, u32) -> Ptr),
        entry!(0x00b8_dbf0, fn_00b8dbf0(Ptr, u32) -> Ptr),
        entry!(0x00b8_dd40, fn_00b8dd40(Ptr, u32) -> Ptr),
        entry!(0x00b8_de90, fn_00b8de90(Ptr, u32) -> Ptr),
        entry!(0x00b8_dfe0, fn_00b8dfe0(Ptr, u32) -> Ptr),
        entry!(0x00b8_e130, fn_00b8e130(Ptr, u32) -> Ptr),
        entry!(0x00b8_e280, fn_00b8e280(Ptr, u32) -> Ptr),
        entry!(0x00b8_e420, fn_00b8e420(Ptr, u32) -> Ptr),
        entry!(0x00b8_e5c0, fn_00b8e5c0(Ptr, u32) -> Ptr),
        entry!(0x00b8_e760, fn_00b8e760(Ptr, u32) -> Ptr),
        entry!(0x00b8_e900, fn_00b8e900(Ptr, u32) -> Ptr),
        entry!(0x00b8_eaa0, fn_00b8eaa0(Ptr, u32) -> Ptr),
        entry!(0x00b8_ec40, fn_00b8ec40(Ptr, u32) -> Ptr),
        entry!(0x00b8_ede0, fn_00b8ede0(Ptr, u32) -> Ptr),
        entry!(0x00b8_ef60, fn_00b8ef60(Ptr, u32) -> Ptr),
        entry!(0x00b8_f120, fn_00b8f120(Ptr, u32) -> Ptr),
        entry!(0x00b8_f2b0, fn_00b8f2b0(Ptr, u32) -> Ptr),
        entry!(0x00b8_f470, fn_00b8f470(Ptr, u32) -> Ptr),
        entry!(0x00b8_f620, fn_00b8f620(Ptr, u32) -> Ptr),
        entry!(0x00b8_f6d0, fn_00b8f6d0(Ptr, u32) -> Ptr),
        entry!(0x00b8_f7a0, fn_00b8f7a0(Ptr, u32) -> Ptr),
        entry!(0x00b8_f8f0, fn_00b8f8f0(Ptr, u32) -> Ptr),
        entry!(0x00b8_fa40, fn_00b8fa40(Ptr, u32) -> Ptr),
        entry!(0x00b8_fba0, fn_00b8fba0(Ptr, u32) -> Ptr),
        entry!(0x00b8_fd00, fn_00b8fd00(Ptr, u32) -> Ptr),
        entry!(0x00b8_fe60, fn_00b8fe60(Ptr, u32) -> Ptr),
        entry!(0x00b8_ffb0, fn_00b8ffb0(Ptr, u32) -> Ptr),
        entry!(0x00b9_0150, fn_00b90150(Ptr, u32) -> Ptr),
        entry!(0x00b9_02f0, fn_00b902f0(Ptr, u32) -> Ptr),
        entry!(0x00b9_04e0, fn_00b904e0(Ptr, u32) -> Ptr),
        entry!(0x00b9_0670, fn_00b90670(Ptr, u32) -> Ptr),
        entry!(0x00b9_07f0, fn_00b907f0(Ptr, u32) -> Ptr),
        entry!(0x00b9_0980, fn_00b90980(Ptr, u32) -> Ptr),
        entry!(0x00b9_0b10, fn_00b90b10(Ptr, u32) -> Ptr),
        entry!(0x00b9_0ca0, fn_00b90ca0(Ptr, u32) -> Ptr),
        entry!(0x00b9_0e30, fn_00b90e30(Ptr, u32) -> Ptr),
        entry!(0x00b9_0fc0, fn_00b90fc0(Ptr, u32) -> Ptr),
        entry!(0x00b9_1160, fn_00b91160(Ptr, u32) -> Ptr),
        entry!(0x00b9_1300, fn_00b91300(Ptr, u32) -> Ptr),
        entry!(0x00b9_14a0, fn_00b914a0(Ptr, u32) -> Ptr),
        entry!(0x00b9_1640, fn_00b91640(Ptr, u32) -> Ptr),
        entry!(0x00b9_1780, fn_00b91780(Ptr, u32) -> Ptr),
        entry!(0x00b9_18c0, fn_00b918c0(Ptr, u32) -> Ptr),
        entry!(0x00b9_1a00, fn_00b91a00(Ptr, u32) -> Ptr),
        entry!(0x00b9_1cb0, fn_00b91cb0(Ptr, u32) -> Ptr),
        entry!(0x00b9_1e00, fn_00b91e00(Ptr, u32) -> Ptr),
        entry!(0x00b9_1f50, fn_00b91f50(Ptr, u32) -> Ptr),
        entry!(0x00b9_2290, fn_00b92290(Ptr, u32) -> Ptr),
        entry!(0x00b9_2570, fn_00b92570(Ptr, u32) -> Ptr),
        entry!(0x00b9_27a0, fn_00b927a0(Ptr, u32) -> Ptr),
        entry!(0x00b9_28f0, fn_00b928f0(Ptr, u32) -> Ptr),
        entry!(0x00b9_2a40, fn_00b92a40(Ptr, u32) -> Ptr),
        entry!(0x00b9_2b90, fn_00b92b90(Ptr, u32) -> Ptr),
        entry!(0x00b9_2ce0, fn_00b92ce0(Ptr, u32) -> Ptr),
        entry!(0x00b9_2ea0, fn_00b92ea0(Ptr, u32) -> Ptr),
        entry!(0x00b9_2ff0, fn_00b92ff0(Ptr, u32) -> Ptr),
        entry!(0x00b9_3140, fn_00b93140(Ptr, u32) -> Ptr),
        entry!(0x00b9_3290, fn_00b93290(Ptr, u32) -> Ptr),
        entry!(0x00b9_33e0, fn_00b933e0(Ptr, u32) -> Ptr),
        entry!(0x00b9_3710, fn_00b93710(Ptr, u32) -> Ptr),
        entry!(0x00b9_3a60, fn_00b93a60(Ptr, u32) -> Ptr),
        entry!(0x00b9_3bb0, fn_00b93bb0(Ptr, u32) -> Ptr),
        entry!(0x00b9_3d00, fn_00b93d00(Ptr, u32) -> Ptr),
        entry!(0x00b9_3e50, fn_00b93e50(Ptr, u32) -> Ptr),
        entry!(0x00b9_3fa0, fn_00b93fa0(Ptr, u32) -> Ptr),
        entry!(0x00b9_40f0, fn_00b940f0(Ptr, u32) -> Ptr),
        entry!(0x00b9_4240, fn_00b94240(Ptr, u32) -> Ptr),
        entry!(0x00b9_4390, fn_00b94390(Ptr, u32) -> Ptr),
        entry!(0x00b9_44e0, fn_00b944e0(Ptr, u32) -> Ptr),
        entry!(0x00b8_bd80, fn_00b8bd80(Ptr, u32) -> Ptr),
        entry!(0x00b8_bfd0, fn_00b8bfd0(Ptr, u32) -> Ptr),
        entry!(0x00b8_d6f0, fn_00b8d6f0(Ptr, u32) -> Ptr),
        entry!(0x00b8_d840, fn_00b8d840(Ptr, u32) -> Ptr),
        entry!(0x00b8_d980, fn_00b8d980(Ptr, u32) -> Ptr),
        entry!(0x00b8_dad0, fn_00b8dad0(Ptr, u32) -> Ptr),
        entry!(0x00b8_dc20, fn_00b8dc20(Ptr, u32) -> Ptr),
        entry!(0x00b8_dd70, fn_00b8dd70(Ptr, u32) -> Ptr),
        entry!(0x00b8_dec0, fn_00b8dec0(Ptr, u32) -> Ptr),
        entry!(0x00b8_e010, fn_00b8e010(Ptr, u32) -> Ptr),
        entry!(0x00b8_e160, fn_00b8e160(Ptr, u32) -> Ptr),
        entry!(0x00b8_e2b0, fn_00b8e2b0(Ptr, u32) -> Ptr),
        entry!(0x00b8_e450, fn_00b8e450(Ptr, u32) -> Ptr),
        entry!(0x00b8_e5f0, fn_00b8e5f0(Ptr, u32) -> Ptr),
        entry!(0x00b8_e790, fn_00b8e790(Ptr, u32) -> Ptr),
        entry!(0x00b8_e930, fn_00b8e930(Ptr, u32) -> Ptr),
        entry!(0x00b8_ead0, fn_00b8ead0(Ptr, u32) -> Ptr),
        entry!(0x00b8_ec70, fn_00b8ec70(Ptr, u32) -> Ptr),
        entry!(0x00b8_ee10, fn_00b8ee10(Ptr, u32) -> Ptr),
        entry!(0x00b8_ef90, fn_00b8ef90(Ptr, u32) -> Ptr),
        entry!(0x00b8_f150, fn_00b8f150(Ptr, u32) -> Ptr),
        entry!(0x00b8_f2e0, fn_00b8f2e0(Ptr, u32) -> Ptr),
        entry!(0x00b8_f4a0, fn_00b8f4a0(Ptr, u32) -> Ptr),
        entry!(0x00b8_f650, fn_00b8f650(Ptr, u32) -> Ptr),
        entry!(0x00b8_f700, fn_00b8f700(Ptr, u32) -> Ptr),
        entry!(0x00b8_f7d0, fn_00b8f7d0(Ptr, u32) -> Ptr),
        entry!(0x00b8_f920, fn_00b8f920(Ptr, u32) -> Ptr),
        entry!(0x00b8_fa70, fn_00b8fa70(Ptr, u32) -> Ptr),
        entry!(0x00b8_fbd0, fn_00b8fbd0(Ptr, u32) -> Ptr),
        entry!(0x00b8_fd30, fn_00b8fd30(Ptr, u32) -> Ptr),
        entry!(0x00b8_fe90, fn_00b8fe90(Ptr, u32) -> Ptr),
        entry!(0x00b8_ffe0, fn_00b8ffe0(Ptr, u32) -> Ptr),
        entry!(0x00b9_0180, fn_00b90180(Ptr, u32) -> Ptr),
        entry!(0x00b9_0320, fn_00b90320(Ptr, u32) -> Ptr),
        entry!(0x00b9_0510, fn_00b90510(Ptr, u32) -> Ptr),
        entry!(0x00b9_06a0, fn_00b906a0(Ptr, u32) -> Ptr),
        entry!(0x00b9_0820, fn_00b90820(Ptr, u32) -> Ptr),
        entry!(0x00b9_09b0, fn_00b909b0(Ptr, u32) -> Ptr),
        entry!(0x00b9_0b40, fn_00b90b40(Ptr, u32) -> Ptr),
        entry!(0x00b9_0cd0, fn_00b90cd0(Ptr, u32) -> Ptr),
        entry!(0x00b9_0e60, fn_00b90e60(Ptr, u32) -> Ptr),
        entry!(0x00b9_0ff0, fn_00b90ff0(Ptr, u32) -> Ptr),
        entry!(0x00b9_1190, fn_00b91190(Ptr, u32) -> Ptr),
        entry!(0x00b9_1330, fn_00b91330(Ptr, u32) -> Ptr),
        entry!(0x00b9_14d0, fn_00b914d0(Ptr, u32) -> Ptr),
        entry!(0x00b9_1670, fn_00b91670(Ptr, u32) -> Ptr),
        entry!(0x00b9_17b0, fn_00b917b0(Ptr, u32) -> Ptr),
        entry!(0x00b9_18f0, fn_00b918f0(Ptr, u32) -> Ptr),
        entry!(0x00b9_1a30, fn_00b91a30(Ptr, u32) -> Ptr),
        entry!(0x00b9_1ce0, fn_00b91ce0(Ptr, u32) -> Ptr),
        entry!(0x00b9_1e30, fn_00b91e30(Ptr, u32) -> Ptr),
        entry!(0x00b9_1f80, fn_00b91f80(Ptr, u32) -> Ptr),
        entry!(0x00b9_22c0, fn_00b922c0(Ptr, u32) -> Ptr),
        entry!(0x00b9_25a0, fn_00b925a0(Ptr, u32) -> Ptr),
        entry!(0x00b9_27d0, fn_00b927d0(Ptr, u32) -> Ptr),
        entry!(0x00b9_2920, fn_00b92920(Ptr, u32) -> Ptr),
        entry!(0x00b9_2a70, fn_00b92a70(Ptr, u32) -> Ptr),
        entry!(0x00b9_2bc0, fn_00b92bc0(Ptr, u32) -> Ptr),
        entry!(0x00b9_2d10, fn_00b92d10(Ptr, u32) -> Ptr),
        entry!(0x00b9_2ed0, fn_00b92ed0(Ptr, u32) -> Ptr),
        entry!(0x00b9_3020, fn_00b93020(Ptr, u32) -> Ptr),
        entry!(0x00b9_3170, fn_00b93170(Ptr, u32) -> Ptr),
        entry!(0x00b9_32c0, fn_00b932c0(Ptr, u32) -> Ptr),
        entry!(0x00b9_3410, fn_00b93410(Ptr, u32) -> Ptr),
        entry!(0x00b9_3740, fn_00b93740(Ptr, u32) -> Ptr),
        entry!(0x00b9_3a90, fn_00b93a90(Ptr, u32) -> Ptr),
        entry!(0x00b9_3be0, fn_00b93be0(Ptr, u32) -> Ptr),
        entry!(0x00b9_3d30, fn_00b93d30(Ptr, u32) -> Ptr),
        entry!(0x00b9_3e80, fn_00b93e80(Ptr, u32) -> Ptr),
        entry!(0x00b9_3fd0, fn_00b93fd0(Ptr, u32) -> Ptr),
        entry!(0x00b9_4120, fn_00b94120(Ptr, u32) -> Ptr),
        entry!(0x00b9_4270, fn_00b94270(Ptr, u32) -> Ptr),
        entry!(0x00b9_43c0, fn_00b943c0(Ptr, u32) -> Ptr),
        entry!(0x00b9_4510, fn_00b94510(Ptr, u32) -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|c| c.0 == addr)
            .map(|c| c.1.clone())
            .collect()
    }

    fn rig() -> Engine {
        let mut e = Engine::new();
        e.map(0x011f_4000, 0x6000);
        e.map(0x0101_7000, 0x1000);
        e.set_global(OBJECT_011F91A8, 0x0f00_1000u32);
        e.set_global(OBJECT_011F4748, 0x0f00_2000u32);
        // A signalling NaN: the FLD/FSTP copy quiets it.
        e.set_global(VALUE_01017870, 0x7f80_0001u32);
        for addr in [
            SET_UP_00B94520,
            INIT_GEOMETRY,
            SETUP_EFFECTS,
            INIT_CONSTANT_MAP,
            SET_UP_00B8AD70,
        ] {
            e.register(addr, |e, _| {
                // Each step runs with the image-space context.
                let tls = e.tls();
                assert_eq!(e.mem.u32(tls + TLS_CONTEXT), CONTEXT_IMAGE_SPACE);
                Ret::default()
            });
        }
        e.register(MAKE_00B6E110, |_, _| 0x0f00_3000u32.into_ret());
        e
    }

    #[test]
    fn image_space_manager_initialize_sets_up_once() {
        let mut e = rig();
        let tls = e.tls();
        e.mem.set_u32(tls + TLS_CONTEXT, 9);
        let this = e.mem.alloc(0x320);
        for offset in (0..0x320).step_by(4) {
            e.mem.set_u32(this + offset, 0xdead_beef);
        }
        e.mem.set_u8(this + INITIALIZED, 0);
        e.call_log = Some(vec![]);
        e.call(0x00b9_77d0, &args![this]);
        let order: Vec<u32> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|c| c.0)
            .skip(1)
            .collect();
        assert_eq!(
            order,
            [
                SET_UP_00B94520,
                INIT_GEOMETRY,
                SETUP_EFFECTS,
                INIT_CONSTANT_MAP,
                SET_UP_00B8AD70,
                MAKE_00B6E110
            ]
        );
        assert_eq!(calls(&e, SET_UP_00B94520), vec![vec![this]]);
        assert_eq!(
            calls(&e, INIT_CONSTANT_MAP),
            vec![vec![CONSTANT_MAP_01202410, 0, 0]]
        );
        assert_eq!(
            calls(&e, MAKE_00B6E110),
            vec![vec![0x0f00_1000, 0x0f00_2000, 3, 0, 0, 0]]
        );
        assert_eq!(e.mem.u8(this + INITIALIZED), 1);
        for offset in [0x2f8u32, 0x2fc, 0x300, 0x308, 0x314] {
            assert_eq!(e.mem.u32(this + offset), 0, "+{offset:#x}");
        }
        assert_eq!(e.mem.f32(this + 0x30c), 1.0);
        assert_eq!(e.mem.f32(this + 0x310), 1.0);
        assert_eq!(e.mem.u32(this + 0x304), 0x7fc0_0001);
        assert_eq!(e.mem.u32(this), 0);
        assert_eq!(e.mem.u32(this + 0x318), 0x0f00_3000);
        assert_eq!(e.mem.u32(tls + TLS_CONTEXT), 9);

        // A second run does nothing but set and restore the context.
        e.mem.set_u32(this + 0x318, 0x55);
        e.call_log = Some(vec![]);
        e.call(0x00b9_77d0, &args![this]);
        assert_eq!(e.call_log.as_ref().unwrap().len(), 1);
        assert_eq!(e.mem.u32(this + 0x318), 0x55);
        assert_eq!(e.mem.u32(tls + TLS_CONTEXT), 9);
    }
}

#[cfg(test)]
mod setup_effects_tests {
    use super::*;

    #[test]
    fn image_space_manager_setup_effects_calls_each_set_effect() {
        const SETUP: u32 = 0x0e40_0000;
        let mut e = Engine::new();
        e.register(SETUP, |_, _| Ret::default());
        e.put_vtable(0x0e41_0000, &[0, 0, SETUP]);
        let this = e.mem.alloc(0x10);
        let effects = e.mem.alloc(0x88);
        e.mem.set_u32(this + 8, effects);
        let mut set = vec![];
        for index in [0u32, 5, 0x21] {
            let effect = e.mem.alloc(4);
            e.mem.set_u32(effect, 0x0e41_0000);
            e.mem.set_u32(effects + 4 * index, effect);
            set.push(effect);
        }
        e.call_log = Some(vec![]);
        e.call(0x00b8_b360, &args![this]);
        let log = e.call_log.take().unwrap();
        let seen: Vec<_> = log
            .iter()
            .filter(|c| c.0 == SETUP)
            .map(|c| c.1.clone())
            .collect();
        assert_eq!(
            seen,
            set.iter().map(|&s| vec![s, this, 0]).collect::<Vec<_>>()
        );
    }
}

#[cfg(test)]
mod defaults_tests {
    use super::*;

    #[test]
    fn fn_00b8ad70_stores_every_default() {
        let mut e = Engine::new();
        e.map(0x0101_6000, 0x18000);
        e.map(0x011f_f000, 0x1000);
        e.mem.write(0x011f_f940, &[0xee; 0x94]);
        // A signalling NaN is quieted by the load.
        e.set_global::<u32>(HALF_01016248, 0x7f80_0005);
        let sources = [
            0x0101_8f98u32,
            0x0102_2154,
            0x0102_212c,
            0x0101_6088,
            0x0102_1c9c,
            0x0101_df44,
            0x0101_8238,
            0x0102_d934,
            0x0101_84f4,
            0x0102_d930,
            0x0102_d92c,
        ];
        for (index, source) in sources.iter().enumerate() {
            e.set_global::<f32>(*source, 10.0 + index as f32);
        }
        e.call(0x00b8_ad70, &args![]);
        let half = 0x7fc0_0005;
        let one = 0x3f80_0000;
        let value = |index: u32| (10.0 + index as f32).to_bits();
        let expected = [
            (0x940u32, half),
            (0x944, value(0)),
            (0x948, one),
            (0x94c, one),
            (0x950, one),
            (0x954, one),
            (0x958, value(1)),
            (0x95c, value(2)),
            (0x960, one),
            (0x964, 0),
            (0x968, 0),
            (0x96c, value(3)),
            (0x970, value(3)),
            (0x974, one),
            (0x97c, value(4)),
            (0x980, value(6)),
            (0x984, value(5)),
            (0x988, half),
            (0x98c, half),
            (0x990, value(7)),
            (0x994, value(9)),
            (0x998, half),
            (0x99c, value(10)),
            (0x9a0, value(8)),
            (0x9a4, one),
            (0x9a8, half),
            (0x9ac, one),
            (0x9b0, one),
            (0x9b4, one),
            (0x9b8, one),
            (0x9bc, one),
            (0x9c0, 0),
            (0x9c4, one),
            (0x9c8, one),
            (0x9cc, one),
            (0x9d0, 0),
        ];
        for (offset, bits) in expected {
            assert_eq!(e.global::<u32>(0x011f_f000 + offset), bits, "{offset:#x}");
        }
        // 011ff978 is not written.
        assert_eq!(e.global::<u32>(0x011f_f978), 0xeeee_eeee);
    }
}

#[cfg(test)]
mod init_geometry_tests {
    use super::*;

    const DELETE: u32 = 0x0ef0_0000;
    const VTABLE: u32 = 0x0ef1_0000;
    const NAME: u32 = 0x0ef2_0008;
    const DATA_OF_ELEMENTS: u32 = 0x0ef3_0000;

    fn calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        let log = e.call_log.as_ref().unwrap();
        log.iter()
            .filter(|c| c.0 == addr)
            .map(|c| c.1.clone())
            .collect()
    }

    fn engine(allocate: bool) -> Engine {
        let mut e = Engine::new();
        e.register(IMPORT_INTERLOCKED_INCREMENT, |e, a| {
            let count = e.mem.u32(a[0]).wrapping_add(1);
            e.mem.set_u32(a[0], count);
            count.into_ret()
        });
        e.register(IMPORT_INTERLOCKED_DECREMENT, |e, a| {
            let count = e.mem.u32(a[0]).wrapping_sub(1);
            e.mem.set_u32(a[0], count);
            count.into_ret()
        });
        e.register(DELETE, |_, _| Ret::default());
        e.put_vtable(VTABLE, &[0, DELETE]);
        e.register_double(NI_ALLOCATE, move |e, a| {
            if allocate {
                let block = e.mem.alloc(a[0]);
                e.mem.set_u32(block, VTABLE);
                block.into_ret()
            } else {
                0u32.into_ret()
            }
        });
        e.register(SCREEN_ELEMENTS_DATA_CONSTRUCT, |_, a| a[0].into_ret());
        e.register(SCREEN_ELEMENTS_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0] + 0xb8, DATA_OF_ELEMENTS);
            a[0].into_ret()
        });
        for addr in [
            SCREEN_ELEMENTS_DATA_INSERT,
            SCREEN_ELEMENTS_DATA_SET_RECTANGLE,
            SCREEN_ELEMENTS_DATA_SET_TEXTURES,
            SCREEN_ELEMENTS_DATA_UPDATE_BOUND,
            SET_NAME_00A5B950,
            UPDATE_PROPERTIES,
        ] {
            e.register(addr, |_, _| Ret::default());
        }
        e.register(UPDATE_00A59C60, |e, a| {
            assert_eq!(e.mem.u32(a[1]), 0);
            assert_eq!(e.mem.bytes(a[1] + 4, 5), vec![0; 5]);
            Ret::default()
        });
        e.register(MAKE_FIXED_STRING_00A5B690, |_, _| NAME.into_ret());
        e.map(0x0101_2000, 0x5000);
        e.map(0x0104_a000, 0x1000);
        e.map(0x0109_b000, 0x1000);
        e.map(NAME - 8, 0x10);
        e.set_global::<f32>(RECTANGLE_X_01012054, -1.0);
        e.set_global::<f32>(RECTANGLE_WIDTH_010162C0, 2.0);
        e.set_global::<f32>(RECTANGLE_HEIGHT_0104A494, -2.0);
        e.call_log = Some(vec![]);
        e
    }

    #[test]
    fn image_space_manager_init_geometry_builds_the_screen_elements() {
        let mut e = engine(true);
        let this = e.mem.alloc(0x200);
        e.call(INIT_GEOMETRY, &args![this]);
        let data = e.mem.u32(this + 0x1f4);
        let elements = e.mem.u32(this + 0x1f0);
        assert_ne!(data, 0);
        assert_eq!(e.mem.u32(data + 4), 1);
        assert_eq!(e.mem.u32(elements + 4), 1);
        assert_eq!(
            calls(&e, SCREEN_ELEMENTS_DATA_CONSTRUCT),
            vec![vec![data, 0, 0, 1, 1, 1, 4, 1, 2, 1]]
        );
        assert_eq!(
            calls(&e, SCREEN_ELEMENTS_CONSTRUCT),
            vec![vec![elements, data]]
        );
        let d = DATA_OF_ELEMENTS;
        assert_eq!(
            calls(&e, SCREEN_ELEMENTS_DATA_INSERT),
            vec![vec![d, 4, 0, 0]]
        );
        assert_eq!(
            calls(&e, SCREEN_ELEMENTS_DATA_SET_RECTANGLE),
            vec![vec![
                d,
                0,
                (-1.0f32).to_bits(),
                1.0f32.to_bits(),
                2.0f32.to_bits(),
                (-2.0f32).to_bits()
            ]]
        );
        assert_eq!(
            calls(&e, SCREEN_ELEMENTS_DATA_SET_TEXTURES),
            vec![vec![d, 0, 0, 0, 0, 1.0f32.to_bits(), 1.0f32.to_bits()]]
        );
        assert_eq!(calls(&e, SCREEN_ELEMENTS_DATA_UPDATE_BOUND), vec![vec![d]]);
        assert_eq!(
            calls(&e, MAKE_FIXED_STRING_00A5B690),
            vec![vec![GEOMETRY_NAME_010B04AC]]
        );
        let named = calls(&e, SET_NAME_00A5B950);
        assert_eq!(named.len(), 1);
        assert_eq!(named[0][0], elements);
        assert_eq!(
            calls(&e, IMPORT_INTERLOCKED_DECREMENT),
            vec![vec![NAME - 8]]
        );
        assert_eq!(calls(&e, UPDATE_PROPERTIES), vec![vec![elements]]);
        assert_eq!(calls(&e, UPDATE_00A59C60).len(), 1);
    }

    #[test]
    fn image_space_manager_init_geometry_failed_allocations_leave_null_pointers() {
        let mut e = engine(false);
        let this = e.mem.alloc(0x200);
        // The elements' data is then read through a null pointer, as the exe does.
        e.map(0, 0x1000);
        e.call(INIT_GEOMETRY, &args![this]);
        assert_eq!(e.mem.u32(this + 0x1f4), 0);
        assert_eq!(e.mem.u32(this + 0x1f0), 0);
        assert!(calls(&e, SCREEN_ELEMENTS_DATA_CONSTRUCT).is_empty());
        assert!(calls(&e, SCREEN_ELEMENTS_CONSTRUCT).is_empty());
        assert_eq!(
            calls(&e, SCREEN_ELEMENTS_DATA_INSERT),
            vec![vec![0, 4, 0, 0]]
        );
    }
}

#[cfg(test)]
mod set_up_00b94520_tests {
    use super::*;

    const VTABLE: u32 = 0x0e50_0000;
    const SLOT_20: u32 = 0x0e51_0020;
    const SLOT_24: u32 = 0x0e51_0024;
    const SLOT_120: u32 = 0x0e51_0120;

    fn calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|c| c.0 == addr)
            .map(|c| c.1.clone())
            .collect()
    }

    fn rig(ni_objects: bool) -> (Engine, u32) {
        let mut e = Engine::new();
        let mut slots = vec![0u32; 0x49];
        slots[0x20 / 4] = SLOT_20;
        slots[0x24 / 4] = SLOT_24;
        slots[0x120 / 4] = SLOT_120;
        e.put_vtable(VTABLE, &slots);
        for addr in [SLOT_20, SLOT_24, SLOT_120] {
            e.register(addr, |_, _| Ret::default());
        }
        e.register(MEMORY_ALLOCATE, |e, a| e.mem.alloc(a[1]).into_ret());
        if ni_objects {
            e.register(NI_ALLOCATE, |e, a| e.mem.alloc(a[0]).into_ret());
        } else {
            e.register(NI_ALLOCATE, |_, _| Ret::default());
        }
        let constructors = SET_UP_MANAGED
            .iter()
            .map(|c| c.1)
            .chain(SET_UP_NI.iter().map(|c| c.1))
            .chain(SET_UP_NI_ARRAY.iter().map(|c| c.0))
            .chain([CONSTRUCT_00BA4640]);
        for addr in constructors {
            e.register(addr, |e, a| {
                e.mem.set_u32(a[0], VTABLE);
                a[0].into_ret()
            });
        }
        // The array setter stores the pointed-at word under the index.
        e.register(ARRAY_SET_00BA3AA0, |e, a| {
            let block = e.mem.u32(a[0] + 4);
            let word = e.mem.u32(a[2]);
            e.mem.set_u32(block + 4 * a[1], word);
            Ret::default()
        });
        let this = e.mem.alloc(0x20);
        let block = e.mem.alloc(4 * 0x80);
        e.mem.set_u32(this + EFFECTS, block);
        e.call_log = Some(vec![]);
        (e, this)
    }

    #[test]
    fn fn_00b94520_fills_the_effect_array() {
        let (mut e, this) = rig(true);
        e.call(0x00b9_4520, &args![this]);
        let managed = calls(&e, MEMORY_ALLOCATE);
        assert_eq!(managed.len(), 20 + 14);
        assert_eq!(managed[0], vec![MEMORY_MANAGER, 0x68]);
        assert_eq!(managed[20], vec![MEMORY_MANAGER, 0x60]);
        let ni = calls(&e, NI_ALLOCATE);
        assert_eq!(ni.len(), 80);
        assert_eq!(ni.iter().filter(|a| a[0] == 0xd4).count(), 1);
        let stored: Vec<Vec<u32>> = calls(&e, ARRAY_SET_00BA3AA0)
            .iter()
            .map(|a| a[..2].to_vec())
            .collect();
        assert_eq!(stored.len(), 22);
        assert_eq!(stored[0], vec![this + 4, 0xd]);
        assert_eq!(stored[10], vec![this + 4, 7]);
        assert_eq!(stored[20], vec![this + 4, 0x73]);
        assert_eq!(stored[21], vec![this + 4, 0x4d]);
        let block = e.mem.u32(this + EFFECTS);
        // The pairs: vtable +0x20 with k, +0x24 with 0 then 1.
        let ranks: Vec<u32> = calls(&e, SLOT_20).iter().map(|a| a[1]).collect();
        assert_eq!(ranks, vec![1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7]);
        let flags: Vec<u32> = calls(&e, SLOT_24).iter().map(|a| a[1]).collect();
        assert_eq!(flags, [0, 1].repeat(7));
        assert_eq!(calls(&e, SLOT_20)[0][0], e.mem.u32(block + 4 * 0x10));
        assert_eq!(calls(&e, SLOT_20)[13][0], e.mem.u32(block + 4 * 0x1d));
        // The inline stores: elements are the objects +0x6c; size 0x73, 14 + 78 counted.
        let first = ni[0].clone();
        assert_eq!(first, vec![0xd0]);
        assert_eq!(e.mem.u32(e.mem.u32(block + 4 * 0x22) - 0x6c), VTABLE);
        assert_eq!(e.mem.u16(this + EFFECTS_SIZE), 0x73);
        assert_eq!(e.mem.u16(this + EFFECTS_COUNT), 14 + 78);
        // The closing pass over indices 0x22 to 0x73 calls vtable +0x120 of each object.
        let set = (0x22..=0x73u32)
            .filter(|&i| e.mem.u32(block + 4 * i) != 0)
            .count();
        let finals = calls(&e, SLOT_120);
        assert_eq!(finals.len(), set);
        assert_eq!(finals.len(), 80);
        assert_eq!(finals[0], vec![e.mem.u32(block + 4 * 0x22) - 0x6c]);
    }

    #[test]
    fn fn_00b94520_null_elements_empty_full_slots() {
        let (mut e, this) = rig(false);
        let block = e.mem.u32(this + EFFECTS);
        let object = e.mem.alloc(0x100);
        e.mem.set_u32(object, VTABLE);
        for index in 0..0x80u32 {
            e.mem.set_u32(block + 4 * index, object + 0x6c);
        }
        e.mem.set_u16(this + EFFECTS_SIZE, 0x80);
        e.mem.set_u16(this + EFFECTS_COUNT, 0x80);
        e.call(0x00b9_4520, &args![this]);
        // Each null element over a full slot takes one away; the pairs land in full slots.
        assert_eq!(e.mem.u16(this + EFFECTS_SIZE), 0x80);
        assert_eq!(e.mem.u16(this + EFFECTS_COUNT), 0x80 - 78);
        assert!(calls(&e, ARRAY_SET_00BA3AA0).iter().all(|a| a[1] < 0x80));
        assert_eq!(e.mem.u32(block + 4 * 0x73), 0);
        assert_eq!(e.mem.u32(block + 4 * 0x4d), 0);
        // Only the slots nothing stored to are still set at the closing pass.
        let left = (0x22..=0x73u32)
            .filter(|&i| e.mem.u32(block + 4 * i) != 0)
            .count();
        assert_eq!(calls(&e, SLOT_120).len(), left);
        assert!(calls(&e, SLOT_120).iter().all(|a| a == &vec![object]));
    }
}

#[cfg(test)]
mod shader_constructor_tests {
    use super::*;

    fn calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|c| c.0 == addr)
            .map(|c| c.1.clone())
            .collect()
    }

    /// The base constructor writes marker vtables; the resize checks both were replaced
    /// before it runs.
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.register(IMAGE_SPACE_SHADER_CONSTRUCT_00C04570, |e, a| {
            e.mem.set_u32(a[0], 0x1111);
            e.mem.set_u32(a[0] + SHADER_EFFECT_BASE, 0x2222);
            a[0].into_ret()
        });
        e.register(ARRAY_SET_SIZE_00A90180, |e, a| {
            let object = a[0] - SHADER_TEXTURES;
            assert_ne!(e.mem.u32(object), 0x1111);
            assert_ne!(e.mem.u32(object + SHADER_EFFECT_BASE), 0x2222);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e
    }

    fn check(addr: u32, vtable: u32, effect_vtable: u32, textures: u32) {
        let mut e = engine();
        let this = e.mem.alloc(0xd0);
        assert_eq!(e.call(addr, &args![this]).u32(), this);
        let order: Vec<u32> = e.call_log.as_ref().unwrap().iter().map(|c| c.0).collect();
        assert_eq!(
            order,
            [
                addr,
                IMAGE_SPACE_SHADER_CONSTRUCT_00C04570,
                ARRAY_SET_SIZE_00A90180
            ]
        );
        assert_eq!(
            calls(&e, IMAGE_SPACE_SHADER_CONSTRUCT_00C04570),
            vec![vec![this]]
        );
        assert_eq!(
            calls(&e, ARRAY_SET_SIZE_00A90180),
            vec![vec![this + SHADER_TEXTURES, textures]]
        );
        assert_eq!(e.mem.u32(this), vtable);
        assert_eq!(e.mem.u32(this + SHADER_EFFECT_BASE), effect_vtable);
    }

    /// The engine, `this` and the object at `011f9520` (null without one; else with two
    /// references).
    fn texture_mask_rig(with_object: bool) -> (Engine, u32, u32) {
        let mut e = engine();
        let object = if with_object { e.mem.alloc(8) } else { 0 };
        if with_object {
            e.mem.set_u32(object + 4, 2);
        }
        e.register(IMPORT_INTERLOCKED_INCREMENT, |e, a| {
            let count = e.mem.u32(a[0]).wrapping_add(1);
            e.mem.set_u32(a[0], count);
            count.into_ret()
        });
        e.map(0x011f_9000, 0x1000);
        e.set_global(OBJECT_011F9520, object);
        let this = e.mem.alloc(0xd4);
        e.mem.set_u32(this + 0xd0, 0xdead_beef);
        (e, this, object)
    }

    #[test]
    fn fn_00b922d0_takes_a_reference_on_the_object() {
        let (mut e, this, object) = texture_mask_rig(true);
        assert_eq!(e.call(0x00b9_22d0, &args![this]).u32(), this);
        assert_eq!(e.mem.u32(this), 0x010b_5978);
        assert_eq!(e.mem.u32(this + SHADER_EFFECT_BASE), 0x010b_5954);
        assert_eq!(
            calls(&e, ARRAY_SET_SIZE_00A90180),
            vec![vec![this + SHADER_TEXTURES, 2]]
        );
        assert_eq!(e.mem.u32(this + 0xd0), object);
        assert_eq!(e.mem.u32(object + 4), 3);
        // The cleared pointer had nothing to give up.
        assert!(calls(&e, IMPORT_INTERLOCKED_DECREMENT).is_empty());
    }

    #[test]
    fn fn_00b922d0_with_no_object_keeps_the_pointer_clear() {
        let (mut e, this, _) = texture_mask_rig(false);
        e.call(0x00b9_22d0, &args![this]);
        assert_eq!(e.mem.u32(this + 0xd0), 0);
        assert!(calls(&e, IMPORT_INTERLOCKED_INCREMENT).is_empty());
        assert!(calls(&e, IMPORT_INTERLOCKED_DECREMENT).is_empty());
    }

    #[test]
    fn fn_00b8bce0_constructs_the_shader() {
        check(0x00b8_bce0, 0x010a_ffc0, 0x010a_ffa0, 5);
    }

    #[test]
    fn fn_00b8bea0_constructs_the_shader() {
        check(0x00b8_bea0, 0x010b_0198, 0x010b_0174, 2);
    }

    #[test]
    fn fn_00b8c070_constructs_the_shader() {
        check(0x00b8_c070, 0x010b_0338, 0x010b_0318, 4);
    }

    #[test]
    fn fn_00b8d330_constructs_the_shader() {
        check(0x00b8_d330, 0x010b_0518, 0x010b_04f4, 3);
    }

    #[test]
    fn fn_00b8d5b0_constructs_the_shader() {
        check(0x00b8_d5b0, 0x010b_06c0, 0x010b_06a0, 2);
    }

    #[test]
    fn fn_00b8d700_constructs_the_shader() {
        check(0x00b8_d700, 0x010b_0860, 0x010b_083c, 3);
    }

    #[test]
    fn fn_00b8d850_constructs_the_shader() {
        check(0x00b8_d850, 0x010b_0a00, 0x010b_09dc, 2);
    }

    #[test]
    fn fn_00b8d990_constructs_the_shader() {
        check(0x00b8_d990, 0x010b_0b98, 0x010b_0b78, 2);
    }

    #[test]
    fn fn_00b8dae0_constructs_the_shader() {
        check(0x00b8_dae0, 0x010b_0d40, 0x010b_0d1c, 2);
    }

    #[test]
    fn fn_00b8dc30_constructs_the_shader() {
        check(0x00b8_dc30, 0x010b_0ec0, 0x010b_0e9c, 2);
    }

    #[test]
    fn fn_00b8dd80_constructs_the_shader() {
        check(0x00b8_dd80, 0x010b_1040, 0x010b_101c, 2);
    }

    #[test]
    fn fn_00b8ded0_constructs_the_shader() {
        check(0x00b8_ded0, 0x010b_11c0, 0x010b_119c, 2);
    }

    #[test]
    fn fn_00b8e020_constructs_the_shader() {
        check(0x00b8_e020, 0x010b_1340, 0x010b_1320, 2);
    }

    #[test]
    fn fn_00b8e170_constructs_the_shader() {
        check(0x00b8_e170, 0x010b_14c0, 0x010b_14a0, 2);
    }

    #[test]
    fn fn_00b8e2c0_constructs_the_shader() {
        check(0x00b8_e2c0, 0x010b_1640, 0x010b_1620, 3);
    }

    #[test]
    fn fn_00b8e460_constructs_the_shader() {
        check(0x00b8_e460, 0x010b_17f8, 0x010b_17d8, 3);
    }

    #[test]
    fn fn_00b8e600_constructs_the_shader() {
        check(0x00b8_e600, 0x010b_1980, 0x010b_1960, 3);
    }

    #[test]
    fn fn_00b8e7a0_constructs_the_shader() {
        check(0x00b8_e7a0, 0x010b_1b08, 0x010b_1ae8, 3);
    }

    #[test]
    fn fn_00b8e940_constructs_the_shader() {
        check(0x00b8_e940, 0x010b_1c90, 0x010b_1c70, 3);
    }

    #[test]
    fn fn_00b8eae0_constructs_the_shader() {
        check(0x00b8_eae0, 0x010b_1e20, 0x010b_1dfc, 3);
    }

    #[test]
    fn fn_00b8ec80_constructs_the_shader() {
        check(0x00b8_ec80, 0x010b_1fb0, 0x010b_1f8c, 3);
    }

    #[test]
    fn fn_00b8ee20_constructs_the_shader() {
        check(0x00b8_ee20, 0x010b_2140, 0x010b_211c, 2);
    }

    #[test]
    fn fn_00b8efa0_constructs_the_shader() {
        check(0x00b8_efa0, 0x010b_2a98, 0x010b_2a74, 2);
    }

    #[test]
    fn fn_00b8f010_constructs_the_shader() {
        check(0x00b8_f010, 0x010b_2308, 0x010b_22e8, 2);
    }

    #[test]
    fn fn_00b8f1b0_constructs_the_shader() {
        check(0x00b8_f1b0, 0x010b_2490, 0x010b_2470, 2);
    }

    #[test]
    fn fn_00b8f360_constructs_the_shader() {
        check(0x00b8_f360, 0x010b_2610, 0x010b_25f0, 1);
    }

    #[test]
    fn fn_00b8f520_constructs_the_shader() {
        check(0x00b8_f520, 0x010b_27a8, 0x010b_2784, 1);
    }

    #[test]
    fn fn_00b8f660_constructs_the_shader() {
        check(0x00b8_f660, 0x010b_2920, 0x010b_28fc, 3);
    }

    #[test]
    fn fn_00b8f7e0_constructs_the_shader() {
        check(0x00b8_f7e0, 0x010b_2c10, 0x010b_2bec, 4);
    }

    #[test]
    fn fn_00b8f930_constructs_the_shader() {
        check(0x00b8_f930, 0x010b_2db0, 0x010b_2d8c, 2);
    }

    #[test]
    fn fn_00b8fa80_constructs_the_shader() {
        check(0x00b8_fa80, 0x010b_2f50, 0x010b_2f30, 2);
    }

    #[test]
    fn fn_00b8fbe0_constructs_the_shader() {
        check(0x00b8_fbe0, 0x010b_30e0, 0x010b_30c0, 2);
    }

    #[test]
    fn fn_00b8fd40_constructs_the_shader() {
        check(0x00b8_fd40, 0x010b_3268, 0x010b_3244, 5);
    }

    #[test]
    fn fn_00b8fea0_constructs_the_shader() {
        check(0x00b8_fea0, 0x010b_33f0, 0x010b_33cc, 3);
    }

    #[test]
    fn fn_00b8fff0_constructs_the_shader() {
        check(0x00b8_fff0, 0x010b_3598, 0x010b_3574, 4);
    }

    #[test]
    fn fn_00b90190_constructs_the_shader() {
        check(0x00b9_0190, 0x010b_37a8, 0x010b_3788, 4);
    }

    #[test]
    fn fn_00b90330_constructs_the_shader() {
        check(0x00b9_0330, 0x010b_3958, 0x010b_3934, 4);
    }

    #[test]
    fn fn_00b90520_constructs_the_shader() {
        check(0x00b9_0520, 0x010b_3af0, 0x010b_3ad0, 2);
    }

    #[test]
    fn fn_00b906b0_constructs_the_shader() {
        check(0x00b9_06b0, 0x010b_3c78, 0x010b_3c58, 2);
    }

    #[test]
    fn fn_00b90830_constructs_the_shader() {
        check(0x00b9_0830, 0x010b_3e18, 0x010b_3df4, 2);
    }

    #[test]
    fn fn_00b909c0_constructs_the_shader() {
        check(0x00b9_09c0, 0x010b_3fa8, 0x010b_3f88, 2);
    }

    #[test]
    fn fn_00b90b50_constructs_the_shader() {
        check(0x00b9_0b50, 0x010b_4138, 0x010b_4114, 2);
    }

    #[test]
    fn fn_00b90ce0_constructs_the_shader() {
        check(0x00b9_0ce0, 0x010b_42c8, 0x010b_42a4, 3);
    }

    #[test]
    fn fn_00b90e70_constructs_the_shader() {
        check(0x00b9_0e70, 0x010b_4460, 0x010b_443c, 2);
    }

    #[test]
    fn fn_00b91000_constructs_the_shader() {
        check(0x00b9_1000, 0x010b_4600, 0x010b_45dc, 3);
    }

    #[test]
    fn fn_00b911a0_constructs_the_shader() {
        check(0x00b9_11a0, 0x010b_47a0, 0x010b_477c, 2);
    }

    #[test]
    fn fn_00b91340_constructs_the_shader() {
        check(0x00b9_1340, 0x010b_4938, 0x010b_4914, 3);
    }

    #[test]
    fn fn_00b914e0_constructs_the_shader() {
        check(0x00b9_14e0, 0x010b_4ad0, 0x010b_4aac, 2);
    }

    #[test]
    fn fn_00b91680_constructs_the_shader() {
        check(0x00b9_1680, 0x010b_4c68, 0x010b_4c48, 5);
    }

    #[test]
    fn fn_00b917c0_constructs_the_shader() {
        check(0x00b9_17c0, 0x010b_4de8, 0x010b_4dc8, 2);
    }

    #[test]
    fn fn_00b91900_constructs_the_shader() {
        check(0x00b9_1900, 0x010b_4f80, 0x010b_4f60, 3);
    }

    #[test]
    fn fn_00b91a40_constructs_the_shader() {
        check(0x00b9_1a40, 0x010b_5140, 0x010b_511c, 2);
    }

    #[test]
    fn fn_00b91b80_constructs_the_shader() {
        check(0x00b9_1b80, 0x010b_52c0, 0x010b_52a0, 3);
    }

    #[test]
    fn fn_00b91cf0_constructs_the_shader() {
        check(0x00b9_1cf0, 0x010b_5480, 0x010b_5460, 2);
    }

    #[test]
    fn fn_00b91e40_constructs_the_shader() {
        check(0x00b9_1e40, 0x010b_5640, 0x010b_5620, 2);
    }

    #[test]
    fn fn_00b92180_constructs_the_shader() {
        check(0x00b9_2180, 0x010b_57d0, 0x010b_57b0, 3);
    }

    #[test]
    fn fn_00b92500_constructs_the_shader() {
        check(0x00b9_2500, 0x010b_5b10, 0x010b_5af0, 5);
    }

    #[test]
    fn fn_00b92690_constructs_the_shader() {
        check(0x00b9_2690, 0x010b_5cb8, 0x010b_5c94, 2);
    }

    #[test]
    fn fn_00b927e0_constructs_the_shader() {
        check(0x00b9_27e0, 0x010b_5ea8, 0x010b_5e84, 2);
    }

    #[test]
    fn fn_00b92930_constructs_the_shader() {
        check(0x00b9_2930, 0x010b_6048, 0x010b_6028, 2);
    }

    #[test]
    fn fn_00b92a80_constructs_the_shader() {
        check(0x00b9_2a80, 0x010b_61f0, 0x010b_61d0, 2);
    }

    #[test]
    fn fn_00b92bd0_constructs_the_shader() {
        check(0x00b9_2bd0, 0x010b_6398, 0x010b_6374, 2);
    }

    #[test]
    fn fn_00b92d90_constructs_the_shader() {
        check(0x00b9_2d90, 0x010b_6548, 0x010b_6524, 2);
    }

    #[test]
    fn fn_00b92ee0_constructs_the_shader() {
        check(0x00b9_2ee0, 0x010b_66f0, 0x010b_66cc, 3);
    }

    #[test]
    fn fn_00b93030_constructs_the_shader() {
        check(0x00b9_3030, 0x010b_68a0, 0x010b_687c, 2);
    }

    #[test]
    fn fn_00b93180_constructs_the_shader() {
        check(0x00b9_3180, 0x010b_6a40, 0x010b_6a20, 2);
    }

    #[test]
    fn fn_00b932d0_constructs_the_shader() {
        check(0x00b9_32d0, 0x010b_6bd8, 0x010b_6bb8, 3);
    }

    #[test]
    fn fn_00b93600_constructs_the_shader() {
        check(0x00b9_3600, 0x010b_6d90, 0x010b_6d70, 4);
    }

    #[test]
    fn fn_00b93950_constructs_the_shader() {
        check(0x00b9_3950, 0x010b_6f38, 0x010b_6f14, 4);
    }

    #[test]
    fn fn_00b93aa0_constructs_the_shader() {
        check(0x00b9_3aa0, 0x010b_70d8, 0x010b_70b8, 4);
    }

    #[test]
    fn fn_00b93bf0_constructs_the_shader() {
        check(0x00b9_3bf0, 0x010b_7278, 0x010b_7258, 4);
    }

    #[test]
    fn fn_00b93d40_constructs_the_shader() {
        check(0x00b9_3d40, 0x010b_7418, 0x010b_73f4, 2);
    }

    #[test]
    fn fn_00b93e90_constructs_the_shader() {
        check(0x00b9_3e90, 0x010b_7598, 0x010b_7578, 2);
    }

    #[test]
    fn fn_00b93fe0_constructs_the_shader() {
        check(0x00b9_3fe0, 0x010b_7720, 0x010b_7700, 2);
    }

    #[test]
    fn fn_00b94130_constructs_the_shader() {
        check(0x00b9_4130, 0x010b_78b8, 0x010b_7898, 2);
    }

    #[test]
    fn fn_00b94280_constructs_the_shader() {
        check(0x00b9_4280, 0x010b_7a50, 0x010b_7a2c, 2);
    }

    #[test]
    fn fn_00b943d0_constructs_the_shader() {
        check(0x00b9_43d0, 0x010b_7bf0, 0x010b_7bcc, 2);
    }
}

#[cfg(test)]
mod b000n_tests {
    use super::*;

    #[test]
    fn fn_00b8bc20_sets_up_the_four_entries() {
        let mut e = Engine::new();
        // The double notes the index and the byte it was given.
        let seen = e.mem.alloc(0x10);
        e.register_double(SET_UP_00B8B270, move |e, a| {
            let value = e.mem.u8(a[2]);
            e.mem.set_u8(seen + a[1], value + 0x10);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        let this = e.mem.alloc(0x60);
        e.call(0x00b8_bc20, &args![this]);
        assert_eq!(e.mem.bytes(seen, 4), vec![0x11, 0x11, 0x11, 0x10]);
        let log = e.call_log.take().unwrap();
        let targets: Vec<(u32, u32)> = log
            .iter()
            .filter(|c| c.0 == SET_UP_00B8B270)
            .map(|c| (c.1[0], c.1[1]))
            .collect();
        assert_eq!(
            targets,
            vec![
                (this + 0x58, 0),
                (this + 0x58, 1),
                (this + 0x58, 2),
                (this + 0x58, 3)
            ]
        );
    }
}

#[cfg(test)]
mod b000p_tests {
    use super::*;

    #[test]
    fn fn_00b8b270_stores_bytes_and_counts_the_set_ones() {
        let mut e = Engine::new();
        let array = e.mem.alloc(0x10);
        let bytes = e.mem.alloc(8);
        e.mem.set_u32(array + 4, bytes);
        let value = e.mem.alloc(4);
        // Past the size: the size grows, a set byte counts.
        e.mem.set_u8(value, 1);
        e.call(0x00b8_b270, &args![array, 2u32, value]);
        assert_eq!(e.mem.u16(array + 0xa), 3);
        assert_eq!(e.mem.u16(array + 0xc), 1);
        assert_eq!(e.mem.u8(bytes + 2), 1);
        // A clear byte past the size: the size grows, no count.
        e.mem.set_u8(value, 0);
        e.call(0x00b8_b270, &args![array, 4u32, value]);
        assert_eq!(e.mem.u16(array + 0xa), 5);
        assert_eq!(e.mem.u16(array + 0xc), 1);
        // Below the size: set over clear counts up, set over set does not, clear over set down.
        e.mem.set_u8(value, 7);
        e.call(0x00b8_b270, &args![array, 0u32, value]);
        assert_eq!(e.mem.u16(array + 0xc), 2);
        e.call(0x00b8_b270, &args![array, 0u32, value]);
        assert_eq!(e.mem.u16(array + 0xc), 2);
        assert_eq!(e.mem.u8(bytes), 7);
        e.mem.set_u8(value, 0);
        e.call(0x00b8_b270, &args![array, 2u32, value]);
        assert_eq!(e.mem.u16(array + 0xc), 1);
        assert_eq!(e.mem.u8(bytes + 2), 0);
        e.call(0x00b8_b270, &args![array, 1u32, value]);
        assert_eq!(e.mem.u16(array + 0xc), 1);
        assert_eq!(e.mem.u16(array + 0xa), 5);
    }
}

#[cfg(test)]
mod delete_tests {
    use super::*;

    fn calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|c| c.0 == addr)
            .map(|c| c.1.clone())
            .collect()
    }

    /// The doubles of `~ImageSpaceShader` and the free; the destructor double notes at +0xc8 the
    /// vtable it finds, so the tests see it was set before it runs.
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.register(IMAGE_SPACE_SHADER_DESTRUCT_00C03A20, |e, a| {
            let vtable = e.mem.u32(a[0]);
            e.mem.set_u32(a[0] + 0xc8, vtable);
            Ret::default()
        });
        e.register(NI_SIZED_FREE_00AA1460, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e
    }

    fn check_delete(addr: u32, vtable: u32, effect_vtable: u32) {
        for flags in [0u32, 1, 0xfe, 0x101] {
            let mut e = engine();
            let this = e.mem.alloc(0xd0);
            assert_eq!(e.call(addr, &args![this, flags]).u32(), this);
            assert_eq!(e.mem.u32(this), vtable);
            assert_eq!(e.mem.u32(this + 0xc8), vtable);
            assert_eq!(e.mem.u32(this + SHADER_EFFECT_BASE), effect_vtable);
            assert_eq!(
                calls(&e, IMAGE_SPACE_SHADER_DESTRUCT_00C03A20),
                vec![vec![this]]
            );
            let freed = if flags & 1 != 0 {
                vec![vec![this, 0xd0]]
            } else {
                vec![]
            };
            assert_eq!(calls(&e, NI_SIZED_FREE_00AA1460), freed);
        }
    }

    fn check_effect_delete(addr: u32, vtable: u32, effect_vtable: u32) {
        let mut e = engine();
        let this = e.mem.alloc(0xd0);
        let effect = this + SHADER_EFFECT_BASE;
        assert_eq!(e.call(addr, &args![effect, 1u32]).u32(), this);
        assert_eq!(e.mem.u32(this), vtable);
        assert_eq!(e.mem.u32(effect), effect_vtable);
        assert_eq!(
            calls(&e, IMAGE_SPACE_SHADER_DESTRUCT_00C03A20),
            vec![vec![this]]
        );
        assert_eq!(calls(&e, NI_SIZED_FREE_00AA1460), vec![vec![this, 0xd0]]);
    }

    #[test]
    fn fn_00b91b50_destroys_without_vtables() {
        for flags in [0u32, 3] {
            let mut e = engine();
            let this = e.mem.alloc(0xd0);
            e.mem.set_u32(this, 0x77);
            assert_eq!(e.call(0x00b9_1b50, &args![this, flags]).u32(), this);
            assert_eq!(e.mem.u32(this), 0x77);
            assert_eq!(
                calls(&e, IMAGE_SPACE_SHADER_DESTRUCT_00C03A20),
                vec![vec![this]]
            );
            assert_eq!(
                calls(&e, NI_SIZED_FREE_00AA1460).len(),
                (flags & 1) as usize
            );
        }
    }

    #[test]
    fn fn_00b91b40_adjusts_to_the_shader() {
        let mut e = engine();
        let this = e.mem.alloc(0xd0);
        assert_eq!(e.call(0x00b9_1b40, &args![this + 0x6c, 1u32]).u32(), this);
        assert_eq!(
            calls(&e, IMAGE_SPACE_SHADER_DESTRUCT_00C03A20),
            vec![vec![this]]
        );
        assert_eq!(calls(&e, NI_SIZED_FREE_00AA1460), vec![vec![this, 0xd0]]);
    }

    #[test]
    fn fn_00b92450_releases_the_pointer() {
        let mut e = engine();
        e.register(IMPORT_INTERLOCKED_DECREMENT, |e, a| {
            let left = e.mem.u32(a[0]) - 1;
            e.mem.set_u32(a[0], left);
            left.into_ret()
        });
        e.register(0x0f00_0004, |_, _| Ret::default());
        let vtable = e.mem.alloc(8);
        e.mem.set_u32(vtable + 4, 0x0f00_0004);
        let held = e.mem.alloc(8);
        e.mem.set_u32(held, vtable);
        // Two references: one given up, nothing deleted.
        e.mem.set_u32(held + 4, 2);
        let this = e.mem.alloc(0xd4);
        e.mem.set_u32(this + 0xd0, held);
        e.call(0x00b9_2450, &args![this]);
        assert_eq!(e.mem.u32(this), 0x010b_5978);
        assert_eq!(e.mem.u32(this + 0x6c), 0x010b_5954);
        assert_eq!(e.mem.u32(held + 4), 1);
        assert!(calls(&e, 0x0f00_0004).is_empty());
        // The last reference: deleted through slot +4.
        e.call(0x00b9_2450, &args![this]);
        assert_eq!(calls(&e, 0x0f00_0004), vec![vec![held]]);
        // No pointer: no decrement.
        e.mem.set_u32(this + 0xd0, 0);
        e.call_log = Some(vec![]);
        e.call(0x00b9_2450, &args![this]);
        assert!(calls(&e, IMPORT_INTERLOCKED_DECREMENT).is_empty());
        assert_eq!(
            calls(&e, IMAGE_SPACE_SHADER_DESTRUCT_00C03A20),
            vec![vec![this]]
        );
    }

    #[test]
    fn fn_00b924d0_frees_0xd4_bytes() {
        let mut e = engine();
        let this = e.mem.alloc(0xd4);
        assert_eq!(e.call(0x00b9_24d0, &args![this, 1u32]).u32(), this);
        assert_eq!(e.mem.u32(this), 0x010b_5978);
        assert_eq!(calls(&e, NI_SIZED_FREE_00AA1460), vec![vec![this, 0xd4]]);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x00b9_24d0, &args![this, 0u32]).u32(), this);
        assert!(calls(&e, NI_SIZED_FREE_00AA1460).is_empty());
    }

    #[test]
    fn fn_00b92440_adjusts_to_the_shader() {
        let mut e = engine();
        let this = e.mem.alloc(0xd4);
        assert_eq!(e.call(0x00b9_2440, &args![this + 0x6c, 1u32]).u32(), this);
        assert_eq!(e.mem.u32(this), 0x010b_5978);
        assert_eq!(calls(&e, NI_SIZED_FREE_00AA1460), vec![vec![this, 0xd4]]);
    }

    /// The volumetric fog shader with a texture block whose index 2 holds `texture`.
    fn fog(e: &mut Engine, texture: u32) -> u32 {
        e.register(RETURN_VOLUMETRIC_FOG_TEXTURE_00B63D10, |_, _| {
            Ret::default()
        });
        e.register(IMAGE_SPACE_TEXTURE_DESTRUCT_00BA3A00, |_, _| Ret::default());
        e.register(MEMORY_FREE_00AA4060, |_, _| Ret::default());
        e.register(ARRAY_SET_00BA3AA0, |e, a| {
            assert_eq!(e.mem.u32(a[2]), 0);
            let block = e.mem.u32(a[0] + 4);
            e.mem.set_u32(block + 4 * a[1], 0);
            Ret::default()
        });
        let this = e.mem.alloc(0xd0);
        let block = e.mem.alloc(0x10);
        e.mem.set_u32(this + 0x98, block);
        e.mem.set_u32(block + 8, texture);
        this
    }

    #[test]
    fn image_space_shader_volumetric_fog_destructor_drops_texture_two() {
        let mut e = engine();
        let this = fog(&mut e, 0x4400);
        e.call(0x00b8_d4d0, &args![this]);
        assert_eq!(e.mem.u32(this), 0x010b_0518);
        assert_eq!(e.mem.u32(this + 0x6c), 0x010b_04f4);
        assert_eq!(
            calls(&e, RETURN_VOLUMETRIC_FOG_TEXTURE_00B63D10),
            vec![vec![this]]
        );
        assert_eq!(
            calls(&e, IMAGE_SPACE_TEXTURE_DESTRUCT_00BA3A00),
            vec![vec![0x4400]]
        );
        assert_eq!(
            calls(&e, MEMORY_FREE_00AA4060),
            vec![vec![MEMORY_MANAGER, 0x4400]]
        );
        let set = calls(&e, ARRAY_SET_00BA3AA0);
        assert_eq!(set.len(), 1);
        assert_eq!(&set[0][..2], &[this + 0x94, 2]);
        let order: Vec<u32> = e.call_log.as_ref().unwrap().iter().map(|c| c.0).collect();
        assert_eq!(*order.last().unwrap(), IMAGE_SPACE_SHADER_DESTRUCT_00C03A20);
        // No texture at index 2: only the two destructor steps.
        let mut e = engine();
        let this = fog(&mut e, 0);
        e.call(0x00b8_d4d0, &args![this]);
        assert!(calls(&e, ARRAY_SET_00BA3AA0).is_empty());
        assert!(calls(&e, MEMORY_FREE_00AA4060).is_empty());
        assert_eq!(
            calls(&e, IMAGE_SPACE_SHADER_DESTRUCT_00C03A20),
            vec![vec![this]]
        );
    }

    #[test]
    fn fn_00b8d580_frees_the_fog_shader() {
        let mut e = engine();
        let this = fog(&mut e, 0);
        assert_eq!(e.call(0x00b8_d580, &args![this, 1u32]).u32(), this);
        assert_eq!(e.mem.u32(this), 0x010b_0518);
        assert_eq!(calls(&e, NI_SIZED_FREE_00AA1460), vec![vec![this, 0xd0]]);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x00b8_d580, &args![this, 2u32]).u32(), this);
        assert!(calls(&e, NI_SIZED_FREE_00AA1460).is_empty());
    }

    #[test]
    fn fn_00b8d430_adjusts_to_the_fog_shader() {
        let mut e = engine();
        let this = fog(&mut e, 0);
        assert_eq!(e.call(0x00b8_d430, &args![this + 0x6c, 1u32]).u32(), this);
        assert_eq!(e.mem.u32(this), 0x010b_0518);
        assert_eq!(calls(&e, NI_SIZED_FREE_00AA1460), vec![vec![this, 0xd0]]);
    }

    #[test]
    fn fn_00b8bda0_answers_the_byte_at_4() {
        let mut e = Engine::new();
        let this = e.mem.alloc(8);
        e.mem.set_u32(this + 4, 0x1234_5601);
        assert_eq!(e.call(0x00b8_bda0, &args![this]).u32() & 0xff, 1);
        e.mem.set_u8(this + 4, 0);
        assert_eq!(e.call(0x00b8_bda0, &args![this]).u32() & 0xff, 0);
    }

    #[test]
    fn fn_00b8bd50_destroys_the_shader() {
        check_delete(0x00b8_bd50, 0x010a_ffc0, 0x010a_ffa0);
    }

    #[test]
    fn fn_00b8bfa0_destroys_the_shader() {
        check_delete(0x00b8_bfa0, 0x010b_0198, 0x010b_0174);
    }

    #[test]
    fn fn_00b8d6c0_destroys_the_shader() {
        check_delete(0x00b8_d6c0, 0x010b_06c0, 0x010b_06a0);
    }

    #[test]
    fn fn_00b8d810_destroys_the_shader() {
        check_delete(0x00b8_d810, 0x010b_0860, 0x010b_083c);
    }

    #[test]
    fn fn_00b8d950_destroys_the_shader() {
        check_delete(0x00b8_d950, 0x010b_0a00, 0x010b_09dc);
    }

    #[test]
    fn fn_00b8daa0_destroys_the_shader() {
        check_delete(0x00b8_daa0, 0x010b_0b98, 0x010b_0b78);
    }

    #[test]
    fn fn_00b8dbf0_destroys_the_shader() {
        check_delete(0x00b8_dbf0, 0x010b_0d40, 0x010b_0d1c);
    }

    #[test]
    fn fn_00b8dd40_destroys_the_shader() {
        check_delete(0x00b8_dd40, 0x010b_0ec0, 0x010b_0e9c);
    }

    #[test]
    fn fn_00b8de90_destroys_the_shader() {
        check_delete(0x00b8_de90, 0x010b_1040, 0x010b_101c);
    }

    #[test]
    fn fn_00b8dfe0_destroys_the_shader() {
        check_delete(0x00b8_dfe0, 0x010b_11c0, 0x010b_119c);
    }

    #[test]
    fn fn_00b8e130_destroys_the_shader() {
        check_delete(0x00b8_e130, 0x010b_1340, 0x010b_1320);
    }

    #[test]
    fn fn_00b8e280_destroys_the_shader() {
        check_delete(0x00b8_e280, 0x010b_14c0, 0x010b_14a0);
    }

    #[test]
    fn fn_00b8e420_destroys_the_shader() {
        check_delete(0x00b8_e420, 0x010b_1640, 0x010b_1620);
    }

    #[test]
    fn fn_00b8e5c0_destroys_the_shader() {
        check_delete(0x00b8_e5c0, 0x010b_17f8, 0x010b_17d8);
    }

    #[test]
    fn fn_00b8e760_destroys_the_shader() {
        check_delete(0x00b8_e760, 0x010b_1980, 0x010b_1960);
    }

    #[test]
    fn fn_00b8e900_destroys_the_shader() {
        check_delete(0x00b8_e900, 0x010b_1b08, 0x010b_1ae8);
    }

    #[test]
    fn fn_00b8eaa0_destroys_the_shader() {
        check_delete(0x00b8_eaa0, 0x010b_1c90, 0x010b_1c70);
    }

    #[test]
    fn fn_00b8ec40_destroys_the_shader() {
        check_delete(0x00b8_ec40, 0x010b_1e20, 0x010b_1dfc);
    }

    #[test]
    fn fn_00b8ede0_destroys_the_shader() {
        check_delete(0x00b8_ede0, 0x010b_1fb0, 0x010b_1f8c);
    }

    #[test]
    fn fn_00b8ef60_destroys_the_shader() {
        check_delete(0x00b8_ef60, 0x010b_2140, 0x010b_211c);
    }

    #[test]
    fn fn_00b8f120_destroys_the_shader() {
        check_delete(0x00b8_f120, 0x010b_2308, 0x010b_22e8);
    }

    #[test]
    fn fn_00b8f2b0_destroys_the_shader() {
        check_delete(0x00b8_f2b0, 0x010b_2490, 0x010b_2470);
    }

    #[test]
    fn fn_00b8f470_destroys_the_shader() {
        check_delete(0x00b8_f470, 0x010b_2610, 0x010b_25f0);
    }

    #[test]
    fn fn_00b8f620_destroys_the_shader() {
        check_delete(0x00b8_f620, 0x010b_27a8, 0x010b_2784);
    }

    #[test]
    fn fn_00b8f6d0_destroys_the_shader() {
        check_delete(0x00b8_f6d0, 0x010b_2920, 0x010b_28fc);
    }

    #[test]
    fn fn_00b8f7a0_destroys_the_shader() {
        check_delete(0x00b8_f7a0, 0x010b_2a98, 0x010b_2a74);
    }

    #[test]
    fn fn_00b8f8f0_destroys_the_shader() {
        check_delete(0x00b8_f8f0, 0x010b_2c10, 0x010b_2bec);
    }

    #[test]
    fn fn_00b8fa40_destroys_the_shader() {
        check_delete(0x00b8_fa40, 0x010b_2db0, 0x010b_2d8c);
    }

    #[test]
    fn fn_00b8fba0_destroys_the_shader() {
        check_delete(0x00b8_fba0, 0x010b_2f50, 0x010b_2f30);
    }

    #[test]
    fn fn_00b8fd00_destroys_the_shader() {
        check_delete(0x00b8_fd00, 0x010b_30e0, 0x010b_30c0);
    }

    #[test]
    fn fn_00b8fe60_destroys_the_shader() {
        check_delete(0x00b8_fe60, 0x010b_3268, 0x010b_3244);
    }

    #[test]
    fn fn_00b8ffb0_destroys_the_shader() {
        check_delete(0x00b8_ffb0, 0x010b_33f0, 0x010b_33cc);
    }

    #[test]
    fn fn_00b90150_destroys_the_shader() {
        check_delete(0x00b9_0150, 0x010b_3598, 0x010b_3574);
    }

    #[test]
    fn fn_00b902f0_destroys_the_shader() {
        check_delete(0x00b9_02f0, 0x010b_37a8, 0x010b_3788);
    }

    #[test]
    fn fn_00b904e0_destroys_the_shader() {
        check_delete(0x00b9_04e0, 0x010b_3958, 0x010b_3934);
    }

    #[test]
    fn fn_00b90670_destroys_the_shader() {
        check_delete(0x00b9_0670, 0x010b_3af0, 0x010b_3ad0);
    }

    #[test]
    fn fn_00b907f0_destroys_the_shader() {
        check_delete(0x00b9_07f0, 0x010b_3c78, 0x010b_3c58);
    }

    #[test]
    fn fn_00b90980_destroys_the_shader() {
        check_delete(0x00b9_0980, 0x010b_3e18, 0x010b_3df4);
    }

    #[test]
    fn fn_00b90b10_destroys_the_shader() {
        check_delete(0x00b9_0b10, 0x010b_3fa8, 0x010b_3f88);
    }

    #[test]
    fn fn_00b90ca0_destroys_the_shader() {
        check_delete(0x00b9_0ca0, 0x010b_4138, 0x010b_4114);
    }

    #[test]
    fn fn_00b90e30_destroys_the_shader() {
        check_delete(0x00b9_0e30, 0x010b_42c8, 0x010b_42a4);
    }

    #[test]
    fn fn_00b90fc0_destroys_the_shader() {
        check_delete(0x00b9_0fc0, 0x010b_4460, 0x010b_443c);
    }

    #[test]
    fn fn_00b91160_destroys_the_shader() {
        check_delete(0x00b9_1160, 0x010b_4600, 0x010b_45dc);
    }

    #[test]
    fn fn_00b91300_destroys_the_shader() {
        check_delete(0x00b9_1300, 0x010b_47a0, 0x010b_477c);
    }

    #[test]
    fn fn_00b914a0_destroys_the_shader() {
        check_delete(0x00b9_14a0, 0x010b_4938, 0x010b_4914);
    }

    #[test]
    fn fn_00b91640_destroys_the_shader() {
        check_delete(0x00b9_1640, 0x010b_4ad0, 0x010b_4aac);
    }

    #[test]
    fn fn_00b91780_destroys_the_shader() {
        check_delete(0x00b9_1780, 0x010b_4c68, 0x010b_4c48);
    }

    #[test]
    fn fn_00b918c0_destroys_the_shader() {
        check_delete(0x00b9_18c0, 0x010b_4de8, 0x010b_4dc8);
    }

    #[test]
    fn fn_00b91a00_destroys_the_shader() {
        check_delete(0x00b9_1a00, 0x010b_4f80, 0x010b_4f60);
    }

    #[test]
    fn fn_00b91cb0_destroys_the_shader() {
        check_delete(0x00b9_1cb0, 0x010b_52c0, 0x010b_52a0);
    }

    #[test]
    fn fn_00b91e00_destroys_the_shader() {
        check_delete(0x00b9_1e00, 0x010b_5480, 0x010b_5460);
    }

    #[test]
    fn fn_00b91f50_destroys_the_shader() {
        check_delete(0x00b9_1f50, 0x010b_5640, 0x010b_5620);
    }

    #[test]
    fn fn_00b92290_destroys_the_shader() {
        check_delete(0x00b9_2290, 0x010b_57d0, 0x010b_57b0);
    }

    #[test]
    fn fn_00b92570_destroys_the_shader() {
        check_delete(0x00b9_2570, 0x010b_5b10, 0x010b_5af0);
    }

    #[test]
    fn fn_00b927a0_destroys_the_shader() {
        check_delete(0x00b9_27a0, 0x010b_5cb8, 0x010b_5c94);
    }

    #[test]
    fn fn_00b928f0_destroys_the_shader() {
        check_delete(0x00b9_28f0, 0x010b_5ea8, 0x010b_5e84);
    }

    #[test]
    fn fn_00b92a40_destroys_the_shader() {
        check_delete(0x00b9_2a40, 0x010b_6048, 0x010b_6028);
    }

    #[test]
    fn fn_00b92b90_destroys_the_shader() {
        check_delete(0x00b9_2b90, 0x010b_61f0, 0x010b_61d0);
    }

    #[test]
    fn fn_00b92ce0_destroys_the_shader() {
        check_delete(0x00b9_2ce0, 0x010b_6398, 0x010b_6374);
    }

    #[test]
    fn fn_00b92ea0_destroys_the_shader() {
        check_delete(0x00b9_2ea0, 0x010b_6548, 0x010b_6524);
    }

    #[test]
    fn fn_00b92ff0_destroys_the_shader() {
        check_delete(0x00b9_2ff0, 0x010b_66f0, 0x010b_66cc);
    }

    #[test]
    fn fn_00b93140_destroys_the_shader() {
        check_delete(0x00b9_3140, 0x010b_68a0, 0x010b_687c);
    }

    #[test]
    fn fn_00b93290_destroys_the_shader() {
        check_delete(0x00b9_3290, 0x010b_6a40, 0x010b_6a20);
    }

    #[test]
    fn fn_00b933e0_destroys_the_shader() {
        check_delete(0x00b9_33e0, 0x010b_6bd8, 0x010b_6bb8);
    }

    #[test]
    fn fn_00b93710_destroys_the_shader() {
        check_delete(0x00b9_3710, 0x010b_6d90, 0x010b_6d70);
    }

    #[test]
    fn fn_00b93a60_destroys_the_shader() {
        check_delete(0x00b9_3a60, 0x010b_6f38, 0x010b_6f14);
    }

    #[test]
    fn fn_00b93bb0_destroys_the_shader() {
        check_delete(0x00b9_3bb0, 0x010b_70d8, 0x010b_70b8);
    }

    #[test]
    fn fn_00b93d00_destroys_the_shader() {
        check_delete(0x00b9_3d00, 0x010b_7278, 0x010b_7258);
    }

    #[test]
    fn fn_00b93e50_destroys_the_shader() {
        check_delete(0x00b9_3e50, 0x010b_7418, 0x010b_73f4);
    }

    #[test]
    fn fn_00b93fa0_destroys_the_shader() {
        check_delete(0x00b9_3fa0, 0x010b_7598, 0x010b_7578);
    }

    #[test]
    fn fn_00b940f0_destroys_the_shader() {
        check_delete(0x00b9_40f0, 0x010b_7720, 0x010b_7700);
    }

    #[test]
    fn fn_00b94240_destroys_the_shader() {
        check_delete(0x00b9_4240, 0x010b_78b8, 0x010b_7898);
    }

    #[test]
    fn fn_00b94390_destroys_the_shader() {
        check_delete(0x00b9_4390, 0x010b_7a50, 0x010b_7a2c);
    }

    #[test]
    fn fn_00b944e0_destroys_the_shader() {
        check_delete(0x00b9_44e0, 0x010b_7bf0, 0x010b_7bcc);
    }

    #[test]
    fn fn_00b8bd80_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_bd80, 0x010a_ffc0, 0x010a_ffa0);
    }

    #[test]
    fn fn_00b8bfd0_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_bfd0, 0x010b_0198, 0x010b_0174);
    }

    #[test]
    fn fn_00b8d6f0_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_d6f0, 0x010b_06c0, 0x010b_06a0);
    }

    #[test]
    fn fn_00b8d840_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_d840, 0x010b_0860, 0x010b_083c);
    }

    #[test]
    fn fn_00b8d980_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_d980, 0x010b_0a00, 0x010b_09dc);
    }

    #[test]
    fn fn_00b8dad0_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_dad0, 0x010b_0b98, 0x010b_0b78);
    }

    #[test]
    fn fn_00b8dc20_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_dc20, 0x010b_0d40, 0x010b_0d1c);
    }

    #[test]
    fn fn_00b8dd70_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_dd70, 0x010b_0ec0, 0x010b_0e9c);
    }

    #[test]
    fn fn_00b8dec0_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_dec0, 0x010b_1040, 0x010b_101c);
    }

    #[test]
    fn fn_00b8e010_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_e010, 0x010b_11c0, 0x010b_119c);
    }

    #[test]
    fn fn_00b8e160_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_e160, 0x010b_1340, 0x010b_1320);
    }

    #[test]
    fn fn_00b8e2b0_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_e2b0, 0x010b_14c0, 0x010b_14a0);
    }

    #[test]
    fn fn_00b8e450_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_e450, 0x010b_1640, 0x010b_1620);
    }

    #[test]
    fn fn_00b8e5f0_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_e5f0, 0x010b_17f8, 0x010b_17d8);
    }

    #[test]
    fn fn_00b8e790_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_e790, 0x010b_1980, 0x010b_1960);
    }

    #[test]
    fn fn_00b8e930_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_e930, 0x010b_1b08, 0x010b_1ae8);
    }

    #[test]
    fn fn_00b8ead0_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_ead0, 0x010b_1c90, 0x010b_1c70);
    }

    #[test]
    fn fn_00b8ec70_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_ec70, 0x010b_1e20, 0x010b_1dfc);
    }

    #[test]
    fn fn_00b8ee10_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_ee10, 0x010b_1fb0, 0x010b_1f8c);
    }

    #[test]
    fn fn_00b8ef90_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_ef90, 0x010b_2140, 0x010b_211c);
    }

    #[test]
    fn fn_00b8f150_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_f150, 0x010b_2308, 0x010b_22e8);
    }

    #[test]
    fn fn_00b8f2e0_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_f2e0, 0x010b_2490, 0x010b_2470);
    }

    #[test]
    fn fn_00b8f4a0_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_f4a0, 0x010b_2610, 0x010b_25f0);
    }

    #[test]
    fn fn_00b8f650_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_f650, 0x010b_27a8, 0x010b_2784);
    }

    #[test]
    fn fn_00b8f700_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_f700, 0x010b_2920, 0x010b_28fc);
    }

    #[test]
    fn fn_00b8f7d0_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_f7d0, 0x010b_2a98, 0x010b_2a74);
    }

    #[test]
    fn fn_00b8f920_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_f920, 0x010b_2c10, 0x010b_2bec);
    }

    #[test]
    fn fn_00b8fa70_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_fa70, 0x010b_2db0, 0x010b_2d8c);
    }

    #[test]
    fn fn_00b8fbd0_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_fbd0, 0x010b_2f50, 0x010b_2f30);
    }

    #[test]
    fn fn_00b8fd30_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_fd30, 0x010b_30e0, 0x010b_30c0);
    }

    #[test]
    fn fn_00b8fe90_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_fe90, 0x010b_3268, 0x010b_3244);
    }

    #[test]
    fn fn_00b8ffe0_adjusts_to_the_shader() {
        check_effect_delete(0x00b8_ffe0, 0x010b_33f0, 0x010b_33cc);
    }

    #[test]
    fn fn_00b90180_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_0180, 0x010b_3598, 0x010b_3574);
    }

    #[test]
    fn fn_00b90320_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_0320, 0x010b_37a8, 0x010b_3788);
    }

    #[test]
    fn fn_00b90510_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_0510, 0x010b_3958, 0x010b_3934);
    }

    #[test]
    fn fn_00b906a0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_06a0, 0x010b_3af0, 0x010b_3ad0);
    }

    #[test]
    fn fn_00b90820_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_0820, 0x010b_3c78, 0x010b_3c58);
    }

    #[test]
    fn fn_00b909b0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_09b0, 0x010b_3e18, 0x010b_3df4);
    }

    #[test]
    fn fn_00b90b40_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_0b40, 0x010b_3fa8, 0x010b_3f88);
    }

    #[test]
    fn fn_00b90cd0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_0cd0, 0x010b_4138, 0x010b_4114);
    }

    #[test]
    fn fn_00b90e60_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_0e60, 0x010b_42c8, 0x010b_42a4);
    }

    #[test]
    fn fn_00b90ff0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_0ff0, 0x010b_4460, 0x010b_443c);
    }

    #[test]
    fn fn_00b91190_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_1190, 0x010b_4600, 0x010b_45dc);
    }

    #[test]
    fn fn_00b91330_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_1330, 0x010b_47a0, 0x010b_477c);
    }

    #[test]
    fn fn_00b914d0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_14d0, 0x010b_4938, 0x010b_4914);
    }

    #[test]
    fn fn_00b91670_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_1670, 0x010b_4ad0, 0x010b_4aac);
    }

    #[test]
    fn fn_00b917b0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_17b0, 0x010b_4c68, 0x010b_4c48);
    }

    #[test]
    fn fn_00b918f0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_18f0, 0x010b_4de8, 0x010b_4dc8);
    }

    #[test]
    fn fn_00b91a30_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_1a30, 0x010b_4f80, 0x010b_4f60);
    }

    #[test]
    fn fn_00b91ce0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_1ce0, 0x010b_52c0, 0x010b_52a0);
    }

    #[test]
    fn fn_00b91e30_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_1e30, 0x010b_5480, 0x010b_5460);
    }

    #[test]
    fn fn_00b91f80_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_1f80, 0x010b_5640, 0x010b_5620);
    }

    #[test]
    fn fn_00b922c0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_22c0, 0x010b_57d0, 0x010b_57b0);
    }

    #[test]
    fn fn_00b925a0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_25a0, 0x010b_5b10, 0x010b_5af0);
    }

    #[test]
    fn fn_00b927d0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_27d0, 0x010b_5cb8, 0x010b_5c94);
    }

    #[test]
    fn fn_00b92920_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_2920, 0x010b_5ea8, 0x010b_5e84);
    }

    #[test]
    fn fn_00b92a70_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_2a70, 0x010b_6048, 0x010b_6028);
    }

    #[test]
    fn fn_00b92bc0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_2bc0, 0x010b_61f0, 0x010b_61d0);
    }

    #[test]
    fn fn_00b92d10_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_2d10, 0x010b_6398, 0x010b_6374);
    }

    #[test]
    fn fn_00b92ed0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_2ed0, 0x010b_6548, 0x010b_6524);
    }

    #[test]
    fn fn_00b93020_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_3020, 0x010b_66f0, 0x010b_66cc);
    }

    #[test]
    fn fn_00b93170_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_3170, 0x010b_68a0, 0x010b_687c);
    }

    #[test]
    fn fn_00b932c0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_32c0, 0x010b_6a40, 0x010b_6a20);
    }

    #[test]
    fn fn_00b93410_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_3410, 0x010b_6bd8, 0x010b_6bb8);
    }

    #[test]
    fn fn_00b93740_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_3740, 0x010b_6d90, 0x010b_6d70);
    }

    #[test]
    fn fn_00b93a90_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_3a90, 0x010b_6f38, 0x010b_6f14);
    }

    #[test]
    fn fn_00b93be0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_3be0, 0x010b_70d8, 0x010b_70b8);
    }

    #[test]
    fn fn_00b93d30_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_3d30, 0x010b_7278, 0x010b_7258);
    }

    #[test]
    fn fn_00b93e80_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_3e80, 0x010b_7418, 0x010b_73f4);
    }

    #[test]
    fn fn_00b93fd0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_3fd0, 0x010b_7598, 0x010b_7578);
    }

    #[test]
    fn fn_00b94120_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_4120, 0x010b_7720, 0x010b_7700);
    }

    #[test]
    fn fn_00b94270_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_4270, 0x010b_78b8, 0x010b_7898);
    }

    #[test]
    fn fn_00b943c0_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_43c0, 0x010b_7a50, 0x010b_7a2c);
    }

    #[test]
    fn fn_00b94510_adjusts_to_the_shader() {
        check_effect_delete(0x00b9_4510, 0x010b_7bf0, 0x010b_7bcc);
    }
}
