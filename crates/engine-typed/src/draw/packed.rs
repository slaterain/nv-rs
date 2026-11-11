//! The renderer's draws of a packed geometry that the render family's packed draw (`00e74840`) leaves
//! out: a triangle shape's with its partial records ([`draw_shape`], the renderer's slot 0x1b4, about
//! 64 a frame in the crowded street), a skin partition's strips ([`draw_partition`], the skinned batch
//! draw's, about 100 a frame), and the particles' camera-facing quads ([`draw_points`], the slot 0x1bc,
//! about 11 a frame, the largest translated time in the world draw), with the quad builders and the
//! two vertex declarations it uses.
//!
//! The Direct3D calls go to the device as the exe makes them. What a draw writes that is its own (the
//! data's changed flags, the renderer's copy of the drawn bound and camera vectors, the device's
//! world transform, streams and indices, the locked vertex block) goes through [`super::own`]; what
//! it reads that the frame changes (the world transform, the bound, the particles' arrays) through
//! [`super::packet`].

use super::{own, packet, packing, shapes, FAMILY};
use crate::shader::{self, accumulator, state};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::{
    fistp, x87_add, x87_fsincos32, x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32,
    x87_sub,
};
use engine_core::{args, typed_call, Engine};

/// `IDirect3DDevice9::DrawPrimitive` (slot 81) and `DrawIndexedPrimitive` (slot 82).
const DEVICE_DRAW_PRIMITIVE: u32 = 0x144;
const DEVICE_DRAW_INDEXED_PRIMITIVE: u32 = 0x148;
/// `IDirect3DDevice9::CreateVertexDeclaration` (slot 86).
const DEVICE_CREATE_VERTEX_DECLARATION: u32 = 0x158;
/// The shader's slot 0x90 that asks whether a pass is set, and the render family's `00e80580` there.
const SLOT_PASS_IS_SET: u32 = 0x90;
/// A geometry's slots for its partial-draw records (0x28) and its partial skin draw (0x2c).
const SLOT_PARTIAL_RECORDS: u32 = 0x28;
const SLOT_PARTIAL_SKIN: u32 = 0x2c;

/// The low 16 bits of a word, what a 16-bit store or a `u16` argument keeps.
#[inline(always)]
#[allow(clippy::cast_possible_truncation)]
fn low16(word: u32) -> u16 {
    word as u16
}

/// The shader's "is a pass set" (slot 0x90, its answer unused): the render family's `00e80580`
/// directly when the slot holds it, as every `NiD3DShader` class has.
#[inline(always)]
fn ask_pass_is_set(e: &mut Engine, shader: u32) {
    if crate::slot_target(e, shader, SLOT_PASS_IS_SET) == 0x00e8_0580 {
        let _: u32 = typed_call!(
            crate::render::FAMILY,
            0x00e8_0580,
            crate::render::draw::pass_is_set(e, shader)
        );
    } else {
        e.vcall(shader, SLOT_PASS_IS_SET, &args![]);
    }
}

/// A geometry's word-answering slot with no stack words (0x28, 0x2c): the leaf `00acbb70` (0) that a
/// plain shape holds there, or `00e68810` (the object itself) that the segmented shape holds at 0x28,
/// directly; another target through the vtable.
#[inline(always)]
fn geometry_slot_word(e: &mut Engine, geometry: u32, slot: u32) -> u32 {
    match crate::slot_target(e, geometry, slot) {
        0x00ac_bb70 => typed_call!(leaf::FAMILY, 0x00ac_bb70, leaf::zero_of_this(e, geometry)),
        0x00e6_8810 => typed_call!(
            crate::render::FAMILY,
            0x00e6_8810,
            crate::render::stage::this_word(e, geometry)
        ),
        _ => e.vcall(geometry, slot, &[]).u32(),
    }
}

/// One strip's primitive count: its length less 2 from the lengths array (+0x4c), or the count at
/// +0x40 without one.
#[inline(always)]
fn strip_length(e: &Engine, data: u32, strip: u32) -> u32 {
    let lengths = e.mem.u32(data.wrapping_add(0x4c));
    if lengths != 0 {
        u32::from(e.mem.u16(lengths.wrapping_add(strip.wrapping_mul(2)))).wrapping_sub(2)
    } else {
        e.mem.u32(data.wrapping_add(0x40))
    }
}

/// `DrawIndexedPrimitive` on the renderer's device (+0x288, read at each draw).
#[inline(always)]
fn draw_indexed(e: &mut Engine, renderer: u32, words: [u32; 6]) {
    let device = packet::renderer_device(e, renderer);
    e.vcall(device, DEVICE_DRAW_INDEXED_PRIMITIVE, &words);
}

