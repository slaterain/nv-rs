//! The geometry morpher (`NiGeomMorpherController`): its update, which blends the morph targets into
//! the target geometry's position array each time the controller runs (`GenMorphInterp`), the
//! position array it blends into (`GetVertexData`, which makes one when the data has none) and the
//! weighted add of one target (`00a7ec90`).

use super::data;
use super::layout::{GeometryData, MorphData, MorpherController};
use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::x87::{x87_add, x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32};
use engine_core::{args, typed_call, Engine};

/// A target geometry's data (`NiGeometry::m_spModelData`, Xbox PDB, +0xb8).
const MODEL_DATA: u32 = 0xb8;
/// An interpolator item: 8 bytes, the interpolator then the weight.
const ITEM_SIZE: u32 = 8;
const ITEM_WEIGHT: u32 = 4;
/// A morph target: 16 bytes, its vertices first.
const TARGET_SIZE: u32 = 0x10;

/// The weight thresholds a target is added above or below (`01017d00`, `0104ffb8`), the scaled time
/// of a forced update (`01096324`), and the update task switches (`011f3725`, a byte; the task
/// owner `011f5b04`).
const WEIGHT_ABOVE: u32 = 0x0101_7d00;
const WEIGHT_BELOW: u32 = 0x0104_ffb8;
const FORCED_UPDATE_TIME: u32 = 0x0109_6324;
const UPDATE_TASKS_ENABLED: u32 = 0x011f_3725;
const UPDATE_TASK_OWNER: u32 = 0x011f_5b04;

/// The memory manager (`011f6238`) and its allocate (`00aa3e40`, `thiscall`, the size), `memset`
/// (`00ec61c0`), `NiGeometryData::MarkAsChanged` (`00a67090`, `thiscall`, the flags),
/// `DontDoUpdate` (`00a36250`, `thiscall`, the time; a byte), the update task checks and calls
/// (`00714a00`, no words, a byte; `00a56f90`, no words, a task or null; `00a56c60`, `thiscall` on the
/// task: the controller and the target; `00a3a730`, `thiscall` on the owner: the task and 2, a byte).
const MEMORY_MANAGER: u32 = 0x011f_6238;
const ALLOCATE: u32 = 0x00aa_3e40;
const MEMSET: u32 = 0x00ec_61c0;
const MARK_AS_CHANGED: u32 = 0x00a6_7090;
const DONT_DO_UPDATE: u32 = 0x00a3_6250;
const UPDATE_TASKS_ALLOWED: u32 = 0x0071_4a00;
const NEW_UPDATE_TASK: u32 = 0x00a5_6f90;
const UPDATE_TASK_SET: u32 = 0x00a5_6c60;
const UPDATE_TASK_QUEUE: u32 = 0x00a3_a730;

/// Slots: on the controller 0xc4 (the interpolator of an item) and 0xe8 (the position array:
/// `GetVertexData`); on an interpolator 0x9c (its float at a time: the time, the target and the
/// address of the float); on the target 0x18; on an update task 0x94 (run it now).
const SLOT_INTERPOLATOR: u32 = 0xc4;
const SLOT_VERTEX_DATA: u32 = 0xe8;
const SLOT_UPDATE_FLOAT: u32 = 0x9c;
const SLOT_TARGET_CHECK: u32 = 0x18;
const SLOT_TASK_RUN: u32 = 0x94;

