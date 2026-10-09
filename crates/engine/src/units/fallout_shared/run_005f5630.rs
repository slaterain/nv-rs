//! `fallout shared/run_005f5630` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 005f65d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the singly linked list starting at node `this` (value word at
/// `+0`, next node at `+4`; `this` may be null) contains a node whose value
/// equals the word at `wanted`. `__thiscall`, one stack word (`RET 4`).
pub fn fn_005f65d0(e: &mut Engine, this: Ptr, wanted: Ptr) -> bool {
    let mut node = this.addr();
    while node != 0 && e.mem.u32(node) != e.mem.u32(wanted.addr()) {
        node = e.mem.u32(node + 4);
    }
    node != 0
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x005f65d0, fn_005f65d0(Ptr, Ptr) -> bool)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(e: &mut Engine, value: u32) -> Ptr {
        let p: Ptr = Ptr::new(e.mem.alloc(4));
        e.mem.set_u32(p.addr(), value);
        p
    }

    #[test]
    fn finds_a_value_anywhere_in_the_list() {
        let mut e = Engine::new();
        let second = e.mem.alloc(8);
        e.mem.set_u32(second, 22);
        let first: Ptr = Ptr::new(e.mem.alloc(8));
        e.mem.set_u32(first.addr(), 11);
        e.mem.set_u32(first.addr() + 4, second);
        let eleven = word(&mut e, 11);
        let twenty_two = word(&mut e, 22);
        let missing = word(&mut e, 33);
        assert!(e.call(0x005f_65d0, &args![first, eleven]).bool());
        assert!(e.call(0x005f_65d0, &args![first, twenty_two]).bool());
        assert!(!e.call(0x005f_65d0, &args![first, missing]).bool());
    }

    #[test]
    fn an_empty_list_contains_nothing() {
        let mut e = Engine::new();
        let any = word(&mut e, 0);
        assert!(!e.call(0x005f_65d0, &args![Ptr::<()>::NULL, any]).bool());
    }
}