// Typed from 00e745a0
/// The renderer's shape draw (its vtable +0x1b4), `thiscall(renderer, geometry)`: nothing with the byte
/// +0x6fc set or for data (+0xb8) without vertices. After the renderer's skinned test (`00e72c20`) and
/// the shader's "is a pass set", a skinned geometry draws each partition's strips (type 5 when the first
/// partition has strips, else 4), a partition without strip lengths the count at +0xc4 of the
/// geometry's slot 0x2c object when there is one; an unskinned one with an index buffer draws each
/// strip, or, when the geometry's slot 0x28 answers a draw object (the segmented shape itself), each of
/// its active records (0x14 bytes from +0xc4, active by the byte +0x10: start +0, count +0xc; one record
/// of the model data's triangle count from 0 with the byte +0xcd set); one without an index buffer one
/// `DrawPrimitive`. Then the data's changed flags keep only their top four bits.
pub fn draw_shape(e: &mut Engine, this: Ptr, geometry: Ptr) {
    let t = this.addr();
    let g = geometry.addr();
    if packet::renderer_skips(e, t) {
        return;
    }
    let data = packet::model_data(e, g);
    if shapes::vertex_count(e, data) == 0 {
        return;
    }
    let skin = packet::skin_instance(e, g);
    let skinned: bool = typed_call!(
        crate::render::FAMILY,
        0x00e7_2c20,
        crate::render::draw::is_skinned(e, this, geometry, 0u32)
    );
    let render_data = e.mem.u32(packet::model_data(e, g).wrapping_add(0x34));
    let shader = e.mem.u32(g.wrapping_add(0xc0));
    ask_pass_is_set(e, shader);
    if skinned {
        let instance = e.mem.u32(skin.wrapping_add(0xc));
        let mut remaining = e.mem.u32(instance.wrapping_add(8));
        let partitions = e.mem.u32(instance.wrapping_add(0xc));
        let kind = u32::from(e.mem.u16(partitions.wrapping_add(0x22)) != 0) + 4;
        if remaining != 0 {
            let mut at = partitions.wrapping_add(0x1c);
            loop {
                let part_data = e.mem.u32(at.wrapping_add(0xc));
                let mut strip = 0u32;
                let mut start = 0u32;
                while strip < e.mem.u32(part_data.wrapping_add(0x48)) {
                    let count = if e.mem.u32(part_data.wrapping_add(0x4c)) != 0 {
                        strip_length(e, part_data, strip)
                    } else {
                        let mut count = e.mem.u32(part_data.wrapping_add(0x40));
                        if geometry_slot_word(e, g, SLOT_PARTIAL_SKIN) != 0 {
                            let partial = geometry_slot_word(e, g, SLOT_PARTIAL_SKIN);
                            count = e.mem.u32(partial.wrapping_add(0xc4));
                        }
                        count
                    };
                    let vertices = u32::from(e.mem.u16(at));
                    let base = e.mem.u32(part_data.wrapping_add(0x38));
                    draw_indexed(e, t, [kind, base, 0, vertices, start, count]);
                    strip += 1;
                    start = start.wrapping_add(count).wrapping_add(2);
                }
                at = at.wrapping_add(0x2c);
                remaining = remaining.wrapping_sub(1);
                if remaining == 0 {
                    break;
                }
            }
        }
    } else if e.mem.u32(render_data.wrapping_add(0x34)) != 0 {
        let mut start = 0u32;
        let mut strip = 0u32;
        while strip < e.mem.u32(render_data.wrapping_add(0x48)) {
            let count = strip_length(e, render_data, strip);
            if geometry_slot_word(e, g, SLOT_PARTIAL_RECORDS) == 0 {
                let vertices = e.mem.u32(render_data.wrapping_add(0x18));
                let base = e.mem.u32(render_data.wrapping_add(0x38));
                let kind = e.mem.u32(render_data.wrapping_add(0x3c));
                draw_indexed(e, t, [kind, base, 0, vertices, start, count]);
            } else {
                let draw = geometry_slot_word(e, g, SLOT_PARTIAL_RECORDS);
                let records = if packet::draws_whole(e, draw) {
                    1
                } else {
                    e.mem.u32(draw.wrapping_add(0xc8))
                };
                let mut offset = 0u32;
                let mut left = records;
                while left != 0 {
                    let record = e.mem.u32(draw.wrapping_add(0xc4)).wrapping_add(offset);
                    if e.mem.u8(record.wrapping_add(0x10)) != 0 {
                        let (part_start, part_count) = if packet::draws_whole(e, draw) {
                            let draw_data = e.mem.u32(draw.wrapping_add(0xb8));
                            (0, u32::from(e.mem.u16(draw_data.wrapping_add(0x40))))
                        } else {
                            (e.mem.u32(record), e.mem.u32(record.wrapping_add(0xc)))
                        };
                        let vertices = e.mem.u32(render_data.wrapping_add(0x18));
                        let base = e.mem.u32(render_data.wrapping_add(0x38));
                        let kind = e.mem.u32(render_data.wrapping_add(0x3c));
                        draw_indexed(e, t, [kind, base, 0, vertices, part_start, part_count]);
                    }
                    offset = offset.wrapping_add(0x14);
                    left -= 1;
                }
            }
            start = start.wrapping_add(count).wrapping_add(2);
            strip += 1;
        }
    } else {
        let count = e.mem.u32(render_data.wrapping_add(0x40));
        let first = e.mem.u32(render_data.wrapping_add(0x38));
        let kind = e.mem.u32(render_data.wrapping_add(0x3c));
        let device = packet::renderer_device(e, t);
        e.vcall(device, DEVICE_DRAW_PRIMITIVE, &args![kind, first, count]);
    }
    own::settle_changed_flags(e, data);
}

