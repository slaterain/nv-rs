//! `BSShader/bsshadermanager.cpp` (Xbox PDB source unit), subsystem `BSShader`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// CRT `sin` (`00eca0a0`): the argument is the x87 `ST0`, so a leading `f64`; the result is `ST0`.
const CRT_SIN: u32 = 0x00ec_a0a0;
/// CRT `cos` (`00ec9f70`), same convention.
const CRT_COS: u32 = 0x00ec_9f70;
/// The first `double` the argument is multiplied by.
const FIRST_FACTOR: u32 = 0x0101_1588;
/// The second `double` the argument is multiplied by.
const SECOND_FACTOR: u32 = 0x0102_3128;
/// The `double` angle the ratio is divided by `tan` of.
const REFERENCE_ANGLE: u32 = 0x010a_d6d8;
/// The `float` the result is stored in.
const RATIO_RESULT: u32 = 0x011a_dcc8;

// Translated from 00b54000 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (`bsshadermanager.cpp`). `cdecl`, one `float` stack word: scales it
/// by the doubles at `01011588` and `01023128` (stored as a `float`), takes
/// `sin / cos` of the result and of the double at `010ad6d8` (each `sin` and `cos` stored as
/// a `float` first) and stores the first quotient divided by the second in the `float` at
/// `011adcc8`. Computed on the x87 as the exe does (53-bit precision, NaN choice of the x87).
pub fn fn_00b54000(e: &mut Engine, value: f32) {
    let first_factor: f64 = e.global(FIRST_FACTOR);
    let second_factor: f64 = e.global(SECOND_FACTOR);
    let scaled = x87_store32(x87_mul(
        x87_mul(x87_load32(value.to_bits()), first_factor),
        second_factor,
    ));
    let angle = x87_load32(scaled);
    let sine = x87_quiet_bits(x87_store32(e.call(CRT_SIN, &args![angle]).f64()));
    let cosine = x87_store32(e.call(CRT_COS, &args![angle]).f64());
    let quotient = x87_quiet_bits(x87_store32(x87_div(x87_m32(sine), x87_load32(cosine))));

    let reference: f64 = e.global(REFERENCE_ANGLE);
    let reference_sine = x87_quiet_bits(x87_store32(e.call(CRT_SIN, &args![reference]).f64()));
    let reference_cosine = x87_store32(e.call(CRT_COS, &args![reference]).f64());
    let reference_quotient = x87_div(x87_m32(reference_sine), x87_load32(reference_cosine));
    let ratio = x87_div(x87_m32(quotient), reference_quotient);
    e.set_global(RATIO_RESULT, f32::from_bits(x87_store32(ratio)));
}

/// The table of shadow scene nodes, one word per index (`011f91c8`).
const SHADOW_SCENE_NODES: u32 = 0x011f_91c8;
/// The word `BSShaderManager::GetAccumulator` answers (`011f95ec`).
const ACCUMULATOR: u32 = 0x011f_95ec;
/// `ShadowSceneNode::SetFogProperty` (engine map, `00b54110`): `thiscall`, `RET 4`.
const SHADOW_SCENE_NODE_SET_FOG_PROPERTY: u32 = 0x00b5_4110;

