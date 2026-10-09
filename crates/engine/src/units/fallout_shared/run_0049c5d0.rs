//! `fallout shared/run_0049c5d0` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// The first node of the list (`00559450`: the word at `+0` of the list).
const FIRST_NODE: u32 = 0x0055_9450;
/// Steps through the list (`0057cbe0`, `RET 4`): takes the address of a
/// node-pointer variable, advances it to the node's next pointer (the word
/// at the node's `+0`) and returns the address of the node's value (`+8`).
const NEXT_VALUE: u32 = 0x0057_cbe0;

// Translated from 0049c680 (decompiled, FalloutNV.exe 1.4.0.525)
/// Finds the first node of the list `this`, from `start` (or from the first
/// node when `start` is null), whose value word equals the word at `wanted`;
/// returns that node, or null. `__thiscall`, two stack words (`RET 8`).
pub fn fn_0049c680(e: &mut Engine, this: Ptr, wanted: Ptr, start: Ptr) -> Ptr {
    let mut node = start.addr();
    if node == 0 {
        node = e.call(FIRST_NODE, &args![this]).u32();
    }
    while node != 0 {
        let current = node;
        // The game passes the address of its local `node`, which the step
        // advances.
        let value = e.with_stack(4, |e, variable| {
            e.mem.set_u32(variable.addr(), node);
            let value = e.call(NEXT_VALUE, &args![this, variable]).u32();
            node = e.mem.u32(variable.addr());
            value
        });
        if e.mem.u32(wanted.addr()) == e.mem.u32(value) {
            return Ptr::new(current);
        }
    }
    Ptr::NULL
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x0049c680, fn_0049c680(Ptr, Ptr, Ptr) -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A list of three nodes (next at +0, value at +8) holding 10, 20, 30,
    /// with doubles for the two list functions. Returns (list, nodes).
    fn list(e: &mut Engine) -> (Ptr, [u32; 3]) {
        let nodes = [e.mem.alloc(12), e.mem.alloc(12), e.mem.alloc(12)];
        for (i, node) in nodes.iter().enumerate() {
            e.mem.set_u32(*node, nodes.get(i + 1).copied().unwrap_or(0));
            e.mem.set_u32(*node + 8, 10 * (i as u32 + 1));
        }
        let list: Ptr = Ptr::new(e.mem.alloc(4));
        e.mem.set_u32(list.addr(), nodes[0]);
        e.register(FIRST_NODE, |e, a| Ret {
            eax: e.mem.u32(a[0]),
            ..Ret::default()
        });
        e.register(NEXT_VALUE, |e, a| {
            let node = e.mem.u32(a[1]);
            e.mem.set_u32(a[1], e.mem.u32(node));
            Ret {
                eax: node + 8,
                ..Ret::default()
            }
        });
        (list, nodes)
    }

    fn wanted(e: &mut Engine, value: u32) -> Ptr {
        let p: Ptr = Ptr::new(e.mem.alloc(4));
        e.mem.set_u32(p.addr(), value);
        p
    }

    #[test]
    fn finds_the_node_holding_the_value() {
        let mut e = Engine::new();
        let (list, nodes) = list(&mut e);
        let twenty = wanted(&mut e, 20);
        let found = e.call(0x0049_c680, &args![list, twenty, Ptr::<()>::NULL]);
        assert_eq!(found.u32(), nodes[1]);
    }

    #[test]
    fn a_start_node_skips_the_earlier_ones() {
        let mut e = Engine::new();
        let (list, nodes) = list(&mut e);
        let ten = wanted(&mut e, 10);
        // Starting from the second node the first (10) is never seen.
        let found = e.call(0x0049_c680, &args![list, ten, Ptr::<()>::new(nodes[1])]);
        assert_eq!(found.u32(), 0);
        let thirty = wanted(&mut e, 30);
        let found = e.call(0x0049_c680, &args![list, thirty, Ptr::<()>::new(nodes[1])]);
        assert_eq!(found.u32(), nodes[2]);
    }

    #[test]
    fn a_missing_value_gives_null() {
        let mut e = Engine::new();
        let (list, _) = list(&mut e);
        let missing = wanted(&mut e, 99);
        assert_eq!(
            e.call(0x0049_c680, &args![list, missing, Ptr::<()>::NULL])
                .u32(),
            0
        );
    }
}