// Typed from 00e6d310
/// The renderer's draw of one skin partition (`thiscall(renderer, partition, object, unused)`, what the
/// skinned batch draw calls for each): each of the partition's strips (+0x48) drawn indexed, type 5
/// when the object's 16-bit +0x22 is set else 4, the partition's base vertex (+0x38), the object's
/// vertex count (16-bit +0x1c), running start indices; a strip's count from the lengths (+0x4c) less
/// 2, or +0x40 without them.
pub fn draw_partition(e: &mut Engine, this: Ptr, partition: u32, object: u32, _unused: u32) {
    let t = this.addr();
    if e.mem.u32(partition.wrapping_add(0x48)) == 0 {
        return;
    }
    let mut start = 0u32;
    let mut strip = 0u32;
    loop {
        let count = strip_length(e, partition, strip);
        let kind = if e.mem.u16(object.wrapping_add(0x22)) != 0 {
            5
        } else {
            4
        };
        let vertices = u32::from(e.mem.u16(object.wrapping_add(0x1c)));
        let base = e.mem.u32(partition.wrapping_add(0x38));
        draw_indexed(e, t, [kind, base, 0, vertices, start, count]);
        strip += 1;
        start = start.wrapping_add(count).wrapping_add(2);
        if strip >= e.mem.u32(partition.wrapping_add(0x48)) {
            break;
        }
    }
}

// ---- The particles' camera-facing quads. ----

/// The renderer's index buffer maker for points (`00e8f360`, on its +0x8c0: count, second count, two
/// zeros, the "not Ex" word) and for a size (`00e8fc50`: size, 0, 0x65, the "not Ex" word), the buffer
/// data's stream count (`00e8ef10`) and declaration (`00e8f040`), the point rotation (`004b3ae0`,
/// `cdecl(out, point, rotation)`), and the message function (`00a29680`).
const POINT_INDICES: u32 = 0x00e8_f360;
const NEW_INDEX_BUFFER: u32 = 0x00e8_fc50;
const SET_STREAM_COUNT: u32 = 0x00e8_ef10;
const SET_DECLARATION: u32 = 0x00e8_f040;
const ROTATE_POINT: u32 = 0x004b_3ae0;
const REPORT: u32 = 0x00a2_9680;
/// The message the index buffer's failed lock prints (`010eddd0`, the index buffer manager's "Failed to
/// lock index buffer").
const LOCK_FAILED_MESSAGE: u32 = 0x010e_ddd0;
/// Whether the device is Direct3D 9Ex (`0126f0d0`, a byte).
const DIRECT3D_IS_EX: u32 = 0x0126_f0d0;
/// `IDirect3DIndexBuffer9::Lock` (slot 11) and `Unlock` (slot 12).
const INDEX_BUFFER_LOCK: u32 = 0x2c;
const INDEX_BUFFER_UNLOCK: u32 = 0x30;
/// The renderer's camera right and up vectors in model space, which the points draw writes
/// (+0x64c, +0x658) from the camera's (+0x634, +0x640).
const QUAD_RIGHT: u32 = 0x64c;
const QUAD_UP: u32 = 0x658;
/// The colour scale (`double` 255.0 at `0101e568`).
const SCALE_TO_BYTE: u32 = 0x0101_e568;

