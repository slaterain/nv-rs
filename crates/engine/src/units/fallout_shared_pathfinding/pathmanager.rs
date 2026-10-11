//! FalloutNV.exe 1.4.0.525 functions from `fallout shared/pathfinding/pathmanager.cpp`.

#[allow(unused_imports)]
use crate::prelude::*;

const PATHING_TASK_DATA_VTABLE: u32 = 0x0106_d550;
const PATH_MANAGER_IMPL_STATE: u32 = 0x0c24;
const PATH_MANAGER_IMPL_PAUSE_RETRIES: u32 = 0x0c28;
const PATH_MANAGER_IMPL_CANCEL_PATHS: u32 = 0x0c34;

// Translated from 006e9bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ActorPathingMessage::ActorPathingMessage` (Xbox PDB): initializes its
/// message type and empty solution pointer.
pub fn actor_pathing_message_construct(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), u32::MAX);
    e.call(0x0063_3c90, &args![this.addr() + 4, 0u32]);
    this
}

// Translated from 006e9c00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructs a message from a type and solution pointer.
pub fn fn_006e9c00(e: &mut Engine, this: Ptr, message_type: u32, solution: u32) -> Ptr {
    e.mem.set_u32(this.addr(), message_type);
    e.call(0x0055_9a40, &args![this.addr() + 4, solution]);
    this
}

// Translated from 006e9c30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the type and reference-counted pointer in a small pathing closure.
pub fn fn_006e9c30(e: &mut Engine, this: Ptr, source: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), e.mem.u32(source.addr()));
    e.call(0x0055_9a40, &args![this.addr() + 4, source.addr() + 4]);
    this
}

// Translated from 006e9c60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the type and `NiPointer<PathingDebugGeometryData>` member.
pub fn fn_006e9c60(e: &mut Engine, this: Ptr, source: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), e.mem.u32(source.addr()));
    e.call(0x006e_5cc0, &args![this.addr() + 4, source.addr() + 4]);
    this
}

// Translated from 006e9c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destroys the data then frees it only when the scalar delete flag is set.
pub fn pathing_task_data_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_006e9e20(e, this);
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}

// Translated from 006e9cc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructs the pathing task data object. PC code initializes its embedded
/// arrays at `+0x40`, `+0x50`, and quick-bit storage at `+0x60`.
pub fn pathing_task_data_construct(e: &mut Engine, this: Ptr, request: u32, source: u32) -> Ptr {
    e.call(0x006c_7850, &args![this]);
    e.mem.set_u32(this.addr(), PATHING_TASK_DATA_VTABLE);
    e.call(0x0063_3c90, &args![this.addr() + 0x18, 0u32]);
    e.call(0x0063_3c90, &args![this.addr() + 0x1c, 0u32]);
    e.call(0x0096_a2d0, &args![this.addr() + 0x20]);
    e.call(0x006e_be90, &args![this.addr() + 0x40]);
    e.call(0x006e_bee0, &args![this.addr() + 0x50]);
    e.call(0x006e_bf60, &args![this.addr() + 0x60, 0u32]);
    e.mem.set_u8(this.addr() + 0x64, 0);
    e.with_stack(8, |e, temporaries| {
        e.mem.set_u32(temporaries.addr() + 4, source);
        e.call(
            0x006e_bf30,
            &args![temporaries, Ptr::<()>::new(temporaries.addr() + 4)],
        );
        fn_006e9db0(
            e,
            this,
            Ptr::new(request),
            Ptr::<()>::new(temporaries.addr() + 4).addr(),
        );
        e.call(0x006e_bf90, &args![Ptr::<()>::new(temporaries.addr() + 4)]);
    });
    this
}

// Translated from 006e9db0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Assigns the request pointer and copies the cancellation source into the
/// task data's embedded state.
pub fn fn_006e9db0(e: &mut Engine, this: Ptr, request: Ptr, source: u32) {
    e.call(0x006e_5cc0, &args![this.addr() + 0x18, request]);
    e.with_stack(4, |e, source_slot| {
        e.mem.set_u32(source_slot.addr(), source);
        e.call(0x006e_bfb0, &args![this.addr() + 0x60, source_slot]);
        e.call(0x006e_bf90, &args![source_slot]);
    });
}

// Translated from 006e9e20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Releases each non-null pathing request and ray-cast result, then destroys
/// the PC object's embedded arrays and synchronization state.
pub fn fn_006e9e20(e: &mut Engine, this: Ptr) {
    let count = e.call(0x0044_ddc0, &args![this.addr() + 0x40]).u32();
    for index in 0..count {
        let request = e.call(0x0087_7a30, &args![this.addr() + 0x40, index]).u32();
        if request != 0 {
            pathing_task_data_request_deleting_destructor(e, Ptr::new(request), 1);
        }
        let path = e.call(0x0087_7a30, &args![this.addr() + 0x50, index]).u32();
        if path != 0 {
            pathing_task_data_path_deleting_destructor(e, Ptr::new(path), 1);
        }
    }
    e.call(0x006e_bf90, &args![this.addr() + 0x60]);
    e.call(0x006e_bf10, &args![this.addr() + 0x50]);
    e.call(0x006e_bec0, &args![this.addr() + 0x40]);
    e.call(0x0045_cec0, &args![this.addr() + 0x1c]);
    e.call(0x0045_cec0, &args![this.addr() + 0x18]);
    e.call(0x00b0_0ec0, &args![this]);
}

// Translated from 006e9f60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destroys one path request and frees it when the scalar delete bit is set.
pub fn pathing_task_data_request_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x006c_b410, &args![this]);
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}

// Translated from 006e9f90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destroys one ray-cast path and frees it when the scalar delete bit is set.
pub fn pathing_task_data_path_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x006c_b4a0, &args![this]);
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}

// Translated from 006e9fc0 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 006e9fc0 - compiler-generated SEH cleanup remains unimplemented
/// Processes a pathing request and sends the resulting success, failure, or
/// cancellation message to its owning actor and the manager's message queue.
/// The compiler-generated SEH cleanup state is omitted.
pub fn pathing_task_data_process(e: &mut Engine, this: Ptr) {
    e.with_stack(0x10, |e, logging| {
        e.call(
            0x0040_4eb0,
            &args![logging, 0x2du32, 1u32, 0x0106_d560u32, 0x163u32],
        );
        let category = e.call(0x0047_d0b0, &args![]).u32();
        let category = e.call(0x0071_7e50, &args![category]).u32();
        e.call(0x0048_3710, &args![category]);
        let category = e.call(0x0047_d0b0, &args![]).u32();
        let category = e.call(0x0055_9450, &args![category]).u32();
        let was_signaled = fn_006eb790(e, Ptr::new(category));

        if was_signaled {
            if !fn_006ea490(e, this) {
                fn_006ea330(e, this);
            } else {
                let allocation = e.call(0x00aa_13e0, &args![0x44u32]).u32();
                let token = if allocation == 0 {
                    0
                } else {
                    e.call(0x006e_7650, &args![allocation]).u32()
                };
                e.call(0x0066_b0d0, &args![this.addr() + 0x1c, token]);
                let request_owner = e.call(0x0055_9450, &args![this.addr() + 0x1c]).u32();
                let request = e.call(0x0055_9450, &args![this.addr() + 0x18]).u32();
                let request_finished = e.call(0x006d_0900, &args![request, request_owner]).bool();
                if !e.call(0x005d_c980, &args![this]).bool() {
                    if request_finished {
                        e.mem.set_u8(this.addr() + 4, 0);
                        let queue = e.call(0x0055_9450, &args![this.addr() + 0x60]).u32();
                        e.with_stack(8, |e, message| {
                            fn_006e9c00(e, message, 0, this.addr() + 0x1c);
                            e.vcall(queue, 4, &args![message]);
                            e.call(0x0080_0190, &args![message]);
                        });
                        e.with_stack(8, |e, message| {
                            e.call(0x004b_0680, &args![message, 0u32, this]);
                            let category = e.call(0x0047_d0b0, &args![message]).u32();
                            let queue = e.call(0x0055_9450, &args![category]).u32();
                            fn_006eb7f0(e, Ptr::new(queue), message.addr());
                            e.call(0x0048_3710, &args![message]);
                        });
                    } else {
                        let queue = e.call(0x0055_9450, &args![this.addr() + 0x60]).u32();
                        e.with_stack(8, |e, message| {
                            fn_006e9c00(e, message, 1, this.addr() + 0x1c);
                            e.vcall(queue, 4, &args![message]);
                            e.call(0x0080_0190, &args![message]);
                        });
                        e.with_stack(8, |e, message| {
                            e.call(0x004b_0680, &args![message, 1u32, this]);
                            let category = e.call(0x0047_d0b0, &args![message]).u32();
                            let queue = e.call(0x0055_9450, &args![category]).u32();
                            fn_006eb7f0(e, Ptr::new(queue), message.addr());
                            e.call(0x0048_3710, &args![message]);
                        });
                    }
                }
            }
            let category = e.call(0x0047_d0b0, &args![]).u32();
            let category = e.call(0x0055_9450, &args![category]).u32();
            fn_006eb7b0(e, Ptr::new(category));
        } else {
            let category = e.call(0x0047_d0b0, &args![]).u32();
            let category = e.call(0x0055_9450, &args![category]).u32();
            if fn_006eb7d0(e, Ptr::new(category)) {
                let queue = e.call(0x0055_9450, &args![this.addr() + 0x60]).u32();
                e.with_stack(8, |e, message| {
                    fn_006e9c00(e, message, 1, this.addr() + 0x1c);
                    e.vcall(queue, 4, &args![message]);
                    e.call(0x0080_0190, &args![message]);
                });
                e.with_stack(8, |e, message| {
                    e.call(0x004b_0680, &args![message, 1u32, this]);
                    let category = e.call(0x0047_d0b0, &args![message]).u32();
                    let queue = e.call(0x0055_9450, &args![category]).u32();
                    fn_006eb7f0(e, Ptr::new(queue), message.addr());
                    e.call(0x0048_3710, &args![message]);
                });
            } else {
                fn_006ea330(e, this);
            }
        }

        let category = e.call(0x0047_d0b0, &args![]).u32();
        let category = e.call(0x0071_7e50, &args![category]).u32();
        e.call(0x0048_3710, &args![category]);
        e.call(0x0040_4ee0, &args![logging]);
    });
}

