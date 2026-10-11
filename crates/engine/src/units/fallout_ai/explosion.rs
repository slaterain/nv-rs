//! `fallout/ai/combat/explosion.cpp` (Xbox PDB source unit), subsystem `fallout/ai`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00471840, fn_00471840(Ptr) -> u32),
        entry!(0x009ac170, fn_009ac170(Ptr, Ptr)),
        entry!(0x009ac190, fn_009ac190(Ptr) -> Ptr),
        entry!(0x009ac1d0, fn_009ac1d0(Ptr, u32) -> Ptr),
        entry!(0x009ac550, fn_009ac550(Ptr) -> f32),
        entry!(0x009ac910, fn_009ac910(Ptr, u32) -> Ptr),
        entry!(0x009ac940, fn_009ac940(Ptr)),
        entry!(0x009ac9a0, fn_009ac9a0(Ptr)),
        entry!(0x009acce0, fn_009acce0(Ptr) -> Ptr),
        entry!(0x009acd00, explosion_attach_distant(Ptr)),
        entry!(0x009ad5a0, fn_009ad5a0(Ptr) -> f32),
        entry!(0x009ad5f0, fn_009ad5f0(Ptr) -> bool),
        entry!(0x009ad610, fn_009ad610(Ptr) -> Ptr),
        entry!(0x009ae0f0, fn_009ae0f0(Ptr)),
        entry!(0x009ae240, fn_009ae240(Ptr) -> Ptr),
        entry!(0x009ae250, fn_009ae250(Ptr, u32) -> Ptr),
        entry!(0x009ae540, fn_009ae540(Ptr, Ptr)),
        entry!(0x009ae580, fn_009ae580(Ptr, f32)),
        entry!(0x009afbb0, fn_009afbb0(Ptr, u32) -> Ptr),
        entry!(0x009afc10, fn_009afc10(Ptr)),
        entry!(0x009afc30, fn_009afc30(Ptr)),
        entry!(0x009afc50, fn_009afc50(Ptr)),
        entry!(0x009afc70, fn_009afc70(Ptr, u32) -> u32),
        entry!(0x009afca0, fn_009afca0(Ptr, Ptr)),
    ]
}

// Translated from 00471840 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkLiquidAction::QCinfoSize` (Xbox PDB): returns the fixed size 0x18.
pub fn fn_00471840(_e: &mut Engine, _this: Ptr) -> u32 {
    0x18
}

// Translated from 009ac170 (decompiled, FalloutNV.exe 1.4.0.525)
/// Assigns the actor-cause pointer held at `this+0xD0` through the engine's
/// reference-pointer helper.
pub fn fn_009ac170(e: &mut Engine, this: Ptr, cause: Ptr) {
    e.call(0x0042_f780, &args![this.addr() + 0xd0, cause]);
}

// Translated from 009ac190 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the pointer at `this+0xDC`, advanced by 0x0C when non-null.
pub fn fn_009ac190(e: &mut Engine, this: Ptr) -> Ptr {
    let pointer = e.mem.u32(this.addr() + 0xdc);
    Ptr::new(if pointer == 0 { 0 } else { pointer + 0x0c })
}

// Translated from 009ac1d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scalar deleting destructor wrapper: runs the destructor and frees `this`
/// when bit 0 of `flags` is set.
pub fn fn_009ac1d0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x009a_c5a0, &args![this]);
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}

// Translated from 009ac550 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSExplosion::GetImageSpaceRadiusBSUnits` (Xbox PDB): returns the stored
/// radius when the unit flag is set; otherwise scales by the game setting.
pub fn fn_009ac550(e: &mut Engine, this: Ptr) -> f32 {
    if e.call(0x0047_7950, &args![this, 1u32]).bool() {
        return e.mem.f32(this.addr() + 0x8c);
    }
    let setting = e.call(0x0040_3e20, &args![0x011d_1218u32]).ptr::<u32>();
    ((e.mem.f32(this.addr() + 0x8c) as f64) * (e.mem.f32(setting.addr()) as f64)) as f32
}