/// The memory manager singleton (`011f6238`) and its allocate (`00aa3e40`, `thiscall`,
/// size) and deallocate (`00aa4060`, `thiscall`, block).
const MEMORY_MANAGER: u32 = 0x011f_6238;
const MEMORY_ALLOCATE: u32 = 0x00aa_3e40;
const MEMORY_DEALLOCATE: u32 = 0x00aa_4060;
/// The allocation `00aa13e0` (`cdecl`, size).
const NEW_OBJECT: u32 = 0x00aa_13e0;
/// Imported `InterlockedIncrement` / `InterlockedDecrement` (import slots).
const IMPORT_INTERLOCKED_INCREMENT: u32 = 0x00fd_f1c0;
const IMPORT_INTERLOCKED_DECREMENT: u32 = 0x00fd_f1c4;
/// The thread block's word at `+0x2b4` (the allocation context), set to 5 during the set-up.
const TLS_CONTEXT: u32 = 0x2b4;
/// The set-up's first-run byte (`011f91c6`).
const SET_UP_DONE_011F91C6: u32 = 0x011f_91c6;
/// The objects the first run creates and the globals it reads.
const OBJECT_011F91A8: u32 = 0x011f_91a8;
const OBJECT_011F9498: u32 = 0x011f_9498;
const OBJECT_011F91AC: u32 = 0x011f_91ac;
const HOLDER_011F94BC: u32 = 0x011f_94bc;
const GLOBAL_011F9508: u32 = 0x011f_9508;
/// The four floats set to (0.5, 0.5, 0.5, 1.0) on the first run, and the `float` 0.5 the
/// code loads (`01016248`).
const FLOATS_011FA090: u32 = 0x011f_a090;
const HALF_01016248: u32 = 0x0101_6248;
/// The string the first run's holder is made from (`010ada3c`, passed to `00a5b690`).
const HOLDER_NAME_010ADA3C: u32 = 0x010a_da3c;
/// The accumulator array: count (`011f916c`) and pointer (`011f9170`); the word cleared at
/// the end (`011f9168`); the word copied (`011f94f0` into `011a9608`).
const ACCUMULATOR_COUNT_011F916C: u32 = 0x011f_916c;
const ACCUMULATOR_ARRAY_011F9170: u32 = 0x011f_9170;
const GLOBAL_011F9168: u32 = 0x011f_9168;
const GLOBAL_011F94F0: u32 = 0x011f_94f0;
const GLOBAL_011A9608: u32 = 0x011a_9608;
const GLOBAL_011FFE2C: u32 = 0x011f_fe2c;
const GLOBAL_011FFE30: u32 = 0x011f_fe30;

/// The function-pointer words `00b57420` clears, in the exe's order.
const CLEARED_WORDS_00B57420: [u32; 32] = [
    0x011f_9f40,
    0x011f_9f68,
    0x011f_9f70,
    0x011f_9f74,
    0x011f_9f78,
    0x011f_9f80,
    0x011f_9fa0,
    0x011f_9fa4,
    0x011f_9fa8,
    0x011f_9fac,
    0x011f_9fb4,
    0x011f_9fb8,
    0x011f_9f44,
    0x011f_9f48,
    0x011f_9f4c,
    0x011f_9f50,
    0x011f_9f54,
    0x011f_9f58,
    0x011f_9f5c,
    0x011f_9f60,
    0x011f_9f64,
    0x011f_9f6c,
    0x011f_9f7c,
    0x011f_9f84,
    0x011f_9f88,
    0x011f_9f8c,
    0x011f_9f90,
    0x011f_9f94,
    0x011f_9f98,
    0x011f_9f9c,
    0x011f_9fb0,
    0x011f_9fbc,
];

/// The (word, code address) stores `00b57420` makes after clearing, in the exe's order
/// (immediates of its `MOV` instructions).
const FUNCTION_WORDS_00B57420: [(u32, u32); 14] = [
    (0x011f_9f80, 0x00b6_4020),
    (0x011f_9f3c, 0x00b6_4020),
    (0x011f_9fa8, 0x00b6_5a90),
    (0x011f_9fac, 0x00b6_06e0),
    (0x011f_9fb4, 0x00b6_3f90),
    (0x011f_9fa4, 0x00b6_4440),
    (0x011f_9fb8, 0x00b6_44b0),
    (0x011f_9fa0, 0x00b6_4490),
    (0x011f_9f40, 0x00b6_65a0),
    (0x011f_9f70, 0x00b6_5dc0),
    (0x011f_9f68, 0x00b6_5e80),
    (0x011f_9f74, 0x00a2_9680),
    (0x011f_9f78, 0x00b6_4530),
    (0x011f_9f38, 0x00b6_65a0),
];