// Typed from 00e70140
/// The renderer's points draw (its vtable +0x1bc), `thiscall(renderer, geometry)`: draws a geometry's
/// points as camera-facing quads. Nothing with the byte +0x6fc set, or when the current property state's
/// alpha property blends (bit 0 of its +0x18) and its third word's float +0x3c is not above 0 (a NaN
/// draws), or for data without vertices. The world transform and the bound are copied into locals; the
/// data joins the geometry group; its buffer data gets one stream, four vertices and two triangles a
/// point, the type word `0x1c00000`, and the renderer the bound at +0x664. The indices: with the data's
/// index array (+0x44) six a point written into the buffer data's index buffer (made by `00e8fc50`
/// when missing), else `00e8f360`'s. The world matrix (+0x940) goes to the device and the render state,
/// the camera's right and up through the world rotation give +0x64c and +0x658, and the quads are built
/// into the stream's locked block (0x28-byte vertices with the data's byte array, else 0x24). Then the
/// property draws each pass with the stream and the indices.
pub fn draw_points(e: &mut Engine, this: Ptr, geometry: Ptr) {
    let t = this.addr();
    let g = geometry.addr();
    if packet::renderer_skips(e, t) {
        return;
    }
    let state = packet::current_property_state(e, t);
    if e.mem.u8(e.mem.u32(state).wrapping_add(0x18)) & 1 != 0 {
        let limit = x87_m32(
            e.mem
                .u32(e.mem.u32(state.wrapping_add(8)).wrapping_add(0x3c)),
        );
        // `FCOMP` of 0 with it: drawn only when 0 is below it or the compare is unordered.
        if limit <= 0.0 {
            return;
        }
    }
    e.with_stack(0x7c, |e, frame| {
        let f = frame.addr();
        let data = packet::model_data(e, g);
        packet::copy_world_transform(e, g, f + 0x48);
        let bound = packet::bound_or_empty(e, g);
        for i in 0..4 {
            let word = e.mem.u32(bound.wrapping_add(4 * i));
            e.mem.set_u32(f + 0x38 + 4 * i, word);
        }
        let count = u32::from(shapes::vertex_count(e, data));
        let second = u32::from(packet::data_vertices(e, data));
        if count == 0 {
            return;
        }
        let property = e.mem.u32(g.wrapping_add(0xc0));
        let manager = e.mem.u32(t + 0x8ac);
        let group = e.mem.u32(t + 0x8b4);
        let _: bool = typed_call!(
            shader::FAMILY,
            0x00e8_8dc0,
            state::add_object_to_group(
                e,
                manager,
                group,
                Ptr::new(data),
                Ptr::new(0),
                0u8,
                0u32,
                0u32
            )
        );
        let buffer = e.mem.u32(data.wrapping_add(0x34));
        let vertices = count * 4;
        e.call(SET_STREAM_COUNT, &args![buffer, 1u32]);
        let radius = x87_quiet_bits(e.mem.u32(f + 0x44));
        own::points_buffer_counts(e, buffer, vertices, second, count);
        let centre = [
            e.mem.u32(f + 0x38),
            e.mem.u32(f + 0x3c),
            e.mem.u32(f + 0x40),
        ];
        own::drawn_bound(e, t, centre, radius);
        e.mem.set_u32(buffer.wrapping_add(4), 0x01c0_0000);
        let not_ex = u32::from(e.mem.u8(DIRECT3D_IS_EX) == 0);
        let index_buffer = if packet::point_indices(e, data) == 0 {
            let indices = e.mem.u32(t + 0x8c0);
            e.call(
                POINT_INDICES,
                &args![indices, count, second, 0u32, 0u32, not_ex],
            )
            .u32()
        } else {
            let mut index_buffer = e.mem.u32(buffer.wrapping_add(0x34));
            let size = second * 12;
            if index_buffer == 0 {
                let indices = e.mem.u32(t + 0x8c0);
                index_buffer = e
                    .call(
                        NEW_INDEX_BUFFER,
                        &args![indices, size, 0u32, 0x65u32, not_ex],
                    )
                    .u32();
                own::points_index_buffer(e, buffer, index_buffer, second * 6, size);
                if index_buffer == 0 {
                    return;
                }
            }
            e.mem.set_u32(f + 0x1c, 0);
            let locked = e
                .vcall(
                    index_buffer,
                    INDEX_BUFFER_LOCK,
                    &args![0u32, count * 12, f + 0x1c, 0u32],
                )
                .u32() as i32;
            if locked < 0 {
                e.call(REPORT, &args![LOCK_FAILED_MESSAGE]);
                e.vcall(index_buffer, INDEX_BUFFER_UNLOCK, &args![]);
                own::release(e, index_buffer);
                return;
            }
            let array = packet::point_indices(e, data);
            let mut at = e.mem.u32(f + 0x1c);
            let mut point = 0u32;
            while point < count {
                let first = u32::from(e.mem.u16(array.wrapping_add(point * 2))) * 4;
                for value in [first, first + 1, first + 2, first, first + 2, first + 3] {
                    e.mem.set_u16(at, low16(value));
                    at = at.wrapping_add(2);
                }
                point += 1;
            }
            e.vcall(index_buffer, INDEX_BUFFER_UNLOCK, &args![]);
            index_buffer
        };
        if index_buffer == 0 {
            return;
        }
        let arrays = packet::particle_arrays(e, data);
        let render_state = e.mem.u32(t + 0x8b8);
        e.vcall(render_state, 0x2c, &args![0u32]);
        let scale = x87_quiet_bits(e.mem.u32(f + 0x78));
        let world = t + 0x940;
        typed_call!(
            shader::FAMILY,
            0x00b7_1a40,
            accumulator::make_matrix(
                e,
                world,
                Ptr::new(f + 0x48),
                Ptr::new(f + 0x6c),
                f32::from_bits(scale)
            )
        );
        own::device_world_transform(e, t, world);
        let render_state = e.mem.u32(t + 0x8b8);
        e.vcall(render_state, 0x30, &args![f + 0x48]);
        for (from, to) in [(0x634u32, QUAD_RIGHT), (0x640, QUAD_UP)] {
            let out = e
                .call(ROTATE_POINT, &args![f + 0x38, t + from, f + 0x48])
                .u32();
            own::camera_vector(e, t, to, out);
        }
        let (stride, declaration) = if arrays.bytes != 0 {
            (0x28u32, declaration_with_byte(e, this))
        } else {
            (0x24u32, declaration(e, this))
        };
        e.call(SET_DECLARATION, &args![buffer, declaration]);
        if e.mem.u32(buffer.wrapping_add(0x20)) > 0 {
            let strides = e.mem.u32(buffer.wrapping_add(0x24));
            e.mem.set_u32(strides, stride);
        }
        let streams = e.mem.u32(t + 0x8bc);
        let _: bool = typed_call!(
            FAMILY,
            0x00e8_bfa0,
            packing::remake_chip(e, Ptr::new(streams), Ptr::new(buffer), 0u32)
        );
        let stream = if e.mem.u32(buffer.wrapping_add(0x20)) > 0 {
            e.mem.u32(e.mem.u32(buffer.wrapping_add(0x28)))
        } else {
            0
        };
        let vertex_buffer = e.mem.u32(stream.wrapping_add(8));
        if vertex_buffer == 0 {
            return;
        }
        let start = e.mem.u32(stream.wrapping_add(0xc));
        let flags = e.mem.u32(stream.wrapping_add(0x10));
        let streams = e.mem.u32(t + 0x8bc);
        let base: u32 = typed_call!(
            FAMILY,
            0x00e8_bff0,
            packing::lock_vertices(
                e,
                Ptr::new(streams),
                vertex_buffer,
                start,
                vertices * stride,
                flags
            )
        );
        if base == 0 {
            return;
        }
        let points = low16(count);
        if arrays.bytes != 0 {
            build_quads_with_byte(
                e,
                this,
                base,
                arrays.positions,
                arrays.colors,
                arrays.sizes,
                arrays.scales,
                arrays.angles,
                arrays.bytes,
                points,
            );
        } else {
            build_quads(
                e,
                this,
                base,
                arrays.positions,
                arrays.colors,
                arrays.sizes,
                arrays.scales,
                arrays.angles,
                points,
            );
        }
        let streams = e.mem.u32(t + 0x8bc);
        let vertex_buffer = e.mem.u32(stream.wrapping_add(8));
        let _: bool = typed_call!(
            FAMILY,
            0x00e8_c0d0,
            packing::unlock_vertices(e, Ptr::new(streams), vertex_buffer)
        );
        let passes = e.vcall(property, 0x90, &args![]).u32();
        let property_state = g.wrapping_add(0x9c);
        for slot in [0x78u32, 0x7c, 0x80, 0x84] {
            e.vcall(property, slot, &args![property_state]);
        }
        e.vcall(property, 0x88, &args![property_state, 1u32]);
        let render_state = e.mem.u32(t + 0x8b8);
        let word_10 = e.mem.u32(buffer.wrapping_add(0x10));
        e.vcall(render_state, 0xac, &args![word_10, 0u32]);
        let stride = if e.mem.u32(buffer.wrapping_add(0x20)) > 0 {
            e.mem.u32(e.mem.u32(buffer.wrapping_add(0x24)))
        } else {
            0
        };
        let vertex_buffer = e.mem.u32(stream.wrapping_add(8));
        own::device_stream(e, t, vertex_buffer, stride);
        own::device_indices(e, t, index_buffer);
        if passes != 0 {
            loop {
                let b = |e: &Engine, offset: u32| e.mem.u32(buffer.wrapping_add(offset));
                let (kind, first, vertices, triangles) =
                    (b(e, 0x3c), b(e, 0x38), b(e, 0x18), b(e, 0x40));
                draw_indexed(e, t, [kind, first, 0, vertices, 0, triangles]);
                if e.vcall(property, 0x94, &args![]).u32() == 0 {
                    break;
                }
            }
        }
        e.vcall(property, 0x8c, &args![property_state]);
    });
}

