//! The LOS pathing request (Xbox PDB names): its constructor, destructor, scalar deleting
//! destructor, copy constructor, `CopyTo`, the search-centre setter, and the save/load pair.
//!
//! The floats are only moved (read and stored, or loaded through the x87 and stored back), so
//! they are copied plain; everything else is words and the calls the translations make, by
//! address.

use super::layout::PathingRequestLOS;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The `PathingRequestLOS` vtable, at `0106d38c`.
const VTABLE: u32 = 0x0106_d38c;
/// The `float` the constructor gives `fIgnoreLocationRadius` (500.0 in the exe's data), at
/// `01013d84`.
const DEFAULT_IGNORE_RADIUS: u32 = 0x0101_3d84;
/// `006e2420` / `006e26a0`: `PathingRequest::PathingRequest` / `PathingRequest_ov2` (Xbox PDB).
const BASE_CTOR: u32 = 0x006e_2420;
const BASE_COPY_CTOR: u32 = 0x006e_26a0;
/// `006e2620`: the base class teardown the destructor ends with.
const BASE_TEARDOWN: u32 = 0x006e_2620;
/// `006e2750`: `PathingRequest::CopyTo` (Xbox PDB).
const BASE_COPY_TO: u32 = 0x006e_2750;
/// `006e3230` / `006e34f0`: `PathingRequest::SaveGame` / `LoadGame` (Xbox PDB).
const BASE_SAVE: u32 = 0x006e_3230;
const BASE_LOAD: u32 = 0x006e_34f0;
/// `00633c90` / `00559a40` / `0045cec0` / `006e5cc0`: the `NiPointer` constructor from a raw
/// pointer, copy constructor, destructor and `operator=`.
const NI_POINTER_FROM_RAW: u32 = 0x0063_3c90;
const NI_POINTER_COPY: u32 = 0x0055_9a40;
const NI_POINTER_DESTRUCT: u32 = 0x0045_cec0;
const NI_POINTER_ASSIGN: u32 = 0x006e_5cc0;
/// `0049f1c0` / `0049f1f0`: the `IgnoreLocations` array's constructor and destructor.
const ARRAY_CTOR: u32 = 0x0049_f1c0;
const ARRAY_DTOR: u32 = 0x0049_f1f0;
/// `0069cfe0`: copies the ignore-locations array `(source, dest)`; `0069cea0(array, size, keep)`:
/// sizes it on load.
const ARRAY_COPY_TO: u32 = 0x0069_cfe0;
const ARRAY_SET_SIZE: u32 = 0x0069_cea0;
/// `00979260(request)`: `fSearchCenterRadius` (`+0xD4`), in ST0; `006ab150(request, out)`: copies
/// `SearchCenter` (`+0xC8`) to `out`.
const SEARCH_CENTER_RADIUS: u32 = 0x0097_9260;
const SEARCH_CENTER: u32 = 0x006a_b150;
/// `00aa1460(block, size)` (cdecl): frees a block through the memory manager.
const DEALLOCATE: u32 = 0x00aa_1460;
/// `006e58d0`: the destructor (registered in this family).
const DESTRUCTOR: u32 = 0x006e_58d0;
/// `006e5a40`: the search-centre setter (registered in this family).
const SET_SEARCH_CENTER: u32 = 0x006e_5a40;
/// `006a7af0(array, index)`: the address of a 12-byte element.
const POINT_ELEMENT: u32 = 0x006a_7af0;
/// `00865e50(buffer, data, size, 0)` / `00865f60(buffer, value)`: write bytes / a variable-sized
/// value to the save buffer.
const SAVE_DATA: u32 = 0x0086_5e50;
const SAVE_VARIABLE: u32 = 0x0086_5f60;
/// `00864980(buffer, data, size)` / `00864a60(buffer)`: read bytes / a variable-sized value from
/// the load buffer.
const LOAD_DATA: u32 = 0x0086_4980;
const LOAD_VARIABLE: u32 = 0x0086_4a60;

/// Offsets of the members after the base.
const LOS_GRID_MAP: u32 = 0xb0;
const IGNORE_LOCATIONS: u32 = 0xb4;
const IGNORE_LOCATION_RADIUS: u32 = 0xc4;
const SEARCH_CENTER_OFFSET: u32 = 0xc8;
const SEARCH_CENTER_RADIUS_OFFSET: u32 = 0xd4;