// Translated from 009ac910 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scalar deleting destructor wrapper for the embedded ATL module object.
pub fn fn_009ac910(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x009a_c940, &args![this]);
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}

// Translated from 009ac940 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destroys the ATL module subobject at `this+8`.
pub fn fn_009ac940(e: &mut Engine, this: Ptr) {
    let module = this.addr() + 8;
    e.call(0x0047_0470, &args![module]);
    e.call(0x0046_ffb0, &args![module]);
}

// Translated from 009ac9a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the object's virtual function at byte offset 0xC4 with argument 1.
pub fn fn_009ac9a0(e: &mut Engine, this: Ptr) {
    e.vcall(this.addr(), 0xc4, &args![1u32]);
}

// Translated from 009acce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the word at `this+0xC8`.
pub fn fn_009acce0(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0xc8))
}

// Translated from 009acd00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Attaches this explosion to distant terrain when its virtual target exists.
pub fn explosion_attach_distant(e: &mut Engine, this: Ptr) {
    let target = e.vcall(this.addr(), 0x1d0, &args![]).u32();
    if target == 0 {
        return;
    }
    let tes = e.call(0x004f_d3e0, &args![0x011d_ea10u32]).u32();
    let world = e.call(0x0058_6170, &args![tes]).u32();
    let terrain = e.call(0x0055_85e0, &args![world]).u32();
    if terrain != 0 {
        e.vcall(terrain, 0xdc, &args![target, 1u32]);
    }
}

// Translated from 009ad5a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSExplosion::GetRadiationRadiusBSUnits` (Xbox PDB), using the same
/// unit-flag branch as the image-space radius accessor.
pub fn fn_009ad5a0(e: &mut Engine, this: Ptr) -> f32 {
    if e.call(0x0047_7950, &args![this, 1u32]).bool() {
        return e.mem.f32(this.addr() + 0xa0);
    }
    let setting = e.call(0x0040_3e20, &args![0x011d_1218u32]).ptr::<u32>();
    ((e.mem.f32(this.addr() + 0xa0) as f64) * (e.mem.f32(setting.addr()) as f64)) as f32
}

// Translated from 009ad5f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns true when the cell pointer returned by the callee is null.
pub fn fn_009ad5f0(e: &mut Engine, this: Ptr) -> bool {
    e.call(0x008d_6f30, &args![this]).u32() == 0
}

// Translated from 009ad610 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the 32-bit word at `this+0xCC`.
pub fn fn_009ad610(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0xcc))
}

// Translated from 009ae0f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Initializes a four-float point record from executable constants.
pub fn fn_009ae0f0(e: &mut Engine, this: Ptr) {
    e.call(0x0053_7bb0, &args![this]);
    for (offset, address) in [
        (8, 0x0103_0020),
        (12, 0x0101_e580),
        (16, 0x0101_e6ec),
        (20, 0x0101_e580),
    ] {
        let value: f32 = e.global(address);
        e.mem.set_f32(this.addr() + offset, value);
    }
}

// Translated from 009ae240 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkLiquidAction::GetRTTI` (Xbox PDB): returns its RTTI descriptor address.
pub fn fn_009ae240(_e: &mut Engine, _this: Ptr) -> Ptr {
    Ptr::new(0x0126_8420)
}

// Translated from 009ae250 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkLiquidAction` (Xbox PDB) scalar deleting destructor wrapper; the
/// allocation size passed to the engine's delete helper is 0x40.
pub fn fn_009ae250(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x00cb_c920, &args![this]);
    if flags & 1 != 0 {
        e.call(0x00aa_1460, &args![this, 0x40u32]);
    }
    this
}

// Translated from 009ae540 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies three floats from `source` and writes zero to the fourth component.
pub fn fn_009ae540(e: &mut Engine, this: Ptr, source: Ptr) {
    for offset in [0, 4, 8] {
        let value = e.mem.f32(source.addr() + offset);
        e.mem.set_f32(this.addr() + offset, value);
    }
    e.mem.set_f32(this.addr() + 0x0c, 0.0);
}