/// One `D3DVERTEXELEMENT9` (stream, offset, type, method, usage, usage index).
type VertexElement = (u16, u16, u8, u8, u8, u8);

/// The renderer's declaration in `slot`, made on first use from `elements` (written as the stack array
/// the exe builds, `CreateVertexDeclaration` into the slot); a failed creation clears the slot and
/// answers 0.
fn made_declaration(e: &mut Engine, this: Ptr, slot: u32, elements: &[VertexElement]) -> u32 {
    let slot = this.addr().wrapping_add(slot);
    if e.mem.u32(slot) != 0 {
        return e.mem.u32(slot);
    }
    let device = packet::renderer_device(e, this.addr());
    let size = 8 * u32::try_from(elements.len()).unwrap_or(0);
    let failed = e.with_stack(size, |e, array| {
        let mut at = array.addr();
        for (stream, offset, kind, method, usage, index) in elements {
            e.mem.set_u16(at, *stream);
            e.mem.set_u16(at + 2, *offset);
            e.mem.set_u8(at + 4, *kind);
            e.mem.set_u8(at + 5, *method);
            e.mem.set_u8(at + 6, *usage);
            e.mem.set_u8(at + 7, *index);
            at += 8;
        }
        (e.vcall(
            device,
            DEVICE_CREATE_VERTEX_DECLARATION,
            &args![array, slot],
        )
        .u32() as i32)
            < 0
    });
    if failed {
        e.mem.set_u32(slot, 0);
        return 0;
    }
    e.mem.u32(slot)
}

