//! Lifetime of the path manager's request and ray-cast result arrays.

use super::{layout::SimpleArray, search, FAMILY};
use crate::dispatch;
use engine_core::{args, ptr::Ptr, typed_call, Engine};

// Typed from 006ebe90
pub fn ray_request_array_new(e: &mut Engine, this: Ptr<SimpleArray>) -> Ptr<SimpleArray> {
    e.set(this, SimpleArray::vtable, 0x0106_d650);
    typed_call!(FAMILY, 0x006b_3eb0, search::init_vertices(e, this, 0, 0));
    this
}

// Typed from 006ebec0
pub fn ray_request_array_destruct(e: &mut Engine, this: Ptr<SimpleArray>) {
    e.set(this, SimpleArray::vtable, 0x0106_d650);
    typed_call!(
        dispatch::FAMILY,
        0x0084_54f0,
        dispatch::refs::array_clear(e, this.cast(), 1)
    );
}

// Typed from 006eca20
pub fn ray_request_array_delete(
    e: &mut Engine,
    this: Ptr<SimpleArray>,
    flags: u32,
) -> Ptr<SimpleArray> {
    typed_call!(FAMILY, 0x006e_bec0, ray_request_array_destruct(e, this));
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}

// Typed from 006ebee0
pub fn ray_result_array_new(e: &mut Engine, this: Ptr<SimpleArray>) -> Ptr<SimpleArray> {
    e.set(this, SimpleArray::vtable, 0x0106_d664);
    typed_call!(FAMILY, 0x006b_3eb0, search::init_vertices(e, this, 0, 0));
    this
}

// Typed from 006ebf10
pub fn ray_result_array_destruct(e: &mut Engine, this: Ptr<SimpleArray>) {
    e.set(this, SimpleArray::vtable, 0x0106_d664);
    typed_call!(
        dispatch::FAMILY,
        0x0084_54f0,
        dispatch::refs::array_clear(e, this.cast(), 1)
    );
}

// Typed from 006eca50
pub fn ray_result_array_delete(
    e: &mut Engine,
    this: Ptr<SimpleArray>,
    flags: u32,
) -> Ptr<SimpleArray> {
    typed_call!(FAMILY, 0x006e_bf10, ray_result_array_destruct(e, this));
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}