// Translated from 006ea330 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the path-data yield flag.
pub fn fn_006ea330(e: &mut Engine, this: Ptr) {
    e.mem.set_u8(this.addr() + 4, 1);
}

// Translated from 006ea350 (decompiled, FalloutNV.exe 1.4.0.525)
/// Builds pending ray-cast paths until the queue is exhausted or the world
/// space budget exceeds ten.
pub fn fn_006ea350(e: &mut Engine, this: Ptr) -> i32 {
    let mut built = 0i32;
    e.call(0x0040_fbf0, &args![this.addr() + 0x20, 0u32]);
    let count = e.call(0x0044_ddc0, &args![this.addr() + 0x40]).u32();
    e.with_stack(0x10, |e, ray_cast| {
        for index in 0..count {
            let request_slot = e.call(0x0087_7a30, &args![this.addr() + 0x40, index]).u32();
            let request = e.mem.u32(request_slot);
            if e.mem.u32(request + 4) != 0 {
                continue;
            }
            e.call(0x006e_61a0, &args![ray_cast]);
            let path_slot = e.call(0x0087_7a30, &args![this.addr() + 0x50, index]).u32();
            let path = e.mem.u32(path_slot);
            e.call(0x006e_6320, &args![ray_cast, request, index, path]);
            built += e.call(0x004f_d3e0, &args![ray_cast]).i32();
            e.call(0x006e_62b0, &args![ray_cast]);
            if built > 10 {
                break;
            }
        }
    });
    e.call(0x0040_fba0, &args![this.addr() + 0x20]);
    built
}

// Translated from 006ea490 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns false if any current pathing request has no result yet.
pub fn fn_006ea490(e: &mut Engine, this: Ptr) -> bool {
    e.call(0x0040_fbf0, &args![this.addr() + 0x20, 0u32]);
    let count = e.call(0x0044_ddc0, &args![this.addr() + 0x50]).u32();
    for index in 0..count {
        let path_slot = e.call(0x0087_7a30, &args![this.addr() + 0x50, index]).u32();
        let path = e.mem.u32(path_slot);
        if e.mem.u32(path + 4) == 0 {
            e.call(0x0040_fba0, &args![this.addr() + 0x20]);
            return false;
        }
    }
    e.call(0x0040_fba0, &args![this.addr() + 0x20]);
    true
}

// Translated from 006ea510 (decompiled, FalloutNV.exe 1.4.0.525)
/// Finds a matching ray-cast request or appends a new request and result slot.
/// The executable's tolerance is read from its float constant at `0x0101e2bc`.
pub fn fn_006ea510(e: &mut Engine, this: Ptr, start: Ptr, goal: Ptr, result_out: u32) -> i32 {
    let mut result = 0i32;
    e.call(0x0040_fbf0, &args![this.addr() + 0x20, 0u32]);
    let count = e.call(0x0044_ddc0, &args![this.addr() + 0x50]).u32();
    for index in 0..count {
        let path_slot = e.call(0x0087_7a30, &args![this.addr() + 0x50, index]).u32();
        let path = e.mem.u32(path_slot);
        if e.mem.u32(path) != 2 {
            continue;
        }
        let request_slot = e.call(0x0087_7a30, &args![this.addr() + 0x40, index]).u32();
        let request = e.mem.u32(request_slot);
        let tolerance = e.global::<f32>(0x0101_e2bc);
        if !e
            .call(0x006d_eeb0, &args![request + 8, start, tolerance])
            .bool()
            || !e
                .call(0x006d_eeb0, &args![request + 0x30, goal, tolerance])
                .bool()
        {
            continue;
        }
        result = e.mem.u32(path + 4) as i32;
        match result {
            0 => fn_006ea330(e, this),
            1 => {
                let result_slot = e
                    .call(0x0087_7a30, &args![this.addr() + 0x50, index, result_out])
                    .u32();
                e.call(0x006c_f940, &args![e.mem.u32(result_slot) + 8]);
            }
            2 => {}
            _ => result = 2,
        }
        e.call(0x0040_fba0, &args![this.addr() + 0x20]);
        return result;
    }

    let request_storage = e.call(0x0040_1000, &args![0x68u32]).u32();
    let request = e.call(0x006c_b3a0, &args![request_storage]).u32();
    let request_pointer = Ptr::<()>::new(request);
    let request_owner = e.call(0x0055_9450, &args![this.addr() + 0x18]).u32();
    e.mem.set_u32(request + 4, request_owner);
    e.call(0x006d_cce0, &args![request + 8, start]);
    e.call(0x006d_cce0, &args![request + 0x30, goal]);
    e.mem.set_u32(request, 2);
    let path_storage = e.call(0x0040_1000, &args![0x18u32]).u32();
    let path = e.call(0x006c_b480, &args![path_storage]).u32();
    e.mem.set_u32(path, e.mem.u32(request));
    e.mem.set_u32(path + 4, 0);
    e.with_stack(8, |e, holders| {
        e.mem.set_u32(holders.addr(), request_pointer.addr());
        e.mem.set_u32(holders.addr() + 4, path);
        e.call(0x007c_b2e0, &args![this.addr() + 0x40, holders.addr()]);
        e.call(0x007c_b2e0, &args![this.addr() + 0x50, holders.addr() + 4]);
    });
    e.call(0x0040_fba0, &args![this.addr() + 0x20]);
    e.call(0x0060_d4b0, &args![this, 1u32]);
    result
}

// Translated from 006ea780 (decompiled, FalloutNV.exe 1.4.0.525)
/// Finds a ray-cast request by its navmesh mode and goal, or appends a new
/// request and path result. The mode is stored as `!navmesh_mode` by the PC
/// executable.
pub fn fn_006ea780(
    e: &mut Engine,
    this: Ptr,
    start: Ptr,
    goal: Ptr,
    result_out: u32,
    navmesh_mode: u8,
) -> i32 {
    let mut result = 0i32;
    e.call(0x0040_fbf0, &args![this.addr() + 0x20, 0u32]);
    let count = e.call(0x0044_ddc0, &args![this.addr() + 0x50]).u32();
    let path_kind = u32::from(navmesh_mode == 0);
    for index in 0..count {
        let path_slot = e.call(0x0087_7a30, &args![this.addr() + 0x50, index]).u32();
        let path = e.mem.u32(path_slot);
        if e.mem.u32(path) != path_kind {
            continue;
        }
        let request_slot = e.call(0x0087_7a30, &args![this.addr() + 0x40, index]).u32();
        let request = e.mem.u32(request_slot);
        let tolerance = e.global::<f32>(0x0101_e2bc);
        if !e
            .call(0x006d_eeb0, &args![request + 0x30, goal, tolerance])
            .bool()
        {
            continue;
        }
        result = e.mem.u32(path + 4) as i32;
        match result {
            0 => fn_006ea330(e, this),
            1 => {
                let result_slot = e
                    .call(0x0087_7a30, &args![this.addr() + 0x50, index, result_out])
                    .u32();
                e.call(0x006c_f940, &args![e.mem.u32(result_slot) + 8]);
            }
            2 => {}
            _ => result = 2,
        }
        e.call(0x0040_fba0, &args![this.addr() + 0x20]);
        return result;
    }

    let request_storage = e.call(0x0040_1000, &args![0x68u32]).u32();
    let request = e.call(0x006c_b3a0, &args![request_storage]).u32();
    let request_pointer = Ptr::<()>::new(request);
    let request_owner = e.call(0x0055_9450, &args![this.addr() + 0x18]).u32();
    e.mem.set_u32(request + 4, request_owner);
    e.call(0x006d_cce0, &args![request + 0x30, goal]);
    e.call(0x006c_f8a0, &args![start, request + 0x58]);
    e.mem.set_u32(request, path_kind);
    let path_storage = e.call(0x0040_1000, &args![0x18u32]).u32();
    let path = e.call(0x006c_b480, &args![path_storage]).u32();
    e.mem.set_u32(path, e.mem.u32(request));
    e.mem.set_u32(path + 4, 0);
    e.with_stack(8, |e, holders| {
        e.mem.set_u32(holders.addr(), request_pointer.addr());
        e.mem.set_u32(holders.addr() + 4, path);
        e.call(0x007c_b2e0, &args![this.addr() + 0x40, holders.addr()]);
        e.call(0x007c_b2e0, &args![this.addr() + 0x50, holders.addr() + 4]);
    });
    e.call(0x0040_fba0, &args![this.addr() + 0x20]);
    e.call(0x0060_d4b0, &args![this, 1u32]);
    result
}