// Typed from 006e57e0
/// `PathingRequestLOS::PathingRequestLOS` (Xbox PDB): runs the `PathingRequest` constructor,
/// stores the vtable, constructs the grid map pointer (null, `00633c90`), the ignore array and
/// the search centre, sets the ignore radius to the `float` at `01013d84` (500.0) and the search
/// radius to 0.0. Returns `this`.
pub fn new(e: &mut Engine, this: Ptr<PathingRequestLOS>) -> Ptr<PathingRequestLOS> {
    let t = this.addr();
    e.call(BASE_CTOR, &args![this]);
    e.mem.set_u32(t, VTABLE);
    e.call(NI_POINTER_FROM_RAW, &args![t + LOS_GRID_MAP, 0u32]);
    e.call(ARRAY_CTOR, &args![t + IGNORE_LOCATIONS]);
    typed_call!(
        leaf::FAMILY,
        0x0068_15c0,
        leaf::same_word(e, t + SEARCH_CENTER_OFFSET)
    );
    let radius = e.global::<f32>(DEFAULT_IGNORE_RADIUS);
    e.set(this, PathingRequestLOS::fIgnoreLocationRadius, radius);
    e.set(this, PathingRequestLOS::fSearchCenterRadius, 0.0);
    this
}

// Typed from 006e5890
/// `PathingRequestLOS::__vecDelDtor` (Xbox PDB): runs the destructor (`006e58d0`) and, when bit 0
/// of `flags` is set, frees the 0xD8 bytes (`00aa1460`). Returns `this`. `RET 4`.
pub fn scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<PathingRequestLOS>,
    flags: u32,
) -> Ptr<PathingRequestLOS> {
    e.call(DESTRUCTOR, &args![this]);
    if flags & 1 != 0 {
        e.call(DEALLOCATE, &args![this, 0xd8u32]);
    }
    this
}

// Typed from 006e58d0
/// `PathingRequestLOS::~PathingRequestLOS` (Xbox PDB): destroys the ignore array (`0049f1f0`) and
/// the grid map pointer (`0045cec0`), then runs the base teardown (`006e2620`). It does not store
/// the vtable.
pub fn destructor(e: &mut Engine, this: Ptr<PathingRequestLOS>) {
    let t = this.addr();
    e.call(ARRAY_DTOR, &args![t + IGNORE_LOCATIONS]);
    e.call(NI_POINTER_DESTRUCT, &args![t + LOS_GRID_MAP]);
    e.call(BASE_TEARDOWN, &args![this]);
}

// Typed from 006e5940
/// Unnamed in the engine map: the copy constructor: `PathingRequest_ov2(this, source)`, the
/// vtable, the grid map pointer copied (`00559a40`), the ignore array constructed, the ignore
/// radius copied, the search centre constructed, `source`'s ignore locations copied into this
/// one's (`0069cfe0`), and the search centre and radius set from `source`'s (`006ab150`,
/// `00979260`) by [`set_search_center`]. Returns `this`. `RET 4`.
pub fn copy_new(
    e: &mut Engine,
    this: Ptr<PathingRequestLOS>,
    source: Ptr<PathingRequestLOS>,
) -> Ptr<PathingRequestLOS> {
    let t = this.addr();
    let s = source.addr();
    e.call(BASE_COPY_CTOR, &args![this, source]);
    e.mem.set_u32(t, VTABLE);
    e.call(NI_POINTER_COPY, &args![t + LOS_GRID_MAP, s + LOS_GRID_MAP]);
    e.call(ARRAY_CTOR, &args![t + IGNORE_LOCATIONS]);
    let radius = e.get(source, PathingRequestLOS::fIgnoreLocationRadius);
    e.set(this, PathingRequestLOS::fIgnoreLocationRadius, radius);
    typed_call!(
        leaf::FAMILY,
        0x0068_15c0,
        leaf::same_word(e, t + SEARCH_CENTER_OFFSET)
    );
    e.call(
        ARRAY_COPY_TO,
        &args![s + IGNORE_LOCATIONS, t + IGNORE_LOCATIONS],
    );
    let search_radius = e.call(SEARCH_CENTER_RADIUS, &args![source]).f32();
    e.with_stack(0xc, |e, center| {
        let copy = e.call(SEARCH_CENTER, &args![source, center]).u32();
        let x = e.mem.u32(copy);
        let y = e.mem.u32(copy + 4);
        let z = e.mem.u32(copy + 8);
        e.call(SET_SEARCH_CENTER, &args![this, x, y, z, search_radius]);
    });
    this
}

