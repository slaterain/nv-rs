//! Platform functions: the game's own versions are replaced by Rust
//! (docs/ENGINE_PORT_PLAN.md, "platform"). Hand-written, owned by the lead.

#[allow(unused_imports)]
use crate::prelude::*;

/// The `MemoryManager` singleton (`011f6238`); [`get_memory_manager`]
/// returns its address.
pub const MEMORY_MANAGER: u32 = 0x011f_6238;

// Translated from 00401020 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the address of the `MemoryManager` singleton.
pub fn get_memory_manager(_e: &mut Engine) -> Ptr {
    Ptr::new(MEMORY_MANAGER)
}

// Translated from 00401030 (decompiled, FalloutNV.exe 1.4.0.525)
/// `operator delete`: frees through the `MemoryManager`.
pub fn operator_delete(e: &mut Engine, block: Ptr) {
    let mm: Ptr = get_memory_manager(e);
    e.call(0x00aa_4060, &args![mm, block]);
}

// Platform: replaces 00aa3e40 (MemoryManager::Allocate (Xbox PDB), FalloutNV.exe 1.4.0.525)
/// Allocates from [`crate::Mem`]'s heap instead of the game's pools.
/// Blocks are 8-byte aligned and zero-filled.
pub fn memory_manager_allocate(e: &mut Engine, _this: Ptr, size: u32) -> Ptr {
    Ptr::new(e.mem.alloc(size))
}

// Platform: replaces 00aa4060 (MemoryManager::Deallocate (Xbox PDB), FalloutNV.exe 1.4.0.525)
/// Frees a block from [`memory_manager_allocate`]; null does nothing.
pub fn memory_manager_deallocate(e: &mut Engine, _this: Ptr, block: Ptr) {
    e.mem.free(block.addr());
}

pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00401020, get_memory_manager() -> Ptr),
        entry!(0x00401030, operator_delete(Ptr)),
        entry!(0x00aa3e40, memory_manager_allocate(Ptr, u32) -> Ptr),
        entry!(0x00aa4060, memory_manager_deallocate(Ptr, Ptr)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_then_delete_through_the_game_calls() {
        let mut e = Engine::new();
        let mm = MEMORY_MANAGER;
        let p: Ptr = e.call_as(0x00aa_3e40, &args![mm, 24u32]);
        assert_eq!(e.mem.block_size(p.addr()), Some(24));
        e.call(0x0040_1030, &args![p]);
        assert_eq!(e.mem.block_size(p.addr()), None);
    }
}