// Translated from 009ae580 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds a setting-derived step to `this+0x88` and forwards it through the
/// object's virtual update slot at byte offset 0x2F8.
pub fn fn_009ae580(e: &mut Engine, this: Ptr, _unused_1: f32) {
    let delta = e.call(0x0084_d030, &args![0x011f_6394u32]).f32();
    e.vcall(this.addr(), 0x2f8, &args![delta]);
    let old = e.mem.f32(this.addr() + 0x88);
    e.mem
        .set_f32(this.addr() + 0x88, (old as f64 + delta as f64) as f32);
}

// Translated from 009afbb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scalar deleting destructor wrapper for the Havok contact collector.
pub fn fn_009afbb0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x009a_fc10, &args![this]);
    if flags & 1 != 0 {
        e.call(0x009a_fbe0, &args![this]);
    }
    this
}

// Translated from 009afc10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Releases the collector's base object through its base destructor.
pub fn fn_009afc10(e: &mut Engine, this: Ptr) {
    e.call(0x0062_1100, &args![this]);
}

// Translated from 009afc30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destroys the embedded object at `this+8`.
pub fn fn_009afc30(e: &mut Engine, this: Ptr) {
    e.call(0x0045_8b50, &args![this.addr() + 8]);
}

// Translated from 009afc50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destroys the embedded object at `this+0x14`.
pub fn fn_009afc50(e: &mut Engine, this: Ptr) {
    e.call(0x0045_8b50, &args![this.addr() + 0x14]);
}

// Translated from 009afc70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Asserts the embedded object's `this+0x14` value is positive, then returns
/// the second argument unchanged.
pub fn fn_009afc70(e: &mut Engine, this: Ptr, value: u32) -> u32 {
    let count = e.call(0x0045_8b50, &args![this.addr() + 0x14]).i32();
    e.call(0x0062_2570, &args![value, u32::from(count > 0)]);
    value
}