// Typed from 006e5a40
/// Unnamed in the engine map: sets the search centre (the three words of an `NiPoint3` passed by
/// value) and the search radius. `RET 0x10`.
pub fn set_search_center(
    e: &mut Engine,
    this: Ptr<PathingRequestLOS>,
    x: u32,
    y: u32,
    z: u32,
    radius: f32,
) {
    let t = this.addr();
    e.mem.set_u32(t + SEARCH_CENTER_OFFSET, x);
    e.mem.set_u32(t + SEARCH_CENTER_OFFSET + 4, y);
    e.mem.set_u32(t + SEARCH_CENTER_OFFSET + 8, z);
    e.set(this, PathingRequestLOS::fSearchCenterRadius, radius);
}

// Typed from 006e5a80
/// `PathingRequestLOS::CopyTo` (Xbox PDB): copies the base (`PathingRequest::CopyTo`), assigns
/// the grid map pointer (`006e5cc0`), copies the ignore locations (`0069cfe0`), the ignore
/// radius and the search centre and radius ([`set_search_center`]) into `dest`. `RET 4`.
pub fn copy_to(e: &mut Engine, this: Ptr<PathingRequestLOS>, dest: Ptr<PathingRequestLOS>) {
    let t = this.addr();
    let d = dest.addr();
    e.call(BASE_COPY_TO, &args![this, dest]);
    e.call(
        NI_POINTER_ASSIGN,
        &args![d + LOS_GRID_MAP, t + LOS_GRID_MAP],
    );
    e.call(
        ARRAY_COPY_TO,
        &args![t + IGNORE_LOCATIONS, d + IGNORE_LOCATIONS],
    );
    let radius = e.get(this, PathingRequestLOS::fIgnoreLocationRadius);
    e.set(dest, PathingRequestLOS::fIgnoreLocationRadius, radius);
    let search_radius = e.get(this, PathingRequestLOS::fSearchCenterRadius);
    let x = e.mem.u32(t + SEARCH_CENTER_OFFSET);
    let y = e.mem.u32(t + SEARCH_CENTER_OFFSET + 4);
    let z = e.mem.u32(t + SEARCH_CENTER_OFFSET + 8);
    e.call(SET_SEARCH_CENTER, &args![dest, x, y, z, search_radius]);
}

// Typed from 006e5b20
/// `PathingRequestLOS::SaveGame` (Xbox PDB): saves the base, the number of ignore locations
/// (`SaveVariableSizedValue`; 0 when the array's address is 0) and each location (12 bytes),
/// then the ignore radius (4), the search centre (12) and the search radius (4). `RET 4`.
pub fn save_game(e: &mut Engine, this: Ptr<PathingRequestLOS>, buffer: Ptr) {
    let t = this.addr();
    e.call(BASE_SAVE, &args![this, buffer]);
    let array = t.wrapping_add(IGNORE_LOCATIONS);
    let count = if array == 0 {
        0
    } else {
        typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, array))
    };
    e.call(SAVE_VARIABLE, &args![buffer, count]);
    let mut index = 0u32;
    while index < count {
        let item = e.call(POINT_ELEMENT, &args![array, index]).u32();
        e.call(SAVE_DATA, &args![buffer, item, 0xcu32, 0u32]);
        index += 1;
    }
    for (offset, size) in [
        (IGNORE_LOCATION_RADIUS, 4u32),
        (SEARCH_CENTER_OFFSET, 0xc),
        (SEARCH_CENTER_RADIUS_OFFSET, 4),
    ] {
        e.call(
            SAVE_DATA,
            &args![buffer, t.wrapping_add(offset), size, 0u32],
        );
    }
}

// Typed from 006e5c00
/// `PathingRequestLOS::LoadGame` (Xbox PDB): loads what [`save_game`] saves: the base, the count
/// (`LoadVariableSizedValue`), the array sized to it (`0069cea0(array, n, 1)`) and each
/// location, then the three fields. `RET 4`.
pub fn load_game(e: &mut Engine, this: Ptr<PathingRequestLOS>, buffer: Ptr) {
    let t = this.addr();
    e.call(BASE_LOAD, &args![this, buffer]);
    let count = e.call(LOAD_VARIABLE, &args![buffer]).u32();
    let array = t.wrapping_add(IGNORE_LOCATIONS);
    e.call(ARRAY_SET_SIZE, &args![array, count, 1u32]);
    let mut index = 0u32;
    while index < count {
        let item = e.call(POINT_ELEMENT, &args![array, index]).u32();
        e.call(LOAD_DATA, &args![buffer, item, 0xcu32]);
        index += 1;
    }
    for (offset, size) in [
        (IGNORE_LOCATION_RADIUS, 4u32),
        (SEARCH_CENTER_OFFSET, 0xc),
        (SEARCH_CENTER_RADIUS_OFFSET, 4),
    ] {
        e.call(LOAD_DATA, &args![buffer, t.wrapping_add(offset), size]);
    }
}