// Translated from 0070b760 (decompiled, FalloutNV.exe 1.4.0.525)
/// Named `BSShaderManager::SetShadowSceneNode` in the engine map (Xbox PDB), but the body is a
/// `thiscall` on the node itself (`RET 4`): it stores the index byte at `+0x120` of `this` and
/// then puts `this` into the shadow-scene-node table at that index (`00b4f2f0`).
pub fn fn_0070b760(e: &mut Engine, this: Ptr, index: u8) {
    e.mem.set_u8(this.addr().wrapping_add(0x120), index);
    fn_00b4f2f0(e, u32::from(index), this);
}

// Translated from 00b4f2f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (`bsshadermanager.cpp`). `cdecl` (index, node): stores the node in
/// the shadow-scene-node table at `011f91c8 + 4 * index` (no bounds check).
pub fn fn_00b4f2f0(e: &mut Engine, index: u32, node: Ptr) {
    e.mem.set_u32(
        SHADOW_SCENE_NODES.wrapping_add(index.wrapping_mul(4)),
        node.addr(),
    );
}

// Translated from 00b4f5c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSShaderManager::GetAccumulator` (Xbox PDB): the word at `011f95ec`.
pub fn bs_shader_manager_get_accumulator(e: &mut Engine) -> Ptr {
    Ptr::new(e.global::<u32>(ACCUMULATOR))
}

// Translated from 00b55540 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSShaderManager::SetFogProperty` (Xbox PDB). `cdecl` (index, fog property): calls
/// `ShadowSceneNode::SetFogProperty` (`00b54110`) on the shadow scene node at that index of the
/// table (`011f91c8`) with the fog property.
pub fn bs_shader_manager_set_fog_property(e: &mut Engine, index: u32, fog: Ptr) {
    let node = e
        .mem
        .u32(SHADOW_SCENE_NODES.wrapping_add(index.wrapping_mul(4)));
    e.call(SHADOW_SCENE_NODE_SET_FOG_PROPERTY, &args![node, fog]);
}

/// `MemoryManager::Allocate` on the singleton.
fn allocate(e: &mut Engine, size: u32) -> u32 {
    e.call(MEMORY_ALLOCATE, &args![MEMORY_MANAGER, size]).u32()
}

/// The first-run object at `011f9498` released: `00b7fd50` on it, then the memory manager
/// frees it and the global is cleared.
fn release_object_011f9498(e: &mut Engine, object: u32) {
    e.call(0x00b7_fd50, &args![object]);
    e.call(MEMORY_DEALLOCATE, &args![MEMORY_MANAGER, object]);
    e.set_global(OBJECT_011F9498, 0u32);
}