// Typed from 00a7ec90
/// The weighted add of a morph target (unnamed in the engine map; `cdecl`): `dest[i] += source[i] *
/// scale` for each of `count` three-float points, each product stored as a float before the sums. The
/// exe unrolls the loop by four while at least four points remain (a signed count), then finishes one
/// by one; the unrolled and the single-point code put the operands of some `FADD`s and the tail's first
/// `FMUL` in another order, which only decides which of two NaNs comes out, and is kept.
pub fn add_scaled_points(e: &mut Engine, dest: Ptr, scale: f32, source: Ptr, count: u32) {
    // `true`: `FLD dest; FADD product`; `false`: `FLD product; FADD dest`.
    const UNROLLED_ORDER: [[bool; 3]; 4] = [
        [true, true, true],
        [true, false, true],
        [false, true, true],
        [true, true, false],
    ];
    const TAIL_ORDER: [bool; 3] = [false, true, true];
    let scale = x87_load32(scale.to_bits());
    let unrolled = if count as i32 >= 4 {
        ((count - 4) >> 2).wrapping_add(1).wrapping_mul(4)
    } else {
        0
    };
    let mut index = 0u32;
    while index < count {
        let from = source.addr().wrapping_add(index.wrapping_mul(12));
        let to = dest.addr().wrapping_add(index.wrapping_mul(12));
        let in_tail = index >= unrolled;
        let products = [0u32, 1, 2].map(|k| {
            let component = e.mem.u32(from.wrapping_add(4 * k));
            let value = if in_tail && k == 0 {
                x87_mul(scale, x87_m32(component))
            } else {
                x87_mul(x87_load32(component), scale)
            };
            x87_store32(value)
        });
        let order = if in_tail {
            TAIL_ORDER
        } else {
            UNROLLED_ORDER[(index % 4) as usize]
        };
        for (k, (product, dest_first)) in (0u32..).zip(products.iter().zip(order)) {
            let at = to.wrapping_add(4 * k);
            let current = e.mem.u32(at);
            let sum = if dest_first {
                x87_add(x87_load32(current), x87_m32(*product))
            } else {
                x87_add(x87_load32(*product), x87_m32(current))
            };
            e.mem.set_u32(at, x87_store32(sum));
        }
        index += 1;
    }
}

