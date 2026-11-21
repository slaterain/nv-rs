//! The shadow scene node's light at an index (`00b5ac20`) for callers that ask the indexes in
//! order. The exe walks from the list's head for every index, so a caller walking `n` lights takes
//! about `n * n / 2` list steps; [`NodeLights`] keeps the item of its last answer, so the next index
//! is one step on.

use engine_core::Engine;

/// The scene node's light list (`lLightList`, +0xb4: an `NiTPointerList`, head first) and its
/// count (`m_uiCount`, +0xbc).
const LIGHT_LIST: u32 = 0xb4;
const LIGHT_COUNT: u32 = 0xbc;

/// `00b5ac20`'s answers for indexes asked in order. The answer for `index` is the element of the
/// item `index` steps after the list's head, or 0 when `index` is not below the count (read at
/// each ask, as the exe does). An index one past the last answer's steps from the kept item;
/// index 0 is the head; any other index walks from the head as the exe does. Nothing between two
/// asks may change the list (the callers only read lights and work out floats in between).
pub(super) struct NodeLights {
    item: u32,
    index: Option<u32>,
}

impl NodeLights {
    pub(super) fn new() -> NodeLights {
        NodeLights {
            item: 0,
            index: None,
        }
    }

    /// The element at `index` of the scene node `node`'s light list.
    pub(super) fn light_at(&mut self, e: &Engine, node: u32, index: u32) -> u32 {
        if index >= e.mem.u32(node.wrapping_add(LIGHT_COUNT)) {
            return 0;
        }
        let head = e.mem.u32(node.wrapping_add(LIGHT_LIST));
        let item = match self.index {
            _ if index == 0 => head,
            Some(last) if index == last.wrapping_add(1) => e.mem.u32(self.item),
            _ => {
                let mut item = head;
                for _ in 0..index {
                    item = e.mem.u32(item);
                }
                item
            }
        };
        self.item = item;
        self.index = Some(index);
        e.mem.u32(item.wrapping_add(8))
    }
}
