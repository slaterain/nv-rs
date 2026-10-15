//! `Havok SDK/hkpmopplongrayvirtualmachine.cpp` (Xbox PDB source unit), subsystem `Havok SDK`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! Not translated here: `00dd1f30` (the recursive ray query over the bytecode tree, 3505
//! bytes), which the two query entry points below call.

#[allow(unused_imports)]
use crate::prelude::*;

/// The tree query both entry points end in (`this`, `(state, bytecode, ray, 0)`).
const QUERY_RAY_ON_TREE: u32 = 0x00dd_1f30;
/// `2^-16`, the `double` at `010d5198` that scales the extent.
const SCALE: f32 = 1.0 / 65536.0;

// Translated from 00dd1f00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the 0x20-byte record at `source` into `this`: 16 raw bytes, a word, a `float`
/// (through the x87) and two more words.
pub fn fn_00dd1f00(e: &mut Engine, this: Ptr, source: Ptr) {
    for offset in (0..0x10).step_by(4) {
        let word = e.mem.u32(source.addr() + offset);
        e.mem.set_u32(this.addr() + offset, word);
    }
    let word = e.mem.u32(source.addr() + 0x10);
    e.mem.set_u32(this.addr() + 0x10, word);
    let float = x87_quiet_bits(e.mem.u32(source.addr() + 0x14));
    e.mem.set_u32(this.addr() + 0x14, float);
    for offset in [0x18, 0x1c] {
        let word = e.mem.u32(source.addr() + offset);
        e.mem.set_u32(this.addr() + offset, word);
    }
}

/// Reads four `float` lanes.
fn lanes(e: &Engine, at: u32) -> [f32; 4] {
    [0, 4, 8, 12].map(|offset| e.mem.f32(at + offset))
}

/// The part both query entry points share: stores the tree object at `this + 0x10` and
/// `1 / extent` at `this + 0x14`, the ray at `this + 0x20` / `0x30` / `0x40` / `0x44`,
/// clears the byte at `this + 0x50`, stores `-1` (first bytecode byte is 13) or `0` at
/// `this + 0x68`, then runs the tree query with a zeroed 0x20-byte state whose word at +0x14
/// is the extent scaled by `2^-16`, and with the ray moved relative to the tree object's
/// vector at +0x10 and scaled by that value.
fn run_query(e: &mut Engine, this: Ptr, tree: Ptr, ray: Ptr) {
    let this_addr = this.addr();
    let extent = e.mem.f32(tree.addr() + 0x1c);
    e.mem.set_u32(this_addr + 0x10, tree.addr());
    e.mem
        .set_f32(this_addr + 0x14, (1.0f64 / f64::from(extent)) as f32);
    let bytecode = e.mem.u32(tree.addr() + 0x20);
    let scale = extent * SCALE;
    let origin = lanes(e, tree.addr() + 0x10);
    let start = lanes(e, ray.addr());
    let end = lanes(e, ray.addr() + 0x10);
    for offset in (0..0x10).step_by(4) {
        let from = e.mem.u32(ray.addr() + offset);
        e.mem.set_u32(this_addr + 0x20 + offset, from);
        let to = e.mem.u32(ray.addr() + 0x10 + offset);
        e.mem.set_u32(this_addr + 0x30 + offset, to);
    }
    let first = e.mem.u32(ray.addr() + 0x20);
    e.mem.set_u32(this_addr + 0x40, first);
    let second = e.mem.u32(ray.addr() + 0x24);
    e.mem.set_u32(this_addr + 0x44, second);
    e.mem.set_u8(this_addr + 0x50, 0);
    let opcode = e.mem.u8(bytecode);
    let mode = if opcode != 0x0d { 0 } else { u32::MAX };
    e.mem.set_u32(this_addr + 0x68, mode);
    e.with_stack(0x60, |e, block| {
        let state = block.addr();
        let segment = state + 0x20;
        e.mem.set_f32(state + 0x14, scale);
        for lane in 0..4u32 {
            let i = lane as usize;
            e.mem
                .set_f32(segment + 4 * lane, scale * (start[i] - origin[i]));
            e.mem
                .set_f32(segment + 0x10 + 4 * lane, scale * (end[i] - origin[i]));
        }
        e.call(
            QUERY_RAY_ON_TREE,
            &args![this, state, bytecode, segment, 0u32],
        );
    });
}