// Translated from 00b57420 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (`bsshadermanager.cpp`); no arguments. The shader manager's
/// set-up, with the thread block's allocation context (`+0x2b4`) at 5 and restored at the end:
///
/// * On the first run (byte `011f91c6` clear): makes the 0xb0-byte object at `011f91a8`
///   (`00b6dd80`) and the 0x1c-byte one at `011f9498` (`00b7ff40` with `00b7faf0`'s value),
///   runs `00b7fe40` on the latter with its first word (read even when the allocation
///   failed) and releases it (`00b7fd50`, free) when its word at `+4` is 0; runs `00b55bb0`;
///   sets the floats at `011fa090` to 0.5, 0.5, 0.5 (the `float` at `01016248`) and 1.0; when
///   `011f9508` is not 0, gets shaders 1 to 0x22 (`BSShaderManager::GetShader`, `00b55560`),
///   makes the 0x31c-byte object at `011f91ac` (`00b97a80`) and releases `011f9498` if it is
///   still there; registers the streamables (`00b4f1a0`), runs `00b6e450` on `011f91a8` with
///   `011f9508`, makes a one-word holder of `00a5b690(010ada3c)` at `011f94bc` and sets the
///   first-run byte.
/// * Every run: clears 32 function-pointer words of the `011f9f38` block and stores 14 code
///   addresses into it, clears `011ffe2c` and `011ffe30`, runs `MTRenderingSystem::CreateThreads`
///   (`00ba3720`) and `00b97c10`, copies `011f94f0` to `011a9608`, then makes the array of
///   `011f916c` accumulator pointers (a count word before the elements, the elements
///   constructed by `00ec782f` with `00b54620`/`0045cec0`) at `011f9170` and fills each slot
///   with a new `BSShaderAccumulator` (`00b660d0(0x63, 1, 0x2f7)` on 0x280 bytes) through the
///   `NiPointer` assignment (the old one released, deleted through its vtable `+4` when its
///   count reaches 0), and clears `011f9168`.
///
/// The `ECX` the exe loads before `CreateThreads` (`01200088`) is not read by it.
// Translation incomplete: 00b57420 - the C++ exception frame (FS:[0] chain, unwind states 1 to 6) is not translated.
pub fn fn_00b57420(e: &mut Engine) {
    let tls = e.tls();
    let saved_context = e.mem.u32(tls.wrapping_add(TLS_CONTEXT));
    e.mem.set_u32(tls.wrapping_add(TLS_CONTEXT), 5);

    if e.global::<u8>(SET_UP_DONE_011F91C6) == 0 {
        let memory = allocate(e, 0xb0);
        let object = if memory != 0 {
            e.call(0x00b6_dd80, &args![memory]).u32()
        } else {
            0
        };
        e.set_global(OBJECT_011F91A8, object);

        let memory = allocate(e, 0x1c);
        let object = if memory != 0 {
            let value = e.call(0x00b7_faf0, &args![]).u32();
            e.call(0x00b7_ff40, &args![memory, value]).u32()
        } else {
            0
        };
        e.set_global(OBJECT_011F9498, object);
        let first = e.mem.u32(object);
        e.call(0x00b7_fe40, &args![object, first]);
        let object = e.global::<u32>(OBJECT_011F9498);
        if e.mem.u32(object.wrapping_add(4)) == 0 {
            release_object_011f9498(e, object);
        }

        e.call(0x00b5_5bb0, &args![]);
        let half = x87_quiet_bits(e.global::<u32>(HALF_01016248));
        e.set_global(FLOATS_011FA090, half);
        e.set_global(FLOATS_011FA090 + 4, half);
        e.set_global(FLOATS_011FA090 + 8, half);
        e.set_global(FLOATS_011FA090 + 0xc, 1.0f32.to_bits());

        if e.global::<u32>(GLOBAL_011F9508) != 0 {
            for shader in 1u32..0x23 {
                e.call(0x00b5_5560, &args![shader]);
            }
            let memory = allocate(e, 0x31c);
            let object = if memory != 0 {
                e.call(0x00b9_7a80, &args![memory]).u32()
            } else {
                0
            };
            e.set_global(OBJECT_011F91AC, object);
            let object = e.global::<u32>(OBJECT_011F9498);
            if object != 0 {
                release_object_011f9498(e, object);
            }
        }

        e.call(0x00b4_f1a0, &args![]);
        let word = e.global::<u32>(GLOBAL_011F9508);
        let object = e.global::<u32>(OBJECT_011F91A8);
        e.call(0x00b6_e450, &args![object, word]);

        let holder = allocate(e, 4);
        let holder = if holder != 0 {
            let text = e.call(0x00a5_b690, &args![HOLDER_NAME_010ADA3C]).u32();
            e.mem.set_u32(holder, text);
            holder
        } else {
            0
        };
        e.set_global(HOLDER_011F94BC, holder);
        e.set_global(SET_UP_DONE_011F91C6, 1u8);
    }

    for word in CLEARED_WORDS_00B57420 {
        e.set_global(word, 0u32);
    }
    for (word, code) in FUNCTION_WORDS_00B57420 {
        e.set_global(word, code);
    }
    e.set_global(GLOBAL_011FFE2C, 0u32);
    e.set_global(GLOBAL_011FFE30, 0u32);
    e.call(0x00ba_3720, &args![]);
    e.call(0x00b9_7c10, &args![]);
    let copied = e.global::<u32>(GLOBAL_011F94F0);
    let count = e.global::<u32>(ACCUMULATOR_COUNT_011F916C);
    e.set_global(GLOBAL_011A9608, copied);

    // `count * 4 + 4` for the count word, all ones on overflow at either step
    // (`MUL; SETO; NEG; OR` then `ADD; SETC; NEG; OR`).
    let size = count
        .checked_mul(4)
        .unwrap_or(u32::MAX)
        .checked_add(4)
        .unwrap_or(u32::MAX);
    let memory = allocate(e, size);
    let array = if memory != 0 {
        e.mem.set_u32(memory, count);
        let elements = memory.wrapping_add(4);
        e.call(
            0x00ec_782f,
            &args![elements, 4u32, count, 0x00b5_4620u32, 0x0045_cec0u32],
        );
        elements
    } else {
        0
    };
    e.set_global(ACCUMULATOR_ARRAY_011F9170, array);

    let mut index = 0u32;
    if e.global::<u32>(ACCUMULATOR_COUNT_011F916C) != 0 {
        loop {
            let memory = e.call(NEW_OBJECT, &args![0x280u32]).u32();
            let accumulator = if memory != 0 {
                e.call(0x00b6_60d0, &args![memory, 0x63u32, 1u32, 0x2f7u32])
                    .u32()
            } else {
                0
            };
            let array = e.global::<u32>(ACCUMULATOR_ARRAY_011F9170);
            let slot = array.wrapping_add(index.wrapping_mul(4));
            let old = e.mem.u32(slot);
            if old != accumulator {
                if old != 0 {
                    let left = e
                        .call(IMPORT_INTERLOCKED_DECREMENT, &args![old.wrapping_add(4)])
                        .u32();
                    if left == 0 {
                        e.vcall(old, 4, &args![]);
                    }
                }
                e.mem.set_u32(slot, accumulator);
                if accumulator != 0 {
                    e.call(
                        IMPORT_INTERLOCKED_INCREMENT,
                        &args![accumulator.wrapping_add(4)],
                    );
                }
            }
            index = index.wrapping_add(1);
            if index >= e.global::<u32>(ACCUMULATOR_COUNT_011F916C) {
                break;
            }
        }
    }
    e.mem.set_u32(tls.wrapping_add(TLS_CONTEXT), saved_context);
    e.set_global(GLOBAL_011F9168, 0u32);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00b5_4000, fn_00b54000(f32)),
        entry!(0x0070_b760, fn_0070b760(Ptr, u8)),
        entry!(0x00b4_f2f0, fn_00b4f2f0(u32, Ptr)),
        entry!(0x00b4_f5c0, bs_shader_manager_get_accumulator() -> Ptr),
        entry!(0x00b5_5540, bs_shader_manager_set_fog_property(u32, Ptr)),
        entry!(0x00b5_7420, fn_00b57420()),
    ]
}

