//! `fallout shared/bgsprimitive.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 004a7290 (decompiled, FalloutNV.exe 1.4.0.525)
/// No map name. `__fastcall` on a `float[3]`: returns its squared length
/// `z*z + y*y + x*x` in ST0, stored to a `float` local and reloaded. Computed
/// in `f64` and rounded to `f32` once, as the crate does for x87 code.
pub fn fn_004a7290(e: &mut Engine, this: Ptr) -> f32 {
    let x = e.mem.f32(this.addr()) as f64;
    let y = e.mem.f32(this.addr() + 4) as f64;
    let z = e.mem.f32(this.addr() + 8) as f64;
    (x * x + y * y + z * z) as f32
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x004a7290, fn_004a7290(Ptr) -> f32)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_the_squared_length() {
        let mut e = Engine::new();
        let v = Ptr::<()>::new(e.mem.alloc(12));
        e.mem.set_f32(v.addr(), 1.0);
        e.mem.set_f32(v.addr() + 4, 2.0);
        e.mem.set_f32(v.addr() + 8, 3.0);
        assert_eq!(e.call(0x004a_7290, &args![v]).f32(), 14.0);
    }
}