// Translated from 006ea9d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Initializes the two-word completion result used by the path-manager worker.
pub fn fn_006ea9d0(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), u32::MAX);
    e.mem.set_u32(this.addr() + 4, 0);
    this
}

// Translated from 006eaa00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies both words of the path-manager completion result.
pub fn fn_006eaa00(e: &mut Engine, this: Ptr, source: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), e.mem.u32(source.addr()));
    e.mem.set_u32(this.addr() + 4, e.mem.u32(source.addr() + 4));
    this
}

// Translated from 006eaa30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructs `PathManagerImpl`, its queues, locks and request map. The state
/// offsets match the PC disassembly and `PathManagerImpl` (Xbox PDB).
pub fn path_manager_impl_construct(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(0x0044_dee0, &args![this]);
    e.call(0x006e_c000, &args![this.addr() + 4]);
    e.call(0x006e_c090, &args![this.addr() + 0x0c10, this.addr() + 4]);
    e.call(0x00aa_5fc0, &args![this.addr() + 0x0c2c]);
    e.call(0x0044_fae0, &args![this.addr() + 0x0c38]);
    e.call(0x006e_c340, &args![this.addr() + 0x0c50]);
    e.call(0x006e_c340, &args![this.addr() + 0x1064]);
    e.call(
        0x006e_c660,
        &args![this.addr() + 0x1480, 0x0cu32, 0x25u32, 0x0cu32],
    );
    e.mem.set_u32(this.addr() + PATH_MANAGER_IMPL_STATE, 0);
    e.mem
        .set_u32(this.addr() + PATH_MANAGER_IMPL_PAUSE_RETRIES, 0);
    e.mem
        .set_u8(this.addr() + PATH_MANAGER_IMPL_CANCEL_PATHS, 0);
    this
}

// Translated from 006eab30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destroys the request map, queues, lock and tasklet group in reverse order.
pub fn path_manager_impl_destruct(e: &mut Engine, this: Ptr) {
    e.call(0x006e_c870, &args![this.addr() + 0x1480]);
    e.call(0x006e_c3f0, &args![this.addr() + 0x1064]);
    e.call(0x006e_c3f0, &args![this.addr() + 0x0c50]);
    e.call(0x0044_fb00, &args![this.addr() + 0x0c38]);
    e.call(0x006e_c130, &args![this.addr() + 0x0c10]);
    e.call(0x006e_bd30, &args![this.addr() + 4]);
    e.call(0x0048_3710, &args![this]);
}

// Translated from 006eabe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Creates the task group and resets the manager's state after both manager
/// and group creation succeed.
pub fn path_manager_impl_initialize(e: &mut Engine, this: Ptr) -> bool {
    let manager = e.call(0x00b0_0a00, &args![this]).u32();
    if !e.call(0x00b0_0a80, &args![manager]).bool() {
        return false;
    }
    if !e.call(0x006c_08e0, &args![this]).bool() {
        return false;
    }
    e.mem.set_u32(this.addr() + PATH_MANAGER_IMPL_STATE, 0);
    e.mem
        .set_u32(this.addr() + PATH_MANAGER_IMPL_PAUSE_RETRIES, 0);
    e.mem
        .set_u8(this.addr() + PATH_MANAGER_IMPL_CANCEL_PATHS, 0);
    true
}

// Translated from 006eac40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destroys the path-manager task group after cancelling it.
pub fn path_manager_impl_destroy(e: &mut Engine, this: Ptr) {
    e.call(0x006c_09d0, &args![this]);
    let manager = e.call(0x00b0_0a00, &args![this]).u32();
    e.call(0x00b0_0c20, &args![manager]);
}

// Translated from 006eac70 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 006eac70 - compiler-generated SEH cleanup remains unimplemented
/// Builds task data from the two stack-owned request sources and reports the
/// cancellation result. SEH and compiler-generated cancellation cleanup are
/// omitted; source ownership is passed through the observed helper calls.
pub fn fn_006eac70(e: &mut Engine, this: Ptr, source_a: u32, source_b: u32) -> bool {
    let allocation = e.call(0x0040_1000, &args![0x80u32]).u32();
    let data = if allocation == 0 {
        0
    } else {
        pathing_task_data_construct(e, Ptr::new(allocation), source_a, source_b).addr()
    };
    e.with_stack(0x48, |e, locals| {
        let data_ref = locals.addr();
        let guard = locals.addr() + 8;
        e.mem.set_u32(data_ref, data);
        e.call(0x006c_5e40, &args![guard]);
        e.call(0x006e_cd40, &args![guard, data_ref]);
        let key = e.call(0x0055_9450, &args![source_a]).u32();
        let map = this.addr() + 0x1480;
        e.vcall(map, 0x10, &args![key, data_ref, 1u32]);
        let cancelled = e.call(0x006c_5ec0, &args![this, guard, 0u32]).bool();
        if !cancelled {
            e.vcall(map, 0x14, &args![key]);
            let retained = e.mem.u32(data_ref);
            if retained != 0 {
                e.vcall(retained, 0, &args![1u32]);
            }
        }
        e.call(0x006c_5e70, &args![guard]);
        e.call(0x0045_cec0, &args![Ptr::<()>::new(source_a)]);
        e.call(0x006e_bf90, &args![Ptr::<()>::new(source_b)]);
        cancelled
    })
}

// Translated from 006eae40 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 006eae40 - compiler-generated SEH cleanup remains unimplemented
/// Updates pause state, processes task completions, drains add/remove queues,
/// and notifies the path listener when the queue changed. SEH cleanup is omitted.
pub fn fn_006eae40(e: &mut Engine, this: Ptr) {
    e.call(0x0045_38a0, &args![this.addr() + 0x0c38, 0x0106_d5b0u32]);
    match e.mem.u32(this.addr() + PATH_MANAGER_IMPL_STATE) {
        1 => {
            if e.call(0x00aa_6220, &args![this.addr() + 0x0c2c]).bool() {
                e.mem.set_u32(this.addr() + PATH_MANAGER_IMPL_STATE, 2);
            } else {
                let retries = e.mem.u32(this.addr() + PATH_MANAGER_IMPL_PAUSE_RETRIES);
                e.mem.set_u32(
                    this.addr() + PATH_MANAGER_IMPL_PAUSE_RETRIES,
                    retries.wrapping_add(1),
                );
            }
        }
        3 => {
            e.call(0x00aa_62e0, &args![this.addr() + 0x0c2c]);
            e.mem.set_u32(this.addr() + PATH_MANAGER_IMPL_STATE, 0);
            e.mem
                .set_u32(this.addr() + PATH_MANAGER_IMPL_PAUSE_RETRIES, 0);
        }
        _ => {}
    }
    e.with_stack(0x70, |e, locals| {
        let task_stack = locals.addr();
        let completion = locals.addr() + 0x20;
        let status_out = locals.addr() + 0x30;
        let task_out = locals.addr() + 0x34;
        let path_holder = locals.addr() + 0x38;
        let changed = locals.addr() + 0x3c;
        let point = locals.addr() + 0x40;
        e.mem.set_u32(locals.addr() + 0x48, 0);
        e.call(0x006e_c970, &args![task_stack]);
        while !e.call(0x006e_bde0, &args![task_stack]).bool() {
            let popped = e
                .call(
                    0x0044_c640,
                    &args![this.addr() + 0x1480, task_stack, status_out, task_out, 1u32],
                )
                .bool();
            if popped {
                // disassembly reads the third c640 out-argument at EBP-0x34;
                // the preceding out-argument at EBP-0x38 is not task data.
                let task_data = Ptr::new(e.mem.u32(task_out));
                if e.call(0x005d_c980, &args![task_data]).bool() {
                    let built = fn_006ea350(e, task_data);
                    let total = e.mem.u32(locals.addr() + 0x48).wrapping_add(built as u32);
                    e.mem.set_u32(locals.addr() + 0x48, total);
                    if total > 10 {
                        break;
                    }
                    if fn_006ea490(e, task_data) {
                        e.call(0x0060_d4b0, &args![task_data, 0u32]);
                        e.call(0x006c_5e40, &args![point]);
                        e.call(0x006e_cd40, &args![point, task_data]);
                        e.call(0x006c_5ec0, &args![this, point, 0u32]);
                        e.call(0x006c_5e70, &args![point]);
                    }
                }
            }
        }
        fn_006ea9d0(e, Ptr::new(completion));
        while e
            .vcall(this.addr() + 0x0c10, 0x10, &args![completion])
            .bool()
        {
            let kind = e.mem.i32(completion);
            if (0..=1).contains(&kind) {
                let key = e.call(0x0055_9450, &args![completion + 0x18]).u32();
                e.vcall(this.addr() + 0x1480, 0x14, &args![key]);
                let reference = e.mem.u32(completion + 4);
                if reference != 0 {
                    e.vcall(reference, 0, &args![1u32]);
                }
            }
        }
        e.mem.set_u8(changed, 0);
        e.call(0x0063_3c90, &args![path_holder, 0u32]);
        while e
            .vcall(this.addr() + 0x1064, 0x10, &args![path_holder])
            .bool()
        {
            if e.call(0x008c_7aa0, &args![]).bool() {
                let item = e.call(0x0055_9450, &args![path_holder]).u32();
                let resolved = e.call(0x0045_37b0, &args![item]).u32();
                e.call(0x0087_ac60, &args![resolved]);
            } else {
                let listener = e.call(0x0055_9450, &args![0x011d_6e5cu32]).u32();
                let item = e.call(0x0055_9450, &args![path_holder]).u32();
                e.vcall(listener, 0xe8, &args![item]);
            }
            e.mem.set_u8(changed, 1);
        }
        while e
            .vcall(this.addr() + 0x0c50, 0x10, &args![path_holder])
            .bool()
        {
            if e.call(0x008c_7aa0, &args![]).bool() {
                let item = e.call(0x0055_9450, &args![path_holder]).u32();
                let global = e.call(0x0055_9450, &args![0x011d_6e5cu32]).u32();
                let resolved = e.call(0x0045_37b0, &args![item, global, 1u32, 0u32]).u32();
                e.call(0x0087_ac00, &args![resolved]);
            } else {
                let listener = e.call(0x0055_9450, &args![0x011d_6e5cu32]).u32();
                let item = e.call(0x0055_9450, &args![path_holder]).u32();
                e.vcall(listener, 0xdc, &args![item, 1u32]);
            }
            e.mem.set_u8(changed, 1);
        }
        if e.mem.u8(changed) != 0 {
            if e.call(0x008c_7aa0, &args![]).bool() {
                let global = e.call(0x0055_9450, &args![0x011d_6e5cu32]).u32();
                let resolved = e.call(0x0045_37b0, &args![global]).u32();
                e.call(0x0087_ad50, &args![resolved]);
            } else {
                e.call(0x0043_d410, &args![point, 0.0f32, 0u32, 0u32]);
                let global = e.call(0x0055_9450, &args![0x011d_6e5cu32]).u32();
                e.call(0x00a5_9c60, &args![global, point]);
            }
        }
        e.call(0x0045_38c0, &args![this.addr() + 0x0c38]);
        e.call(0x0045_cec0, &args![path_holder]);
        e.call(0x0048_3710, &args![completion]);
        e.call(0x006e_bdc0, &args![task_stack]);
    });
}
// Translated from 006eb200 (decompiled, FalloutNV.exe 1.4.0.525)
/// Pushes a path message to the regular queue or ray-cast queue by mode.
pub fn fn_006eb200(e: &mut Engine, this: Ptr, message: u32, ray_cast: u8) -> bool {
    let offset = if ray_cast == 0 { 0x1064 } else { 0x0c50 };
    e.vcall(this.addr() + offset, 4, &args![message]);
    true
}