#[cfg(test)]
mod physics_work_ahead_tests {
    use super::*;

    fn rig() -> Engine {
        let mut e = Engine::new();
        e.call_log = Some(vec![]);
        e.map(FIRST_FACTOR, 8);
        e.map(SECOND_FACTOR, 8);
        e.map(REFERENCE_ANGLE, 8);
        e.map(RATIO_RESULT, 4);
        e.set_global(FIRST_FACTOR, 0.5f64);
        e.set_global(SECOND_FACTOR, 2.0f64);
        e.set_global(REFERENCE_ANGLE, 3.0f64);
        // sin(x) = x + 1, cos(x) = 2: the quotients are exact in binary.
        e.register(CRT_SIN, |_, a| {
            let x = f64::from_bits(u64::from(a[0]) | u64::from(a[1]) << 32);
            Ret {
                st0: x + 1.0,
                ..Ret::default()
            }
        });
        e.register(CRT_COS, |_, _| Ret {
            st0: 2.0,
            ..Ret::default()
        });
        e
    }

    #[test]
    fn at_00b54000_divides_tan_of_the_scaled_angle_by_tan_of_the_reference() {
        let mut e = rig();
        // 0.5 * 0.5 * 2.0 = 0.5; (0.5 + 1) / 2 = 0.75; (3 + 1) / 2 = 2; 0.75 / 2 = 0.375.
        e.call(0x00b5_4000, &args![0.5f32]);
        assert_eq!(e.global::<f32>(RATIO_RESULT), 0.375);
        let log = e.call_log.take().unwrap();
        let sines: Vec<&Vec<u32>> = log
            .iter()
            .filter(|c| c.0 == CRT_SIN)
            .map(|c| &c.1)
            .collect();
        let half = 0.5f64.to_bits();
        let three = 3.0f64.to_bits();
        assert_eq!(
            sines,
            vec![
                &vec![half as u32, (half >> 32) as u32],
                &vec![three as u32, (three >> 32) as u32]
            ]
        );
    }