// Typed from 00a3aa10
/// `NiGeomMorpherController::GenMorphInterp` (Xbox PDB; `thiscall`, a word it does not read): each
/// item's weight is brought up to date, then the targets are blended into the position array (slot
/// 0xe8: [`vertex_data`] for the controller's own class). For each target: the first one of relative
/// morph data gets 0.0 when the base target is ignored and 1.0 otherwise; the others ask their
/// interpolator (slot 0xc4 gives it; its slot 0x9c with the scaled time, the target and the item's
/// current weight in a stack word) and keep the old weight when it answers false or there is none.
/// Then the array is cleared (unless the base target is ignored) and every target whose weight is at
/// least the float at `01017d00` or at most the one at `0104ffb8` is added ([`add_scaled_points`]); if
/// any was and the target's slot 0x18 answers non-zero, its geometry data is marked changed
/// (`MarkAsChanged(1)`).
pub fn gen_morph_interp(e: &mut Engine, this: Ptr, _unused: u32) {
    let controller = this.cast::<MorpherController>();
    let base = this.addr();
    let buffer = if crate::slot_target(e, base, SLOT_VERTEX_DATA) == 0x00a3_b010 {
        typed_call!(FAMILY, 0x00a3_b010, vertex_data(e, this))
    } else {
        e.vcall(base, SLOT_VERTEX_DATA, &args![]).u32()
    };
    let target = e.get(controller, MorpherController::m_pkTarget);
    let geometry = Ptr::<GeometryData>::new(e.mem.u32(target.wrapping_add(MODEL_DATA)));
    let vertex_count = u32::from(e.get(geometry, GeometryData::m_usVertices));
    let data = Ptr::<MorphData>::new(e.get(controller, MorpherController::m_spMorphData));
    let num_targets = e.get(data, MorphData::m_uiNumTargets);
    e.with_stack(4, |e, value| {
        let value = value.addr();
        for i in 0..num_targets {
            let interpolator = e.vcall(base, SLOT_INTERPOLATOR, &args![i]).u32();
            let items = e.get(controller, MorpherController::m_pkInterpItems);
            let mut weight = 0u32;
            if items != 0 {
                let at = items.wrapping_add(i.wrapping_mul(ITEM_SIZE).wrapping_add(ITEM_WEIGHT));
                weight = x87_quiet_bits(e.mem.u32(at));
            }
            e.mem.set_u32(value, weight);
            let data = Ptr::<MorphData>::new(e.get(controller, MorpherController::m_spMorphData));
            let store = if i == 0 && e.get(data, MorphData::m_bRelativeTargets) != 0 {
                let base_weight = if e.get(controller, MorpherController::m_bIgnoreBaseTarget) != 0
                {
                    0.0f32
                } else {
                    1.0
                };
                e.mem.set_u32(value, base_weight.to_bits());
                true
            } else if interpolator != 0 {
                let time = x87_quiet_bits(e.get(controller, MorpherController::m_fScaledTime));
                let target = e.get(controller, MorpherController::m_pkTarget);
                e.vcall(interpolator, SLOT_UPDATE_FLOAT, &args![time, target, value])
                    .u8()
                    != 0
            } else {
                false
            };
            if store {
                let items = e.get(controller, MorpherController::m_pkInterpItems);
                if items != 0 {
                    let at =
                        items.wrapping_add(i.wrapping_mul(ITEM_SIZE).wrapping_add(ITEM_WEIGHT));
                    e.mem.set_u32(at, x87_quiet_bits(e.mem.u32(value)));
                }
            }
        }
    });
    if e.get(controller, MorpherController::m_bIgnoreBaseTarget) == 0 {
        e.call(MEMSET, &args![buffer, 0u32, vertex_count.wrapping_mul(12)]);
    }
    let mut changed = false;
    for i in 0..num_targets {
        let items = e.get(controller, MorpherController::m_pkInterpItems);
        if items == 0 {
            continue;
        }
        let weight_bits = x87_quiet_bits(
            e.mem
                .u32(items.wrapping_add(i.wrapping_mul(ITEM_SIZE).wrapping_add(ITEM_WEIGHT))),
        );
        let weight = x87_load32(weight_bits);
        let above = x87_load32(e.global::<u32>(WEIGHT_ABOVE));
        let apply = weight >= above || weight <= x87_load32(e.global::<u32>(WEIGHT_BELOW));
        if apply {
            let data = Ptr::<MorphData>::new(e.get(controller, MorpherController::m_spMorphData));
            let targets = e.get(data, MorphData::m_aMorphTargets);
            let vertices = e.mem.u32(targets.wrapping_add(i * TARGET_SIZE));
            typed_call!(
                FAMILY,
                0x00a7_ec90,
                add_scaled_points(
                    e,
                    Ptr::new(buffer),
                    f32::from_bits(weight_bits),
                    Ptr::new(vertices),
                    vertex_count
                )
            );
            changed = true;
        }
    }
    if changed {
        let target = e.get(controller, MorpherController::m_pkTarget);
        if e.vcall(target, SLOT_TARGET_CHECK, &args![]).u32() != 0 {
            let target = e.get(controller, MorpherController::m_pkTarget);
            let geometry = e.mem.u32(target.wrapping_add(MODEL_DATA));
            e.call(MARK_AS_CHANGED, &args![geometry, 1u32]);
        }
    }
}

// Typed from 00a3b010
/// `NiGeomMorpherController::GetVertexData` (Xbox PDB): the target's position array; when the data
/// has none, a new one (12 bytes a vertex, from the memory manager) is given to the data with
/// `NiGeometryData::ReplaceData` (keeping its normals, colours, texture coordinates and the data flags'
/// bits 0x0001 and 0xf000) and answered. 0 without a target.
pub fn vertex_data(e: &mut Engine, this: Ptr) -> u32 {
    let controller = this.cast::<MorpherController>();
    let target = e.get(controller, MorpherController::m_pkTarget);
    if target == 0 {
        return 0;
    }
    let geometry = Ptr::<GeometryData>::new(e.mem.u32(target.wrapping_add(MODEL_DATA)));
    let vertices = e.get(geometry, GeometryData::m_pkVertex);
    if vertices != 0 {
        return vertices;
    }
    let count = e.get(geometry, GeometryData::m_usVertices);
    let bytes = u32::from(count).saturating_mul(12);
    let vertices = e.call(ALLOCATE, &args![MEMORY_MANAGER, bytes]).u32();
    let geometry = Ptr::<GeometryData>::new(e.mem.u32(target.wrapping_add(MODEL_DATA)));
    let flags = u32::from(e.get(geometry, GeometryData::m_usDataFlags));
    let normals = e.get(geometry, GeometryData::m_pkNormal);
    let colours = e.get(geometry, GeometryData::m_pkColor);
    let textures = e.get(geometry, GeometryData::m_pkTexture);
    typed_call!(
        FAMILY,
        0x00a6_6bb0,
        data::replace_data(
            e,
            geometry.cast(),
            u32::from(count),
            vertices,
            normals,
            colours,
            textures,
            flags & 1,
            flags & 0xf000
        )
    );
    vertices
}

