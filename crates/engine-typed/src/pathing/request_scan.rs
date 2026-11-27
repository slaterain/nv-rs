//! Scan registered requests while holding the path manager's recursive lock.

use crate::dispatch::{
    self,
    layout::{LightLock, LightLockImage},
    refs,
};
use crate::image;
use engine_core::{
    args,
    ptr::{Inline, Ptr},
    typed_call, Engine,
};

image! {
    pub struct RequestScanner / RequestScannerImage: 0x28 {
        0x00 vtable: u32,
        0x04 _pad04: [u8; 4],
        0x08 count: u32,
        0x0c _pad0c: [u8; 0x14],
        0x20 lock: Inline<LightLock> = LightLockImage,
    }
}

// Typed from 006ec900
pub fn match_item(e: &mut Engine, this: Ptr<RequestScanner>, item: u32) -> u8 {
    let lock = this.at(RequestScanner::lock);
    typed_call!(dispatch::FAMILY, 0x0040_fbf0, refs::lock(e, lock, 0));
    let mut matched = 0;
    let mut index = 0u32;
    while index < e.get(this, RequestScanner::count) && matched == 0 {
        matched = e.vcall(this.addr(), 0x18, &args![index, item]).u8();
        index = index.wrapping_add(1);
    }
    typed_call!(dispatch::FAMILY, 0x0040_fba0, refs::unlock(e, lock));
    matched
}