// Translated from 00dd2df0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `this + 0x54` to 1, `this + 0x64` to `collector`, `this + 0x58` to `extra` and runs
/// the shared query (see `run_query`) on `tree` and `ray`; then stores the byte at
/// `this + 0x50` in the byte `out_hit` points to.
pub fn fn_00dd2df0(
    e: &mut Engine,
    this: Ptr,
    out_hit: Ptr,
    collector: u32,
    tree: Ptr,
    ray: Ptr,
    extra: u32,
) {
    let one = x87_quiet_bits(0x3f80_0000);
    e.mem.set_u32(this.addr() + 0x54, one);
    e.mem.set_u32(this.addr() + 0x64, collector);
    e.mem.set_u32(this.addr() + 0x58, extra);
    run_query(e, this, tree, ray);
    let hit = e.mem.u8(this.addr() + 0x50);
    e.mem.set_u8(out_hit.addr(), hit);
}

// Translated from 00dd2ed0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The variant without an output byte: stores `collector` at `this + 0x64`, `second` at
/// `this + 0x5c`, `first` at `this + 0x60`, 0 at `this + 0x58`, the `float` at
/// `second + 4` at `this + 0x54`, and runs the shared query (see `run_query`).
pub fn fn_00dd2ed0(
    e: &mut Engine,
    this: Ptr,
    collector: u32,
    tree: Ptr,
    ray: Ptr,
    first: u32,
    second: Ptr,
) {
    e.mem.set_u32(this.addr() + 0x64, collector);
    e.mem.set_u32(this.addr() + 0x5c, second.addr());
    e.mem.set_u32(this.addr() + 0x60, first);
    e.mem.set_u32(this.addr() + 0x58, 0);
    let weight = x87_quiet_bits(e.mem.u32(second.addr() + 4));
    e.mem.set_u32(this.addr() + 0x54, weight);
    run_query(e, this, tree, ray);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00dd_1f00, fn_00dd1f00(Ptr, Ptr)),
        entry!(0x00dd_2df0, fn_00dd2df0(Ptr, Ptr, u32, Ptr, Ptr, u32)),
        entry!(0x00dd_2ed0, fn_00dd2ed0(Ptr, u32, Ptr, Ptr, u32, Ptr)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn record_copy_moves_all_32_bytes() {
        let mut e = Engine::new();
        let source = e.mem.alloc(0x20);
        let this = e.mem.alloc(0x20);
        for i in 0..8u32 {
            e.mem.set_u32(source + 4 * i, 0x1111_0000 + i);
        }
        e.mem.set_u32(source + 0x14, 0x3fc0_0000);
        e.call(0x00dd_1f00, &args![this, source]);
        for i in 0..8u32 {
            let expected = if i == 5 { 0x3fc0_0000 } else { 0x1111_0000 + i };
            assert_eq!(e.mem.u32(this + 4 * i), expected);
        }
    }

    /// What the tree-query double saw: (this, bytecode, last word, state words, segment, scale).
    type Seen = Rc<RefCell<Vec<(Vec<u32>, Vec<f32>, f32)>>>;

    /// A tree object with extent 4 at +0x1c and bytecode at +0x20 whose first byte is
    /// `opcode`; a ray from (3,4,5,6) to (7,8,9,10); a double for the tree query that
    /// records its arguments and the segment, and sets the hit byte.
    fn rig(e: &mut Engine, opcode: u8) -> (u32, u32, u32, Seen) {
        let this = e.mem.alloc(0x80);
        let tree = e.mem.alloc(0x40);
        let code = e.mem.alloc(8);
        e.mem.set_u8(code, opcode);
        e.mem.set_u32(tree + 0x20, code);
        e.mem.set_f32(tree + 0x1c, 4.0);
        for i in 0..4u32 {
            e.mem.set_f32(tree + 0x10 + 4 * i, 1.0 + i as f32);
        }
        let ray = e.mem.alloc(0x30);
        for i in 0..8u32 {
            e.mem.set_f32(ray + 4 * i, 3.0 + i as f32);
        }
        e.mem.set_u32(ray + 0x20, 0xaaaa);
        e.mem.set_u32(ray + 0x24, 0xbbbb);
        let seen: Seen = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&seen);
        e.register_double(QUERY_RAY_ON_TREE, move |e, a| {
            let mut words = vec![a[0], a[2], a[4]];
            words.extend((0..8).map(|i| e.mem.u32(a[1] + 4 * i)));
            let segment: Vec<f32> = (0..8).map(|i| e.mem.f32(a[3] + 4 * i)).collect();
            let scale = e.mem.f32(a[1] + 0x14);
            e.mem.set_u8(a[0] + 0x50, 7);
            log.borrow_mut().push((words, segment, scale));
            Ret::default()
        });
        (this, tree, ray, seen)
    }

    #[test]
    fn long_ray_query_fills_the_machine_and_reports_the_hit() {
        let mut e = Engine::new();
        let (this, tree, ray, seen) = rig(&mut e, 0x0d);
        let out = e.mem.alloc(4);
        let extra = 0x5555u32;
        e.call(0x00dd_2df0, &args![this, out, 0x77u32, tree, ray, extra]);
        assert_eq!(e.mem.f32(this + 0x54), 1.0);
        assert_eq!(e.mem.u32(this + 0x64), 0x77);
        assert_eq!(e.mem.u32(this + 0x58), extra);
        assert_eq!(e.mem.u32(this + 0x10), tree);
        assert_eq!(e.mem.f32(this + 0x14), 0.25);
        assert_eq!(e.mem.f32(this + 0x20), 3.0);
        assert_eq!(e.mem.f32(this + 0x3c), 10.0);
        assert_eq!(e.mem.u32(this + 0x40), 0xaaaa);
        assert_eq!(e.mem.u32(this + 0x44), 0xbbbb);
        assert_eq!(e.mem.u32(this + 0x68), u32::MAX);
        assert_eq!(e.mem.u8(out), 7);
        let seen = seen.borrow();
        let (words, segment, scale) = &seen[0];
        assert_eq!(words[0], this);
        assert_eq!(words[2], 0);
        assert!(words[3..7].iter().all(|w| *w == 0));
        let unit = 4.0 / 65536.0;
        assert_eq!(*scale, unit);
        let expected: Vec<f32> = (0..8)
            .map(|i| unit * ((3.0 + i as f32) - (1.0 + (i % 4) as f32)))
            .collect();
        assert_eq!(*segment, expected);
    }

    #[test]
    fn other_first_opcode_stores_zero_mode() {
        let mut e = Engine::new();
        let (this, tree, ray, _) = rig(&mut e, 0x30);
        let out = e.mem.alloc(4);
        e.call(0x00dd_2df0, &args![this, out, 1u32, tree, ray, 2u32]);
        assert_eq!(e.mem.u32(this + 0x68), 0);
    }

    #[test]
    fn long_ray_query_variant_uses_the_shifted_arguments() {
        let mut e = Engine::new();
        let (this, tree, ray, seen) = rig(&mut e, 0x0d);
        let second = e.mem.alloc(0x10);
        e.mem.set_f32(second + 4, 2.5);
        e.call(
            0x00dd_2ed0,
            &args![this, 0x77u32, tree, ray, 0x66u32, second],
        );
        assert_eq!(e.mem.u32(this + 0x64), 0x77);
        assert_eq!(e.mem.u32(this + 0x5c), second);
        assert_eq!(e.mem.u32(this + 0x60), 0x66);
        assert_eq!(e.mem.u32(this + 0x58), 0);
        assert_eq!(e.mem.f32(this + 0x54), 2.5);
        assert_eq!(e.mem.u32(this + 0x10), tree);
        assert_eq!(e.mem.u32(this + 0x68), u32::MAX);
        assert_eq!(seen.borrow().len(), 1);
    }
}