// Typed from 00a3b080
/// `NiGeomMorpherController::Update` (Xbox PDB; `thiscall`, the update data). Nothing without morph
/// data. When the byte at +5 of the update data is set, its byte at +7 is set and nothing else
/// happens. A forced update (the time controller's flags bit 5) asks for the pre-display and takes the
/// scaled time from the float at `01096324`; otherwise the pre-display is asked for when `DontDoUpdate`
/// (on the update data's first float) says to run or the controller always updates. With a target and
/// the pre-display asked for: [`gen_morph_interp`], then, when the byte at `011f3725` is set, `00714a00`
/// agrees and the geometry data's word at +4 is 1, a new update task (`00a56f90`) is set up for the
/// controller and target (`00a56c60`) and queued (`00a3a730` on the owner at `011f5b04`, with 2); when
/// queueing fails the task runs at once (its slot 0x94).
pub fn update(e: &mut Engine, this: Ptr, update: Ptr) {
    let controller = this.cast::<MorpherController>();
    let base = this.addr();
    if e.get(controller, MorpherController::m_spMorphData) == 0 {
        return;
    }
    let update = update.addr();
    if e.mem.u8(update.wrapping_add(5)) != 0 {
        e.mem.set_u8(update.wrapping_add(7), 1);
        return;
    }
    if (e.get(controller, MorpherController::m_uFlags) >> 5) & 1 != 0 {
        e.set(controller, MorpherController::m_bNeedPreDisplay, 1);
        let forced = e.global::<u32>(FORCED_UPDATE_TIME);
        e.set(
            controller,
            MorpherController::m_fScaledTime,
            x87_quiet_bits(forced),
        );
    } else {
        let time = x87_quiet_bits(e.mem.u32(update));
        if e.call(DONT_DO_UPDATE, &args![base, time]).u8() == 0
            || e.get(controller, MorpherController::m_bAlwaysUpdate) != 0
        {
            e.set(controller, MorpherController::m_bNeedPreDisplay, 1);
        }
    }
    if e.get(controller, MorpherController::m_pkTarget) == 0 {
        return;
    }
    if e.get(controller, MorpherController::m_bNeedPreDisplay) == 0 {
        return;
    }
    let time = x87_quiet_bits(e.get(controller, MorpherController::m_fScaledTime));
    typed_call!(FAMILY, 0x00a3_aa10, gen_morph_interp(e, this, time));
    if e.mem.u8(UPDATE_TASKS_ENABLED) == 0 {
        return;
    }
    if e.call(UPDATE_TASKS_ALLOWED, &args![]).u8() == 0 {
        return;
    }
    let target = e.get(controller, MorpherController::m_pkTarget);
    let geometry = e.mem.u32(target.wrapping_add(MODEL_DATA));
    if e.mem.u32(geometry.wrapping_add(4)) != 1 {
        return;
    }
    let task = e.call(NEW_UPDATE_TASK, &args![]).u32();
    if task == 0 {
        return;
    }
    let target = e.get(controller, MorpherController::m_pkTarget);
    e.call(UPDATE_TASK_SET, &args![task, base, target]);
    let owner = e.global::<u32>(UPDATE_TASK_OWNER);
    if e.call(UPDATE_TASK_QUEUE, &args![owner, task, 2u32]).u8() == 0 {
        e.vcall(task, SLOT_TASK_RUN, &args![]);
    }
}