    #[test]
    fn at_00b54000_rounds_each_stage_to_a_float() {
        let mut e = rig();
        // The scaled angle (0.2 * 0.5 * 1.0) is a float before sin sees it.
        e.set_global(SECOND_FACTOR, 1.0f64);
        e.call(0x00b5_4000, &args![0.2f32]);
        let scaled = f64::from(0.2f32 / 2.0);
        let sine = f64::from((scaled + 1.0) as f32);
        let quotient = f64::from((sine / 2.0) as f32);
        let expected = (quotient / 2.0) as f32;
        assert_eq!(e.global::<f32>(RATIO_RESULT), expected);
    }

    #[test]
    fn at_00b54000_gives_a_nan_for_a_nan_argument() {
        let mut e = rig();
        e.call(0x00b5_4000, &args![f32::from_bits(0x7f80_0001)]);
        assert!(e.global::<f32>(RATIO_RESULT).is_nan());
    }
}

#[cfg(test)]
mod b000_tests {
    use super::*;

    fn calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, w)| w.clone())
            .collect()
    }

    #[test]
    fn fn_0070b760_stores_the_index_and_fills_the_table() {
        let mut e = Engine::new();
        e.map(SHADOW_SCENE_NODES, 0x40);
        let node = e.mem.alloc(0x130);
        e.call(0x0070_b760, &args![node, 0xffff_ff03u32]);
        assert_eq!(e.mem.u8(node + 0x120), 3);
        assert_eq!(e.mem.u32(SHADOW_SCENE_NODES + 12), node);
    }

    #[test]
    fn fn_00b4f2f0_stores_the_node_at_its_index() {
        let mut e = Engine::new();
        e.map(SHADOW_SCENE_NODES, 0x40);
        e.call(0x00b4_f2f0, &args![2u32, 0x1234_5678u32]);
        assert_eq!(e.mem.u32(SHADOW_SCENE_NODES + 8), 0x1234_5678);
        assert_eq!(e.mem.u32(SHADOW_SCENE_NODES + 4), 0);
    }

    #[test]
    fn get_accumulator_reads_its_global() {
        let mut e = Engine::new();
        e.map(ACCUMULATOR, 4);
        e.set_global(ACCUMULATOR, 0x0bad_f00du32);
        assert_eq!(e.call(0x00b4_f5c0, &args![]).u32(), 0x0bad_f00d);
    }

    #[test]
    fn set_fog_property_calls_the_node_of_the_index() {
        let mut e = Engine::new();
        e.map(SHADOW_SCENE_NODES, 0x40);
        e.mem.set_u32(SHADOW_SCENE_NODES + 4, 0x5000);
        e.register(SHADOW_SCENE_NODE_SET_FOG_PROPERTY, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x00b5_5540, &args![1u32, 0x6000u32]);
        assert_eq!(
            calls(&e, SHADOW_SCENE_NODE_SET_FOG_PROPERTY),
            vec![vec![0x5000, 0x6000]]
        );
    }

    /// Doubles for every callee of `00b57420`; constructors answer `this`. The vector
    /// constructor fills the slots with `old` (0 for none).
    fn set_up_rig(old: u32) -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_6000,
            0x010a_d000,
            0x011a_9000,
            0x011f_9000,
            0x011f_a000,
            0x011f_f000,
        ] {
            e.map(page, 0x1000);
        }
        e.set_global(HALF_01016248, 0.5f32);
        e.register(MEMORY_ALLOCATE, |e, a| e.mem.alloc(a[1].max(8)).into_ret());
        e.register(NEW_OBJECT, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(MEMORY_DEALLOCATE, |_, _| Ret::default());
        for this_returning in [0x00b6_dd80u32, 0x00b9_7a80, 0x00b6_60d0] {
            e.register(this_returning, |_, a| a[0].into_ret());
        }
        // `00b7ff40` keeps `this`, with its word at +4 as 1 (kept by the first check).
        e.register(0x00b7_ff40, |e, a| {
            e.mem.set_u32(a[0] + 4, 1);
            a[0].into_ret()
        });
        e.register(0x00b7_faf0, |_, _| 7u32.into_ret());
        e.register(0x00a5_b690, |_, _| 0x0777_0000u32.into_ret());
        for plain in [
            0x00b7_fe40u32,
            0x00b7_fd50,
            0x00b5_5bb0,
            0x00b5_5560,
            0x00b4_f1a0,
            0x00b6_e450,
            0x00ba_3720,
            0x00b9_7c10,
        ] {
            e.register(plain, |_, _| Ret::default());
        }
        e.register_double(0x00ec_782f, move |e, a| {
            for i in 0..a[2] {
                e.mem.set_u32(a[0] + 4 * i, old);
            }
            Ret::default()
        });
        e.register(IMPORT_INTERLOCKED_INCREMENT, |e, a| {
            let v = e.mem.u32(a[0]).wrapping_add(1);
            e.mem.set_u32(a[0], v);
            v.into_ret()
        });
        e.register(IMPORT_INTERLOCKED_DECREMENT, |e, a| {
            let v = e.mem.u32(a[0]).wrapping_sub(1);
            e.mem.set_u32(a[0], v);
            v.into_ret()
        });
        e
    }

    #[test]
    fn fn_00b57420_first_run_builds_the_manager_and_the_accumulators() {
        let mut e = set_up_rig(0);
        e.set_global(GLOBAL_011F9508, 1u32);
        e.set_global(ACCUMULATOR_COUNT_011F916C, 2u32);
        e.set_global(GLOBAL_011F94F0, 0x4444u32);
        let tls = e.tls();
        e.mem.set_u32(tls + TLS_CONTEXT, 9);
        e.register_double(0x00b5_5bb0, move |e, _| {
            // The context is 5 while the set-up runs.
            assert_eq!(e.mem.u32(tls + TLS_CONTEXT), 5);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(0x00b5_7420, &args![]);

        let shaders: Vec<u32> = calls(&e, 0x00b5_5560).iter().map(|w| w[0]).collect();
        assert_eq!(shaders, (1..0x23).collect::<Vec<u32>>());
        // Kept by the first check, released after the shaders.
        assert_eq!(calls(&e, 0x00b7_fd50).len(), 1);
        assert_eq!(e.global::<u32>(OBJECT_011F9498), 0);
        assert_ne!(e.global::<u32>(OBJECT_011F91A8), 0);
        assert_ne!(e.global::<u32>(OBJECT_011F91AC), 0);
        let first = e.global::<u32>(OBJECT_011F91A8);
        assert_eq!(calls(&e, 0x00b6_e450), vec![vec![first, 1]]);
        assert_eq!(calls(&e, 0x00a5_b690), vec![vec![HOLDER_NAME_010ADA3C]]);
        let holder = e.global::<u32>(HOLDER_011F94BC);
        assert_eq!(e.mem.u32(holder), 0x0777_0000);
        assert_eq!(e.global::<u8>(SET_UP_DONE_011F91C6), 1);
        for (i, want) in [0.5f32, 0.5, 0.5, 1.0].iter().enumerate() {
            assert_eq!(e.global::<f32>(FLOATS_011FA090 + 4 * i as u32), *want);
        }
        for (word, code) in FUNCTION_WORDS_00B57420 {
            assert_eq!(e.global::<u32>(word), code);
        }
        assert_eq!(e.global::<u32>(0x011f_9f44), 0);
        assert_eq!(e.global::<u32>(GLOBAL_011A9608), 0x4444);

        // Two accumulators, each held once, after the count word.
        let array = e.global::<u32>(ACCUMULATOR_ARRAY_011F9170);
        assert_eq!(e.mem.u32(array - 4), 2);
        let sizes: Vec<u32> = calls(&e, MEMORY_ALLOCATE).iter().map(|w| w[1]).collect();
        assert_eq!(sizes, vec![0xb0, 0x1c, 0x31c, 4, 12]);
        let vector = calls(&e, 0x00ec_782f);
        assert_eq!(vector, vec![vec![array, 4, 2, 0x00b5_4620, 0x0045_cec0]]);
        let accumulators = calls(&e, 0x00b6_60d0);
        assert_eq!(accumulators.len(), 2);
        assert_eq!(&accumulators[0][1..], &[0x63, 1, 0x2f7]);
        for (i, call) in accumulators.iter().enumerate() {
            let accumulator = e.mem.u32(array + 4 * i as u32);
            assert_eq!(accumulator, call[0]);
            assert_eq!(e.mem.u32(accumulator + 4), 1);
        }
        assert_eq!(e.mem.u32(tls + TLS_CONTEXT), 9);
    }

    #[test]
    fn fn_00b57420_first_run_without_shaders_releases_an_empty_object_at_once() {
        let mut e = set_up_rig(0);
        e.register(0x00b7_ff40, |_, a| a[0].into_ret());
        e.call_log = Some(vec![]);
        e.call(0x00b5_7420, &args![]);
        assert!(calls(&e, 0x00b5_5560).is_empty());
        assert!(calls(&e, 0x00b9_7a80).is_empty());
        assert_eq!(calls(&e, 0x00b7_fd50).len(), 1);
        assert_eq!(calls(&e, MEMORY_DEALLOCATE).len(), 1);
        assert_eq!(e.global::<u32>(OBJECT_011F9498), 0);
        // No accumulators: the array has only its count word.
        assert!(calls(&e, NEW_OBJECT).is_empty());
        let array = e.global::<u32>(ACCUMULATOR_ARRAY_011F9170);
        assert_eq!(e.mem.u32(array - 4), 0);
    }

    #[test]
    fn fn_00b57420_later_runs_skip_the_set_up_and_release_old_slots() {
        let mut e = set_up_rig(0);
        let old = e.mem.alloc(8);
        e.put_vtable(0x0f00_1000, &[0, 0x0f00_2000]);
        e.mem.set_u32(old, 0x0f00_1000);
        e.mem.set_u32(old + 4, 1);
        e.register(0x0f00_2000, |_, _| Ret::default());
        e.register_double(0x00ec_782f, move |e, a| {
            for i in 0..a[2] {
                e.mem.set_u32(a[0] + 4 * i, old);
            }
            Ret::default()
        });
        e.set_global(SET_UP_DONE_011F91C6, 1u8);
        e.set_global(ACCUMULATOR_COUNT_011F916C, 1u32);
        e.call_log = Some(vec![]);
        e.call(0x00b5_7420, &args![]);
        assert!(calls(&e, 0x00b6_dd80).is_empty());
        assert!(calls(&e, 0x00b4_f1a0).is_empty());
        // The old pointer's count went to 0: deleted through its vtable +4.
        assert_eq!(calls(&e, 0x0f00_2000), vec![vec![old]]);
        let array = e.global::<u32>(ACCUMULATOR_ARRAY_011F9170);
        assert_ne!(e.mem.u32(array), old);
        assert_eq!(calls(&e, 0x00ba_3720).len(), 1);
        assert_eq!(e.global::<u32>(GLOBAL_011F9168), 0);
    }
}