// Translated from 006eb250 (decompiled, FalloutNV.exe 1.4.0.525)
/// Finds request task data in the manager map, returning status 2 if the key
/// has no task data.
pub fn fn_006eb250(
    e: &mut Engine,
    this: Ptr,
    key: u32,
    start: Ptr,
    goal: Ptr,
    result_out: u32,
    navmesh_mode: u8,
) -> i32 {
    e.with_stack(4, |e, task_data| {
        e.mem.set_u32(task_data.addr(), 0);
        if !e
            .vcall(this.addr() + 0x1480, 8, &args![key, task_data])
            .bool()
        {
            2
        } else {
            fn_006ea780(
                e,
                Ptr::new(e.mem.u32(task_data.addr())),
                start,
                goal,
                result_out,
                navmesh_mode,
            )
        }
    })
}

// Translated from 006eb2b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Finds request task data in the manager map, returning status 2 if the key
/// has no task data.
pub fn fn_006eb2b0(
    e: &mut Engine,
    this: Ptr,
    key: u32,
    start: Ptr,
    goal: Ptr,
    result_out: u32,
) -> i32 {
    e.with_stack(4, |e, task_data| {
        e.mem.set_u32(task_data.addr(), 0);
        if !e
            .vcall(this.addr() + 0x1480, 8, &args![key, task_data])
            .bool()
        {
            2
        } else {
            fn_006ea510(
                e,
                Ptr::new(e.mem.u32(task_data.addr())),
                start,
                goal,
                result_out,
            )
        }
    })
}

// Translated from 006eb310 (decompiled, FalloutNV.exe 1.4.0.525)
/// Pauses the manager and clears the pause retry counter when running.
pub fn fn_006eb310(e: &mut Engine, this: Ptr) {
    e.call(0x0045_38a0, &args![this.addr() + 0x0c38, 0x0106_d5c8u32]);
    match e.mem.u32(this.addr() + PATH_MANAGER_IMPL_STATE) {
        0 => {
            e.mem.set_u32(this.addr() + PATH_MANAGER_IMPL_STATE, 1);
            e.mem
                .set_u32(this.addr() + PATH_MANAGER_IMPL_PAUSE_RETRIES, 0);
        }
        3 => e.mem.set_u32(this.addr() + PATH_MANAGER_IMPL_STATE, 2),
        _ => {}
    }
    e.call(0x0045_38c0, &args![this.addr() + 0x0c38]);
}

// Translated from 006eb390 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reports whether the path manager is in paused state 2.
pub fn fn_006eb390(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + PATH_MANAGER_IMPL_STATE) == 2
}

// Translated from 006eb3b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Resumes a paused manager, retaining the intermediate state if it was 2.
pub fn fn_006eb3b0(e: &mut Engine, this: Ptr) {
    e.call(0x0045_38a0, &args![this.addr() + 0x0c38, 0x0106_d5e0u32]);
    match e.mem.u32(this.addr() + PATH_MANAGER_IMPL_STATE) {
        1 => e.mem.set_u32(this.addr() + PATH_MANAGER_IMPL_STATE, 0),
        2 => e.mem.set_u32(this.addr() + PATH_MANAGER_IMPL_STATE, 3),
        _ => {}
    }
    e.call(0x0045_38c0, &args![this.addr() + 0x0c38]);
}

// Translated from 006eb420 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reports whether the path manager is in running state 0.
pub fn fn_006eb420(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + PATH_MANAGER_IMPL_STATE) == 0
}

// Translated from 006eb440 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 006eb440 - compiler-generated SEH cleanup remains unimplemented
/// Cancels queued completions and tasks, then enters paused state. SEH cleanup is omitted.
pub fn fn_006eb440(e: &mut Engine, this: Ptr) {
    e.call(0x0045_38a0, &args![this.addr() + 0x0c38, 0x0106_d5f8u32]);
    e.call(0x006c_09d0, &args![this]);
    e.mem
        .set_u8(this.addr() + PATH_MANAGER_IMPL_CANCEL_PATHS, 1);
    fn_006eb670(e, this, 0);
    e.with_stack(0x70, |e, locals| {
        let completion = locals.addr();
        loop {
            fn_006ea9d0(e, Ptr::new(completion));
            if !e
                .vcall(this.addr() + 0x0c10, 0x10, &args![completion])
                .bool()
            {
                break;
            }
            let kind = e.mem.i32(completion);
            if (0..=1).contains(&kind) {
                let key = e.call(0x0055_9450, &args![completion + 0x18]).u32();
                e.vcall(this.addr() + 0x1480, 0x14, &args![key]);
                let reference = e.mem.u32(completion + 4);
                if reference != 0 {
                    e.vcall(reference, 0, &args![1u32]);
                }
            }
        }
        let task_stack = locals.addr() + 0x20;
        let first_out = locals.addr() + 0x40;
        let task_out = locals.addr() + 0x44;
        let actor_message = locals.addr() + 0x50;
        e.call(0x006e_c970, &args![task_stack]);
        while !e.call(0x006e_bde0, &args![task_stack]).bool() {
            if e.call(
                0x0044_c640,
                &args![this.addr() + 0x1480, task_stack, task_out, first_out, 1u32],
            )
            .bool()
            {
                let data = e.mem.u32(task_out);
                let listener = e.call(0x0055_9450, &args![data + 0x60]).u32();
                fn_006e9c00(e, Ptr::new(actor_message), 1, data + 0x1c);
                e.vcall(listener, 4, &args![actor_message]);
                e.call(0x0080_0190, &args![actor_message]);
                if data != 0 {
                    e.vcall(data, 0, &args![1u32]);
                }
            }
        }
        e.call(0x0044_c480, &args![this.addr() + 0x1480, 0u32]);
        e.call(0x00aa_6060, &args![this.addr() + 0x0c2c]);
        e.mem.set_u32(this.addr() + PATH_MANAGER_IMPL_STATE, 2);
        e.mem
            .set_u8(this.addr() + PATH_MANAGER_IMPL_CANCEL_PATHS, 0);
        e.call(0x006c_08e0, &args![this]);
        e.call(0x0045_38c0, &args![this.addr() + 0x0c38]);
        e.call(0x006e_bdc0, &args![task_stack]);
        e.call(0x0048_3710, &args![completion]);
    });
}

// Translated from 006eb670 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the cancellation object's vtable slot 4 unless it is already complete.
pub fn fn_006eb670(e: &mut Engine, this: Ptr, action: u8) {
    let cancellation = e.mem.u32(this.addr());
    if cancellation != 0 && !e.call(0x004f_1540, &args![cancellation]).bool() {
        e.vcall(cancellation, 4, &args![action]);
    }
}