// Typed from 00e696a0
/// The renderer's position, normal, colour and texture declaration (+0x280), `thiscall(renderer)`,
/// made on first use: `FLOAT3` position at 0, `FLOAT3` normal at 12, `D3DCOLOR` at 24, `FLOAT2` texture
/// coordinate 0 at 28, the end.
pub fn declaration(e: &mut Engine, this: Ptr) -> u32 {
    made_declaration(
        e,
        this,
        0x280,
        &[
            (0, 0, 2, 0, 0, 0),
            (0, 12, 2, 0, 3, 0),
            (0, 24, 4, 0, 10, 0),
            (0, 28, 1, 0, 5, 0),
            (0xff, 0, 0x11, 0, 0, 0),
        ],
    )
}

// Typed from 00e69790
/// [`declaration`] with a `FLOAT1` texture coordinate 1 at 36 (+0x284).
pub fn declaration_with_byte(e: &mut Engine, this: Ptr) -> u32 {
    made_declaration(
        e,
        this,
        0x284,
        &[
            (0, 0, 2, 0, 0, 0),
            (0, 12, 2, 0, 3, 0),
            (0, 24, 4, 0, 10, 0),
            (0, 28, 1, 0, 5, 0),
            (0, 36, 0, 0, 5, 1),
            (0xff, 0, 0x11, 0, 0, 0),
        ],
    )
}

/// How a quad builder's instructions take the camera vectors and the positions (the two builders
/// differ only here, and it decides which NaN an instruction keeps): `true` where the component is the
/// instruction's `float` memory operand (`x87_m32`, a signalling NaN still signalling), `false` where
/// it was loaded with `FLD` (quiet). Values the code computed and stored are never signalling.
struct QuadOperands {
    /// The rotated corner offsets with colours: up·B, right·A, up·A, right·(−B) (x, y, z each).
    rotated_colored: [bool; 12],
    /// The same without colours.
    rotated_plain: [bool; 12],
    /// The normal's z subtracts `up.x·right.y` with `up.x` loaded (`true`) or `right.y` (`false`).
    normal_z_up_loaded: bool,
    /// The fourth corner's x adds the position as the memory operand.
    corner3_x_memory: bool,
}

/// An array index as the word the exe adds (0..12 here).
#[inline(always)]
#[allow(clippy::cast_possible_truncation)]
fn idx(i: usize) -> u32 {
    i as u32
}

/// A camera component (`index` 0..3 of the vector at `vector`).
#[inline(always)]
fn quad_camera(e: &Engine, this: u32, vector: u32, index: u32) -> u32 {
    e.mem.u32(this.wrapping_add(vector).wrapping_add(4 * index))
}

/// `FLD dword` (`false`) or a `float` memory operand (`true`).
#[inline(always)]
fn quad_operand(bits: u32, memory: bool) -> f64 {
    if memory {
        x87_m32(bits)
    } else {
        x87_load32(bits)
    }
}

/// `value · component`, stored as a float.
#[inline(always)]
fn quad_times(value: f64, bits: u32, memory: bool) -> u32 {
    x87_store32(x87_mul(value, quad_operand(bits, memory)))
}

/// The packed colour of an `NiColorA` (r, g, b, a): each channel times 255.0, stored as a float and
/// converted with `FISTP` (round to nearest), packed `((a << 8 | r) << 8 | g) << 8 | b`.
fn quad_color(e: &Engine, color: u32) -> u32 {
    let scale = f64::from_bits(e.mem.u64(SCALE_TO_BYTE));
    let channel = |at: u32| {
        let product = x87_store32(x87_mul(x87_load32(e.mem.u32(at)), scale));
        let rounded: i32 = fistp(x87_load32(product));
        rounded as u32
    };
    let alpha = channel(color.wrapping_add(0xc));
    let red = channel(color);
    let green = channel(color.wrapping_add(4));
    let blue = channel(color.wrapping_add(8));
    (((((alpha << 8) | red) << 8) | green) << 8) | blue
}

/// The face normal every vertex gets: `up × right` component by component (`up.z·right.y −
/// up.y·right.z`, `right.z·up.x − up.z·right.x`, `up.y·right.x − up.x·right.y`).
fn quad_normal(e: &Engine, this: u32, ops: &QuadOperands) -> [u32; 3] {
    let r = |i| quad_camera(e, this, QUAD_RIGHT, i);
    let u = |i| quad_camera(e, this, QUAD_UP, i);
    let x = x87_sub(
        x87_mul(x87_load32(u(2)), x87_m32(r(1))),
        x87_mul(x87_load32(u(1)), x87_m32(r(2))),
    );
    let y = x87_sub(
        x87_mul(x87_load32(r(2)), x87_m32(u(0))),
        x87_mul(x87_load32(u(2)), x87_m32(r(0))),
    );
    let last = if ops.normal_z_up_loaded {
        x87_mul(x87_load32(u(0)), x87_m32(r(1)))
    } else {
        x87_mul(x87_load32(r(1)), x87_m32(u(0)))
    };
    let z = x87_sub(x87_mul(x87_load32(u(1)), x87_m32(r(0))), last);
    [x87_store32(x), x87_store32(y), x87_store32(z)]
}