// Translated from 009afca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls the function pointer stored at 0x01268430 with `(point, this)`.
pub fn fn_009afca0(e: &mut Engine, this: Ptr, point: Ptr) {
    let target = e.global::<u32>(0x0126_8430);
    e.call(target, &args![point, this]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_with(addrs: &[u32]) -> Engine {
        let mut e = Engine::new();
        e.call_log = Some(vec![]);
        for &addr in addrs {
            e.register(addr, |_, _| Ret::default());
        }
        e
    }

    fn mapped(e: &mut Engine, address: u32) -> Ptr {
        e.map(address & !0xfff, 0x1000);
        Ptr::new(address)
    }

    #[test]
    fn qcinfo_size_is_fixed() {
        assert_eq!(fn_00471840(&mut Engine::new(), Ptr::new(0)), 0x18);
    }

    #[test]
    fn actor_cause_release_passes_adjusted_member_and_pointer() {
        let mut e = engine_with(&[0x0042_f780]);
        fn_009ac170(&mut e, Ptr::new(0x1000), Ptr::new(0x2000));
        assert_eq!(e.call_log.unwrap()[0].1, vec![0x10d0, 0x2000]);
    }

    #[test]
    fn exploded_reference_adjusts_non_null_pointer() {
        let mut e = Engine::new();
        let this = mapped(&mut e, 0x1000);
        e.mem.set_u32(this.addr() + 0xdc, 0x2000);
        assert_eq!(fn_009ac190(&mut e, this).addr(), 0x200c);
        e.mem.set_u32(this.addr() + 0xdc, 0);
        assert!(fn_009ac190(&mut e, this).is_null());
    }

    #[test]
    fn scalar_destructor_frees_only_when_flag_is_set() {
        let mut e = engine_with(&[0x009a_c5a0, 0x0040_1030]);
        fn_009ac1d0(&mut e, Ptr::new(0x1000), 0);
        assert_eq!(e.call_log.unwrap().len(), 1);
    }

    #[test]
    fn image_space_radius_uses_stored_value_when_flag_is_set() {
        let mut e = Engine::new();
        e.register(0x0047_7950, |_, _| true.into_ret());
        let this = mapped(&mut e, 0x1000);
        e.mem.set_f32(this.addr() + 0x8c, 42.5);
        assert_eq!(fn_009ac550(&mut e, this), 42.5);
    }

    #[test]
    fn atl_scalar_destructor_honors_flag() {
        let mut e = engine_with(&[0x009a_c940, 0x0040_1030]);
        fn_009ac910(&mut e, Ptr::new(0x1000), 1);
        assert_eq!(e.call_log.unwrap().len(), 2);
    }

    #[test]
    fn atl_destruction_uses_adjusted_member() {
        let mut e = engine_with(&[0x0047_0470, 0x0046_ffb0]);
        fn_009ac940(&mut e, Ptr::new(0x1000));
        assert_eq!(e.call_log.unwrap()[0].1[0], 0x1008);
    }

    #[test]
    fn explosion_activation_uses_slot_c4_with_one() {
        let mut e = Engine::new();
        let this = mapped(&mut e, 0x1000);
        e.map(0x3000, 0x1000);
        e.mem.set_u32(this.addr(), 0x3000);
        e.mem.set_u32(0x30c4, 0x6000_00c4);
        e.register(0x6000_00c4, |_, a| a[0].into_ret());
        e.call_log = Some(vec![]);
        fn_009ac9a0(&mut e, this);
        assert_eq!(e.call_log.unwrap().last().unwrap().1, vec![0x1000, 1]);
    }

    #[test]
    fn word_at_c8_is_returned() {
        let mut e = Engine::new();
        let this = mapped(&mut e, 0x1000);
        e.mem.set_u32(this.addr() + 0xc8, 0x2000);
        assert_eq!(fn_009acce0(&mut e, this).addr(), 0x2000);
    }

    #[test]
    fn attach_distant_stops_without_target() {
        let mut e = Engine::new();
        let this = mapped(&mut e, 0x1000);
        e.map(0x3000, 0x1000);
        e.mem.set_u32(this.addr(), 0x3000);
        e.mem.set_u32(0x31d0, 0x6000_01d0);
        e.register(0x6000_01d0, |_, _| 0u32.into_ret());
        e.call_log = Some(vec![]);
        explosion_attach_distant(&mut e, this);
        assert_eq!(e.call_log.unwrap().len(), 1);
    }

    #[test]
    fn radiation_radius_uses_stored_value_when_flag_is_set() {
        let mut e = Engine::new();
        e.register(0x0047_7950, |_, _| true.into_ret());
        let this = mapped(&mut e, 0x1000);
        e.mem.set_f32(this.addr() + 0xa0, 20.0);
        assert_eq!(fn_009ad5a0(&mut e, this), 20.0);
    }

    #[test]
    fn null_cell_is_reported_as_true() {
        let mut e = Engine::new();
        e.register(0x008d_6f30, |_, _| 0u32.into_ret());
        assert!(fn_009ad5f0(&mut e, Ptr::new(0x1000)));
    }

    #[test]
    fn word_at_cc_is_returned() {
        let mut e = Engine::new();
        let this = mapped(&mut e, 0x1000);
        e.mem.set_u32(this.addr() + 0xcc, 1);
        assert_eq!(fn_009ad610(&mut e, this).addr(), 1);
    }

    #[test]
    fn point_record_constructor_copies_constants() {
        let mut e = engine_with(&[0x0053_7bb0]);
        let this = mapped(&mut e, 0x1000);
        e.map(0x0101_e000, 0x1000);
        e.map(0x0103_0000, 0x1000);
        e.set_global(0x0103_0020, 1.0f32);
        e.set_global(0x0101_e580, 2.0f32);
        e.set_global(0x0101_e6ec, 3.0f32);
        fn_009ae0f0(&mut e, this);
        assert_eq!(e.mem.f32(this.addr() + 0x10), 3.0);
        assert_eq!(e.mem.f32(this.addr() + 0x14), 2.0);
    }

    #[test]
    fn rtti_accessor_returns_constant_address() {
        assert_eq!(
            fn_009ae240(&mut Engine::new(), Ptr::new(0)).addr(),
            0x0126_8420
        );
    }

    #[test]
    fn collector_scalar_destructor_honors_flag() {
        let mut e = engine_with(&[0x00cb_c920, 0x00aa_1460]);
        fn_009ae250(&mut e, Ptr::new(0x1000), 1);
        assert_eq!(e.call_log.unwrap().len(), 2);
    }

    #[test]
    fn point_copy_writes_zero_w_component() {
        let mut e = Engine::new();
        let this = mapped(&mut e, 0x1000);
        let source = mapped(&mut e, 0x2000);
        e.mem.set_f32(source.addr(), 1.0);
        e.mem.set_f32(source.addr() + 4, -2.0);
        e.mem.set_f32(source.addr() + 8, 3.5);
        fn_009ae540(&mut e, this, source);
        assert_eq!(e.mem.f32(this.addr() + 8), 3.5);
        assert_eq!(e.mem.f32(this.addr() + 12), 0.0);
    }

    #[test]
    fn frame_step_calls_virtual_update_and_adds_delta() {
        let mut e = Engine::new();
        e.map(0x3000, 0x1000);
        let this = Ptr::new(e.mem.alloc(0x100));
        e.mem.set_u32(this.addr(), 0x3000);
        e.mem.set_u32(0x3000 + 0x2f8, 0x6000_02f8);
        e.register(0x0084_d030, |_, _| 2.5f32.into_ret());
        e.register(0x6000_02f8, |_, a| a[1].into_ret());
        e.call_log = Some(vec![]);
        fn_009ae580(&mut e, this, 9.0);
        assert_eq!(e.mem.f32(this.addr() + 0x88), 2.5);
        assert!(e
            .call_log
            .unwrap()
            .iter()
            .any(|(address, _)| *address == 0x6000_02f8));
    }

    #[test]
    fn collector_base_destructor_is_called() {
        let mut e = engine_with(&[0x0062_1100]);
        fn_009afc10(&mut e, Ptr::new(0x1000));
        assert_eq!(e.call_log.unwrap().len(), 1);
    }

    #[test]
    fn contact_collector_deleting_destructor_honors_flag() {
        let mut e = engine_with(&[0x009a_fc10, 0x009a_fbe0]);
        let this = Ptr::new(0x1000);
        assert_eq!(fn_009afbb0(&mut e, this, 0).addr(), this.addr());
        assert_eq!(e.call_log.unwrap().len(), 1);
    }

    #[test]
    fn embedded_destructor_adjusts_this_by_eight() {
        let mut e = engine_with(&[0x0045_8b50]);
        fn_009afc30(&mut e, Ptr::new(0x1000));
        assert_eq!(e.call_log.unwrap()[0].1[0], 0x1008);
    }

    #[test]
    fn proxy_destructor_adjusts_this_by_twenty() {
        let mut e = engine_with(&[0x0045_8b50]);
        fn_009afc50(&mut e, Ptr::new(0x1000));
        assert_eq!(e.call_log.unwrap()[0].1[0], 0x1014);
    }

    #[test]
    fn proxy_assertion_returns_input() {
        let mut e = engine_with(&[0x0045_8b50, 0x0062_2570]);
        let value = 0x1234;
        assert_eq!(fn_009afc70(&mut e, Ptr::new(0x1000), value), value);
        assert_eq!(e.call_log.unwrap().len(), 2);
    }

    #[test]
    fn contact_callback_uses_global_target_and_ordered_arguments() {
        let mut e = engine_with(&[0x7000]);
        e.map(0x0126_8000, 0x1000);
        e.set_global(0x0126_8430, 0x7000);
        fn_009afca0(&mut e, Ptr::new(0x1000), Ptr::new(0x2000));
        assert_eq!(e.call_log.unwrap().last().unwrap().1, vec![0x2000, 0x1000]);
    }
}