// Translated from 006eb6b0 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 006eb6b0 - compiler-generated SEH cleanup remains unimplemented
/// Clears the matching request key from queued task cancellation data. SEH cleanup is omitted.
pub fn fn_006eb6b0(e: &mut Engine, this: Ptr, key: u32) {
    e.call(0x0045_38a0, &args![this.addr() + 0x0c38, 0x0106_d5f8u32]);
    e.with_stack(0x30, |e, locals| {
        let task_stack = locals.addr();
        let first_out = locals.addr() + 0x20;
        let task_out = locals.addr() + 0x24;
        e.call(0x006e_c970, &args![task_stack]);
        while !e.call(0x006e_bde0, &args![task_stack]).bool() {
            if e.call(
                0x0044_c640,
                &args![this.addr() + 0x1480, task_stack, task_out, first_out, 1u32],
            )
            .bool()
            {
                let data = e.mem.u32(task_out);
                let cancel_data = e.call(0x006c_9900, &args![data]).u32();
                if e.mem.u32(cancel_data + 4) == key {
                    let cancel_data = e.call(0x006c_9900, &args![data]).u32();
                    e.mem.set_u32(cancel_data + 4, 0);
                    let cancel_data = e.call(0x006c_9900, &args![data]).u32();
                    e.mem.set_u32(cancel_data, 0);
                }
            }
        }
        e.call(0x0045_38c0, &args![this.addr() + 0x0c38]);
        e.call(0x006e_bdc0, &args![task_stack]);
    });
}

// Translated from 006eb790 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the manager pause event's signal state.
pub fn fn_006eb790(e: &mut Engine, this: Ptr) -> bool {
    e.call(0x00aa_6120, &args![this.addr() + 0x0c2c]).bool()
}

// Translated from 006eb7b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Signals the manager pause event.
pub fn fn_006eb7b0(e: &mut Engine, this: Ptr) {
    e.call(0x00aa_62c0, &args![this.addr() + 0x0c2c]);
}

// Translated from 006eb7d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the path cancellation flag.
pub fn fn_006eb7d0(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + PATH_MANAGER_IMPL_CANCEL_PATHS) != 0
}

// Translated from 006eb7f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Enqueues a message on the path manager's task queue.
pub fn fn_006eb7f0(e: &mut Engine, this: Ptr, message: u32) {
    e.vcall(this.addr() + 0x0c10, 4, &args![message]);
}

// Translated from 006eb820 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructs the named Pathing/BuildPath message and clears its state word.
pub fn fn_006eb820(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(
        0x0045_d260,
        &args![this.addr() + 4, 0x0103_a1bcu32, 0x0106_d624u32, 0u32, 1u32],
    );
    e.mem.set_u32(this.addr(), 0);
    this
}