/// The corner offsets of a rotated quad: `FSINCOS` of the angle (sine and cosine stored as floats),
/// `A = (sin + cos)·size`, `B = (cos − sin)·size` (floats), then `right·A + up·B` and `right·(−B) +
/// up·A`, every product and sum a float.
fn rotated_offsets(
    e: &Engine,
    this: u32,
    angle: u32,
    size: u32,
    kinds: &[bool; 12],
) -> ([u32; 3], [u32; 3]) {
    let (sin, cos) = x87_fsincos32(angle);
    let sin = x87_load32(sin);
    let cos = x87_load32(cos);
    let size = x87_load32(size);
    let a = x87_load32(x87_store32(x87_mul(x87_add(sin, cos), size)));
    let b = x87_load32(x87_store32(x87_mul(x87_sub(cos, sin), size)));
    let r = |i: usize| quad_camera(e, this, QUAD_RIGHT, idx(i));
    let u = |i: usize| quad_camera(e, this, QUAD_UP, idx(i));
    let up_b: [u32; 3] = std::array::from_fn(|i| quad_times(b, u(i), kinds[i]));
    let right_a: [u32; 3] = std::array::from_fn(|i| quad_times(a, r(i), kinds[3 + i]));
    let first: [u32; 3] =
        std::array::from_fn(|i| x87_store32(x87_add(x87_load32(right_a[i]), x87_load32(up_b[i]))));
    let up_a: [u32; 3] = std::array::from_fn(|i| quad_times(a, u(i), kinds[6 + i]));
    // `FCHS` of B, stored as a float.
    let minus_b = x87_load32(x87_store32(-b));
    let right_minus_b: [u32; 3] = std::array::from_fn(|i| quad_times(minus_b, r(i), kinds[9 + i]));
    let second: [u32; 3] = std::array::from_fn(|i| {
        x87_store32(x87_add(x87_load32(right_minus_b[i]), x87_load32(up_a[i])))
    });
    (first, second)
}

/// One quad: four vertices `stride` bytes apart (position, the normal, the colour, two texture
/// coordinates, then with `extra` its byte as a float). The corners are the position minus the first
/// offset (u 0, v 1), minus the second (1, 1), plus the first (1, 0) and plus the second (0, 0); each
/// corner reads the position again, then writes the vertex, then reads the byte.
#[allow(clippy::too_many_arguments)]
fn write_quad(
    e: &mut Engine,
    dest: u32,
    stride: u32,
    position: u32,
    color: u32,
    offsets: &([u32; 3], [u32; 3]),
    normal: &[u32; 3],
    extra: Option<u32>,
    corner3_x_memory: bool,
) {
    const ONE_BITS: u32 = 0x3f80_0000;
    let uv = [(0, ONE_BITS), (ONE_BITS, ONE_BITS), (ONE_BITS, 0), (0, 0)];
    for corner in 0..4u32 {
        let vertex = dest.wrapping_add(corner * stride);
        let offset = if corner % 2 == 0 {
            &offsets.0
        } else {
            &offsets.1
        };
        let mut xyz = [0u32; 3];
        for j in 0..3u32 {
            let p = e.mem.u32(position.wrapping_add(4 * j));
            let o = x87_load32(offset[j as usize]);
            xyz[j as usize] = if corner < 2 {
                x87_store32(x87_sub(x87_load32(p), o))
            } else {
                let memory = j == 1 || (corner == 3 && j == 0 && corner3_x_memory);
                x87_store32(x87_add(quad_operand(p, memory), o))
            };
        }
        e.mem.set_u32(vertex.wrapping_add(0x18), color);
        for j in 0..3u32 {
            e.mem.set_u32(vertex.wrapping_add(4 * j), xyz[j as usize]);
            e.mem
                .set_u32(vertex.wrapping_add(0xc + 4 * j), normal[j as usize]);
        }
        let (u, v) = uv[corner as usize];
        e.mem.set_u32(vertex.wrapping_add(0x1c), u);
        e.mem.set_u32(vertex.wrapping_add(0x20), v);
        if let Some(bytes) = extra {
            let value = e.mem.u8(bytes);
            e.mem
                .set_u32(vertex.wrapping_add(0x24), x87_store32(f64::from(value)));
        }
    }
}

/// The particles' arrays a quad builder walks, as the builders' words hold them.
struct QuadArrays {
    positions: u32,
    colors: u32,
    sizes: u32,
    scales: u32,
    angles: u32,
    extra: Option<u32>,
}

/// The body both quad builders share: a quad a point, from its position, its size (the two sizes
/// multiplied), the camera's right and up in model space and, with angles, its angle.
fn quads(
    e: &mut Engine,
    this: Ptr,
    mut dest: u32,
    arrays: QuadArrays,
    count: u16,
    stride: u32,
    ops: &QuadOperands,
) {
    let t = this.addr();
    let QuadArrays {
        mut positions,
        mut colors,
        mut sizes,
        mut scales,
        mut angles,
        mut extra,
    } = arrays;
    let normal = quad_normal(e, t, ops);
    let rotated = angles != 0;
    let with_colors = colors != 0;
    let (sum, difference) = if rotated {
        ([0u32; 3], [0u32; 3])
    } else {
        let r = |e: &Engine, i| quad_camera(e, t, QUAD_RIGHT, i);
        let u = |e: &Engine, i| quad_camera(e, t, QUAD_UP, i);
        let difference: [u32; 3] = std::array::from_fn(|i| {
            let i = idx(i);
            x87_store32(x87_sub(x87_load32(u(e, i)), x87_m32(r(e, i))))
        });
        let sum = [
            x87_store32(x87_add(x87_load32(u(e, 0)), x87_m32(r(e, 0)))),
            x87_store32(x87_add(x87_load32(r(e, 1)), x87_m32(u(e, 1)))),
            x87_store32(x87_add(x87_load32(r(e, 2)), x87_m32(u(e, 2)))),
        ];
        (sum, difference)
    };
    let mut left = u32::from(count);
    while left != 0 {
        let color = if with_colors {
            quad_color(e, colors)
        } else {
            0xffff_ffff
        };
        let size = x87_store32(x87_mul(
            x87_load32(e.mem.u32(sizes)),
            x87_m32(e.mem.u32(scales)),
        ));
        let offsets = if rotated {
            let kinds = if with_colors {
                &ops.rotated_colored
            } else {
                &ops.rotated_plain
            };
            rotated_offsets(e, t, e.mem.u32(angles), size, kinds)
        } else {
            let s = x87_load32(size);
            let first: [u32; 3] =
                std::array::from_fn(|i| x87_store32(x87_mul(x87_load32(sum[i]), s)));
            let second: [u32; 3] =
                std::array::from_fn(|i| x87_store32(x87_mul(x87_load32(difference[i]), s)));
            (first, second)
        };
        write_quad(
            e,
            dest,
            stride,
            positions,
            color,
            &offsets,
            &normal,
            extra,
            ops.corner3_x_memory,
        );
        if with_colors {
            colors = colors.wrapping_add(0x10);
        }
        sizes = sizes.wrapping_add(4);
        scales = scales.wrapping_add(4);
        if rotated {
            angles = angles.wrapping_add(4);
        }
        positions = positions.wrapping_add(0xc);
        extra = extra.map(|bytes| bytes.wrapping_add(1));
        dest = dest.wrapping_add(4 * stride);
        left -= 1;
    }
}

// Typed from 00e69ea0
/// The renderer's quad builder (`thiscall`, seven words: the vertices, the positions, the colours or
/// 0, two size arrays, the angles or 0, the count as a `u16`): a camera-facing quad a point, four
/// 0x24-byte vertices (position, normal, `D3DCOLOR`, u, v), the normal `up × right` for all, the colour
/// the packed `NiColorA` (`0xffffffff` without colours); without angles the corner offsets are
/// `(up + right)·size` and `(up − right)·size`.
#[allow(clippy::too_many_arguments)]
pub fn build_quads(
    e: &mut Engine,
    this: Ptr,
    vertices: u32,
    positions: u32,
    colors: u32,
    sizes: u32,
    scales: u32,
    angles: u32,
    count: u16,
) {
    const OPS: QuadOperands = QuadOperands {
        rotated_colored: [
            true, false, false, false, false, false, false, false, true, false, false, true,
        ],
        rotated_plain: [
            false, false, false, true, false, false, false, false, true, false, true, true,
        ],
        normal_z_up_loaded: false,
        corner3_x_memory: false,
    };
    let arrays = QuadArrays {
        positions,
        colors,
        sizes,
        scales,
        angles,
        extra: None,
    };
    quads(e, this, vertices, arrays, count, 0x24, &OPS);
}

// Typed from 00e6ab40
/// [`build_quads`] with one more array, of bytes (the seventh word, before the count): 0x28-byte
/// vertices with the point's byte as a float at +0x24.
#[allow(clippy::too_many_arguments)]
pub fn build_quads_with_byte(
    e: &mut Engine,
    this: Ptr,
    vertices: u32,
    positions: u32,
    colors: u32,
    sizes: u32,
    scales: u32,
    angles: u32,
    bytes: u32,
    count: u16,
) {
    const OPS: QuadOperands = QuadOperands {
        rotated_colored: [
            false, false, false, false, false, false, false, false, true, false, false, true,
        ],
        rotated_plain: [
            false, false, false, false, false, false, false, true, true, true, false, true,
        ],
        normal_z_up_loaded: true,
        corner3_x_memory: true,
    };
    let arrays = QuadArrays {
        positions,
        colors,
        sizes,
        scales,
        angles,
        extra: Some(bytes),
    };
    quads(e, this, vertices, arrays, count, 0x28, &OPS);
}