// Translated from 006eb850 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stops the owned task with the flag passed by the executable when non-null.
pub fn fn_006eb850(e: &mut Engine, this: Ptr) {
    let task = e.mem.u32(this.addr());
    if task != 0 {
        e.call(0x006e_b890, &args![task, 1u32]);
    }
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x006e_9bd0, actor_pathing_message_construct(Ptr) -> Ptr),
        entry!(0x006e_9c00, fn_006e9c00(Ptr, u32, u32) -> Ptr),
        entry!(0x006e_9c30, fn_006e9c30(Ptr, Ptr) -> Ptr),
        entry!(0x006e_9c60, fn_006e9c60(Ptr, Ptr) -> Ptr),
        entry!(0x006e_9c90, pathing_task_data_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x006e_9cc0, pathing_task_data_construct(Ptr, u32, u32) -> Ptr),
        entry!(0x006e_9db0, fn_006e9db0(Ptr, Ptr, u32)),
        entry!(0x006e_9e20, fn_006e9e20(Ptr)),
        entry!(0x006e_9f60, pathing_task_data_request_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x006e_9f90, pathing_task_data_path_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x006e_9fc0, pathing_task_data_process(Ptr)),
        entry!(0x006e_a330, fn_006ea330(Ptr)),
        entry!(0x006e_a350, fn_006ea350(Ptr) -> i32),
        entry!(0x006e_a490, fn_006ea490(Ptr) -> bool),
        entry!(0x006e_a510, fn_006ea510(Ptr, Ptr, Ptr, u32) -> i32),
        entry!(0x006e_a780, fn_006ea780(Ptr, Ptr, Ptr, u32, u8) -> i32),
        entry!(0x006e_a9d0, fn_006ea9d0(Ptr) -> Ptr),
        entry!(0x006e_aa00, fn_006eaa00(Ptr, Ptr) -> Ptr),
        entry!(0x006e_aa30, path_manager_impl_construct(Ptr) -> Ptr),
        entry!(0x006e_ab30, path_manager_impl_destruct(Ptr)),
        entry!(0x006e_abe0, path_manager_impl_initialize(Ptr) -> bool),
        entry!(0x006e_ac40, path_manager_impl_destroy(Ptr)),
        entry!(0x006e_ac70, fn_006eac70(Ptr, u32, u32) -> bool),
        entry!(0x006e_ae40, fn_006eae40(Ptr)),
        entry!(0x006e_b200, fn_006eb200(Ptr, u32, u8) -> bool),
        entry!(0x006e_b250, fn_006eb250(Ptr, u32, Ptr, Ptr, u32, u8) -> i32),
        entry!(0x006e_b2b0, fn_006eb2b0(Ptr, u32, Ptr, Ptr, u32) -> i32),
        entry!(0x006e_b310, fn_006eb310(Ptr)),
        entry!(0x006e_b390, fn_006eb390(Ptr) -> bool),
        entry!(0x006e_b3b0, fn_006eb3b0(Ptr)),
        entry!(0x006e_b420, fn_006eb420(Ptr) -> bool),
        entry!(0x006e_b440, fn_006eb440(Ptr)),
        entry!(0x006e_b670, fn_006eb670(Ptr, u8)),
        entry!(0x006e_b6b0, fn_006eb6b0(Ptr, u32)),
        entry!(0x006e_b790, fn_006eb790(Ptr) -> bool),
        entry!(0x006e_b7b0, fn_006eb7b0(Ptr)),
        entry!(0x006e_b7d0, fn_006eb7d0(Ptr) -> bool),
        entry!(0x006e_b7f0, fn_006eb7f0(Ptr, u32)),
        entry!(0x006e_b820, fn_006eb820(Ptr) -> Ptr),
        entry!(0x006e_b850, fn_006eb850(Ptr)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    fn result(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn noop_addresses(e: &mut Engine, addresses: &[u32]) {
        for address in addresses {
            e.register(*address, |_, _| Ret::default());
        }
    }

    #[test]
    fn actor_message_constructor_sets_empty_type_and_constructs_pointer_member() {
        let mut e = Engine::new();
        let object = 0x1000;
        e.mem.map(object, 8);
        e.register(0x0063_3c90, |_, _| Ret::default());
        actor_pathing_message_construct(&mut e, Ptr::new(object));
        assert_eq!(e.mem.u32(object), u32::MAX);
        assert_eq!(e.call_log, None);
    }

    #[test]
    fn message_from_type_writes_type_and_passes_solution_pointer() {
        let mut e = Engine::new();
        let object = 0x1100;
        e.mem.map(object, 8);
        e.register(0x0055_9a40, |_, _| Ret::default());
        fn_006e9c00(&mut e, Ptr::new(object), 1, 0x3344);
        assert_eq!(e.mem.u32(object), 1);
        assert_eq!(e.mem.u32(object + 4), 0);
    }

    #[test]
    fn completion_value_constructor_and_copy_preserve_both_words() {
        let mut e = Engine::new();
        let object = 0x1200;
        let source = 0x1300;
        e.mem.map(object, 8);
        e.mem.map(source, 8);
        fn_006ea9d0(&mut e, Ptr::new(object));
        assert_eq!(e.mem.u32(object), u32::MAX);
        assert_eq!(e.mem.u32(object + 4), 0);
        e.mem.set_u32(source, 1);
        e.mem.set_u32(source + 4, 0x5678);
        fn_006eaa00(&mut e, Ptr::new(object), Ptr::new(source));
        assert_eq!(e.mem.u32(object), 1);
        assert_eq!(e.mem.u32(object + 4), 0x5678);
    }

    #[test]
    fn manager_state_queries_and_pause_resume_transitions() {
        let mut e = Engine::new();
        let manager = 0x2000;
        e.mem.map(manager, 0x1500);
        e.mem.set_u32(manager + PATH_MANAGER_IMPL_STATE, 0);
        e.register(0x0045_38a0, |_, _| Ret::default());
        e.register(0x0045_38c0, |_, _| Ret::default());
        assert!(fn_006eb420(&mut e, Ptr::new(manager)));
        fn_006eb310(&mut e, Ptr::new(manager));
        assert_eq!(e.mem.u32(manager + PATH_MANAGER_IMPL_STATE), 1);
        assert_eq!(e.mem.u32(manager + PATH_MANAGER_IMPL_PAUSE_RETRIES), 0);
        assert!(!fn_006eb420(&mut e, Ptr::new(manager)));
        fn_006eb3b0(&mut e, Ptr::new(manager));
        assert_eq!(e.mem.u32(manager + PATH_MANAGER_IMPL_STATE), 0);
        e.mem.set_u32(manager + PATH_MANAGER_IMPL_STATE, 2);
        assert!(fn_006eb390(&mut e, Ptr::new(manager)));
        fn_006eb3b0(&mut e, Ptr::new(manager));
        assert_eq!(e.mem.u32(manager + PATH_MANAGER_IMPL_STATE), 3);
    }

    #[test]
    fn manager_pause_retry_and_intermediate_state_transitions() {
        let mut e = Engine::new();
        let manager = 0x4000;
        e.mem.map(manager, 0x1500);
        e.mem.set_u32(manager + PATH_MANAGER_IMPL_STATE, 1);
        e.mem.set_u32(manager + PATH_MANAGER_IMPL_PAUSE_RETRIES, 4);
        e.register(0x0045_38a0, |_, _| Ret::default());
        e.register(0x0045_38c0, |_, _| Ret::default());
        e.register(0x00aa_6220, |_, _| Ret::default());
        fn_006eb310(&mut e, Ptr::new(manager));
        assert_eq!(e.mem.u32(manager + PATH_MANAGER_IMPL_STATE), 1);
        assert_eq!(e.mem.u32(manager + PATH_MANAGER_IMPL_PAUSE_RETRIES), 4);
        e.mem.set_u32(manager + PATH_MANAGER_IMPL_STATE, 3);
        fn_006eb310(&mut e, Ptr::new(manager));
        assert_eq!(e.mem.u32(manager + PATH_MANAGER_IMPL_STATE), 2);
    }

    #[test]
    fn cancellation_state_flag_queries_and_event_helper_return() {
        let mut e = Engine::new();
        let manager = 0x6000;
        e.mem.map(manager, 0x1500);
        e.mem.set_u8(manager + PATH_MANAGER_IMPL_CANCEL_PATHS, 1);
        e.mem.set_u32(manager + 0x0c2c, 0x1234);
        assert!(fn_006eb7d0(&mut e, Ptr::new(manager)));
        e.register(0x00aa_6120, |_, _| result(1));
        assert!(fn_006eb790(&mut e, Ptr::new(manager)));
    }

    #[test]
    fn update_adds_task_data_to_map_with_key_and_owner_and_removes_when_not_cancelled() {
        let mut e = Engine::new();
        let manager = 0x8000;
        let map = manager + 0x1480;
        e.mem.map(manager, 0x1500);
        e.mem.map(0x4000, 4);
        e.mem.set_u32(0x4000, 0x8765);
        const INSERT: u32 = 0x0077_1000;
        const REMOVE: u32 = 0x0077_1010;
        let map_vtable = 0x0017_0000;
        e.mem.set_u32(map, map_vtable);
        e.put_vtable(map_vtable, &[0, 0, 0, 0, INSERT, REMOVE]);
        let inserted = Rc::new(Cell::new(false));
        let inserted_by_call = inserted.clone();
        e.register_double(INSERT, move |e, args| {
            assert_eq!(args[0], map);
            assert_eq!(args[1], 0x8765);
            let task_data = e.mem.u32(args[2]);
            assert_eq!(e.mem.u32(task_data), PATHING_TASK_DATA_VTABLE);
            assert_eq!(args[3], 1);
            inserted_by_call.set(true);
            result(1)
        });
        let removed = Rc::new(Cell::new(false));
        let removed_by_call = removed.clone();
        e.register_double(REMOVE, move |_, args| {
            assert_eq!(args[0], map);
            assert_eq!(args[1], 0x8765);
            removed_by_call.set(true);
            Ret::default()
        });
        let allocated = 0x0016_0000;
        e.mem.map(allocated, 0x80);
        e.mem.map(PATHING_TASK_DATA_VTABLE, 0x20);
        e.mem.set_u32(PATHING_TASK_DATA_VTABLE, 0x0077_1020);
        let release_flags = Rc::new(Cell::new(0));
        let release_flags_for_double = release_flags.clone();
        e.register_double(0x0077_1020, move |_, args| {
            assert_eq!(args[0], allocated);
            release_flags_for_double.set(args[1]);
            Ret::default()
        });
        e.register_double(0x0040_1000, move |_, _| result(allocated));
        e.register(0x0055_9450, |_, _| result(0x8765));
        e.register(0x006c_5ec0, |_, _| Ret::default());
        noop_addresses(
            &mut e,
            &[
                0x006c_5e40,
                0x006e_cd40,
                0x006c_5e70,
                0x0045_cec0,
                0x006e_bf90,
                0x006c_7850,
                0x0063_3c90,
                0x0096_a2d0,
                0x006e_be90,
                0x006e_bee0,
                0x006e_bf60,
                0x006e_bf30,
                0x006e_5cc0,
                0x006e_bfb0,
            ],
        );
        assert!(!fn_006eac70(&mut e, Ptr::new(manager), 0x4000, 0x5000));
        assert!(inserted.get());
        assert!(removed.get());
        assert_eq!(release_flags.get(), 1);
    }

    #[test]
    fn cancel_all_drains_a_nonempty_completion_and_task_queue() {
        let mut e = Engine::new();
        let manager = 0x10000;
        let completion_queue = manager + 0x0c10;
        let map = manager + 0x1480;
        let task_data = 0x12000;
        let listener = 0x13000;
        let retained = 0x14000;
        let completion_count = Rc::new(Cell::new(0));
        let completion_count_for_double = completion_count.clone();
        const POP_COMPLETION: u32 = 0x0077_2010;
        const POP_TASK: u32 = 0x0077_2020;
        const REMOVE_KEY: u32 = 0x0077_2030;
        const RELEASE: u32 = 0x0077_2040;
        e.mem.map(manager, 0x1500);
        e.mem.map(task_data, 0x80);
        e.mem.map(listener, 8);
        e.mem.map(retained, 8);
        e.mem.set_u32(task_data + 0x60, listener);
        e.mem.set_u32(task_data + 0x1c, 0xabcdef);
        let completion_vtable = 0x0017_1000;
        let map_vtable = 0x0017_1100;
        let listener_vtable = 0x0017_1200;
        let retained_vtable = 0x0017_1300;
        let task_vtable = 0x0017_1400;
        e.mem.set_u32(completion_queue, completion_vtable);
        e.mem.set_u32(map, map_vtable);
        e.mem.set_u32(listener, listener_vtable);
        e.mem.set_u32(retained, retained_vtable);
        e.mem.set_u32(task_data, task_vtable);
        e.put_vtable(completion_vtable, &[0, 0, 0, 0, POP_COMPLETION]);
        e.put_vtable(map_vtable, &[0, 0, 0, 0, 0, REMOVE_KEY]);
        e.put_vtable(listener_vtable, &[0, POP_TASK, 0, 0, 0]);
        e.put_vtable(retained_vtable, &[RELEASE]);
        e.put_vtable(task_vtable, &[RELEASE]);
        e.register(0x0045_38a0, |_, _| Ret::default());
        e.register(0x0045_38c0, |_, _| Ret::default());
        e.register(0x006c_09d0, |_, _| Ret::default());
        e.register(0x004f_1540, |_, _| result(1));
        e.register_double(POP_COMPLETION, move |e, args| {
            let index = completion_count_for_double.get();
            completion_count_for_double.set(index + 1);
            if index == 0 {
                e.mem.set_u32(args[1], 1);
                e.mem.set_u32(args[1] + 4, retained);
                e.mem.set_u32(args[1] + 0x18, 0x9988);
                result(1)
            } else {
                Ret::default()
            }
        });
        let removed_key = Rc::new(Cell::new(false));
        let removed_key_for_double = removed_key.clone();
        e.register_double(REMOVE_KEY, move |_, args| {
            assert_eq!(args[1], 0x9988);
            removed_key_for_double.set(true);
            Ret::default()
        });
        let got_task = Rc::new(Cell::new(false));
        let got_task_for_predicate = got_task.clone();
        e.register(0x006e_c970, |_, _| Ret::default());
        e.register_double(0x006e_bde0, move |_, _| {
            if got_task_for_predicate.get() {
                result(1)
            } else {
                result(0)
            }
        });
        let got_task_for_double = got_task.clone();
        e.register_double(0x0044_c640, move |e, args| {
            if !got_task_for_double.replace(true) {
                e.mem.set_u32(args[2], task_data);
                result(1)
            } else {
                Ret::default()
            }
        });
        e.register(0x0055_9450, |e, args| result(e.mem.u32(args[0])));
        e.register(0x0055_9a40, |e, args| {
            e.mem.set_u32(args[0], e.mem.u32(args[1]));
            Ret::default()
        });
        let dispatched = Rc::new(Cell::new(false));
        let dispatched_by_double = dispatched.clone();
        e.register_double(POP_TASK, move |e, args| {
            assert_eq!(e.mem.u32(args[1]), 1);
            assert_eq!(e.mem.u32(args[1] + 4), 0xabcdef);
            dispatched_by_double.set(true);
            Ret::default()
        });
        let releases = Rc::new(Cell::new(0));
        let releases_by_double = releases.clone();
        e.register_double(RELEASE, move |_, args| {
            assert_eq!(args[1], 1);
            releases_by_double.set(releases_by_double.get() + 1);
            Ret::default()
        });
        noop_addresses(
            &mut e,
            &[
                0x0080_0190,
                0x0044_c480,
                0x00aa_6060,
                0x006c_08e0,
                0x006e_bdc0,
                0x0048_3710,
            ],
        );
        fn_006eb440(&mut e, Ptr::new(manager));
        assert_eq!(completion_count.get(), 2);
        assert!(got_task.get());
        assert!(removed_key.get());
        assert!(dispatched.get());
        assert_eq!(releases.get(), 2);
        assert_eq!(e.mem.u32(manager + PATH_MANAGER_IMPL_STATE), 2);
        assert_eq!(e.mem.u8(manager + PATH_MANAGER_IMPL_CANCEL_PATHS), 0);
    }
}

#[cfg(test)]
mod remaining_tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    fn value(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn noops(e: &mut Engine, addresses: &[u32]) {
        for address in addresses {
            e.register(*address, |_, _| Ret::default());
        }
    }

    #[test]
    fn actor_message_copy_and_path_data_construction_execute() {
        let mut e = Engine::new();
        let source = 0x3000;
        let target = 0x3100;
        let task = 0x3200;
        e.mem.map(source, 8);
        e.mem.map(target, 8);
        e.mem.map(task, 0x80);
        e.mem.set_u32(source, 7);
        noops(
            &mut e,
            &[
                0x0055_9a40,
                0x006e_5cc0,
                0x006c_7850,
                0x0063_3c90,
                0x0096_a2d0,
                0x006e_be90,
                0x006e_bee0,
                0x006e_bf60,
                0x006e_bf30,
                0x006e_bf90,
                0x006e_bfb0,
            ],
        );
        fn_006e9c30(&mut e, Ptr::new(target), Ptr::new(source));
        assert_eq!(e.mem.u32(target), 7);
        fn_006e9c60(&mut e, Ptr::new(target), Ptr::new(source));
        assert_eq!(e.mem.u32(target), 7);
        pathing_task_data_construct(&mut e, Ptr::new(task), source, source + 4);
        assert_eq!(e.mem.u32(task), PATHING_TASK_DATA_VTABLE);
        assert_eq!(e.mem.u8(task + 0x64), 0);
        fn_006e9db0(&mut e, Ptr::new(task), Ptr::new(source), source + 4);
    }

    #[test]
    fn request_path_and_task_data_destructors_run_their_cleanup_paths() {
        let mut e = Engine::new();
        let task = 0x5000;
        let request = 0x5100;
        let path = 0x5200;
        e.mem.map(task, 0x80);
        e.mem.map(request, 0x20);
        e.mem.map(path, 0x20);
        e.register(0x0044_ddc0, |_, _| value(0));
        noops(
            &mut e,
            &[
                0x0040_1030,
                0x006c_b410,
                0x006c_b4a0,
                0x006e_bf90,
                0x006e_bf10,
                0x006e_bec0,
                0x0045_cec0,
                0x00b0_0ec0,
            ],
        );
        pathing_task_data_request_deleting_destructor(&mut e, Ptr::new(request), 1);
        pathing_task_data_path_deleting_destructor(&mut e, Ptr::new(path), 1);
        pathing_task_data_scalar_deleting_destructor(&mut e, Ptr::new(task), 0);
        fn_006e9e20(&mut e, Ptr::new(task));
    }

    #[test]
    fn raycast_task_pending_and_new_request_paths_execute() {
        let mut e = Engine::new();
        let task = 0x7000;
        e.mem.map(task, 0x80);
        let alloc = Rc::new(Cell::new(0x0018_0000u32));
        let alloc_for_double = alloc.clone();
        e.register_double(0x0040_1000, move |e, args| {
            let ptr = alloc_for_double.get();
            alloc_for_double.set(ptr + 0x100);
            e.mem.map(ptr, args[0].max(0x80));
            value(ptr)
        });
        e.register(0x0044_ddc0, |_, _| value(0));
        e.register(0x006c_b3a0, |_, args| value(args[0]));
        e.register(0x006c_b480, |_, args| value(args[0]));
        noops(
            &mut e,
            &[
                0x0040_fbf0,
                0x0040_fba0,
                0x0055_9450,
                0x006d_cce0,
                0x007c_b2e0,
                0x0060_d4b0,
                0x006c_f8a0,
            ],
        );
        fn_006ea330(&mut e, Ptr::new(task));
        assert_eq!(e.mem.u8(task + 4), 1);
        assert_eq!(fn_006ea350(&mut e, Ptr::new(task)), 0);
        assert!(fn_006ea490(&mut e, Ptr::new(task)));
        assert_eq!(
            fn_006ea510(
                &mut e,
                Ptr::new(task),
                Ptr::new(0x7300),
                Ptr::new(0x7400),
                0
            ),
            0
        );
        assert_eq!(
            fn_006ea780(
                &mut e,
                Ptr::new(task),
                Ptr::new(0x7300),
                Ptr::new(0x7400),
                0,
                1
            ),
            0
        );
    }

    #[test]
    fn raycast_builder_reads_the_request_array_and_builds_nonempty_path() {
        let mut e = Engine::new();
        let task = 0x7500;
        let request = 0x7600;
        let path = 0x7700;
        let request_slot = 0x7800;
        let path_slot = 0x7810;
        e.mem.map(task, 0x80);
        e.mem.map(request, 0x68);
        e.mem.map(path, 0x18);
        e.mem.map(request_slot, 4);
        e.mem.map(path_slot, 4);
        e.mem.set_u32(request_slot, request);
        e.mem.set_u32(path_slot, path);
        let lock_entries = Rc::new(Cell::new(0));
        let lock_entries_by_double = lock_entries.clone();
        e.register_double(0x0040_fbf0, move |_, args| {
            assert_eq!(args[0], task + 0x20);
            lock_entries_by_double.set(lock_entries_by_double.get() + 1);
            Ret::default()
        });
        e.register(0x0040_fba0, |_, _| Ret::default());
        e.register_double(0x0044_ddc0, move |_, args| {
            assert_eq!(args[0], task + 0x40);
            value(1)
        });
        e.register_double(0x0087_7a30, move |_, args| {
            assert_eq!(args[1], 0);
            value(if args[0] == task + 0x40 {
                request_slot
            } else {
                assert_eq!(args[0], task + 0x50);
                path_slot
            })
        });
        let constructed = Rc::new(Cell::new(false));
        let constructed_by_double = constructed.clone();
        e.register_double(0x006e_61a0, move |_, args| {
            assert_eq!(args.len(), 1);
            constructed_by_double.set(true);
            Ret::default()
        });
        let built = Rc::new(Cell::new(false));
        let built_by_double = built.clone();
        e.register_double(0x006e_6320, move |_, args| {
            assert_eq!(args[1], request);
            assert_eq!(args[2], 0);
            assert_eq!(args[3], path);
            built_by_double.set(true);
            Ret::default()
        });
        e.register(0x004f_d3e0, |_, _| value(3));
        e.register(0x006e_62b0, |_, _| Ret::default());

        assert_eq!(fn_006ea350(&mut e, Ptr::new(task)), 3);
        assert_eq!(lock_entries.get(), 1);
        assert!(constructed.get());
        assert!(built.get());
    }

    #[test]
    fn manager_lifecycle_construct_initialize_and_destroy_paths_execute() {
        let mut e = Engine::new();
        let manager = 0x8000;
        e.mem.map(manager, 0x1500);
        noops(
            &mut e,
            &[
                0x0044_dee0,
                0x006e_c000,
                0x006e_c090,
                0x00aa_5fc0,
                0x0044_fae0,
                0x006e_c340,
                0x006e_c660,
                0x006e_c870,
                0x006e_c3f0,
                0x0044_fb00,
                0x006e_c130,
                0x006e_bd30,
                0x006c_09d0,
                0x00b0_0c20,
            ],
        );
        e.register(0x00b0_0a00, |_, _| value(0x9000));
        e.register(0x00b0_0a80, |_, _| value(1));
        e.register(0x006c_08e0, |_, _| value(1));
        path_manager_impl_construct(&mut e, Ptr::new(manager));
        assert_eq!(e.mem.u32(manager + PATH_MANAGER_IMPL_STATE), 0);
        assert!(path_manager_impl_initialize(&mut e, Ptr::new(manager)));
        path_manager_impl_destroy(&mut e, Ptr::new(manager));
        path_manager_impl_destruct(&mut e, Ptr::new(manager));
    }

    #[test]
    fn initialize_failure_does_not_reset_state() {
        let mut e = Engine::new();
        let manager = 0xa000;
        e.mem.map(manager, 0x1500);
        e.mem.set_u32(manager + PATH_MANAGER_IMPL_STATE, 3);
        e.register(0x00b0_0a00, |_, _| value(0x9000));
        e.register(0x00b0_0a80, |_, _| Ret::default());
        assert!(!path_manager_impl_initialize(&mut e, Ptr::new(manager)));
        assert_eq!(e.mem.u32(manager + PATH_MANAGER_IMPL_STATE), 3);
    }

    #[test]
    fn request_lookup_misses_return_status_two_for_both_modes() {
        let mut e = Engine::new();
        let manager = 0xb000;
        let map = manager + 0x1480;
        let vtable = 0x0019_0000;
        const LOOKUP: u32 = 0x0077_3000;
        e.mem.map(manager, 0x1500);
        e.mem.set_u32(map, vtable);
        e.put_vtable(vtable, &[0, 0, LOOKUP]);
        e.register(LOOKUP, |_, _| Ret::default());
        assert_eq!(
            fn_006eb250(
                &mut e,
                Ptr::new(manager),
                9,
                Ptr::new(0x10),
                Ptr::new(0x20),
                0,
                1
            ),
            2
        );
        assert_eq!(
            fn_006eb2b0(
                &mut e,
                Ptr::new(manager),
                9,
                Ptr::new(0x10),
                Ptr::new(0x20),
                0
            ),
            2
        );
    }

    #[test]
    fn update_advances_pause_retry_and_exits_empty_queues() {
        let mut e = Engine::new();
        let manager = 0xc000;
        e.mem.map(manager, 0x1500);
        e.mem.set_u32(manager + PATH_MANAGER_IMPL_STATE, 1);
        let queue_vtable = 0x0019_1000;
        const POP: u32 = 0x0077_3040;
        for offset in [0x0c10, 0x0c50, 0x1064] {
            e.mem.set_u32(manager + offset, queue_vtable);
        }
        e.put_vtable(queue_vtable, &[0, 0, 0, 0, POP]);
        e.register(POP, |_, _| Ret::default());
        e.register(0x0045_38a0, |_, _| Ret::default());
        e.register(0x0045_38c0, |_, _| Ret::default());
        e.register(0x00aa_6220, |_, _| Ret::default());
        e.register(0x006e_c970, |_, _| Ret::default());
        e.register(0x006e_bde0, |_, _| value(1));
        noops(
            &mut e,
            &[0x00aa_62e0, 0x006e_bdc0, 0x0045_cec0, 0x0048_3710],
        );
        fn_006eae40(&mut e, Ptr::new(manager));
        assert_eq!(e.mem.u32(manager + PATH_MANAGER_IMPL_STATE), 1);
        assert_eq!(e.mem.u32(manager + PATH_MANAGER_IMPL_PAUSE_RETRIES), 1);
    }

    #[test]
    fn request_message_routes_to_selected_embedded_queue() {
        let mut e = Engine::new();
        let manager = 0xd000;
        e.mem.map(manager, 0x1500);
        const PUSH: u32 = 0x0077_3010;
        let push_calls = Rc::new(Cell::new(0));
        let calls = push_calls.clone();
        for offset in [0x0c50, 0x1064] {
            let queue = manager + offset;
            let table = 0x0019_2000 + offset;
            e.mem.set_u32(queue, table);
            e.put_vtable(table, &[0, PUSH]);
        }
        e.register_double(PUSH, move |_, args| {
            assert_eq!(args.len(), 2);
            if args[1] == 0x1111 {
                assert_eq!(args[0], manager + 0x1064);
            } else {
                assert_eq!(args[1], 0x2222);
                assert_eq!(args[0], manager + 0x0c50);
            }
            calls.set(calls.get() + 1);
            value(0)
        });
        assert!(fn_006eb200(&mut e, Ptr::new(manager), 0x1111, 0));
        assert!(fn_006eb200(&mut e, Ptr::new(manager), 0x2222, 1));
        assert_eq!(push_calls.get(), 2);
    }

    #[test]
    fn process_cancellation_flag_false_marks_task_data_for_retry() {
        let mut e = Engine::new();
        let task = 0xe000;
        let context = 0xe100;
        e.mem.map(task, 0x80);
        e.mem.map(context, 0x0d00);
        e.register(0x0040_4eb0, |_, _| Ret::default());
        e.register(0x0040_4ee0, |_, _| Ret::default());
        e.register_double(0x0047_d0b0, move |_, _| value(context));
        e.register_double(0x0071_7e50, move |_, _| value(context));
        e.register_double(0x0055_9450, move |_, _| value(context));
        e.register(0x0048_3710, |_, _| Ret::default());
        e.register(0x00aa_6120, |_, _| Ret::default());
        pathing_task_data_process(&mut e, Ptr::new(task));
        assert_eq!(e.mem.u8(task + 4), 1);
    }

    #[test]
    fn process_incomplete_signaled_path_yields_and_signals_pause_event() {
        let mut e = Engine::new();
        let task = 0xe800;
        let manager = 0xe900;
        let path = 0xea00;
        let path_slot = 0xeb00;
        e.mem.map(task, 0x80);
        e.mem.map(manager, 0x1500);
        e.mem.map(path, 0x18);
        e.mem.map(path_slot, 4);
        e.mem.set_u32(path_slot, path);
        e.register(0x0040_4eb0, |_, _| Ret::default());
        e.register(0x0040_4ee0, |_, _| Ret::default());
        e.register_double(0x0047_d0b0, move |_, _| value(manager));
        e.register(0x0071_7e50, |_, args| value(args[0]));
        e.register(0x0048_3710, |_, _| Ret::default());
        e.register(0x0055_9450, |_, args| value(args[0]));
        e.register(0x00aa_6120, |_, _| value(1));
        let signal_count = Rc::new(Cell::new(0));
        let signal_count_for_double = signal_count.clone();
        e.register_double(0x00aa_62c0, move |_, args| {
            assert_eq!(args[0], manager + 0x0c2c);
            signal_count_for_double.set(signal_count_for_double.get() + 1);
            Ret::default()
        });
        e.register(0x0040_fbf0, |_, _| Ret::default());
        e.register(0x0040_fba0, |_, _| Ret::default());
        e.register_double(0x0044_ddc0, move |_, args| {
            if args[0] == task + 0x50 {
                value(1)
            } else {
                value(0)
            }
        });
        e.register_double(0x0087_7a30, move |_, args| {
            assert_eq!(args[0], task + 0x50);
            value(path_slot)
        });

        pathing_task_data_process(&mut e, Ptr::new(task));
        assert_eq!(e.mem.u8(task + 4), 1);
        assert_eq!(signal_count.get(), 1);
    }

    #[test]
    fn cancel_matching_request_clears_both_cancellation_words() {
        let mut e = Engine::new();
        let manager = 0xf000;
        let task_data = 0x11000;
        let cancel_data = task_data + 0x10;
        e.mem.map(manager, 0x1500);
        e.mem.map(task_data, 0x80);
        e.mem.set_u32(cancel_data, 0x5566);
        e.mem.set_u32(cancel_data + 4, 0x7788);
        e.register(0x0045_38a0, |_, _| Ret::default());
        e.register(0x0045_38c0, |_, _| Ret::default());
        e.register(0x006e_c970, |_, _| Ret::default());
        let done = Rc::new(Cell::new(false));
        let done_for_pop = done.clone();
        e.register_double(
            0x006e_bde0,
            move |_, _| if done.get() { value(1) } else { value(0) },
        );
        e.register_double(0x0044_c640, move |e, args| {
            if !done_for_pop.replace(true) {
                e.mem.set_u32(args[2], task_data);
                value(1)
            } else {
                Ret::default()
            }
        });
        e.register(0x006c_9900, |_, args| value(args[0] + 0x10));
        noops(&mut e, &[0x006e_bdc0]);
        fn_006eb6b0(&mut e, Ptr::new(manager), 0x7788);
        assert_eq!(e.mem.u32(cancel_data), 0);
        assert_eq!(e.mem.u32(cancel_data + 4), 0);
    }

    #[test]
    fn cancel_token_wrapper_and_debug_message_lifecycle_execute() {
        let mut e = Engine::new();
        let holder = 0x12000;
        let token = 0x12100;
        let debug = 0x12200;
        e.mem.map(holder, 4);
        e.mem.map(token, 4);
        e.mem.map(debug, 8);
        e.mem.set_u32(holder, token);
        let table = 0x0019_3000;
        const CANCEL: u32 = 0x0077_3020;
        e.mem.set_u32(token, table);
        e.put_vtable(table, &[0, CANCEL, 0, 0, 0]);
        e.register(0x004f_1540, |_, _| Ret::default());
        let cancelled = Rc::new(Cell::new(false));
        let cancelled_by_double = cancelled.clone();
        e.register_double(CANCEL, move |_, args| {
            assert_eq!(args[1], 2);
            cancelled_by_double.set(true);
            Ret::default()
        });
        fn_006eb670(&mut e, Ptr::new(holder), 2);
        let stopped = Rc::new(Cell::new(false));
        let stopped_by_double = stopped.clone();
        e.register_double(0x006e_b890, move |_, args| {
            assert_eq!(args[0], 0x1234);
            assert_eq!(args[1], 1);
            stopped_by_double.set(true);
            Ret::default()
        });
        noops(&mut e, &[0x0045_d260]);
        fn_006eb820(&mut e, Ptr::new(debug));
        assert_eq!(e.mem.u32(debug), 0);
        e.mem.set_u32(debug, 0x1234);
        fn_006eb850(&mut e, Ptr::new(debug));
        assert!(cancelled.get());
        assert!(stopped.get());
    }

    #[test]
    fn task_queue_and_pause_event_helpers_call_their_observed_slots() {
        let mut e = Engine::new();
        let manager = 0x13000;
        e.mem.map(manager, 0x1500);
        let queue_table = 0x0019_4000;
        const PUSH: u32 = 0x0077_3030;
        e.mem.set_u32(manager + 0x0c10, queue_table);
        e.put_vtable(queue_table, &[0, PUSH]);
        let queue_calls = Rc::new(Cell::new(0));
        let queue_calls_for_double = queue_calls.clone();
        e.register_double(PUSH, move |_, args| {
            assert_eq!(args[1], 0x9090);
            queue_calls_for_double.set(1);
            Ret::default()
        });
        let event_calls = Rc::new(Cell::new(0));
        let event_calls_for_double = event_calls.clone();
        e.register_double(0x00aa_62c0, move |_, args| {
            assert_eq!(args[0], manager + 0x0c2c);
            event_calls_for_double.set(1);
            Ret::default()
        });
        fn_006eb7f0(&mut e, Ptr::new(manager), 0x9090);
        fn_006eb7b0(&mut e, Ptr::new(manager));
        assert_eq!(queue_calls.get(), 1);
        assert_eq!(event_calls.get(), 1);
    }
}
