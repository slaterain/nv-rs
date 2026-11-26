//! The differential tests of the interface family (`engine_typed::interface`): every typed function
//! against its translation (`engine-fallout-interface`), one Diff each, in the shared world of
//! `typed_interface/arena.rs` (an arena of objects whose words are zero, small numbers, floats,
//! vtables of logging stand-ins and pointers to later objects; the exe's data pages filled alike; a
//! logging stand-in at every address the native code still calls; the words it reads in place set
//! per seed). The family's own functions run for real on both sides (the translation's plain calls
//! of them are listed per test); what the native code does in place runs as its translation on the
//! translation's side and is left out of that side's calls.
//!
//! The files' own unit tests (in `engine-fallout-interface`) keep checking the behaviour on real
//! objects; these check that the native code does what the translation does, call for call and
//! write for write.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::Diff;
use engine_typed::interface;

#[path = "typed_interface/arena.rs"]
mod arena;
use arena::{Arg, InArena};

/// Every address the native code calls by address (and the translations call): a logging stand-in
/// in each test's world.
const CALLEES: &[u32] = &[
    0x0040_1000,
    0x0040_1030,
    0x0040_37d0,
    0x0040_37f0,
    0x0040_4010,
    0x0040_4390,
    0x0040_44d0,
    0x0040_46f0,
    0x0040_48e0,
    0x0040_4a70,
    0x0040_4dc0,
    0x0040_4eb0,
    0x0040_4ee0,
    0x0040_6620,
    0x0040_6d00,
    0x0040_6d30,
    0x0040_6d50,
    0x0040_6d90,
    0x0040_6f60,
    0x0040_8b20,
    0x0040_9480,
    0x0040_fba0,
    0x0040_fbf0,
    0x0043_8390,
    0x0043_9410,
    0x0043_aaf0,
    0x0043_b230,
    0x0043_cfd0,
    0x0043_d090,
    0x0044_31f0,
    0x0044_59e0,
    0x0044_7080,
    0x0044_a670,
    0x0044_f560,
    0x0045_0f90,
    0x0045_6a20,
    0x0045_7ba0,
    0x0045_7fe0,
    0x0045_a5e0,
    0x0045_cec0,
    0x0046_9880,
    0x0046_ffb0,
    0x0047_02f0,
    0x0047_0470,
    0x0047_6b20,
    0x0047_e040,
    0x0047_e0e0,
    0x0048_3a00,
    0x0048_73d0,
    0x0048_ebc0,
    0x0048_f200,
    0x004a_4040,
    0x004a_d050,
    0x004a_d270,
    0x004b_5bf0,
    0x004b_71d0,
    0x004b_75a0,
    0x004b_c810,
    0x004b_cdb0,
    0x004b_d400,
    0x004b_d510,
    0x004b_d570,
    0x004b_d740,
    0x004b_d820,
    0x004b_da70,
    0x004b_e060,
    0x004b_e2d0,
    0x004b_e330,
    0x004b_e380,
    0x004b_f220,
    0x004b_f630,
    0x004c_8f30,
    0x004c_a330,
    0x004d_e490,
    0x004e_4430,
    0x004e_ef00,
    0x004f_8a30,
    0x004f_e160,
    0x0050_3a70,
    0x0052_5980,
    0x0052_5b20,
    0x0055_d520,
    0x0056_7790,
    0x0057_5510,
    0x0057_5610,
    0x0057_6260,
    0x0057_85e0,
    0x0057_c730,
    0x0057_dd70,
    0x005a_3370,
    0x005a_e3d0,
    0x005b_5e40,
    0x005c_5420,
    0x005e_02b0,
    0x005e_58f0,
    0x0063_f7b0,
    0x0064_3c90,
    0x0064_50f0,
    0x0064_5380,
    0x0064_6360,
    0x0064_8b50,
    0x0066_29f0,
    0x0066_b0d0,
    0x0066_ef20,
    0x006b_3eb0,
    0x006f_fe10,
    0x0070_0320,
    0x0070_10e0,
    0x0070_4390,
    0x0070_5fc0,
    0x0070_5fe0,
    0x0070_9430,
    0x0070_9fd0,
    0x0070_a0b0,
    0x0070_b790,
    0x0070_b810,
    0x0070_b870,
    0x0070_b8f0,
    0x0070_bc20,
    0x0070_c4a0,
    0x0071_1ea0,
    0x0071_2450,
    0x0071_33f0,
    0x0071_3c70,
    0x0071_3de0,
    0x0071_3f00,
    0x0071_3fb0,
    0x0071_4c60,
    0x0071_4d90,
    0x0071_4fd0,
    0x0071_51b0,
    0x0071_5860,
    0x0071_5d40,
    0x0071_5da0,
    0x0071_5e00,
    0x0071_5ec0,
    0x0071_6010,
    0x0071_60b0,
    0x0071_6320,
    0x0071_64c0,
    0x0071_65d0,
    0x0071_6a10,
    0x0071_7280,
    0x0071_7600,
    0x0071_78a0,
    0x0071_7990,
    0x0071_7e30,
    0x0071_81d0,
    0x0071_87a0,
    0x0071_8890,
    0x0071_b160,
    0x0071_d0a0,
    0x0071_d580,
    0x0071_dec0,
    0x0072_69d0,
    0x0072_6df0,
    0x0072_6ff0,
    0x0072_8a30,
    0x0072_ceb0,
    0x0072_d250,
    0x0073_0690,
    0x0073_14c0,
    0x0073_15e0,
    0x0073_3500,
    0x0073_acb0,
    0x0073_ad50,
    0x0073_b7e0,
    0x0074_0f30,
    0x0075_3230,
    0x0075_3420,
    0x0075_4b40,
    0x0075_4cf0,
    0x0075_77f0,
    0x0075_7b70,
    0x0075_b0d0,
    0x0075_b310,
    0x0075_b750,
    0x0075_c280,
    0x0075_f070,
    0x0076_17a0,
    0x0076_1a20,
    0x0076_3ff0,
    0x0076_4200,
    0x0076_54f0,
    0x0076_5b80,
    0x0076_b630,
    0x0076_bb40,
    0x0076_bfe0,
    0x0077_1700,
    0x0077_28f0,
    0x0077_2d10,
    0x0077_3050,
    0x0077_3190,
    0x0077_4fd0,
    0x0077_5380,
    0x0077_5920,
    0x0077_5a00,
    0x0077_f0d0,
    0x0077_f270,
    0x0077_f2f0,
    0x0077_f3c0,
    0x0077_f420,
    0x0077_f490,
    0x0077_fa00,
    0x0077_fc10,
    0x0077_ffd0,
    0x0078_2470,
    0x0078_2a90,
    0x0078_3310,
    0x0078_3440,
    0x0078_46a0,
    0x0078_4a40,
    0x0078_4c80,
    0x0078_8440,
    0x0078_8cc0,
    0x0078_d930,
    0x0078_db00,
    0x0078_e7c0,
    0x0079_17a0,
    0x0079_6740,
    0x0079_6b90,
    0x0079_8bf0,
    0x0079_aba0,
    0x0079_c340,
    0x0079_f640,
    0x007a_1490,
    0x007a_17a0,
    0x007a_17e0,
    0x007a_1820,
    0x007a_1850,
    0x007a_1880,
    0x007a_22d0,
    0x007a_7eb0,
    0x007a_8a10,
    0x007a_8ba0,
    0x007a_8e60,
    0x007a_a480,
    0x007a_a530,
    0x007a_a5c0,
    0x007a_a6b0,
    0x007a_b690,
    0x007a_c1f0,
    0x007a_c730,
    0x007b_3fa0,
    0x007b_5400,
    0x007b_5520,
    0x007b_6fb0,
    0x007b_7400,
    0x007b_7570,
    0x007b_8d30,
    0x007b_bcf0,
    0x007b_fb30,
    0x007b_fc30,
    0x007c_01b0,
    0x007c_0580,
    0x007c_06f0,
    0x007c_19c0,
    0x007c_6120,
    0x007c_b330,
    0x007c_b7d0,
    0x007c_e7a0,
    0x007d_86c0,
    0x007d_a2c0,
    0x007d_c7e0,
    0x007d_c9b0,
    0x007d_cc80,
    0x007d_da60,
    0x007d_dc20,
    0x007d_dd10,
    0x007d_de00,
    0x007d_df00,
    0x007d_fc90,
    0x007d_fd00,
    0x007e_0060,
    0x007e_0230,
    0x007e_06b0,
    0x007e_41f0,
    0x007e_47a0,
    0x007e_5890,
    0x007e_6780,
    0x007e_7a80,
    0x007e_9130,
    0x007e_9200,
    0x007e_bd50,
    0x007f_6920,
    0x007f_86f0,
    0x007f_8a80,
    0x007f_a010,
    0x007f_b150,
    0x0083_e510,
    0x0083_fd60,
    0x0084_54f0,
    0x0085_79b0,
    0x0086_4980,
    0x0086_49a0,
    0x0086_5e50,
    0x0087_f4c0,
    0x0087_f6c0,
    0x0087_f9f0,
    0x008c_7aa0,
    0x0090_5330,
    0x0090_5820,
    0x0095_3060,
    0x0096_ad30,
    0x009c_a4a0,
    0x009f_f340,
    0x009f_f690,
    0x00a0_0940,
    0x00a0_0e90,
    0x00a0_0f30,
    0x00a0_11b0,
    0x00a0_11f0,
    0x00a0_1230,
    0x00a0_12d0,
    0x00a0_1350,
    0x00a0_1860,
    0x00a0_3c90,
    0x00a0_3da0,
    0x00a0_3eb0,
    0x00a0_4200,
    0x00a0_7530,
    0x00a0_7c60,
    0x00a0_7dc0,
    0x00a0_87d0,
    0x00a0_8b20,
    0x00a0_9080,
    0x00a0_9130,
    0x00a0_9200,
    0x00a0_9410,
    0x00a0_a270,
    0x00a0_d9e0,
    0x00a0_f8b0,
    0x00a1_c520,
    0x00a1_d910,
    0x00a1_db20,
    0x00a1_dc70,
    0x00a1_ddb0,
    0x00a1_eff0,
    0x00a2_3010,
    0x00a5_d2c0,
    0x00a6_faf0,
    0x00a6_fb90,
    0x00aa_4db0,
    0x00ba_30f0,
    0x00ba_3130,
    0x00c3_a8a0,
    0x00ec_17c0,
    0x00ec_43fb,
    0x00ec_5fce,
    0x00ec_623a,
    0x00ec_62c0,
    0x00ec_782f,
    0x00fd_f294,
    0x00fd_f298,
    0x00fd_f29c,
];

#[test]
fn interface_init_diff() {
    Diff::new(engine_core::entry!(
        0x0070_2250,
        interface::interface_cpp::interface_init(u8)
    ))
    .in_arena(&[Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_007022c0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_22c0,
        interface::interface_cpp::fn_007022c0()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00702330_diff() {
    Diff::new(engine_core::entry!(
        0x0070_2330,
        interface::interface_cpp::fn_00702330()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_is_in_menu_mode_diff() {
    Diff::new(engine_core::entry!(0x0070_2360, interface::interface_cpp::interface_is_in_menu_mode() -> bool))
        .in_arena(&[], CALLEES, &[0x0070_23a0])
        .run()
        .assert_same();
}

#[test]
fn fn_007023a0_diff() {
    Diff::new(engine_core::entry!(0x0070_23a0, interface::interface_cpp::fn_007023a0(u32) -> bool))
        .in_arena(&[Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_get_top_menu_id_diff() {
    Diff::new(engine_core::entry!(0x0070_23c0, interface::interface_cpp::interface_get_top_menu_id() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00702400_diff() {
    Diff::new(engine_core::entry!(0x0070_2400, interface::interface_cpp::fn_00702400(u32) -> u32))
        .in_arena(&[Arg::Int], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00702440_diff() {
    Diff::new(engine_core::entry!(0x0070_2440, interface::interface_cpp::fn_00702440() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_is_top_menu_id_diff() {
    Diff::new(engine_core::entry!(0x0070_2450, interface::interface_cpp::interface_is_top_menu_id(i32) -> u8))
        .in_arena(&[Arg::Int], CALLEES, &[0x0070_27b0, 0x0070_2680])
        .run()
        .assert_same();
}

#[test]
fn interface_is_top_menu_faded_in_diff() {
    Diff::new(engine_core::entry!(0x0070_24e0, interface::interface_cpp::interface_is_top_menu_faded_in() -> bool))
        .in_arena(&[], CALLEES, &[0x0070_27b0, 0x0070_2680])
        .run()
        .assert_same();
}

#[test]
fn interface_get_menu_mode_type_diff() {
    Diff::new(engine_core::entry!(0x0070_2640, interface::interface_cpp::interface_get_menu_mode_type() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_is_menu_id_visible_diff() {
    Diff::new(engine_core::entry!(0x0070_2680, interface::interface_cpp::interface_is_menu_id_visible(i32, u32) -> u8))
        .in_arena(&[Arg::Int, Arg::Word], CALLEES, &[0x0070_27b0])
        .run()
        .assert_same();
}

#[test]
fn fn_007027b0_diff() {
    Diff::new(engine_core::entry!(0x0070_27b0, interface::interface_cpp::fn_007027b0(i32) -> u8))
        .in_arena(&[Arg::Int], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_007027e0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_27e0,
        interface::interface_cpp::fn_007027e0()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00702810_diff() {
    Diff::new(engine_core::entry!(
        0x0070_2810,
        interface::interface_cpp::fn_00702810()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00702840_diff() {
    Diff::new(engine_core::entry!(
        0x0070_2840,
        interface::interface_cpp::fn_00702840()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00702870_diff() {
    Diff::new(engine_core::entry!(
        0x0070_2870,
        interface::interface_cpp::fn_00702870()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_click_diff() {
    Diff::new(engine_core::entry!(
        0x0070_28b0,
        interface::interface_cpp::interface_click(u32, u32)
    ))
    .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[0x0070_2940])
    .run()
    .assert_same();
}

#[test]
fn fn_00702940_diff() {
    Diff::new(engine_core::entry!(
        0x0070_2940,
        interface::interface_cpp::fn_00702940(u32, u8)
    ))
    .in_arena(&[Arg::Word, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00702960_diff() {
    Diff::new(engine_core::entry!(
        0x0070_2960,
        interface::interface_cpp::fn_00702960()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00702990_diff() {
    Diff::new(engine_core::entry!(
        0x0070_2990,
        interface::interface_cpp::fn_00702990(u32, u8, u8, u8)
    ))
    .in_arena(&[Arg::Word, Arg::Byte, Arg::Byte, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_isolate_menu_elements_diff() {
    Diff::new(engine_core::entry!(
        0x0070_2c80,
        interface::interface_cpp::interface_isolate_menu_elements(u8, u8)
    ))
    .in_arena(&[Arg::Byte, Arg::Byte], CALLEES, &[0x0070_2990])
    .run()
    .assert_same();
}

#[test]
fn interface_restore_menu_elements_diff() {
    Diff::new(engine_core::entry!(
        0x0070_2dd0,
        interface::interface_cpp::interface_restore_menu_elements()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_is_debug_text_visible_diff() {
    Diff::new(engine_core::entry!(0x0070_2fc0, interface::interface_cpp::interface_is_debug_text_visible() -> bool))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_toggle_debug_text_visible_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3000,
        interface::interface_cpp::interface_toggle_debug_text_visible()
    ))
    .in_arena(&[], CALLEES, &[0x0070_3030, 0x0070_3060])
    .run()
    .assert_same();
}

#[test]
fn fn_00703030_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3030,
        interface::interface_cpp::fn_00703030(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[0x0070_3060])
    .run()
    .assert_same();
}

#[test]
fn fn_00703060_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3060,
        interface::interface_cpp::fn_00703060(u32, u8)
    ))
    .in_arena(&[Arg::Word, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00703080_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3080,
        interface::interface_cpp::fn_00703080(f32, f32)
    ))
    .in_arena(&[Arg::F32, Arg::F32], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_007030c0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_30c0,
        interface::interface_cpp::fn_007030c0(u32, u32)
    ))
    .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_toggle_full_help_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3100,
        interface::interface_cpp::interface_toggle_full_help()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_get_full_help_diff() {
    Diff::new(
        engine_core::entry!(0x0070_3150, interface::interface_cpp::interface_get_full_help() -> u8),
    )
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_get_target_refr_diff() {
    Diff::new(engine_core::entry!(0x0070_3180, interface::interface_cpp::interface_get_target_refr() -> u32))
        .in_arena(&[], CALLEES, &[0x0070_31c0])
        .run()
        .assert_same();
}

#[test]
fn fn_007031c0_diff() {
    Diff::new(engine_core::entry!(0x0070_31c0, interface::interface_cpp::fn_007031c0(u32) -> u32))
        .in_arena(&[Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_007031e0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_31e0,
        interface::interface_cpp::fn_007031e0(u32)
    ))
    .in_arena_without(&[Arg::Word], CALLEES, &[], &[0x00ec_62c0])
    .run()
    .assert_same();
}

#[test]
fn interface_set_pick_refr_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3310,
        interface::interface_cpp::interface_set_pick_refr(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00703350_diff() {
    Diff::new(engine_core::entry!(0x0070_3350, interface::interface_cpp::fn_00703350() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_get_telekinesis_refr_diff() {
    Diff::new(engine_core::entry!(0x0070_3390, interface::interface_cpp::interface_get_telekinesis_refr() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_007033d0_diff() {
    Diff::new(engine_core::entry!(0x0070_33d0, interface::interface_cpp::fn_007033d0(Ptr) -> Ptr))
        .in_arena(&[Arg::Ptr], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00703430_diff() {
    Diff::new(engine_core::entry!(0x0070_3430, interface::interface_cpp::fn_00703430() -> bool))
        .in_arena(&[], CALLEES, &[0x0070_3470])
        .run()
        .assert_same();
}

#[test]
fn fn_00703470_diff() {
    Diff::new(engine_core::entry!(0x0070_3470, interface::interface_cpp::fn_00703470(u32) -> bool))
        .in_arena(&[Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00703490_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3490,
        interface::interface_cpp::fn_00703490()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_007034c0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_34c0,
        interface::interface_cpp::fn_007034c0(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_show_menus_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3500,
        interface::interface_cpp::interface_show_menus()
    ))
    .in_arena(
        &[],
        CALLEES,
        &[
            0x0070_27b0,
            0x0070_4140,
            0x0070_48c0,
            0x0070_4be0,
            0x0070_2680,
            0x0070_48f0,
            0x0070_4170,
        ],
    )
    .run()
    .assert_same();
}

#[test]
fn interface_hide_menus_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3610,
        interface::interface_cpp::interface_hide_menus()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_emergency_close_all_menus_and_break_stuff_diff() {
    Diff::new(engine_core::entry!(
        0x0070_37e0,
        interface::interface_cpp::interface_emergency_close_all_menus_and_break_stuff()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00703810_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3810,
        interface::interface_cpp::fn_00703810(u8)
    ))
    .in_arena(&[Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_set_crosshair_target_type_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3860,
        interface::interface_cpp::interface_set_crosshair_target_type(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_007038a0_diff() {
    Diff::new(engine_core::entry!(0x0070_38a0, interface::interface_cpp::fn_007038a0() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_set_enemy_actor_diff() {
    Diff::new(engine_core::entry!(
        0x0070_38b0,
        interface::interface_cpp::interface_set_enemy_actor(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_007038d0_diff() {
    Diff::new(engine_core::entry!(0x0070_38d0, interface::interface_cpp::fn_007038d0() -> Ptr))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_toggle_safe_zone_diff() {
    Diff::new(engine_core::entry!(
        0x0070_38e0,
        interface::interface_cpp::interface_toggle_safe_zone(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_set_first_chance_texture_release_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3920,
        interface::interface_cpp::interface_set_first_chance_texture_release()
    ))
    .in_arena(&[], CALLEES, &[0x0070_3960])
    .run()
    .assert_same();
}

#[test]
fn fn_00703960_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3960,
        interface::interface_cpp::fn_00703960(u32, u8)
    ))
    .in_arena(&[Arg::Word, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00703980_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3980,
        interface::interface_cpp::fn_00703980()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_get_control_push_text_for_diff() {
    Diff::new(engine_core::entry!(
        0x0070_39b0,
        interface::interface_cpp::interface_get_control_push_text_for(i32, u32, u8)
    ))
    .in_arena(&[Arg::Int, Arg::Word, Arg::Byte], CALLEES, &[0x0070_3be0])
    .run()
    .assert_same();
}

#[test]
fn fn_00703be0_diff() {
    Diff::new(engine_core::entry!(0x0070_3be0, interface::interface_cpp::fn_00703be0(u32, u32, u32) -> u8))
        .in_arena(&[Arg::Ptr, Arg::Byte, Arg::Byte], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00703c00_diff() {
    Diff::new((0x0070_3c00, interface::interface_cpp::fn_00703c00 as AbiFn))
        .in_arena(&[Arg::Word, Arg::Word, Arg::Word, Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_print_line_diff() {
    Diff::new((
        0x0070_3c80,
        interface::interface_cpp::interface_print_line as AbiFn,
    ))
    .in_arena(&[Arg::Word, Arg::Word, Arg::Word, Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_is_console_visible_diff() {
    Diff::new(engine_core::entry!(0x0070_3d50, interface::interface_cpp::interface_is_console_visible() -> bool))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_close_console_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3da0,
        interface::interface_cpp::interface_close_console()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_open_console_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3e10,
        interface::interface_cpp::interface_open_console()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00703e80_diff() {
    Diff::new((0x0070_3e80, interface::interface_cpp::fn_00703e80 as AbiFn))
        .in_arena(
            &[
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
            ],
            CALLEES,
            &[],
        )
        .run()
        .assert_same();
}

#[test]
fn fn_00703f10_diff() {
    Diff::new((0x0070_3f10, interface::interface_cpp::fn_00703f10 as AbiFn))
        .in_arena(
            &[
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
            ],
            CALLEES,
            &[],
        )
        .run()
        .assert_same();
}

#[test]
fn interface_get_message_menu_result_diff() {
    Diff::new(engine_core::entry!(0x0070_3fa0, interface::interface_cpp::interface_get_message_menu_result() -> u8))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00703fd0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3fd0,
        interface::interface_cpp::fn_00703fd0()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00703fe0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_3fe0,
        interface::interface_cpp::fn_00703fe0(u8)
    ))
    .in_arena(&[Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_close_message_menu_diff() {
    Diff::new(engine_core::entry!(
        0x0070_4000,
        interface::interface_cpp::interface_close_message_menu()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00704010_diff() {
    Diff::new((0x0070_4010, interface::interface_cpp::fn_00704010 as AbiFn))
        .in_arena(
            &[
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
                Arg::Word,
            ],
            CALLEES,
            &[
                0x0070_27e0,
                0x0070_2810,
                0x0070_2840,
                0x0070_2940,
                0x0070_2960,
                0x0070_28b0,
            ],
        )
        .run()
        .assert_same();
}

#[test]
fn fn_00704130_diff() {
    Diff::new(engine_core::entry!(
        0x0070_4130,
        interface::interface_cpp::fn_00704130()
    ))
    .in_arena(&[], CALLEES, &[0x0070_3fa0])
    .run()
    .assert_same();
}

#[test]
fn fn_00704140_diff() {
    Diff::new(engine_core::entry!(0x0070_4140, interface::interface_cpp::fn_00704140() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_set_map_menu_visible_diff() {
    Diff::new(engine_core::entry!(
        0x0070_4170,
        interface::interface_cpp::interface_set_map_menu_visible(u8, u8)
    ))
    .in_arena(
        &[Arg::Byte, Arg::Byte],
        CALLEES,
        &[0x0070_27b0, 0x0070_4140, 0x0070_2680],
    )
    .run()
    .assert_same();
}

#[test]
fn interface_create_repair_menu_diff() {
    Diff::new(engine_core::entry!(0x0070_4490, interface::interface_cpp::interface_create_repair_menu() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_007044c0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_44c0,
        interface::interface_cpp::fn_007044c0(u8, u8, u8)
    ))
    .in_arena(
        &[Arg::Byte, Arg::Byte, Arg::Byte],
        CALLEES,
        &[0x0070_27b0, 0x0070_4490, 0x0070_2680],
    )
    .run()
    .assert_same();
}

#[test]
fn fn_00704690_diff() {
    Diff::new(engine_core::entry!(
        0x0070_4690,
        interface::interface_cpp::fn_00704690(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_create_item_mod_menu_diff() {
    Diff::new(engine_core::entry!(0x0070_46c0, interface::interface_cpp::interface_create_item_mod_menu() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_007046f0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_46f0,
        interface::interface_cpp::fn_007046f0(u8, u8, u8)
    ))
    .in_arena(
        &[Arg::Byte, Arg::Byte, Arg::Byte],
        CALLEES,
        &[0x0070_27b0, 0x0070_46c0, 0x0070_2680],
    )
    .run()
    .assert_same();
}

#[test]
fn fn_007048c0_diff() {
    Diff::new(engine_core::entry!(0x0070_48c0, interface::interface_cpp::fn_007048c0() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_set_inventory_menu_visible_diff() {
    Diff::new(engine_core::entry!(
        0x0070_48f0,
        interface::interface_cpp::interface_set_inventory_menu_visible(u8, u8, u8)
    ))
    .in_arena(
        &[Arg::Byte, Arg::Byte, Arg::Byte],
        CALLEES,
        &[0x0070_27b0, 0x0070_48c0, 0x0070_2680],
    )
    .run()
    .assert_same();
}

#[test]
fn interface_get_inventory_menu_visible_diff() {
    Diff::new(engine_core::entry!(0x0070_4ad0, interface::interface_cpp::interface_get_inventory_menu_visible() -> u8))
        .in_arena(&[], CALLEES, &[0x0070_27b0])
        .run()
        .assert_same();
}

#[test]
fn fn_00704af0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_4af0,
        interface::interface_cpp::fn_00704af0()
    ))
    .in_arena(&[], CALLEES, &[0x0070_27b0, 0x0070_4ad0, 0x0070_2680])
    .run()
    .assert_same();
}

#[test]
fn fn_00704bc0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_4bc0,
        interface::interface_cpp::fn_00704bc0()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00704be0_diff() {
    Diff::new(engine_core::entry!(0x0070_4be0, interface::interface_cpp::fn_00704be0() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_set_stats_menu_visible_diff() {
    Diff::new(engine_core::entry!(
        0x0070_4c10,
        interface::interface_cpp::interface_set_stats_menu_visible(u8, u8)
    ))
    .in_arena(
        &[Arg::Byte, Arg::Byte],
        CALLEES,
        &[0x0070_27b0, 0x0070_4be0, 0x0070_2680],
    )
    .run()
    .assert_same();
}

#[test]
fn fn_00704df0_diff() {
    Diff::new(engine_core::entry!(0x0070_4df0, interface::interface_cpp::fn_00704df0() -> u8))
        .in_arena(&[], CALLEES, &[0x0070_27b0])
        .run()
        .assert_same();
}

#[test]
fn interface_update_actor_value_diff() {
    Diff::new(engine_core::entry!(
        0x0070_4e10,
        interface::interface_cpp::interface_update_actor_value(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[0x0070_27b0, 0x0070_2680])
    .run()
    .assert_same();
}

#[test]
fn interface_create_container_menu_diff() {
    Diff::new(engine_core::entry!(0x0070_4ed0, interface::interface_cpp::interface_create_container_menu(u32, u32) -> u32))
        .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00704f50_diff() {
    Diff::new(engine_core::entry!(
        0x0070_4f50,
        interface::interface_cpp::fn_00704f50()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00704f80_diff() {
    Diff::new(engine_core::entry!(
        0x0070_4f80,
        interface::interface_cpp::fn_00704f80(u32, u32)
    ))
    .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00704fc0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_4fc0,
        interface::interface_cpp::fn_00704fc0(u32, u32)
    ))
    .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00705000_diff() {
    Diff::new(engine_core::entry!(0x0070_5000, interface::interface_cpp::fn_00705000() -> bool))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00705020_diff() {
    Diff::new(engine_core::entry!(0x0070_5020, interface::interface_cpp::fn_00705020() -> bool))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00705040_diff() {
    Diff::new(engine_core::entry!(0x0070_5040, interface::interface_cpp::fn_00705040() -> u32))
        .in_arena(&[], CALLEES, &[0x0070_5050])
        .run()
        .assert_same();
}

#[test]
fn fn_00705050_diff() {
    Diff::new(engine_core::entry!(0x0070_5050, interface::interface_cpp::fn_00705050() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_create_dialog_menu_diff() {
    Diff::new(engine_core::entry!(0x0070_5070, interface::interface_cpp::interface_create_dialog_menu(u32, u32, u32) -> u32))
        .in_arena(&[Arg::Word, Arg::Word, Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_in_dialog_diff() {
    Diff::new(
        engine_core::entry!(0x0070_50d0, interface::interface_cpp::interface_in_dialog() -> u8),
    )
    .in_arena(&[], CALLEES, &[0x0070_5100])
    .run()
    .assert_same();
}

#[test]
fn fn_00705100_diff() {
    Diff::new(engine_core::entry!(0x0070_5100, interface::interface_cpp::fn_00705100() -> u8))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_set_dialog_pause_diff() {
    Diff::new(engine_core::entry!(
        0x0070_5110,
        interface::interface_cpp::interface_set_dialog_pause(u8)
    ))
    .in_arena(&[Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_in_dialog_with_diff() {
    Diff::new(engine_core::entry!(0x0070_5190, interface::interface_cpp::interface_in_dialog_with() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_show_subtitle_diff() {
    Diff::new(engine_core::entry!(0x0070_5210, interface::interface_cpp::interface_show_subtitle(u32, u32, u32, u32, u32, u32, u32, u32, u8) -> u8))
        .in_arena(&[Arg::Word, Arg::Word, Arg::Word, Arg::Word, Arg::Word, Arg::Word, Arg::Word, Arg::Word, Arg::Byte], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_007053b0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_53b0,
        interface::interface_cpp::fn_007053b0()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_007053f0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_53f0,
        interface::interface_cpp::fn_007053f0()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00705420_diff() {
    Diff::new(engine_core::entry!(
        0x0070_5420,
        interface::interface_cpp::fn_00705420(u32, u32, u32, u8)
    ))
    .in_arena(&[Arg::Word, Arg::Word, Arg::Word, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_create_sleep_menu_diff() {
    Diff::new(engine_core::entry!(
        0x0070_54f0,
        interface::interface_cpp::interface_create_sleep_menu(u8)
    ))
    .in_arena(&[Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_close_sleep_menu_diff() {
    Diff::new(engine_core::entry!(
        0x0070_55c0,
        interface::interface_cpp::interface_close_sleep_menu()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_create_vats_menu_diff() {
    Diff::new(engine_core::entry!(
        0x0070_5640,
        interface::interface_cpp::interface_create_vats_menu()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_update_sleeping_diff() {
    Diff::new(engine_core::entry!(0x0070_56f0, interface::interface_cpp::interface_update_sleeping() -> u8))
        .in_arena(&[], CALLEES, &[0x0070_5770])
        .run()
        .assert_same();
}

#[test]
fn fn_00705770_diff() {
    Diff::new(engine_core::entry!(0x0070_5770, interface::interface_cpp::fn_00705770() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_close_vats_menu_diff() {
    Diff::new(engine_core::entry!(
        0x0070_5780,
        interface::interface_cpp::interface_close_vats_menu()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00705800_diff() {
    Diff::new(engine_core::entry!(
        0x0070_5800,
        interface::interface_cpp::fn_00705800()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00705830_diff() {
    Diff::new(engine_core::entry!(
        0x0070_5830,
        interface::interface_cpp::fn_00705830(u32, u32, u8)
    ))
    .in_arena(&[Arg::Word, Arg::Word, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_create_race_sex_menu_diff() {
    Diff::new(engine_core::entry!(
        0x0070_5870,
        interface::interface_cpp::interface_create_race_sex_menu(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[0x0070_5100])
    .run()
    .assert_same();
}

#[test]
fn fn_007058c0_diff() {
    Diff::new(engine_core::entry!(0x0070_58c0, interface::interface_cpp::fn_007058c0(u32) -> u32))
        .in_arena(&[Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_last_minute_update_diff() {
    Diff::new(engine_core::entry!(
        0x0070_58e0,
        interface::interface_cpp::interface_last_minute_update()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00705910_diff() {
    Diff::new(engine_core::entry!(0x0070_5910, interface::interface_cpp::fn_00705910() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00705950_diff() {
    Diff::new(engine_core::entry!(0x0070_5950, interface::interface_cpp::fn_00705950() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_get_pipboy_diff() {
    Diff::new(
        engine_core::entry!(0x0070_5990, interface::interface_cpp::interface_get_pipboy() -> u32),
    )
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_007059d0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_59d0,
        interface::interface_cpp::fn_007059d0()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_is_in_pipboy_menu_diff() {
    Diff::new(engine_core::entry!(0x0070_5a00, interface::interface_cpp::interface_is_in_pipboy_menu() -> u8))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00705a60_diff() {
    Diff::new(engine_core::entry!(
        0x0070_5a60,
        interface::interface_cpp::fn_00705a60()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00709b50_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9b50,
        interface::interface_cpp_p3::fn_00709b50(u8, f32, f32, u32)
    ))
    .in_arena(
        &[Arg::Byte, Arg::F32, Arg::F32, Arg::Word],
        CALLEES,
        &[0x0070_9b80],
    )
    .run()
    .assert_same();
}

#[test]
fn fn_00709b80_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9b80,
        interface::interface_cpp_p3::fn_00709b80(u32, u8, f32, f32, u32)
    ))
    .in_arena(
        &[Arg::Word, Arg::Byte, Arg::F32, Arg::F32, Arg::Word],
        CALLEES,
        &[],
    )
    .run()
    .assert_same();
}

#[test]
fn interface_is_pipboy_opening_diff() {
    Diff::new(engine_core::entry!(0x0070_9bc0, interface::interface_cpp_p3::interface_is_pipboy_opening() -> bool))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00709be0_diff() {
    Diff::new(engine_core::entry!(0x0070_9be0, interface::interface_cpp_p3::fn_00709be0() -> bool))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00709c00_diff() {
    Diff::new(engine_core::entry!(0x0070_9c00, interface::interface_cpp_p3::fn_00709c00() -> bool))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00709c20_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9c20,
        interface::interface_cpp_p3::fn_00709c20(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_set_gun_scope_visible_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9c40,
        interface::interface_cpp_p3::interface_set_gun_scope_visible(u8)
    ))
    .in_arena(&[Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00709c60_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9c60,
        interface::interface_cpp_p3::fn_00709c60(u8)
    ))
    .in_arena(&[Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00709c80_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9c80,
        interface::interface_cpp_p3::fn_00709c80(u8)
    ))
    .in_arena(&[Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00709ca0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9ca0,
        interface::interface_cpp_p3::fn_00709ca0()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00709cb0_diff() {
    Diff::new(engine_core::entry!(0x0070_9cb0, interface::interface_cpp_p3::fn_00709cb0() -> bool))
        .in_arena(&[], CALLEES, &[0x0070_9cc0])
        .run()
        .assert_same();
}

#[test]
fn fn_00709cc0_diff() {
    Diff::new(engine_core::entry!(0x0070_9cc0, interface::interface_cpp_p3::fn_00709cc0() -> bool))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00709d00_diff() {
    Diff::new(engine_core::entry!(0x0070_9d00, interface::interface_cpp_p3::fn_00709d00() -> u8))
        .in_arena(&[], CALLEES, &[0x0070_9d10])
        .run()
        .assert_same();
}

#[test]
fn fn_00709d10_diff() {
    Diff::new(engine_core::entry!(0x0070_9d10, interface::interface_cpp_p3::fn_00709d10() -> u8))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_set_root_map_space_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9d20,
        interface::interface_cpp_p3::interface_set_root_map_space(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00709d40_diff() {
    Diff::new(engine_core::entry!(0x0070_9d40, interface::interface_cpp_p3::fn_00709d40() -> bool))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00709d50_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9d50,
        interface::interface_cpp_p3::fn_00709d50(u32, f32)
    ))
    // `00709ac0` (the camera scale) is a member done in place here: its call left out.
    .in_arena_without(&[Arg::Word, Arg::F32], CALLEES, &[], &[0x0070_9ac0])
    .run()
    .assert_same();
}

#[test]
fn fn_00709e80_diff() {
    Diff::new(engine_core::entry!(0x0070_9e80, interface::interface_cpp_p3::fn_00709e80(u32, u32, u32) -> u32))
        .in_arena(&[Arg::Word, Arg::Word, Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00709eb0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9eb0,
        interface::interface_cpp_p3::fn_00709eb0(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn bs_simple_array_tile_value_p_8_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0070_9ed0, interface::interface_cpp_p3::bs_simple_array_tile_value_p_8_scalar_deleting_destructor(u32, u32) -> u32))
        .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[0x0070_9eb0])
        .run()
        .assert_same();
}

#[test]
fn fn_00705a90_diff() {
    Diff::new(engine_core::entry!(0x0070_5a90, interface::interface_cpp_p2::fn_00705a90() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00705ad0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_5ad0,
        interface::interface_cpp_p2::fn_00705ad0(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[0x0070_5b10])
    .run()
    .assert_same();
}

#[test]
fn fn_00705b10_diff() {
    Diff::new(engine_core::entry!(
        0x0070_5b10,
        interface::interface_cpp_p2::fn_00705b10(u32, u32)
    ))
    .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00705b30_diff() {
    Diff::new(engine_core::entry!(
        0x0070_5b30,
        interface::interface_cpp_p2::fn_00705b30(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00705b70_diff() {
    Diff::new(engine_core::entry!(
        0x0070_5b70,
        interface::interface_cpp_p2::fn_00705b70(u8, u8, u8, i32)
    ))
    .in_arena(
        &[Arg::Byte, Arg::Byte, Arg::Byte, Arg::Int],
        CALLEES,
        &[0x0070_5c40],
    )
    .run()
    .assert_same();
}

#[test]
fn fn_00705c40_diff() {
    Diff::new(
        engine_core::entry!(0x0070_5c40, interface::interface_cpp_p2::fn_00705c40(i32, i8) -> i32),
    )
    .in_arena(&[Arg::Int, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00705c70_diff() {
    Diff::new(engine_core::entry!(0x0070_5c70, interface::interface_cpp_p2::fn_00705c70() -> u8))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00705cb0_diff() {
    Diff::new(engine_core::entry!(0x0070_5cb0, interface::interface_cpp_p2::fn_00705cb0() -> u8))
        .in_arena(&[], CALLEES, &[0x0070_5cf0])
        .run()
        .assert_same();
}

#[test]
fn fn_00705cf0_diff() {
    Diff::new(
        engine_core::entry!(0x0070_5cf0, interface::interface_cpp_p2::fn_00705cf0(u32) -> u8),
    )
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00705d10_diff() {
    Diff::new(engine_core::entry!(0x0070_5d10, interface::interface_cpp_p2::fn_00705d10() -> u8))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00705d50_diff() {
    Diff::new(engine_core::entry!(0x0070_5d50, interface::interface_cpp_p2::fn_00705d50() -> u8))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00705d90_diff() {
    Diff::new(engine_core::entry!(0x0070_5d90, interface::interface_cpp_p2::fn_00705d90() -> i32))
        .in_arena(&[], CALLEES, &[0x0070_5dd0])
        .run()
        .assert_same();
}

#[test]
fn fn_00705dd0_diff() {
    Diff::new(
        engine_core::entry!(0x0070_5dd0, interface::interface_cpp_p2::fn_00705dd0(u32) -> i32),
    )
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_create_loading_menu_diff() {
    Diff::new(engine_core::entry!(0x0070_5e00, interface::interface_cpp_p2::interface_create_loading_menu(u32, u8) -> u32))
        .in_arena(&[Arg::Word, Arg::Byte], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_close_loading_menu_diff() {
    Diff::new(engine_core::entry!(
        0x0070_5e30,
        interface::interface_cpp_p2::interface_close_loading_menu()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_get_loading_menu_visible_diff() {
    Diff::new(engine_core::entry!(0x0070_5e80, interface::interface_cpp_p2::interface_get_loading_menu_visible() -> u8))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_is_in_game_loading_menu_open_diff() {
    Diff::new(engine_core::entry!(0x0070_5ea0, interface::interface_cpp_p2::interface_is_in_game_loading_menu_open() -> bool))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_create_hacking_menu_diff() {
    Diff::new(engine_core::entry!(0x0070_5ec0, interface::interface_cpp_p2::interface_create_hacking_menu(u32) -> bool))
        .in_arena(&[Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_create_computers_menu_diff() {
    Diff::new(engine_core::entry!(0x0070_6130, interface::interface_cpp_p2::interface_create_computers_menu(u32) -> bool))
        .in_arena(&[Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00706230_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6230,
        interface::interface_cpp_p2::fn_00706230(u32, u32)
    ))
    .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_create_level_up_menu_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6270,
        interface::interface_cpp_p2::interface_create_level_up_menu()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_007062a0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_62a0,
        interface::interface_cpp_p2::fn_007062a0(u32, u32, u8, u8)
    ))
    .in_arena(&[Arg::Word, Arg::Word, Arg::Byte, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_set_info_for_ref_diff() {
    Diff::new(engine_core::entry!(0x0070_62e0, interface::interface_cpp_p2::interface_set_info_for_ref(u32, u8, u8) -> u8))
        .in_arena(&[Arg::Word, Arg::Byte, Arg::Byte], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_close_all_support_menus_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6320,
        interface::interface_cpp_p2::interface_close_all_support_menus(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    // The current tile's word is the native code's one local (the walk's cursor is a Rust value).
    .local_at(0x0057_c730, 1, 4)
    .run()
    .assert_same();
}

#[test]
fn fn_007065c0_diff() {
    Diff::new(engine_core::entry!(0x0070_65c0, interface::interface_cpp_p2::fn_007065c0() -> u16))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_save_game_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6610,
        interface::interface_cpp_p2::interface_save_game()
    ))
    .in_arena(
        &[],
        CALLEES,
        &[
            0x0070_5c70,
            0x0070_5cb0,
            0x0070_5cf0,
            0x0070_5d10,
            0x0070_5d50,
            0x0070_5d90,
            0x0070_5dd0,
        ],
    )
    .run()
    .assert_same();
}

#[test]
fn fn_007066d0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_66d0,
        interface::interface_cpp_p2::fn_007066d0(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[0x0070_6810, 0x0070_6830])
    .run()
    .assert_same();
}

#[test]
fn fn_00706810_diff() {
    Diff::new(
        engine_core::entry!(0x0070_6810, interface::interface_cpp_p2::fn_00706810(u32) -> i32),
    )
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00706830_diff() {
    Diff::new(engine_core::entry!(0x0070_6830, interface::interface_cpp_p2::fn_00706830() -> u8))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_load_game_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6860,
        interface::interface_cpp_p2::interface_load_game(u32)
    ))
    .in_arena(
        &[Arg::Word],
        CALLEES,
        &[0x0070_69d0, 0x0070_69f0, 0x0070_6a20, 0x0070_6a40],
    )
    .run()
    .assert_same();
}

#[test]
fn fn_007069d0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_69d0,
        interface::interface_cpp_p2::fn_007069d0(u8)
    ))
    .in_arena(&[Arg::Byte], CALLEES, &[0x0070_69f0])
    .run()
    .assert_same();
}

#[test]
fn fn_007069f0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_69f0,
        interface::interface_cpp_p2::fn_007069f0(u32, u32)
    ))
    .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00706a20_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6a20,
        interface::interface_cpp_p2::fn_00706a20(u8)
    ))
    .in_arena(&[Arg::Byte], CALLEES, &[0x0070_6a40])
    .run()
    .assert_same();
}

#[test]
fn fn_00706a40_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6a40,
        interface::interface_cpp_p2::fn_00706a40(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_finish_load_game_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6a90,
        interface::interface_cpp_p2::interface_finish_load_game()
    ))
    .in_arena(&[], CALLEES, &[0x0070_6ac0])
    .run()
    .assert_same();
}

#[test]
fn fn_00706ac0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6ac0,
        interface::interface_cpp_p2::fn_00706ac0()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00706ae0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6ae0,
        interface::interface_cpp_p2::fn_00706ae0()
    ))
    .in_arena(
        &[],
        CALLEES,
        &[0x0070_5100, 0x0070_6ac0, 0x0070_6b90, 0x0070_6320],
    )
    // CloseAllSupportMenus's tile word: the native code's one local there.
    .local_at(0x0057_c730, 1, 4)
    .run()
    .assert_same();
}

#[test]
fn fn_00706b90_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6b90,
        interface::interface_cpp_p2::fn_00706b90(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_tile_is_being_deleted_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6c20,
        interface::interface_cpp_p2::interface_tile_is_being_deleted(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[0x0070_6cb0])
    .run()
    .assert_same();
}

#[test]
fn fn_00706cb0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6cb0,
        interface::interface_cpp_p2::fn_00706cb0(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_get_menus_root_diff() {
    Diff::new(engine_core::entry!(0x0070_6cd0, interface::interface_cpp_p2::interface_get_menus_root() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00706cf0_diff() {
    Diff::new(engine_core::entry!(0x0070_6cf0, interface::interface_cpp_p2::fn_00706cf0() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_get_debug_text_root_diff() {
    Diff::new(engine_core::entry!(0x0070_6d10, interface::interface_cpp_p2::interface_get_debug_text_root() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_get_interface_root_diff() {
    Diff::new(engine_core::entry!(0x0070_6d30, interface::interface_cpp_p2::interface_get_interface_root() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_attach_default_alpha_property_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6d50,
        interface::interface_cpp_p2::interface_attach_default_alpha_property(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_get_first_chance_texture_release_diff() {
    Diff::new(engine_core::entry!(0x0070_6d70, interface::interface_cpp_p2::interface_get_first_chance_texture_release() -> u8))
        .in_arena(&[], CALLEES, &[0x0070_6d90])
        .run()
        .assert_same();
}

#[test]
fn fn_00706d90_diff() {
    Diff::new(
        engine_core::entry!(0x0070_6d90, interface::interface_cpp_p2::fn_00706d90(u32) -> u8),
    )
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00706db0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6db0,
        interface::interface_cpp_p2::fn_00706db0(u8)
    ))
    .in_arena(&[Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00706dd0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6dd0,
        interface::interface_cpp_p2::fn_00706dd0(u8)
    ))
    .in_arena(&[Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_get_screen_height_diff() {
    Diff::new(engine_core::entry!(0x0070_6df0, interface::interface_cpp_p2::interface_get_screen_height() -> f32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_get_screen_width_diff() {
    Diff::new(engine_core::entry!(0x0070_6e00, interface::interface_cpp_p2::interface_get_screen_width() -> f32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_get_real_screen_height_diff() {
    Diff::new(engine_core::entry!(0x0070_6e10, interface::interface_cpp_p2::interface_get_real_screen_height() -> f32))
        .in_arena(&[], CALLEES, &[0x0070_6e20])
        .run()
        .assert_same();
}

#[test]
fn fn_00706e20_diff() {
    Diff::new(engine_core::entry!(0x0070_6e20, interface::interface_cpp_p2::fn_00706e20() -> f32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_get_real_screen_width_diff() {
    Diff::new(engine_core::entry!(0x0070_6e40, interface::interface_cpp_p2::interface_get_real_screen_width() -> f32))
        .in_arena(&[], CALLEES, &[0x0070_6e50])
        .run()
        .assert_same();
}

#[test]
fn fn_00706e50_diff() {
    Diff::new(engine_core::entry!(0x0070_6e50, interface::interface_cpp_p2::fn_00706e50() -> f32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_get_screen_aspect_ratio_diff() {
    Diff::new(engine_core::entry!(0x0070_6e70, interface::interface_cpp_p2::interface_get_screen_aspect_ratio() -> f32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00706e80_diff() {
    Diff::new(engine_core::entry!(0x0070_6e80, interface::interface_cpp_p2::fn_00706e80() -> f32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00706e90_diff() {
    Diff::new(engine_core::entry!(0x0070_6e90, interface::interface_cpp_p2::fn_00706e90() -> f32))
        .in_arena(&[], CALLEES, &[0x0070_6e80])
        .run()
        .assert_same();
}

#[test]
fn fn_00706f20_diff() {
    Diff::new(engine_core::entry!(0x0070_6f20, interface::interface_cpp_p2::fn_00706f20() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_play_menu_sound_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6f30,
        interface::interface_cpp_p2::interface_play_menu_sound(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_new_timer_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6f50,
        interface::interface_cpp_p2::interface_new_timer(u32, f32)
    ))
    .in_arena(&[Arg::Word, Arg::F32], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_update_all_timers_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6f70,
        interface::interface_cpp_p2::interface_update_all_timers()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_clear_timer_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6f90,
        interface::interface_cpp_p2::interface_clear_timer(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_add_to_enter_stack_diff() {
    Diff::new(engine_core::entry!(0x0070_6fb0, interface::interface_cpp_p2::interface_add_to_enter_stack(u32) -> u32))
        .in_arena(&[Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_pop_from_enter_stack_diff() {
    Diff::new(engine_core::entry!(0x0070_6fd0, interface::interface_cpp_p2::interface_pop_from_enter_stack(u32, u8) -> u32))
        .in_arena(&[Arg::Word, Arg::Byte], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_set_current_focus_target_diff() {
    Diff::new(engine_core::entry!(
        0x0070_6ff0,
        interface::interface_cpp_p2::interface_set_current_focus_target(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_recursive_fade_diff() {
    Diff::new(engine_core::entry!(
        0x0070_7010,
        interface::interface_cpp_p2::interface_recursive_fade(u32, f32, f32)
    ))
    .in_arena(&[Arg::Word, Arg::F32, Arg::F32], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00707040_diff() {
    Diff::new(engine_core::entry!(0x0070_7040, interface::interface_cpp_p2::fn_00707040() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00707050_diff() {
    Diff::new(engine_core::entry!(
        0x0070_7050,
        interface::interface_cpp_p2::fn_00707050(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00707070_diff() {
    Diff::new(engine_core::entry!(0x0070_7070, interface::interface_cpp_p2::fn_00707070() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00707080_diff() {
    Diff::new(engine_core::entry!(
        0x0070_7080,
        interface::interface_cpp_p2::fn_00707080(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_007070a0_diff() {
    Diff::new(engine_core::entry!(0x0070_70a0, interface::interface_cpp_p2::fn_007070a0() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_find_text_replacement_string_diff() {
    Diff::new(engine_core::entry!(0x0070_70c0, interface::interface_cpp_p2::interface_find_text_replacement_string(u32, u32, u32, u8) -> bool))
        .in_arena(&[Arg::Word, Arg::Word, Arg::Word, Arg::Byte], CALLEES, &[0x0070_3be0, 0x0070_7330, 0x0070_73d0])
        .run()
        .assert_same();
}

#[test]
fn fn_00707330_diff() {
    Diff::new(
        engine_core::entry!(0x0070_7330, interface::interface_cpp_p2::fn_00707330(u32) -> i32),
    )
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_007073d0_diff() {
    Diff::new(engine_core::entry!(0x0070_73d0, interface::interface_cpp_p2::fn_007073d0(u32, u32) -> bool))
        .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_release_model_file_diff() {
    Diff::new(engine_core::entry!(
        0x0070_7640,
        interface::interface_cpp_p2::interface_release_model_file(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_load_model_file_diff() {
    Diff::new(engine_core::entry!(0x0070_7660, interface::interface_cpp_p2::interface_load_model_file(u32, u32) -> u32))
        .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_clear_temp_model_diff() {
    Diff::new(engine_core::entry!(
        0x0070_7820,
        interface::interface_cpp_p2::interface_clear_temp_model()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00707860_diff() {
    Diff::new(engine_core::entry!(0x0070_7860, interface::interface_cpp_p2::fn_00707860() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_copy_or_deep_copy_node_diff() {
    Diff::new(engine_core::entry!(0x0070_7870, interface::interface_cpp_p2::interface_copy_or_deep_copy_node(u32) -> u32))
        .in_arena(&[Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_set_need_to_update_diff() {
    Diff::new(engine_core::entry!(
        0x0070_7930,
        interface::interface_cpp_p2::interface_set_need_to_update(u8)
    ))
    .in_arena(&[Arg::Byte], CALLEES, &[0x0070_7950])
    .run()
    .assert_same();
}

#[test]
fn fn_00707950_diff() {
    Diff::new(engine_core::entry!(
        0x0070_7950,
        interface::interface_cpp_p2::fn_00707950(u32, u8)
    ))
    .in_arena(&[Arg::Word, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00707970_diff() {
    Diff::new(
        engine_core::entry!(0x0070_7970, interface::interface_cpp_p2::fn_00707970(u32) -> bool),
    )
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_create_menu_by_class_diff() {
    Diff::new(engine_core::entry!(0x0070_7990, interface::interface_cpp_p2::interface_create_menu_by_class(u32) -> u32))
        .in_arena_without(
            &[Arg::Class],
            CALLEES,
            &[0x0071_e340, 0x0071_e400, 0x0071_f540, 0x0071_f5d0, 0x0072_07a0],
            &[0x00ec_782f],
        )
        .max_cases(96)
        .run()
        .assert_same();
}

#[test]
fn interface_is_in_rendered_menu_diff() {
    Diff::new(engine_core::entry!(0x0070_79b0, interface::interface_cpp_p2::interface_is_in_rendered_menu() -> bool))
        .in_arena(&[], CALLEES, &[0x0070_79f0])
        .run()
        .assert_same();
}

#[test]
fn fn_007079f0_diff() {
    Diff::new(
        engine_core::entry!(0x0070_79f0, interface::interface_cpp_p2::fn_007079f0(u32) -> bool),
    )
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_is_rendered_menu_diff() {
    Diff::new(engine_core::entry!(0x0070_7a30, interface::interface_cpp_p2::interface_is_rendered_menu(u32) -> bool))
        .in_arena(&[Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn interface_get_current_rendered_menu_diff() {
    Diff::new(engine_core::entry!(0x0070_7ad0, interface::interface_cpp_p2::interface_get_current_rendered_menu() -> u32))
        .in_arena(&[], CALLEES, &[0x0070_7af0])
        .run()
        .assert_same();
}

#[test]
fn fn_00707af0_diff() {
    Diff::new(
        engine_core::entry!(0x0070_7af0, interface::interface_cpp_p2::fn_00707af0(u32) -> u32),
    )
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_is_current_rendered_menu_topmost_diff() {
    Diff::new(engine_core::entry!(0x0070_7b30, interface::interface_cpp_p2::interface_is_current_rendered_menu_topmost() -> bool))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00707b40_diff() {
    Diff::new(engine_core::entry!(0x0070_7b40, interface::interface_cpp_p2::fn_00707b40() -> u8))
        .in_arena(&[], CALLEES, &[0x0070_7b50])
        .run()
        .assert_same();
}

#[test]
fn fn_00707b50_diff() {
    Diff::new(engine_core::entry!(0x0070_7b50, interface::interface_cpp_p2::fn_00707b50() -> u8))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00707b60_diff() {
    Diff::new(engine_core::entry!(0x0070_7b60, interface::interface_cpp_p2::fn_00707b60() -> f32))
        .in_arena(&[], CALLEES, &[0x0070_6df0])
        .run()
        .assert_same();
}

#[test]
fn interface_initialize_tabline_diff() {
    Diff::new((
        0x0070_7be0,
        interface::interface_cpp_p2::interface_initialize_tabline as AbiFn,
    ))
    .in_arena(
        &[Arg::Word, Arg::Word, Arg::Word, Arg::Word, Arg::Word],
        CALLEES,
        &[0x0070_7e10],
    )
    .run()
    .assert_same();
}

#[test]
fn fn_00707e10_diff() {
    Diff::new(engine_core::entry!(
        0x0070_7e10,
        interface::interface_cpp_p2::fn_00707e10(u32, u32)
    ))
    .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_populate_item_stats_display_diff() {
    Diff::new(engine_core::entry!(
        0x0070_7e30,
        interface::interface_cpp_p2::interface_populate_item_stats_display(u32, u32)
    ))
    .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn interface_queue_menu_create_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9470,
        interface::interface_cpp_p2::interface_queue_menu_create(u32, u32, u32, u32, u32)
    ))
    .in_arena(
        &[Arg::Word, Arg::Word, Arg::Word, Arg::Word, Arg::Word],
        CALLEES,
        &[
            0x0070_7e10,
            0x0070_9810,
            0x0070_9840,
            0x0070_9860,
            0x0070_9a90,
            0x0070_6130,
            0x0070_5ec0,
            0x0070_94f0,
        ],
    )
    .run()
    .assert_same();
}

/// The queued opens `007094f0` takes, the extra input of its Diff: the first slot's kind (1 to 9),
/// plus `0x100` for a manager whose mode word (`+0x4bc`) is set (only kinds 6 and 7 go on then),
/// plus the message slot's kind times `0x1_0000`.
const QUEUED: &[u32] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 0x101, 0x106, 0x7_0000, 0x7_0004];

/// Writes a [`QUEUED`] value into the world: the two slots' kinds, the manager's mode word, and
/// each slot word an object of its own (so an argument swapped between two of them shows).
fn put_queued(e: &mut Engine, queued: u32) {
    let manager = e.mem.u32(0x011d_8a80);
    if manager != 0 {
        e.mem.set_u32(manager + 0x4bc, queued >> 8 & 1);
    }
    e.mem.set_u32(0x011d_8950, queued & 0xff);
    e.mem.set_u32(0x011d_8998, queued >> 16);
    for (k, at) in (0x011d_8954..0x011d_8968).step_by(4).enumerate() {
        e.mem.set_u32(at, arena::object(60 + k as u32));
    }
    for (k, at) in (0x011d_899c..0x011d_89b0).step_by(4).enumerate() {
        e.mem.set_u32(at, arena::object(70 + k as u32));
    }
}

#[test]
fn interface_handle_queued_menu_open_diff() {
    Diff::new(engine_core::entry!(
        0x0070_94f0,
        interface::interface_cpp_p2::interface_handle_queued_menu_open()
    ))
    .in_arena_and(
        &[],
        CALLEES,
        &[
            0x0070_7e10,
            0x0070_9810,
            0x0070_9840,
            0x0070_9860,
            0x0070_6130,
            0x0070_5ec0,
        ],
        QUEUED,
        put_queued,
    )
    .run()
    .assert_same();
}

#[test]
fn fn_00709810_diff() {
    Diff::new(engine_core::entry!(0x0070_9810, interface::interface_cpp_p2::fn_00709810(u32, u32, u32) -> u32))
        .in_arena(&[Arg::Word, Arg::Word, Arg::Word], CALLEES, &[0x0070_9840, 0x0070_9860])
        .run()
        .assert_same();
}

#[test]
fn fn_00709840_diff() {
    Diff::new(
        engine_core::entry!(0x0070_9840, interface::interface_cpp_p2::fn_00709840(u32) -> u32),
    )
    .in_arena(&[Arg::Word], CALLEES, &[0x0070_9860])
    .run()
    .assert_same();
}

#[test]
fn fn_00709860_diff() {
    Diff::new(
        engine_core::entry!(0x0070_9860, interface::interface_cpp_p2::fn_00709860(u32) -> u32),
    )
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn tile_get_type_name_diff() {
    Diff::new(
        engine_core::entry!(0x0070_9920, interface::interface_cpp_p2::tile_get_type_name() -> u32),
    )
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn tile_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0070_9930, interface::interface_cpp_p2::tile_scalar_deleting_destructor(u32, u32) -> u32))
        .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn tile_rect_get_type_diff() {
    Diff::new(
        engine_core::entry!(0x0070_9960, interface::interface_cpp_p2::tile_rect_get_type() -> u32),
    )
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn tile_rect_get_type_name_diff() {
    Diff::new(engine_core::entry!(0x0070_9970, interface::interface_cpp_p2::tile_rect_get_type_name() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn tile_rect_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0070_9980, interface::interface_cpp_p2::tile_rect_scalar_deleting_destructor(u32, u32) -> u32))
        .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[0x0070_99b0, 0x0070_9a20])
        .run()
        .assert_same();
}

#[test]
fn fn_007099b0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_99b0,
        interface::interface_cpp_p2::fn_007099b0(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[0x0070_9a20])
    .run()
    .assert_same();
}

#[test]
fn fn_00709a20_diff() {
    Diff::new(
        engine_core::entry!(0x0070_9a20, interface::interface_cpp_p2::fn_00709a20(u32) -> bool),
    )
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn tile_menu_get_type_diff() {
    Diff::new(
        engine_core::entry!(0x0070_9a40, interface::interface_cpp_p2::tile_menu_get_type() -> u32),
    )
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn tile_menu_get_type_name_diff() {
    Diff::new(engine_core::entry!(0x0070_9a50, interface::interface_cpp_p2::tile_menu_get_type_name() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn tile_menu_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0070_9a60, interface::interface_cpp_p2::tile_menu_scalar_deleting_destructor(u32, u32) -> u32))
        .in_arena(&[Arg::Word, Arg::Word], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00709a90_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9a90,
        interface::interface_cpp_p2::fn_00709a90()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00709ac0_diff() {
    Diff::new(engine_core::entry!(0x0070_9ac0, interface::interface_cpp_p2::fn_00709ac0() -> f32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00709ae0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9ae0,
        interface::interface_cpp_p2::fn_00709ae0()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00709af0_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9af0,
        interface::interface_cpp_p2::fn_00709af0()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00709b00_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9b00,
        interface::interface_cpp_p2::fn_00709b00()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00709b10_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9b10,
        interface::interface_cpp_p2::fn_00709b10()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00709b20_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9b20,
        interface::interface_cpp_p2::fn_00709b20()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00709b30_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9b30,
        interface::interface_cpp_p2::fn_00709b30()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00709b40_diff() {
    Diff::new(engine_core::entry!(
        0x0070_9b40,
        interface::interface_cpp_p2::fn_00709b40()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn menu_manager_instance_diff() {
    Diff::new(engine_core::entry!(0x0071_e290, interface::menu_manager::menu_manager_instance(bool) -> u32))
    .in_arena_and(
        &[Arg::Byte],
        CALLEES,
        &[0x0071_e340, 0x0071_e400],
        &ALLOCATOR_VARIANTS,
        put_allocator,
    )
    .run()
    .assert_same();
}

#[test]
fn fn_0071e340_diff() {
    Diff::new(engine_core::entry!(0x0071_e340, interface::menu_manager::fn_0071e340(Ptr) -> Ptr))
        .in_arena(&[Arg::Ptr], CALLEES, &[0x0071_e400])
        .run()
        .assert_same();
}

#[test]
fn fn_0071e360_diff() {
    Diff::new(engine_core::entry!(
        0x0071_e360,
        interface::menu_manager::fn_0071e360()
    ))
    .in_arena(&[], CALLEES, &[0x0071_e3b0, 0x0071_e3e0, 0x0071_e3f0])
    .run()
    .assert_same();
}

#[test]
fn fn_0071e3b0_diff() {
    Diff::new(
        engine_core::entry!(0x0071_e3b0, interface::menu_manager::fn_0071e3b0(Ptr, u32) -> Ptr),
    )
    .in_arena(&[Arg::Ptr, Arg::Word], CALLEES, &[0x0071_e3e0, 0x0071_e3f0])
    .run()
    .assert_same();
}

#[test]
fn fn_0071e3e0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_e3e0,
        interface::menu_manager::fn_0071e3e0(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[0x0071_e3f0])
    .run()
    .assert_same();
}

#[test]
fn fn_0071e3f0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_e3f0,
        interface::menu_manager::fn_0071e3f0()
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_0071e400_diff() {
    Diff::new(engine_core::entry!(
        0x0071_e400,
        interface::menu_manager::fn_0071e400(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn menu_manager_create_menu_by_class_diff() {
    Diff::new(engine_core::entry!(0x0071_e420, interface::menu_manager::menu_manager_create_menu_by_class(Ptr, u32) -> u32))
    .in_arena_without_and(
        &[Arg::Ptr, Arg::Class],
        CALLEES,
        &[0x0071_f540, 0x0071_f5d0, 0x0072_07a0],
        &[0x00ec_782f],
        &ALLOCATOR_VARIANTS,
        put_allocator,
    )
    .max_cases(192)
    .run()
    .assert_same();
}

#[test]
fn fn_0071f240_diff() {
    Diff::new(engine_core::entry!(0x0071_f240, interface::menu_manager::fn_0071f240(Ptr) -> Ptr))
        .in_arena(&[Arg::Ptr], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn tutorial_menu_get_class_diff() {
    Diff::new(engine_core::entry!(0x0071_f2d0, interface::menu_manager::tutorial_menu_get_class(Ptr) -> u32))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn tutorial_menu_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0071_f2e0, interface::menu_manager::tutorial_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena(&[Arg::Ptr, Arg::Word], CALLEES, &[0x0071_f310])
    .run()
    .assert_same();
}

#[test]
fn fn_0071f310_diff() {
    Diff::new(engine_core::entry!(
        0x0071_f310,
        interface::menu_manager::fn_0071f310(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_0071f340_diff() {
    Diff::new(engine_core::entry!(0x0071_f340, interface::menu_manager::fn_0071f340(Ptr) -> Ptr))
        .in_arena_without(
            &[Arg::Ptr],
            CALLEES,
            &[0x0071_f540, 0x0071_f5d0],
            &[0x00ec_782f],
        )
        .run()
        .assert_same();
}

#[test]
fn fn_0071f4a0_diff() {
    Diff::new(engine_core::entry!(0x0071_f4a0, interface::menu_manager::fn_0071f4a0(Ptr) -> Ptr))
        .in_arena(&[Arg::Ptr], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_0071f4e0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_f4e0,
        interface::menu_manager::fn_0071f4e0(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_0071f520_diff() {
    Diff::new(engine_core::entry!(0x0071_f520, interface::menu_manager::fn_0071f520(Ptr) -> bool))
        .in_arena(&[Arg::Ptr], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_0071f540_diff() {
    Diff::new(engine_core::entry!(0x0071_f540, interface::menu_manager::fn_0071f540(Ptr) -> Ptr))
        .in_arena(&[Arg::Ptr], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_0071f570_diff() {
    Diff::new(engine_core::entry!(
        0x0071_f570,
        interface::menu_manager::fn_0071f570(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_0071f5d0_diff() {
    Diff::new(engine_core::entry!(0x0071_f5d0, interface::menu_manager::fn_0071f5d0(Ptr) -> Ptr))
        .in_arena(&[Arg::Ptr], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_0071f600_diff() {
    Diff::new(engine_core::entry!(
        0x0071_f600,
        interface::menu_manager::fn_0071f600(Ptr)
    ))
    .in_status_effects(&[Arg::Ptr], &[0x0071_f9f0, 0x0071_f570])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_get_class_diff() {
    Diff::new(
        engine_core::entry!(0x0071_f660, interface::menu_manager::stats_menu_get_class(Ptr) -> u32),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_named_effects_list_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0071_f670, interface::menu_manager::stats_menu_named_effects_list_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena(&[Arg::Ptr, Arg::Word], CALLEES, &[0x0071_f570])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_status_effect_list_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0071_f6a0, interface::menu_manager::stats_menu_status_effect_list_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_status_effects(&[Arg::Ptr, Arg::Word], &[0x0071_f600, 0x0071_f9f0, 0x0071_f570])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0071_f6d0, interface::menu_manager::stats_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_status_effects(&[Arg::Ptr, Arg::Word], &[0x0071_f700, 0x0071_f9f0, 0x0071_f570, 0x0071_f8c0, 0x0071_f900, 0x0071_f930, 0x0071_f600, 0x0072_11e0, 0x0072_1990, 0x0072_2140, 0x0072_28f0, 0x0072_30e0])
    .run()
    .assert_same();
}

#[test]
fn fn_0071f700_diff() {
    Diff::new(engine_core::entry!(
        0x0071_f700,
        interface::menu_manager::fn_0071f700(Ptr)
    ))
    .in_status_effects(
        &[Arg::Ptr],
        &[
            0x0071_f9f0,
            0x0071_f570,
            0x0071_f8c0,
            0x0071_f900,
            0x0071_f930,
            0x0071_f600,
            0x0072_11e0,
            0x0072_1990,
            0x0072_2140,
            0x0072_28f0,
            0x0072_30e0,
        ],
    )
    .run()
    .assert_same();
}

#[test]
fn fn_0071f8c0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_f8c0,
        interface::menu_manager::fn_0071f8c0()
    ))
    .in_arena(&[], CALLEES, &[0x0071_f900, 0x0071_f930])
    .run()
    .assert_same();
}

#[test]
fn fn_0071f900_diff() {
    Diff::new(
        engine_core::entry!(0x0071_f900, interface::menu_manager::fn_0071f900(Ptr, u32) -> Ptr),
    )
    .in_arena(&[Arg::Ptr, Arg::Word], CALLEES, &[0x0071_f930])
    .run()
    .assert_same();
}

#[test]
fn fn_0071f930_diff() {
    Diff::new(engine_core::entry!(
        0x0071_f930,
        interface::menu_manager::fn_0071f930(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_0071f9c0_diff() {
    Diff::new(
        engine_core::entry!(0x0071_f9c0, interface::menu_manager::fn_0071f9c0(Ptr, u32) -> Ptr),
    )
    .in_arena(&[Arg::Ptr, Arg::Word], CALLEES, &[0x0071_f9f0, 0x0071_f570])
    .run()
    .assert_same();
}

#[test]
fn fn_0071f9f0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_f9f0,
        interface::menu_manager::fn_0071f9f0(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[0x0071_f570])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_attach_tile_by_id_diff() {
    Diff::new(engine_core::entry!(
        0x0071_fa50,
        interface::menu_manager::stats_menu_attach_tile_by_id(Ptr, i32, u32)
    ))
    .in_arena(&[Arg::Ptr, Arg::TileId, Arg::Word], CALLEES, &[])
    .max_cases(96)
    .run()
    .assert_same();
}

#[test]
fn fn_0071fa80_diff() {
    Diff::new(engine_core::entry!(0x0071_fa80, interface::menu_manager::fn_0071fa80(Ptr) -> Ptr))
        .in_arena(&[Arg::Ptr], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_0071fb20_diff() {
    Diff::new(engine_core::entry!(0x0071_fb20, interface::menu_manager::fn_0071fb20(Ptr) -> u32))
        .in_arena(&[Arg::Ptr], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_0071fb30_diff() {
    Diff::new(
        engine_core::entry!(0x0071_fb30, interface::menu_manager::fn_0071fb30(Ptr, u32) -> Ptr),
    )
    .in_arena(&[Arg::Ptr, Arg::Word], CALLEES, &[0x0071_fb60])
    .run()
    .assert_same();
}

#[test]
fn fn_0071fb60_diff() {
    Diff::new(engine_core::entry!(
        0x0071_fb60,
        interface::menu_manager::fn_0071fb60(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_0071fbc0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_fbc0,
        interface::menu_manager::fn_0071fbc0(Ptr, i32, u32)
    ))
    .in_arena(&[Arg::Ptr, Arg::TileId, Arg::Word], CALLEES, &[])
    .max_cases(96)
    .run()
    .assert_same();
}

#[test]
fn fn_0071fbf0_diff() {
    Diff::new(engine_core::entry!(0x0071_fbf0, interface::menu_manager::fn_0071fbf0(Ptr) -> Ptr))
        .in_arena_without(&[Arg::Ptr], CALLEES, &[], &[0x00ec_782f])
        .run()
        .assert_same();
}

#[test]
fn surgery_menu_get_class_diff() {
    Diff::new(engine_core::entry!(0x0071_feb0, interface::menu_manager::surgery_menu_get_class(Ptr) -> u32))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn surgery_menu_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0071_fec0, interface::menu_manager::surgery_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena(&[Arg::Ptr, Arg::Word], CALLEES, &[0x0071_fef0])
    .run()
    .assert_same();
}

#[test]
fn fn_0071fef0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_fef0,
        interface::menu_manager::fn_0071fef0(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn surgery_menu_attach_tile_by_id_diff() {
    Diff::new(engine_core::entry!(
        0x0071_ffe0,
        interface::menu_manager::surgery_menu_attach_tile_by_id(Ptr, i32, u32)
    ))
    .in_arena(&[Arg::Ptr, Arg::TileId, Arg::Word], CALLEES, &[0x0072_0010])
    .max_cases(96)
    .run()
    .assert_same();
}

#[test]
fn fn_00720010_diff() {
    Diff::new(
        engine_core::entry!(0x0072_0010, interface::menu_manager::fn_00720010(Ptr, i32) -> i32),
    )
    .in_arena(&[Arg::Ptr, Arg::TileId], CALLEES, &[])
    .max_cases(96)
    .run()
    .assert_same();
}

#[test]
fn fn_00720040_diff() {
    Diff::new(engine_core::entry!(0x0072_0040, interface::menu_manager::fn_00720040(Ptr) -> Ptr))
        .in_arena(&[Arg::Ptr], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn vats_menu_get_class_diff() {
    Diff::new(
        engine_core::entry!(0x0072_0100, interface::menu_manager::vats_menu_get_class(Ptr) -> u32),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn vats_menu_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_0110, interface::menu_manager::vats_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena(&[Arg::Ptr, Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn vats_menu_attach_tile_by_id_diff() {
    Diff::new(engine_core::entry!(
        0x0072_0140,
        interface::menu_manager::vats_menu_attach_tile_by_id(Ptr, i32, u32)
    ))
    .in_arena(&[Arg::Ptr, Arg::TileId, Arg::Word], CALLEES, &[])
    .max_cases(96)
    .run()
    .assert_same();
}

#[test]
fn fn_00720170_diff() {
    Diff::new(engine_core::entry!(0x0072_0170, interface::menu_manager::fn_00720170(Ptr) -> Ptr))
        .in_arena(&[Arg::Ptr], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn special_book_menu_get_class_diff() {
    Diff::new(engine_core::entry!(0x0072_0320, interface::menu_manager::special_book_menu_get_class(Ptr) -> u32))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn special_book_menu_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_0330, interface::menu_manager::special_book_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena(&[Arg::Ptr, Arg::Word], CALLEES, &[0x0072_0420])
    .run()
    .assert_same();
}

#[test]
fn fn_00720360_diff() {
    Diff::new(engine_core::entry!(0x0072_0360, interface::menu_manager::fn_00720360(Ptr) -> Ptr))
        .in_arena(&[Arg::Ptr], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_007203c0_diff() {
    Diff::new(engine_core::entry!(
        0x0072_03c0,
        interface::menu_manager::fn_007203c0(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00720420_diff() {
    Diff::new(engine_core::entry!(
        0x0072_0420,
        interface::menu_manager::fn_00720420(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00720550_diff() {
    Diff::new(engine_core::entry!(0x0072_0550, interface::menu_manager::fn_00720550(Ptr) -> Ptr))
        .in_arena(&[Arg::Ptr], CALLEES, &[0x0072_07a0])
        .run()
        .assert_same();
}

#[test]
fn love_tester_menu_get_class_diff() {
    Diff::new(engine_core::entry!(0x0072_0760, interface::menu_manager::love_tester_menu_get_class(Ptr) -> u32))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn love_tester_menu_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_0770, interface::menu_manager::love_tester_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena(&[Arg::Ptr, Arg::Word], CALLEES, &[0x0072_0860, 0x0072_0800])
    .run()
    .assert_same();
}

#[test]
fn fn_007207a0_diff() {
    Diff::new(engine_core::entry!(0x0072_07a0, interface::menu_manager::fn_007207a0(Ptr) -> Ptr))
        .in_arena(&[Arg::Ptr], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn fn_00720800_diff() {
    Diff::new(engine_core::entry!(
        0x0072_0800,
        interface::menu_manager::fn_00720800(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00720860_diff() {
    Diff::new(engine_core::entry!(
        0x0072_0860,
        interface::menu_manager::fn_00720860(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[0x0072_0800])
    .run()
    .assert_same();
}

#[test]
fn tutorial_menu_attach_tile_by_id_diff() {
    Diff::new(engine_core::entry!(
        0x0072_09e0,
        interface::menu_manager::tutorial_menu_attach_tile_by_id(Ptr, i32, u32)
    ))
    .in_arena(&[Arg::Ptr, Arg::TileId, Arg::Word], CALLEES, &[])
    .max_cases(96)
    .run()
    .assert_same();
}

#[test]
fn fn_00720a10_diff() {
    Diff::new(engine_core::entry!(0x0072_0a10, interface::menu_manager::fn_00720a10(Ptr) -> Ptr))
        .in_arena(&[Arg::Ptr], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn quantity_menu_get_class_diff() {
    Diff::new(engine_core::entry!(0x0072_0a30, interface::menu_manager::quantity_menu_get_class(Ptr) -> u32))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn quantity_menu_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_0a40, interface::menu_manager::quantity_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena(&[Arg::Ptr, Arg::Word], CALLEES, &[0x0072_0a70])
    .run()
    .assert_same();
}

#[test]
fn fn_00720a70_diff() {
    Diff::new(engine_core::entry!(
        0x0072_0a70,
        interface::menu_manager::fn_00720a70(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn menu_manager_reload_menu_diff() {
    Diff::new(engine_core::entry!(0x0072_0aa0, interface::menu_manager::menu_manager_reload_menu(Ptr, u32) -> u32))
    .in_arena_and(
        &[Arg::Ptr, Arg::Class],
        CALLEES,
        &[],
        &RELOAD_VARIANTS,
        put_reload_casts,
    )
    .max_cases(240)
    .run()
    .assert_same();
}

#[test]
fn fn_00720df0_diff() {
    Diff::new(engine_core::entry!(0x0072_0df0, interface::menu_manager::fn_00720df0() -> u32))
        .in_arena(&[], CALLEES, &[])
        .run()
        .assert_same();
}

#[test]
fn menu_manager_delete_menu_diff() {
    Diff::new(engine_core::entry!(0x0072_0e00, interface::menu_manager::menu_manager_delete_menu(Ptr, u32) -> bool))
    .in_arena(&[Arg::Ptr, Arg::Class], CALLEES, &[])
    .max_cases(96)
    .run()
    .assert_same();
}

#[test]
fn menu_manager_get_frontmost_menu_diff() {
    Diff::new(engine_core::entry!(0x0072_0e60, interface::menu_manager::menu_manager_get_frontmost_menu(Ptr) -> u32))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &MENU_LIST_VARIANTS, put_menu_list)
    .max_cases(60)
    .run()
    .assert_same();
}

#[test]
fn menu_manager_is_menu_open_diff() {
    Diff::new(engine_core::entry!(0x0072_0f90, interface::menu_manager::menu_manager_is_menu_open(Ptr, u32) -> bool))
    .in_arena_and(
        &[Arg::Ptr, Arg::Int],
        CALLEES,
        &[],
        &MENU_LIST_VARIANTS,
        put_menu_list,
    )
    .max_cases(480)
    .run()
    .assert_same();
}

#[test]
fn fn_00721060_diff() {
    Diff::new(
        engine_core::entry!(0x0072_1060, interface::menu_manager_p2::fn_00721060(Ptr) -> Ptr),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00721130_diff() {
    Diff::new(engine_core::entry!(
        0x0072_1130,
        interface::menu_manager_p2::fn_00721130(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00721150_diff() {
    Diff::new(engine_core::entry!(
        0x0072_1150,
        interface::menu_manager_p2::fn_00721150(Ptr)
    ))
    .in_arena_and(
        &[Arg::Ptr],
        CALLEES,
        &[0x0072_11e0],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn fn_007211c0_diff() {
    Diff::new(engine_core::entry!(
        0x0072_11c0,
        interface::menu_manager_p2::fn_007211c0(u32)
    ))
    .in_arena(&[Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn list_box_actor_value_remove_all_diff() {
    Diff::new(engine_core::entry!(
        0x0072_11e0,
        interface::menu_manager_p2::list_box_actor_value_remove_all(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_actor_value_sort_diff() {
    Diff::new(engine_core::entry!(
        0x0072_1410,
        interface::menu_manager_p2::list_box_actor_value_sort(Ptr, u32)
    ))
    .in_arena_and(
        &[Arg::Ptr, Arg::Word],
        CALLEES,
        &[],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_actor_value_highlight_diff() {
    Diff::new(engine_core::entry!(0x0072_1440, interface::menu_manager_p2::list_box_actor_value_highlight(Ptr, u32) -> bool))
    .in_arena_and(&[Arg::Ptr, Arg::ListTile], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_actor_value_get_highlight_target_diff() {
    Diff::new(engine_core::entry!(0x0072_14b0, interface::menu_manager_p2::list_box_actor_value_get_highlight_target(Ptr, u32) -> u32))
    .in_arena_and(&[Arg::Ptr, Arg::Direction], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(56)
    .run()
    .assert_same();
}

#[test]
fn fn_00721850_diff() {
    Diff::new(
        engine_core::entry!(0x0072_1850, interface::menu_manager_p2::fn_00721850(Ptr) -> Ptr),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00721920_diff() {
    Diff::new(engine_core::entry!(
        0x0072_1920,
        interface::menu_manager_p2::fn_00721920(Ptr)
    ))
    .in_arena_and(
        &[Arg::Ptr],
        CALLEES,
        &[0x0072_1990],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_perk_rank_data_remove_all_diff() {
    Diff::new(engine_core::entry!(
        0x0072_1990,
        interface::menu_manager_p2::list_box_perk_rank_data_remove_all(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_perk_rank_data_sort_diff() {
    Diff::new(engine_core::entry!(
        0x0072_1bc0,
        interface::menu_manager_p2::list_box_perk_rank_data_sort(Ptr, u32)
    ))
    .in_arena_and(
        &[Arg::Ptr, Arg::Word],
        CALLEES,
        &[],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_perk_rank_data_highlight_diff() {
    Diff::new(engine_core::entry!(0x0072_1bf0, interface::menu_manager_p2::list_box_perk_rank_data_highlight(Ptr, u32) -> bool))
    .in_arena_and(&[Arg::Ptr, Arg::ListTile], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_perk_rank_data_get_highlight_target_diff() {
    Diff::new(engine_core::entry!(0x0072_1c60, interface::menu_manager_p2::list_box_perk_rank_data_get_highlight_target(Ptr, u32) -> u32))
    .in_arena_and(&[Arg::Ptr, Arg::Direction], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(56)
    .run()
    .assert_same();
}

#[test]
fn fn_00722000_diff() {
    Diff::new(
        engine_core::entry!(0x0072_2000, interface::menu_manager_p2::fn_00722000(Ptr) -> Ptr),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_007220d0_diff() {
    Diff::new(engine_core::entry!(
        0x0072_20d0,
        interface::menu_manager_p2::fn_007220d0(Ptr)
    ))
    .in_arena_and(
        &[Arg::Ptr],
        CALLEES,
        &[0x0072_2140],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_misc_stat_id_remove_all_diff() {
    Diff::new(engine_core::entry!(
        0x0072_2140,
        interface::menu_manager_p2::list_box_misc_stat_id_remove_all(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_misc_stat_id_sort_diff() {
    Diff::new(engine_core::entry!(
        0x0072_2370,
        interface::menu_manager_p2::list_box_misc_stat_id_sort(Ptr, u32)
    ))
    .in_arena_and(
        &[Arg::Ptr, Arg::Word],
        CALLEES,
        &[],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_misc_stat_id_highlight_diff() {
    Diff::new(engine_core::entry!(0x0072_23a0, interface::menu_manager_p2::list_box_misc_stat_id_highlight(Ptr, u32) -> bool))
    .in_arena_and(&[Arg::Ptr, Arg::ListTile], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_misc_stat_id_get_highlight_target_diff() {
    Diff::new(engine_core::entry!(0x0072_2410, interface::menu_manager_p2::list_box_misc_stat_id_get_highlight_target(Ptr, u32) -> u32))
    .in_arena_and(&[Arg::Ptr, Arg::Direction], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(56)
    .run()
    .assert_same();
}

#[test]
fn fn_007227b0_diff() {
    Diff::new(
        engine_core::entry!(0x0072_27b0, interface::menu_manager_p2::fn_007227b0(Ptr) -> Ptr),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00722880_diff() {
    Diff::new(engine_core::entry!(
        0x0072_2880,
        interface::menu_manager_p2::fn_00722880(Ptr)
    ))
    .in_arena_and(
        &[Arg::Ptr],
        CALLEES,
        &[0x0072_28f0],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_status_effect_remove_all_diff() {
    Diff::new(engine_core::entry!(
        0x0072_28f0,
        interface::menu_manager_p2::list_box_status_effect_remove_all(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_status_effect_sort_diff() {
    Diff::new(engine_core::entry!(
        0x0072_2b20,
        interface::menu_manager_p2::list_box_status_effect_sort(Ptr, u32)
    ))
    .in_arena_and(
        &[Arg::Ptr, Arg::Word],
        CALLEES,
        &[],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_status_effect_highlight_diff() {
    Diff::new(engine_core::entry!(0x0072_2b50, interface::menu_manager_p2::list_box_status_effect_highlight(Ptr, u32) -> bool))
    .in_arena_and(&[Arg::Ptr, Arg::ListTile], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_status_effect_get_highlight_target_diff() {
    Diff::new(engine_core::entry!(0x0072_2bc0, interface::menu_manager_p2::list_box_status_effect_get_highlight_target(Ptr, u32) -> u32))
    .in_arena_and(&[Arg::Ptr, Arg::Direction], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(56)
    .run()
    .assert_same();
}

#[test]
fn fn_00722f60_diff() {
    Diff::new(
        engine_core::entry!(0x0072_2f60, interface::menu_manager_p2::fn_00722f60(Ptr) -> Ptr),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00723070_diff() {
    Diff::new(engine_core::entry!(
        0x0072_3070,
        interface::menu_manager_p2::fn_00723070(Ptr)
    ))
    .in_arena_and(
        &[Arg::Ptr],
        CALLEES,
        &[0x0072_30e0],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_tes_reputation_remove_all_diff() {
    Diff::new(engine_core::entry!(
        0x0072_30e0,
        interface::menu_manager_p2::list_box_tes_reputation_remove_all(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_tes_reputation_sort_diff() {
    Diff::new(engine_core::entry!(
        0x0072_3310,
        interface::menu_manager_p2::list_box_tes_reputation_sort(Ptr, u32)
    ))
    .in_arena_and(
        &[Arg::Ptr, Arg::Word],
        CALLEES,
        &[],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_tes_reputation_highlight_diff() {
    Diff::new(engine_core::entry!(0x0072_3340, interface::menu_manager_p2::list_box_tes_reputation_highlight(Ptr, u32) -> bool))
    .in_arena_and(&[Arg::Ptr, Arg::ListTile], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_tes_reputation_get_highlight_target_diff() {
    Diff::new(engine_core::entry!(0x0072_33b0, interface::menu_manager_p2::list_box_tes_reputation_get_highlight_target(Ptr, u32) -> u32))
    .in_arena_and(&[Arg::Ptr, Arg::Direction], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(56)
    .run()
    .assert_same();
}

#[test]
fn fn_00723750_diff() {
    Diff::new(
        engine_core::entry!(0x0072_3750, interface::menu_manager_p2::fn_00723750(Ptr) -> Ptr),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00723820_diff() {
    Diff::new(engine_core::entry!(
        0x0072_3820,
        interface::menu_manager_p2::fn_00723820(Ptr)
    ))
    .in_arena_and(
        &[Arg::Ptr],
        CALLEES,
        &[0x0072_3890],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_unsigned_int_remove_all_diff() {
    Diff::new(engine_core::entry!(
        0x0072_3890,
        interface::menu_manager_p2::list_box_unsigned_int_remove_all(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_unsigned_int_sort_diff() {
    Diff::new(engine_core::entry!(
        0x0072_3ac0,
        interface::menu_manager_p2::list_box_unsigned_int_sort(Ptr, u32)
    ))
    .in_arena_and(
        &[Arg::Ptr, Arg::Word],
        CALLEES,
        &[],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_unsigned_int_highlight_diff() {
    Diff::new(engine_core::entry!(0x0072_3af0, interface::menu_manager_p2::list_box_unsigned_int_highlight(Ptr, u32) -> bool))
    .in_arena_and(&[Arg::Ptr, Arg::ListTile], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_unsigned_int_get_highlight_target_diff() {
    Diff::new(engine_core::entry!(0x0072_3b60, interface::menu_manager_p2::list_box_unsigned_int_get_highlight_target(Ptr, u32) -> u32))
    .in_arena_and(&[Arg::Ptr, Arg::Direction], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(56)
    .run()
    .assert_same();
}

#[test]
fn fn_00723f00_diff() {
    Diff::new(
        engine_core::entry!(0x0072_3f00, interface::menu_manager_p2::fn_00723f00(Ptr) -> Ptr),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00723f30_diff() {
    Diff::new(engine_core::entry!(
        0x0072_3f30,
        interface::menu_manager_p2::fn_00723f30(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00723f50_diff() {
    Diff::new(
        engine_core::entry!(0x0072_3f50, interface::menu_manager_p2::fn_00723f50(Ptr) -> Ptr),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00723f80_diff() {
    Diff::new(engine_core::entry!(
        0x0072_3f80,
        interface::menu_manager_p2::fn_00723f80(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00723fa0_diff() {
    Diff::new(
        engine_core::entry!(0x0072_3fa0, interface::menu_manager_p2::fn_00723fa0(Ptr) -> Ptr),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00723fd0_diff() {
    Diff::new(engine_core::entry!(
        0x0072_3fd0,
        interface::menu_manager_p2::fn_00723fd0(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00723ff0_diff() {
    Diff::new(
        engine_core::entry!(0x0072_3ff0, interface::menu_manager_p2::fn_00723ff0(Ptr) -> Ptr),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00724020_diff() {
    Diff::new(engine_core::entry!(
        0x0072_4020,
        interface::menu_manager_p2::fn_00724020(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00724040_diff() {
    Diff::new(
        engine_core::entry!(0x0072_4040, interface::menu_manager_p2::fn_00724040(Ptr) -> Ptr),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn fn_00724070_diff() {
    Diff::new(engine_core::entry!(
        0x0072_4070,
        interface::menu_manager_p2::fn_00724070(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn list_box_actor_value_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_4090, interface::menu_manager_p2::list_box_actor_value_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena_and(&[Arg::Ptr, Arg::Byte], CALLEES, &[0x0072_1150, 0x0072_11e0], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn fn_007240c0_diff() {
    Diff::new(
        engine_core::entry!(0x0072_40c0, interface::menu_manager_p2::fn_007240c0(Ptr) -> Ptr),
    )
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn list_box_perk_rank_data_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_40e0, interface::menu_manager_p2::list_box_perk_rank_data_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena_and(&[Arg::Ptr, Arg::Byte], CALLEES, &[0x0072_1920, 0x0072_1990], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_misc_stat_id_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_4110, interface::menu_manager_p2::list_box_misc_stat_id_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena_and(&[Arg::Ptr, Arg::Byte], CALLEES, &[0x0072_20d0, 0x0072_2140], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_status_effect_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_4140, interface::menu_manager_p2::list_box_status_effect_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena_and(&[Arg::Ptr, Arg::Byte], CALLEES, &[0x0072_2880, 0x0072_28f0], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_tes_reputation_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_4170, interface::menu_manager_p2::list_box_tes_reputation_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena_and(&[Arg::Ptr, Arg::Byte], CALLEES, &[0x0072_3070, 0x0072_30e0], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_unsigned_int_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_41a0, interface::menu_manager_p2::list_box_unsigned_int_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena_and(&[Arg::Ptr, Arg::Byte], CALLEES, &[0x0072_3820, 0x0072_3890], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn simple_array_special_book_text_item_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_41d0, interface::menu_manager_p2::simple_array_special_book_text_item_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena(&[Arg::Ptr, Arg::Byte], CALLEES, &[0x0072_3f30])
    .run()
    .assert_same();
}

#[test]
fn simple_array_special_book_button_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_4200, interface::menu_manager_p2::simple_array_special_book_button_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena(&[Arg::Ptr, Arg::Byte], CALLEES, &[0x0072_3f80])
    .run()
    .assert_same();
}

#[test]
fn simple_array_love_tester_text_item_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_4230, interface::menu_manager_p2::simple_array_love_tester_text_item_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena(&[Arg::Ptr, Arg::Byte], CALLEES, &[0x0072_3fd0])
    .run()
    .assert_same();
}

#[test]
fn simple_array_love_tester_light_bulb_glows_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_4260, interface::menu_manager_p2::simple_array_love_tester_light_bulb_glows_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena(&[Arg::Ptr, Arg::Byte], CALLEES, &[0x0072_4020])
    .run()
    .assert_same();
}

#[test]
fn simple_array_love_tester_button_scalar_deleting_destructor_diff() {
    Diff::new(engine_core::entry!(0x0072_4290, interface::menu_manager_p2::simple_array_love_tester_button_scalar_deleting_destructor(Ptr, u32) -> Ptr))
    .in_arena(&[Arg::Ptr, Arg::Byte], CALLEES, &[0x0072_4070])
    .run()
    .assert_same();
}

#[test]
fn fn_007242c0_diff() {
    Diff::new(
        engine_core::entry!(0x0072_42c0, interface::menu_manager_p2::fn_007242c0(Ptr, u32) -> bool),
    )
    .in_arena_and(
        &[Arg::Ptr, Arg::Int],
        CALLEES,
        &[],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn fn_007242f0_diff() {
    Diff::new(
        engine_core::entry!(0x0072_42f0, interface::menu_manager_p2::fn_007242f0(Ptr, u32) -> bool),
    )
    .in_arena_and(
        &[Arg::Ptr, Arg::ListTile],
        CALLEES,
        &[],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(42)
    .run()
    .assert_same();
}

#[test]
fn list_box_actor_value_synchronize_diff() {
    Diff::new(engine_core::entry!(
        0x0072_4360,
        interface::menu_manager_p2::list_box_actor_value_synchronize(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_actor_value_is_enabled_diff() {
    Diff::new(engine_core::entry!(0x0072_4430, interface::menu_manager_p2::list_box_actor_value_is_enabled(Ptr) -> bool))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_actor_value_update_highlight_box_pos_diff() {
    Diff::new(engine_core::entry!(
        0x0072_44e0,
        interface::menu_manager_p2::list_box_actor_value_update_highlight_box_pos(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_actor_value_set_total_list_height_diff() {
    Diff::new(engine_core::entry!(
        0x0072_4800,
        interface::menu_manager_p2::list_box_actor_value_set_total_list_height(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_perk_rank_data_synchronize_diff() {
    Diff::new(engine_core::entry!(
        0x0072_4930,
        interface::menu_manager_p2::list_box_perk_rank_data_synchronize(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_perk_rank_data_is_enabled_diff() {
    Diff::new(engine_core::entry!(0x0072_4a00, interface::menu_manager_p2::list_box_perk_rank_data_is_enabled(Ptr) -> bool))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_perk_rank_data_update_highlight_box_pos_diff() {
    Diff::new(engine_core::entry!(
        0x0072_4ab0,
        interface::menu_manager_p2::list_box_perk_rank_data_update_highlight_box_pos(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_perk_rank_data_set_total_list_height_diff() {
    Diff::new(engine_core::entry!(
        0x0072_4dd0,
        interface::menu_manager_p2::list_box_perk_rank_data_set_total_list_height(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_misc_stat_id_synchronize_diff() {
    Diff::new(engine_core::entry!(
        0x0072_4f00,
        interface::menu_manager_p2::list_box_misc_stat_id_synchronize(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_misc_stat_id_is_enabled_diff() {
    Diff::new(engine_core::entry!(0x0072_4fd0, interface::menu_manager_p2::list_box_misc_stat_id_is_enabled(Ptr) -> bool))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_misc_stat_id_update_highlight_box_pos_diff() {
    Diff::new(engine_core::entry!(
        0x0072_5080,
        interface::menu_manager_p2::list_box_misc_stat_id_update_highlight_box_pos(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_misc_stat_id_set_total_list_height_diff() {
    Diff::new(engine_core::entry!(
        0x0072_53a0,
        interface::menu_manager_p2::list_box_misc_stat_id_set_total_list_height(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_status_effect_synchronize_diff() {
    Diff::new(engine_core::entry!(
        0x0072_54d0,
        interface::menu_manager_p2::list_box_status_effect_synchronize(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_status_effect_is_enabled_diff() {
    Diff::new(engine_core::entry!(0x0072_55a0, interface::menu_manager_p2::list_box_status_effect_is_enabled(Ptr) -> bool))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_status_effect_update_highlight_box_pos_diff() {
    Diff::new(engine_core::entry!(
        0x0072_5650,
        interface::menu_manager_p2::list_box_status_effect_update_highlight_box_pos(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_status_effect_set_total_list_height_diff() {
    Diff::new(engine_core::entry!(
        0x0072_5970,
        interface::menu_manager_p2::list_box_status_effect_set_total_list_height(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_tes_reputation_synchronize_diff() {
    Diff::new(engine_core::entry!(
        0x0072_5aa0,
        interface::menu_manager_p2::list_box_tes_reputation_synchronize(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_tes_reputation_is_enabled_diff() {
    Diff::new(engine_core::entry!(0x0072_5b70, interface::menu_manager_p2::list_box_tes_reputation_is_enabled(Ptr) -> bool))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_tes_reputation_update_highlight_box_pos_diff() {
    Diff::new(engine_core::entry!(
        0x0072_5c20,
        interface::menu_manager_p2::list_box_tes_reputation_update_highlight_box_pos(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_tes_reputation_set_total_list_height_diff() {
    Diff::new(engine_core::entry!(
        0x0072_5f40,
        interface::menu_manager_p2::list_box_tes_reputation_set_total_list_height(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_unsigned_int_synchronize_diff() {
    Diff::new(engine_core::entry!(
        0x0072_6090,
        interface::menu_manager_p2::list_box_unsigned_int_synchronize(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_unsigned_int_is_enabled_diff() {
    Diff::new(engine_core::entry!(0x0072_6160, interface::menu_manager_p2::list_box_unsigned_int_is_enabled(Ptr) -> bool))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_unsigned_int_update_highlight_box_pos_diff() {
    Diff::new(engine_core::entry!(
        0x0072_6210,
        interface::menu_manager_p2::list_box_unsigned_int_update_highlight_box_pos(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_unsigned_int_set_total_list_height_diff() {
    Diff::new(engine_core::entry!(
        0x0072_6530,
        interface::menu_manager_p2::list_box_unsigned_int_set_total_list_height(Ptr)
    ))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_actor_value_num_unfiltered_diff() {
    Diff::new(engine_core::entry!(0x0072_6660, interface::menu_manager_p2::list_box_actor_value_num_unfiltered(Ptr) -> i32))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_perk_rank_data_num_unfiltered_diff() {
    Diff::new(engine_core::entry!(0x0072_6710, interface::menu_manager_p2::list_box_perk_rank_data_num_unfiltered(Ptr) -> i32))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_misc_stat_id_num_unfiltered_diff() {
    Diff::new(engine_core::entry!(0x0072_67c0, interface::menu_manager_p2::list_box_misc_stat_id_num_unfiltered(Ptr) -> i32))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_status_effect_num_unfiltered_diff() {
    Diff::new(engine_core::entry!(0x0072_6870, interface::menu_manager_p2::list_box_status_effect_num_unfiltered(Ptr) -> i32))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_tes_reputation_num_unfiltered_diff() {
    Diff::new(engine_core::entry!(0x0072_6920, interface::menu_manager_p2::list_box_tes_reputation_num_unfiltered(Ptr) -> i32))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

#[test]
fn list_box_unsigned_int_num_unfiltered_diff() {
    Diff::new(engine_core::entry!(0x0072_6b40, interface::menu_manager_p2::list_box_unsigned_int_num_unfiltered(Ptr) -> i32))
    .in_arena_and(&[Arg::Ptr], CALLEES, &[], &LIST_BOX_VARIANTS, put_list_box)
    .max_cases(28)
    .run()
    .assert_same();
}

/// The variants of [`put_list_box`]'s world, the richest first: the cases that vary one input at a
/// time vary it against this one.
const LIST_BOX_VARIANTS: [u32; 7] = [6, 5, 3, 2, 1, 4, 0];

/// The guard words of the list box instances' function-local statics: the six of the menu manager
/// file (`IsEnabled`'s, `UpdateHighlightBoxPos`'s, `NumUnfiltered`'s of each) and the item list
/// box's (`UpdateHighlightBoxPos`'s, `NumUnfiltered`'s).
const LIST_BOX_GUARDS: [u32; 20] = [
    0x011d_8dd0,
    0x011d_8ddc,
    0x011d_8e48,
    0x011d_8de4,
    0x011d_8df0,
    0x011d_8e50,
    0x011d_8df8,
    0x011d_8e04,
    0x011d_8e58,
    0x011d_8e0c,
    0x011d_8e18,
    0x011d_8e60,
    0x011d_8e20,
    0x011d_8e2c,
    0x011d_8e68,
    0x011d_8e34,
    0x011d_8e40,
    0x011d_8e70,
    0x011d_8cd4,
    0x011d_8cdc,
];

/// A world for the list boxes (`ListBox<T>`, Xbox PDB), built at both pointer arguments (objects 3
/// and 40): the embedded list (its first item at +4, the nodes after it from +8, each node an item
/// and the next node, objects 161 on), each item (objects 170 on) a tile (objects 180 on, the
/// `Arg::ListTile` values), its data (+4) and its filtered byte (+8); the container (+0xc, object
/// 196), the highlighted tile (+0x10), the scroll bar (+0x14, object 197), `usNextIndex` (+0x1c),
/// `fTotalListHeight` (+0x20) and `usFlags` (+0x2c); the six instances' statics (their guards all
/// clear or all set); `RemoveAll`'s keep-the-tiles byte. The tile float getter answers a tile's
/// index (`0xfac`) from its +0x40, its height (`0xfb0`) from +0x44 and any other trait (the scroll
/// delta, the filtered count) from +0x48; `Tile::IsTrue` its +0x4c; `_ftol2` truncates. The
/// variants: no items and a disabled container; one item, highlighted, with a scroll bar, a height
/// to scroll and the statics set; four with one filtered and one disabled, the tiles kept and the
/// heights equal; five enabled and none highlighted; no container; six with only the last enabled
/// (the moves search on to it); six with the first and the last disabled and the last highlighted
/// (the moves walk back from the ends through `GetTileAt`); and (the eighth, the item list box's
/// only, [`ITEM_LIST_VARIANTS`]) one item, highlighted, with a scroll bar, two visible items, flags 0
/// and 1 set.
fn put_list_box(e: &mut Engine, variant: u32) {
    use arena::object;
    let v = variant as usize;
    let counts = [0u32, 1, 4, 5, 3, 6, 6, 1];
    let filtered = [None, None, Some(1u32), None, None, None, None, None];
    let enabled = [
        0u32, 0b1, 0b1011, 0b1_1111, 0b111, 0b10_0000, 0b01_1110, 0b1,
    ];
    let highlighted = [
        None,
        Some(0u32),
        Some(0),
        None,
        Some(1),
        Some(4),
        Some(5),
        Some(0),
    ];
    let visible = [0.0f32, 100.0, 300.0, 25.0, 0.0, 100.0, 100.0, 100.0];
    let other = [0.0f32, 0.0, 1.0, 2.0, 0.0, 50.0, 3.0, 2.0];
    let totals = [0.0f32, 300.0, 300.0, 50.0, 100.0, 1000.0, 600.0, 300.0];
    let flags = [0u16, 2, 0, 2, 2, 0, 3, 3];
    let tile = |k: u32| object(180 + k);
    let item = |k: u32| object(170 + k);
    let node = |k: u32| object(160 + k);
    let n = counts[v];
    let mut unfiltered = 0u16;
    for k in 0..n {
        let i = item(k);
        let t = tile(k);
        let is_filtered = filtered[v] == Some(k);
        e.mem.set_u32(i, t);
        e.mem
            .set_u32(i + 4, if k % 2 == 0 { object(190 + k) } else { 0 });
        e.mem.set_u8(i + 8, u8::from(is_filtered));
        e.mem.set_f32(
            t + 0x40,
            if is_filtered {
                7.0
            } else {
                f32::from(unfiltered)
            },
        );
        if !is_filtered {
            unfiltered += 1;
        }
        e.mem.set_f32(t + 0x44, 20.0);
        e.mem.set_f32(t + 0x48, 0.0);
        e.mem.set_u32(t + 0x4c, (enabled[v] >> k) & 1);
        e.mem.set_u8(t + 0x35, u8::from(k % 2 == 1));
        if k >= 1 {
            e.mem.set_u32(node(k), i);
            e.mem
                .set_u32(node(k) + 4, if k + 1 < n { node(k + 1) } else { 0 });
        }
    }
    let container = if v == 4 { 0 } else { object(196) };
    if container != 0 {
        e.mem.set_f32(container + 0x44, visible[v]);
        e.mem.set_f32(container + 0x48, other[v]);
        e.mem.set_u32(container + 0x4c, u32::from(v != 0));
    }
    for list_box in [object(3), object(40)] {
        e.mem.set_u32(list_box + 4, if n > 0 { item(0) } else { 0 });
        e.mem.set_u32(list_box + 8, if n > 1 { node(1) } else { 0 });
        e.mem.set_u32(list_box + 0xc, container);
        e.mem
            .set_u32(list_box + 0x10, highlighted[v].map_or(0, tile));
        e.mem
            .set_u32(list_box + 0x14, if v % 2 == 1 { object(197) } else { 0 });
        e.mem.set_u16(list_box + 0x1c, unfiltered);
        e.mem.set_f32(list_box + 0x20, totals[v]);
        e.mem.set_u16(list_box + 0x2c, flags[v]);
    }
    let guard = if v % 2 == 1 { 7 } else { 0 };
    for at in LIST_BOX_GUARDS {
        e.mem.set_u32(at, guard);
    }
    e.mem.set_u8(0x011d_8909, u8::from(v == 2));
    e.register_double(0x00a0_11b0, |e, words| {
        let at = match words[1] {
            0xfac => 0x40,
            0xfb0 => 0x44,
            _ => 0x48,
        };
        Ret {
            st0: f64::from(e.mem.f32(words[0].wrapping_add(at))),
            ..Ret::default()
        }
    });
    e.register_double(0x00a0_1230, |e, words| Ret {
        eax: e.mem.u32(words[0].wrapping_add(0x4c)) & 1,
        ..Ret::default()
    });
    e.register_double(0x00ec_62c0, |_, words| {
        let value = f64::from_bits(u64::from(words[0]) | u64::from(words[1]) << 32);
        Ret {
            eax: engine_core::x87::fistp_chop(value) as u32,
            ..Ret::default()
        }
    });
}

/// The variants of [`put_allocator`]'s world.
const ALLOCATOR_VARIANTS: [u32; 2] = [0, 1];

/// The allocator (`00401000`) as the arena has it (variant 0: its stand-in, whose first answer is
/// never 0 in these seeds) or failing (variant 1: every allocation answers 0), so `Instance` and
/// `CreateMenuByClass` take their failed-allocation branches too.
fn put_allocator(e: &mut Engine, variant: u32) {
    if variant == 1 {
        e.register_double(0x0040_1000, |_, _| Ret::default());
    }
}

/// The variants of [`put_menu_list`]'s world.
const MENU_LIST_VARIANTS: [u32; 5] = [0, 1, 2, 3, 4];

/// A world for `GetFrontmostMenu` and `IsMenuOpen` that reaches each of their tests: the manager
/// (`011d8a80`) with a menus root whose tile list has six nodes (the second's tile null, the third's
/// tile with no menu, the fourth's menu with no tile); `Tile::GetMenu` answering a tile's `+0x10`;
/// each menu's class (its vtable's `+0x34`) its `+0x30`, its state its `+0x24`; the tile float
/// getter answering the depth (`0xfad`) from a menu tile's `+0x40`, the kind (`0xfa7`) from its
/// `+0x44`; `Tile::IsTrue` its `+0x48`. The variant picks the six depths, kinds, states, classes and
/// enabled bits: a deeper menu of the passed-over kind (6008.0), equal depths, a NaN depth and
/// kind, states 2, 3 and 4, a matching menu enabled or not; and (the fifth) a deeper menu in state 2
/// above a NaN kind on an ordinary depth, so the state test and the NaN kind each decide the answer.
fn put_menu_list(e: &mut Engine, variant: u32) {
    use arena::object;
    const NAN: f32 = f32::NAN;
    let v = variant as usize;
    let depths: [[f32; 6]; 5] = [
        [1.0, 9.0, 9.0, 9.0, 3.0, 2.0],
        [5.0, 9.0, 9.0, 9.0, 5.0, 5.0],
        [NAN, 9.0, 9.0, 9.0, 1.0, 0.5],
        [-2.0e8, 9.0, 9.0, 9.0, -3.0e8, 7.0],
        [1.0, 9.0, 9.0, 9.0, 3.0, 2.0],
    ];
    let kinds: [[f32; 6]; 5] = [
        [0.0, 0.0, 0.0, 0.0, 6008.0, 0.0],
        [6008.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [NAN, 0.0, 0.0, 0.0, 6008.0, 1.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, 6008.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, NAN],
    ];
    let states: [[u32; 6]; 5] = [
        [0, 0, 0, 0, 1, 4],
        [2, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 2, 3],
        [4, 0, 0, 0, 3, 2],
        [0, 0, 0, 0, 2, 0],
    ];
    let classes: [[u32; 6]; 5] = [
        [0x3e9, 1, 1, 1, 0x3ef, 1],
        [1, 2, 2, 2, 1, 1],
        [2, 0x3e9, 0x3e9, 0x3e9, 0x3e9, 2],
        [0x43d, 0, 0, 0, 0x3e9, 1],
        [1, 1, 1, 1, 0x3e9, 1],
    ];
    let enabled: [[u32; 6]; 5] = [
        [1, 1, 1, 1, 1, 0],
        [1, 1, 1, 1, 0, 1],
        [0, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1],
    ];
    let manager = object(70);
    e.mem.set_u32(0x011d_8a80, manager);
    e.mem.set_u32(manager + 0x9c, object(71));
    e.mem.set_u32(object(71) + 4, object(72));
    let vtable = object(79);
    e.mem.set_u32(vtable + 0x34, 0x7d00_2000);
    for k in 0..6u32 {
        let node = object(72 + k);
        e.mem.set_u32(node, if k < 5 { object(73 + k) } else { 0 });
        let tile = if k == 1 { 0 } else { object(80 + k) };
        e.mem.set_u32(node + 8, tile);
        let menu = if k == 2 { 0 } else { object(90 + k) };
        if tile != 0 {
            e.mem.set_u32(tile + 0x10, menu);
        }
        let menu_tile = if k == 3 { 0 } else { object(100 + k) };
        let i = k as usize;
        if menu == 0 {
            continue;
        }
        e.mem.set_u32(menu, vtable);
        e.mem.set_u32(menu + 4, menu_tile);
        e.mem.set_u32(menu + 0x24, states[v][i]);
        e.mem.set_u32(menu + 0x30, classes[v][i]);
        if menu_tile != 0 {
            e.mem.set_f32(menu_tile + 0x40, depths[v][i]);
            e.mem.set_f32(menu_tile + 0x44, kinds[v][i]);
            e.mem.set_u32(menu_tile + 0x48, enabled[v][i]);
        }
    }
    e.mem.set_f32(0x0107_0218, -1.0e8);
    e.mem.set_u64(0x0107_0210, 6008.0f64.to_bits());
    e.register_double(0x00a0_3c90, |e, words| Ret {
        eax: e.mem.u32(words[0] + 0x10),
        ..Ret::default()
    });
    e.register_double(0x7d00_2000, |e, words| Ret {
        eax: e.mem.u32(words[0] + 0x30),
        ..Ret::default()
    });
    e.register_double(0x00a0_11b0, |e, words| {
        let at = match words[1] {
            0xfad => 0x40,
            0xfa7 => 0x44,
            _ => 0x4c,
        };
        Ret {
            st0: f64::from(e.mem.f32(words[0] + at)),
            ..Ret::default()
        }
    });
    e.register_double(0x00a0_1230, |e, words| Ret {
        eax: e.mem.u32(words[0] + 0x48) & 1,
        ..Ret::default()
    });
}

/// The variants of [`put_reload_casts`]'s world.
const RELOAD_VARIANTS: [u32; 2] = [0, 1];

/// A world for `ReloadMenu`'s container and book menus: the menu of class 0x3f0 (an object whose
/// `+0x70` and `+0x74` differ, its `+0x30` a reference whose `+0x20` is an object), and
/// `__RTDynamicCast` answering the object it is given (variant 0) or null (variant 1).
fn put_reload_casts(e: &mut Engine, variant: u32) {
    use arena::object;
    let menu = object(140);
    e.mem
        .set_u32(e.mem.u32(0x011f_350c) + 4 * (0x3f0 - 0x3e9), menu);
    e.mem.set_u32(menu + 0x70, 0x0bad_0070);
    e.mem.set_u32(menu + 0x74, 0x0bad_0074);
    e.mem.set_u32(menu + 0x30, object(141));
    e.mem.set_u32(object(141) + 0x20, object(142));
    let succeeds = variant == 0;
    e.register_double(0x00ec_43fb, move |_, words| Ret {
        eax: if succeeds { words[0] } else { 0 },
        ..Ret::default()
    });
}

/// Every family the program has: the core's and the ones each crate provides when the engine is put
/// together (`engine_core::typed::provide`, run by `Engine::new`).
fn every_family() -> Vec<&'static engine_core::typed::Family> {
    drop(Engine::new_untyped_for_test());
    engine_core::typed::all_families()
}

/// `every_family` is exactly every crate's families: the core's table (`engine_core::typed::FAMILIES`),
/// `engine-typed`'s list (`engine_typed::FAMILIES`, which its `units::funcs` provides) and the forms
/// crate's two (`topic_search`, `topic_order`). One listed but not provided is never registered, and
/// every guard over `every_family` would skip it; one provided but listed nowhere is a crate's family
/// these lists miss.
#[test]
fn every_family_holds_each_crates_families() {
    let listed: Vec<&'static engine_core::typed::Family> = engine_core::typed::FAMILIES
        .iter()
        .chain(engine_typed::FAMILIES.iter())
        .copied()
        .chain([
            &engine_fallout_forms::topic_search::TOPIC_SEARCH,
            &engine_fallout_forms::topic_order::TOPIC_ORDER,
        ])
        .collect();
    let all = every_family();
    let name = |f: &&engine_core::typed::Family| f.name;
    for family in &listed {
        assert!(
            all.iter().any(|f| std::ptr::eq(*f, *family)),
            "family `{}` is listed but not provided; the families: {:?}",
            family.name,
            all.iter().map(name).collect::<Vec<_>>()
        );
    }
    for family in &all {
        assert!(
            listed.iter().any(|f| std::ptr::eq(*f, *family)),
            "family `{}` is provided but in no crate's list",
            family.name
        );
    }
    assert_eq!(all.len(), listed.len(), "a family provided twice");
}

/// A family module left out of its crate's list is neither provided nor listed, so the test above
/// can't see it: every family the families' sources declare (a `pub static NAME: Family = Family {`
/// at the start of a line, under `engine-typed`, `engine-core`'s `typed` and `engine-fallout-forms`)
/// is listed, by its name.
#[test]
fn every_declared_family_is_listed() {
    fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().is_some_and(|x| x == "rs") {
                out.push(path);
            }
        }
    }
    let listed: Vec<&str> = engine_core::typed::FAMILIES
        .iter()
        .chain(engine_typed::FAMILIES.iter())
        .map(|f| f.name)
        .chain([
            engine_fallout_forms::topic_search::TOPIC_SEARCH.name,
            engine_fallout_forms::topic_order::TOPIC_ORDER.name,
        ])
        .collect();
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for root in [
        "engine-typed/src",
        "engine-core/src/typed",
        "engine-fallout-forms/src",
    ] {
        rust_files(&crates.join(root), &mut files);
    }
    let mut declared = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if !(line.starts_with("pub static ") && line.ends_with(": Family = Family {")) {
                continue;
            }
            let name = lines[i + 1..]
                .iter()
                .take_while(|l| !l.starts_with('}'))
                .find_map(|l| l.trim().strip_prefix("name: \""))
                .and_then(|rest| rest.split('"').next());
            let Some(name) = name else {
                panic!("{}: a family with no name line: {line}", file.display());
            };
            declared.push((name.to_string(), file.display().to_string()));
        }
    }
    assert!(
        declared.len() >= listed.len(),
        "only {} families declared in the sources, {} listed",
        declared.len(),
        listed.len()
    );
    let unlisted: Vec<_> = declared
        .iter()
        .filter(|(name, _)| !listed.contains(&name.as_str()))
        .collect();
    assert!(
        unlisted.is_empty(),
        "families declared but in no crate's list (add each to its crate's FAMILIES): {unlisted:?}"
    );
}

/// An address is in one family at most, whichever families are on by default: `NV_TYPED` can turn
/// any on, and two families holding one address stop `Engine::new` once both are (on 2026-11-25
/// `00717e50` was in `ai` and in `topic_search`). This names both families.
#[test]
fn no_address_is_in_two_families() {
    let mut holder = std::collections::HashMap::new();
    let mut twice = Vec::new();
    for family in every_family() {
        for (addr, _) in (family.funcs)() {
            if let Some(first) = holder.insert(addr, family.name) {
                twice.push((addr, first, family.name));
            }
        }
    }
    assert!(
        twice.is_empty(),
        "addresses in two typed families, or twice in one (address, family, family): {twice:x?}"
    );
}

/// The engine with EVERY family on, whatever its gate says and whatever `NV_TYPED` is in the
/// environment (the switch's work applied with each family named): it starts, with no address in
/// two families and no name or number twice, and each family's every function is registered and
/// called directly. The engine as a run makes it (`Engine::new`, the switch as the environment
/// has it) starts too.
#[test]
fn the_engine_starts_with_its_families_on() {
    drop(Engine::new());
    let families = every_family();
    let names: Vec<&str> = families.iter().map(|f| f.name).collect();
    let mut e = Engine::new_untyped_for_test();
    e.typed_apply_for_test(&families, &names.join(","));
    for family in &families {
        for (addr, _) in (family.funcs)() {
            assert!(
                e.typed_direct(family.id, addr),
                "{addr:08x} of `{}` is not on with every family named",
                family.name
            );
        }
    }
}

/// The families built around xNVSE's wraps (`script_lines`: the script runner's functions xNVSE
/// wraps, also rows of `overridable.tsv`), whose doc says the family goes by address once they are
/// wrapped: its value is the typed bodies the wraps call into.
const BUILT_AROUND_WRAPS: &[&str] = &["script_lines"];

/// Members that something built in wraps or registers over, so their family goes by address while
/// that runs, as the family's or the built-in's doc says. Each is known and left to its family's
/// lane; a new one fails the checks below. `topic_search`'s `0061a310` (`TESTopic::GetOwnerTopic`):
/// the host's dialogue topic index registers over it (`crates/host/src/topic_index.rs` says the
/// family then goes by address), JIP LN's dialogue patch wraps it (`patch_rows/dialogue.rs`).
const KNOWN_TO_GO_BY_ADDRESS: &[(&str, u32)] = &[("topic_search", 0x0061_a310)];

/// A wrap an extender registers over a member of a family (its WRAPS rows, the game patches'
/// rows) clears the whole family's direct bit for the run, so every `typed_call!` of the family
/// goes by address from then on. So no family on by default, in any crate, holds a wrapped address,
/// except one whose doc says it is built around that and the known ones above.
#[test]
fn no_family_past_its_gate_holds_a_wrapped_address() {
    let wrapped: Vec<u32> = engine_core::extend::WRAPS
        .iter()
        .chain(engine_core::extend::PATCH_WRAPS.iter().copied().flatten())
        .map(|w| w.addr)
        .collect();
    let mut held = Vec::new();
    for family in every_family() {
        if !family.past_gate || BUILT_AROUND_WRAPS.contains(&family.name) {
            continue;
        }
        for (addr, _) in (family.funcs)() {
            if wrapped.contains(&addr) && !KNOWN_TO_GO_BY_ADDRESS.contains(&(family.name, addr)) {
                held.push((family.name, addr));
            }
        }
    }
    assert!(
        held.is_empty(),
        "families on by default hold addresses an extender wraps (family, address): {held:x?}; \
         a wrap turns the family's direct calls off for the run"
    );
}

/// A plain registration over a member (a fix row, an nv-rs built-in, a plugin's override: the
/// addresses of `overridable.tsv`) clears the family's direct bit too, for the run, once it is
/// registered. So no family on by default, in any crate, holds one, except the rows below, each
/// overridden only by a plugin that is not built in (kNVSE, ITR) or by a script or frame function, so
/// the family goes by address only while that one runs; the families built around the wraps; and
/// the known ones above.
#[test]
fn no_family_past_its_gate_holds_an_overridable_address() {
    const ONLY_WHILE_A_PLUGIN_RUNS: &[(&str, u32)] = &[
        ("dispatch", 0x00a0_9330),
        ("dispatch", 0x00a0_9410),
        ("animation", 0x004f_0380),
        ("animation", 0x00a4_1110),
        ("scripts", 0x0059_3870),
        ("scripts", 0x005a_c1e0),
        ("scripts", 0x005a_c7a0),
    ];
    let rows: Vec<(u32, &str)> = include_str!("../../engine-core/data/overridable.tsv")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let mut columns = l.split('\t');
            let addr = u32::from_str_radix(columns.next().unwrap_or_default(), 16)
                .unwrap_or_else(|_| panic!("overridable.tsv: no address in {l:?}"));
            (addr, columns.nth(1).unwrap_or_default())
        })
        .collect();
    let mut allowed_seen = Vec::new();
    let mut held = Vec::new();
    for family in every_family() {
        if !family.past_gate || BUILT_AROUND_WRAPS.contains(&family.name) {
            continue;
        }
        for (addr, _) in (family.funcs)() {
            let Some((_, by)) = rows.iter().find(|(a, _)| *a == addr) else {
                continue;
            };
            if KNOWN_TO_GO_BY_ADDRESS.contains(&(family.name, addr)) {
                allowed_seen.push((family.name, addr));
            } else if ONLY_WHILE_A_PLUGIN_RUNS.contains(&(family.name, addr)) {
                assert!(
                    by.contains("kNVSE") || by.contains("ITR") || by.contains("script or frame"),
                    "{addr:08x} is allowed in `{}` as a plugin's, but overridable.tsv now says {by}",
                    family.name
                );
                allowed_seen.push((family.name, addr));
            } else {
                held.push((family.name, addr, *by));
            }
        }
    }
    assert!(
        held.is_empty(),
        "families on by default hold addresses overridable.tsv lists (family, address, by): \
         {held:x?}; a registration turns the family's direct calls off for the run"
    );
    for row in ONLY_WHILE_A_PLUGIN_RUNS
        .iter()
        .chain(KNOWN_TO_GO_BY_ADDRESS)
    {
        assert!(
            allowed_seen.contains(row),
            "{row:x?} is in no family any more: drop it from the list"
        );
    }
}

/// The text of a family's sources: `engine-typed`'s module of that name, or a crate's
/// `<family>.rs` (the forms crate's families, the core's leaves).
fn family_sources(name: &str) -> Vec<String> {
    fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().is_some_and(|x| x == "rs") {
                out.push(path);
            }
        }
    }
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for root in [
        "engine-typed/src",
        "engine-fallout-forms/src",
        "engine-core/src/typed",
    ] {
        rust_files(&crates.join(root).join(name), &mut files);
        let file = crates.join(root).join(format!("{name}.rs"));
        if file.is_file() {
            files.push(file);
        }
    }
    files
        .iter()
        .map(|f| std::fs::read_to_string(f).unwrap_or_else(|why| panic!("{f:?}: {why}")))
        .collect()
}

/// Whether the typed body of `start` (its text from its `Typed from` mark to the next mark) calls
/// `e.site` at `seam`: with the address or a constant holding it, or through a function of the
/// family's sources that calls `e.site` and is given the address or holds it. Comments don't count:
/// the text read is each line's code up to its `//`.
fn body_calls_site(sources: &[String], start: u32, seam: u32) -> bool {
    let bare = |s: &str| {
        s.lines()
            .map(|line| line.find("//").map_or(line, |at| &line[..at]))
            .flat_map(str::chars)
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
    };
    let mut names = vec![
        format!("0x{:04x}_{:04x}", seam >> 16, seam & 0xffff),
        format!("0x{seam:08x}"),
    ];
    for line in sources.iter().flat_map(|text| text.lines()).map(bare) {
        if let (Some(at), Some(eq)) = (line.find("const"), line.find(":u32=")) {
            if at + 5 <= eq && names[..2].iter().any(|n| line[eq + 5..] == format!("{n};")) {
                names.push(line[at + 5..eq].to_string());
            }
        }
    }
    let calls_site = |text: &str| {
        names
            .iter()
            .any(|n| text.contains(&format!(".site({n},")) || text.contains(&format!(".site({n})")))
    };
    let mark = format!("Typed from {start:08x}");
    let Some(body) = sources.iter().find_map(|text| {
        let rest = &text[text.find(&mark)? + mark.len()..];
        Some(bare(
            &rest[..rest.find("Typed from ").unwrap_or(rest.len())],
        ))
    }) else {
        return false;
    };
    if calls_site(&body) {
        return true;
    }
    for text in sources {
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let code = line.find("//").map_or(*line, |at| &line[..at]);
            let Some(at) = code.find("fn ") else {
                continue;
            };
            let Some(open) = code[at..].find('(') else {
                continue;
            };
            let name = &code[at + 3..at + open];
            if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let close = format!("{}}}", &line[..line.len() - line.trim_start().len()]);
            let end = lines[i..]
                .iter()
                .position(|l| *l == close)
                .map_or(lines.len(), |n| i + n);
            let helper = bare(&lines[i..end].join("\n"));
            if helper.contains(".site(")
                && body.contains(&format!("{name}("))
                && (calls_site(&helper)
                    || names
                        .iter()
                        .any(|n| body.contains(&format!("{name}(e,{n}"))))
            {
                return true;
            }
        }
    }
    false
}

/// A seam (`extend.rs`'s SITES and the game patches' rows: an instruction inside a translated
/// function where a built-in takes part through `e.site`) inside a member of a family on by default
/// is skipped unless the typed body keeps its `e.site` call there. So no such member, in any crate's
/// family, holds a seam, except the ones listed, whose typed bodies still call `e.site` at it
/// (read in their sources: the address or its constant in an `e.site` call of the body, or of a
/// function of the family the body calls). The function holding a seam is the translated function
/// that starts nearest below it.
#[test]
fn no_member_of_a_family_on_by_default_holds_a_seam() {
    // Each checked in its typed body (2026-11-25): the `e.site` call at the seam, behind the
    // same extender test as the translation's.
    const KEPT: &[(u32, &str)] = &[
        (0x0040_f9d7, "cell_refs"),
        (0x0045_234e, "cell_refs"),
        (0x00c6_5b5b, "collision"),
        (0x00a0_968c, "dispatch"),
        (0x0041_1978, "extra_data"),
        (0x0041_2675, "extra_data"),
        (0x0041_271e, "extra_data"),
        (0x0041_c833, "extra_data"),
        (0x0041_d0f6, "extra_data"),
        (0x0041_d212, "extra_data"),
        (0x0041_d25a, "extra_data"),
        (0x0048_3669, "forms"),
        (0x0048_449a, "forms"),
        (0x0048_4e03, "forms"),
        (0x0048_6903, "forms"),
        (0x0077_04ae, "hud"),
        (0x0077_0b0e, "hud"),
        (0x0077_14da, "hud"),
        (0x0077_951e, "hud"),
        (0x0077_955d, "hud"),
        (0x007e_0c1f, "interface"),
        (0x00b5_cd2a, "scene_node"),
        (0x0059_3fbc, "scripts"),
        (0x0059_49d4, "scripts"),
        (0x005a_cb9d, "scripts"),
        (0x005a_c763, "script_lines"),
        (0x005a_c782, "script_lines"),
        (0x005a_c78a, "script_lines"),
        (0x005a_cd0d, "script_lines"),
        (0x005e_0dd1, "script_lines"),
        (0x005e_0ec9, "script_lines"),
        (0x005e_0fbe, "script_lines"),
        (0x005e_16c3, "script_lines"),
        (0x005e_1716, "script_lines"),
        (0x005e_1a23, "script_lines"),
        (0x005e_1f0b, "script_lines"),
        (0x005e_2329, "script_lines"),
    ];
    let e = Engine::new_untyped_for_test();
    let mut members = std::collections::HashMap::new();
    for family in engine_core::typed::all_families() {
        if family.past_gate {
            for (addr, _) in (family.funcs)() {
                members.insert(addr, family.name);
            }
        }
    }
    let mut found = Vec::new();
    for site in engine_core::extend::all_sites() {
        let holder = (0..0x4000u32)
            .map(|back| site.addr.wrapping_sub(back))
            .find(|&start| e.is_translated(start));
        if let Some(start) = holder.filter(|start| members.contains_key(start)) {
            found.push((site.addr, members[&start], start));
        }
    }
    let unkept: Vec<_> = found
        .iter()
        .filter(|(seam, family, _)| !KEPT.contains(&(*seam, *family)))
        .collect();
    assert!(
        unkept.is_empty(),
        "seams inside members of families on by default (seam, family, member): {unkept:x?}"
    );
    for &(seam, family) in KEPT {
        let Some(&(_, _, start)) = found.iter().find(|f| (f.0, f.1) == (seam, family)) else {
            panic!("{seam:08x} in `{family}` is no member's seam any more");
        };
        let sources = family_sources(family);
        assert!(
            !sources.is_empty(),
            "no sources found for the family `{family}`"
        );
        assert!(
            body_calls_site(&sources, start, seam),
            "`{family}`'s typed {start:08x} holds the seam {seam:08x}, but its body no longer \
             calls `e.site` there"
        );
    }
}

// ---- The item list box (`ListBox<ItemChange *>`), the item lists' helpers, the small units ------

/// The variants of the item list box's world ([`put_item_list_box`]): [`put_list_box`]'s seven,
/// the richest first, and its eighth (one item and two visible items, so `AddItem`'s height
/// actions run).
const ITEM_LIST_VARIANTS: [u32; 8] = [6, 7, 5, 3, 2, 1, 4, 0];

/// [`put_list_box`]'s world for the item list box, with the groups `RemoveItemGroup` compares (the
/// form word at `+8` of the items' data: 0 for the first and the fifth item, 1 for the third) and
/// the local list's append (`005ae3d0`) doing its work: the word at the address it is given is the
/// list's first item when it has none, else a new node's (objects 230 on) after its last.
fn put_item_list_box(e: &mut Engine, variant: u32) {
    use arena::object;
    put_list_box(e, variant);
    // The richest variant's second item filtered as well (its node is object 161), so the node
    // removal's filtered branch (`_num_filtered` less 1.0) runs.
    if variant == 6 {
        e.mem.set_u8(object(171) + 8, 1);
    }
    for (k, group) in [(0u32, 0u32), (2, 1), (4, 0)] {
        e.mem.set_u32(object(190 + k) + 8, group);
    }
    e.mem.set_u32(object(229), 0);
    e.register_double(0x005a_e3d0, |e, words| {
        let (list, value) = (words[0], e.mem.u32(words[1]));
        if e.mem.u32(list) == 0 {
            e.mem.set_u32(list, value);
        } else {
            let mut node = list;
            while e.mem.u32(node + 4) != 0 {
                node = e.mem.u32(node + 4);
            }
            let made = e.mem.u32(object(229));
            e.mem.set_u32(object(229), made + 1);
            let fresh = object(230 + made % 16);
            e.mem.set_u32(fresh, value);
            e.mem.set_u32(fresh + 4, 0);
            e.mem.set_u32(node + 4, fresh);
        }
        Ret::default()
    });
}

/// The variants of [`put_inventory`]'s world.
const INVENTORY_VARIANTS: [u32; 3] = [0, 1, 2];

/// A world for the inventory fill (`00719ef0`): the list box world ([`put_list_box`]'s richest
/// variant, or in the third its fourth, whose flag 0 is clear) with a template at both list boxes'
/// `+0x18`; the inventory walk (`00575510`) storing an iterator and the changes (none in the second
/// variant, so the fill stops there); the next item (`004ca330`) answering three items in turn, then
/// 0 (their counts 3, 1 and 0, the first two with a form, the last none); `__RTDynamicCast`
/// answering the form it is given.
fn put_inventory(e: &mut Engine, variant: u32) {
    use arena::object;
    put_list_box(e, if variant == 2 { 3 } else { 6 });
    for list_box in [object(3), object(40)] {
        e.mem.set_u32(list_box + 0x18, object(199));
    }
    let walks = variant != 1;
    e.register_double(0x0057_5510, move |e, words| {
        if walks {
            e.mem.set_u32(words[2], object(210));
            e.mem.set_u32(words[3], object(211));
        }
        Ret::default()
    });
    e.mem.set_u32(object(211) + 0x10, 0);
    e.register_double(0x004c_a330, |e, _| {
        let next = e.mem.u32(object(211) + 0x10);
        e.mem.set_u32(object(211) + 0x10, next + 1);
        Ret {
            eax: if next < 3 { object(212 + next) } else { 0 },
            ..Ret::default()
        }
    });
    for (k, count, form) in [(0u32, 3u32, object(215)), (1, 1, object(216)), (2, 0, 0)] {
        e.mem.set_u32(object(212 + k) + 4, count);
        e.mem.set_u32(object(212 + k) + 8, form);
    }
    e.register_double(0x00ec_43fb, |_, words| Ret {
        eax: words[0],
        ..Ret::default()
    });
}

/// The variants of [`put_form_type`]'s world.
const FORM_TYPE_VARIANTS: [u32; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

/// A world for the filters by form type: both pointer arguments' items (objects 3 and 40) with a
/// form (object 201) whose type byte is, by variant, 0x28, 0x18, 0x73, 0x1a, 0 (each filter's
/// match, a later match, and none) or one of the last filter's other types (0x1f, 0x29, 0x67, 0x32,
/// 0x6c, 0x74).
fn put_form_type(e: &mut Engine, variant: u32) {
    use arena::object;
    let kind = [
        0x28u8, 0x18, 0x73, 0x1a, 0, 0x1f, 0x29, 0x67, 0x32, 0x6c, 0x74,
    ][variant as usize];
    for item in [object(3), object(40)] {
        e.mem.set_u32(item + 8, object(201));
    }
    e.mem.set_u8(object(201) + 4, kind);
}

/// The variants of [`put_sort_words`]'s world.
const SORT_VARIANTS: [u32; 4] = [0, 1, 2, 3];

/// A world for the sorts' comparisons: both pointer arguments' items (objects 3 and 40) with counts
/// (`+4`) and healths that tell a signed compare from an unsigned one (a high bit on either side,
/// equal words, two high-bit words). Each item's form (`+8`, objects 222 and 223) has its own vtable
/// (objects 224 and 225) whose health slot (`+0x10`) answers the variant's health; `__RTDynamicCast`
/// answers the form it is given.
fn put_sort_words(e: &mut Engine, variant: u32) {
    use arena::{object, CODE};
    const WORDS: [(u32, u32); 4] = [
        (0xbf80_0000, 3),
        (3, 0x8000_0000),
        (5, 5),
        (0xffff_fff0, 0xffff_ffff),
    ];
    let (a, b) = WORDS[variant as usize];
    for (k, (item, word)) in (0u32..).zip([(object(3), a), (object(40), b)]) {
        e.mem.set_u32(item + 4, word);
        e.mem.set_u32(item + 8, object(222 + k));
        e.mem.set_u32(object(222 + k), object(224 + k));
        e.mem.set_u32(object(224 + k) + 0x10, CODE + 0x200 + 4 * k);
        e.register_double(CODE + 0x200 + 4 * k, move |_, _| Ret {
            eax: word,
            ..Ret::default()
        });
    }
    e.register_double(0x00ec_43fb, |_, words| Ret {
        eax: words[0],
        ..Ret::default()
    });
}

/// [`put_item_list_box`]'s world with `Tile::GetFloat` answering a `_num_filtered` with a fraction
/// for most variants (1.5, 2.75, -0.5, ...), so a truncation tells from a rounding.
fn put_fractional_filtered(e: &mut Engine, variant: u32) {
    const FILTERED: [f32; 8] = [1.5, 2.75, -0.5, 0.75, 1.0, 3.5, 0.25, -2.5];
    put_item_list_box(e, variant);
    let filtered = f64::from(FILTERED[variant as usize]);
    e.register_double(0x00a0_11b0, move |_, _| Ret {
        st0: filtered,
        ..Ret::default()
    });
}

/// The item list box's and the filters' worlds, as settings of a Diff.
trait ItemWorlds {
    /// In [`put_item_list_box`]'s world, with `max` cases.
    fn in_item_list(self, args: &'static [Arg], max: usize) -> Diff;
    /// In [`put_form_type`]'s world, one pointer argument.
    fn in_form_types(self) -> Diff;
    /// In [`put_sort_words`]'s world, two pointer arguments.
    fn in_sort_words(self) -> Diff;
    /// In [`put_fractional_filtered`]'s world, one pointer argument.
    fn in_fractional_filtered(self) -> Diff;
}

impl ItemWorlds for Diff {
    fn in_item_list(self, args: &'static [Arg], max: usize) -> Diff {
        self.in_arena_and(args, CALLEES, &[], &ITEM_LIST_VARIANTS, put_item_list_box)
            .max_cases(max)
    }

    fn in_sort_words(self) -> Diff {
        self.in_arena_and(
            &[Arg::Ptr, Arg::Ptr],
            CALLEES,
            &[],
            &SORT_VARIANTS,
            put_sort_words,
        )
    }

    fn in_fractional_filtered(self) -> Diff {
        self.in_arena_and(
            &[Arg::Ptr],
            CALLEES,
            &[],
            &ITEM_LIST_VARIANTS,
            put_fractional_filtered,
        )
        .max_cases(32)
    }

    fn in_form_types(self) -> Diff {
        self.in_arena_and(
            &[Arg::Ptr],
            CALLEES,
            &[],
            &FORM_TYPE_VARIANTS,
            put_form_type,
        )
    }
}

#[test]
fn item_list_box_remove_item_group_diff() {
    Diff::new(engine_core::entry!(
        0x0071_a200,
        interface::item_list_box::list_box_remove_item_group(Ptr, u32)
    ))
    .in_item_list(&[Arg::Ptr, Arg::Byte], 48)
    .run()
    .assert_same();
}

#[test]
fn item_list_box_add_item_diff() {
    Diff::new(engine_core::entry!(
        0x0071_a320,
        interface::item_list_box::list_box_add_item(Ptr, u32, Ptr, u32, Ptr) -> Ptr
    ))
    .in_item_list(
        &[Arg::Ptr, Arg::Word, Arg::Ptr, Arg::ListNode, Arg::Ptr],
        64,
    )
    .run()
    .assert_same();
}

#[test]
fn item_list_box_remove_item_diff() {
    Diff::new(engine_core::entry!(
        0x0071_a5c0,
        interface::item_list_box::list_box_remove_item(Ptr, Ptr) -> bool
    ))
    .in_item_list(&[Arg::Ptr, Arg::ListTile], 56)
    .run()
    .assert_same();
}

#[test]
fn item_list_box_synchronize_diff() {
    Diff::new(engine_core::entry!(
        0x0071_a670,
        interface::item_list_box::list_box_synchronize(Ptr)
    ))
    .in_item_list(&[Arg::Ptr], 32)
    .run()
    .assert_same();
}

#[test]
fn item_list_box_update_highlight_box_pos_diff() {
    Diff::new(engine_core::entry!(
        0x0071_a740,
        interface::item_list_box::list_box_update_highlight_box_pos(Ptr)
    ))
    .in_item_list(&[Arg::Ptr], 32)
    .run()
    .assert_same();
}

#[test]
fn item_list_box_fn_0071aa60_diff() {
    Diff::new(engine_core::entry!(
        0x0071_aa60,
        interface::item_list_box::fn_0071aa60(Ptr, u8)
    ))
    .in_arena(&[Arg::Ptr, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn item_list_box_fn_0071aa80_diff() {
    Diff::new(engine_core::entry!(
        0x0071_aa80,
        interface::item_list_box::fn_0071aa80(Ptr) -> u8
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn item_list_box_fn_0071aaa0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_aaa0,
        interface::item_list_box::fn_0071aaa0(Ptr, u32, u32) -> bool
    ))
    .in_item_list(&[Arg::Ptr, Arg::ListNode, Arg::ListNode], 64)
    .run()
    .assert_same();
}

#[test]
fn item_list_box_set_total_list_height_diff() {
    Diff::new(engine_core::entry!(
        0x0071_ad30,
        interface::item_list_box::list_box_set_total_list_height(Ptr)
    ))
    .in_item_list(&[Arg::Ptr], 32)
    .run()
    .assert_same();
}

#[test]
fn item_list_box_num_unfiltered_diff() {
    Diff::new(engine_core::entry!(
        0x0071_ae60,
        interface::item_list_box::list_box_num_unfiltered(Ptr) -> i32
    ))
    .in_fractional_filtered()
    .run()
    .assert_same();
}

#[test]
fn item_list_box_fn_00719be0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_9be0,
        interface::item_list_box::fn_00719be0(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn item_list_box_fn_00719c00_diff() {
    Diff::new(engine_core::entry!(
        0x0071_9c00,
        interface::item_list_box::fn_00719c00(Ptr, u32, u32) -> Ptr
    ))
    .in_arena(&[Arg::Object, Arg::Word, Arg::Int], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn item_list_box_fn_00719ef0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_9ef0,
        interface::item_list_box::fn_00719ef0(Ptr, u32, u32, u32, u32, u8, u8)
    ))
    .in_arena_and(
        &[
            Arg::Object,
            Arg::Word,
            Arg::Code,
            Arg::Word,
            Arg::Code,
            Arg::Byte,
            Arg::Byte,
        ],
        CALLEES,
        &[],
        &INVENTORY_VARIANTS,
        put_inventory,
    )
    .max_cases(48)
    .max_calls(800)
    .run()
    .assert_same();
}

#[test]
fn item_list_box_fn_00719c80_diff() {
    Diff::new(engine_core::entry!(
        0x0071_9c80,
        interface::item_list_box::fn_00719c80(u32, u32) -> i32
    ))
    .in_arena(&[Arg::Ptr, Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn item_list_box_fn_00719ca0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_9ca0,
        interface::item_list_box::fn_00719ca0(u32, u32) -> i32
    ))
    .in_sort_words()
    .run()
    .assert_same();
}

#[test]
fn item_list_box_fn_00719cf0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_9cf0,
        interface::item_list_box::fn_00719cf0(u32, u32) -> i32
    ))
    .in_sort_words()
    .run()
    .assert_same();
}

#[test]
fn item_list_box_fn_00719da0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_9da0,
        interface::item_list_box::fn_00719da0(u32) -> bool
    ))
    .in_form_types()
    .run()
    .assert_same();
}

#[test]
fn item_list_box_fn_00719de0_diff() {
    Diff::new(engine_core::entry!(
        0x0071_9de0,
        interface::item_list_box::fn_00719de0(u32) -> bool
    ))
    .in_form_types()
    .run()
    .assert_same();
}

#[test]
fn item_list_box_fn_00719e30_diff() {
    Diff::new(engine_core::entry!(
        0x0071_9e30,
        interface::item_list_box::fn_00719e30(u32) -> bool
    ))
    .in_form_types()
    .run()
    .assert_same();
}

#[test]
fn small_units_fn_0071b020_diff() {
    Diff::new(engine_core::entry!(
        0x0071_b020,
        interface::small_units::fn_0071b020(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn small_units_fn_0071b040_diff() {
    Diff::new(engine_core::entry!(
        0x0071_b040,
        interface::small_units::fn_0071b040(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn small_units_fn_00726bf0_diff() {
    Diff::new(engine_core::entry!(
        0x0072_6bf0,
        interface::small_units::fn_00726bf0(Ptr, u32, u32)
    ))
    .in_arena(&[Arg::Ptr, Arg::Word, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn small_units_fn_00726c40_diff() {
    Diff::new(engine_core::entry!(
        0x0072_6c40,
        interface::small_units::fn_00726c40(Ptr) -> u32
    ))
    .in_item_list(&[Arg::Ptr], 32)
    .run()
    .assert_same();
}

#[test]
fn small_units_fn_00726c60_diff() {
    Diff::new(engine_core::entry!(
        0x0072_6c60,
        interface::small_units::fn_00726c60(Ptr, u32)
    ))
    .in_arena(&[Arg::Ptr, Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn small_units_fn_00726c90_diff() {
    Diff::new(engine_core::entry!(
        0x0072_6c90,
        interface::small_units::fn_00726c90(Ptr) -> Ptr
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn small_units_fn_00726cf0_diff() {
    Diff::new(engine_core::entry!(
        0x0072_6cf0,
        interface::small_units::fn_00726cf0(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn small_units_fn_00726d50_diff() {
    Diff::new(engine_core::entry!(
        0x0072_6d50,
        interface::small_units::fn_00726d50(Ptr, u8)
    ))
    .in_arena(&[Arg::Ptr, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn small_units_fn_00726d70_diff() {
    Diff::new(engine_core::entry!(
        0x0072_6d70,
        interface::small_units::fn_00726d70(Ptr, u32, u8)
    ))
    .in_arena(&[Arg::Ptr, Arg::Int, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn small_units_fn_00726db0_diff() {
    Diff::new(engine_core::entry!(
        0x0072_6db0,
        interface::small_units::fn_00726db0(Ptr, u8)
    ))
    .in_arena(&[Arg::Ptr, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

/// `GetHighlightTarget` (the six instances' one body, here `ListBox<ActorValue::Index>`'s) on the
/// first list box for every move in every list box world and seed: the six instances' Diffs share
/// their cases, which never pair the up moves with the world where the search goes on past a
/// disabled item and walks on from the list's start (the sixth variant), so this one runs them all.
#[test]
fn list_box_get_highlight_target_every_move_diff() {
    Diff::new(engine_core::entry!(
        0x0072_14b0,
        interface::menu_manager_p2::list_box_actor_value_get_highlight_target(Ptr, u32) -> u32
    ))
    .in_arena_and(
        &[Arg::Object, Arg::Direction],
        CALLEES,
        &[],
        &LIST_BOX_VARIANTS,
        put_list_box,
    )
    .max_cases(168)
    .run()
    .assert_same();
}

// ---- The stats menu (`statsmenu.cpp`), its first part -------------------------------------------

/// The variants of [`put_stats_menu`]'s world, the richest first.
const STATS_VARIANTS: [u32; 4] = [0, 3, 1, 2];

/// The stats menu's world. The open menu (`011dace0`) is object 3, the player (`011dea3c`) object
/// 100, both absent in the variant 2. The exe's doubles the menu computes with (1000.0, 2.0, pi,
/// pi / 2, 5.0, 100.0) hold their values. The player: hardcore (`+0x7bc` 1) in the variants 1 and 3;
/// its actor values (`+0xa4`) on a vtable of stand-ins; its active effects (`+0x94`) on their own
/// vtable (object 101) whose list slot answers one node (object 105) with one effect (object 106) of
/// kind 0xa (variants 0 and 3; else 3), active in the variant 0. The hardcore state shown
/// (`011a0ae1`) set in the variants 0 and 3, the next update (`011dad74`) never in the variant 1.
/// The menu: limb mode in the variant 0, the effects already updated this frame in the variant 3,
/// the 60 tiles all set but in the variant 2, the perk tile set; the healing options' callbacks
/// stand-ins, in the variant 3 the first's and the fourth's the exe's own (`007e05f0`,
/// `007e07a0`). The Pip-Boy (the manager's `+0x174`, object 108) up but in the variant 1. The fade
/// object's current group (`011dace4`'s first word) a group number.
fn put_stats_menu(e: &mut Engine, variant: u32) {
    use arena::{object, CODE, VTABLES};
    let live = variant != 2;
    let menu = object(3);
    let player = object(100);
    e.mem.set_u32(0x011d_ace0, if live { menu } else { 0 });
    e.mem.set_u32(0x011d_ea3c, if live { player } else { 0 });
    for (at, value) in [
        (0x0101_7b70u32, 1000.0f64),
        (0x0101_1590, 2.0),
        (0x0101_ff40, std::f64::consts::PI),
        (0x0106_4f50, std::f64::consts::FRAC_PI_2),
        (0x0102_0998, 5.0),
        (0x0101_7a40, 100.0),
    ] {
        e.mem.set_u64(at, value.to_bits());
    }
    e.mem
        .set_u32(player + 0x7bc, u32::from(variant == 1 || variant == 3));
    e.mem.set_u32(player + 0xa4, VTABLES + 0x400);
    e.mem.set_u32(player + 0x94, object(101));
    e.mem.set_u32(object(101) + 8, CODE + 0x208);
    e.register_double(CODE + 0x208, |_, _| Ret {
        eax: arena::object(105),
        ..Ret::default()
    });
    e.mem.set_u32(object(105), object(106));
    e.mem.set_u32(object(105) + 4, 0);
    e.mem.set_u32(
        object(106) + 0x2c,
        if variant == 0 || variant == 3 { 0xa } else { 3 },
    );
    e.mem.set_u8(object(106) + 0x10, u8::from(variant == 0));
    e.mem
        .set_u8(0x011a_0ae1, u8::from(variant == 0 || variant == 3));
    e.mem
        .set_u32(0x011d_ad74, if variant == 1 { u32::MAX } else { 0 });
    if variant != 2 {
        for i in 0..60 {
            e.mem.set_u32(menu + 0x90 + 4 * i, object(110 + i % 40));
        }
    }
    e.mem.set_u8(menu + 0x2a0, u8::from(variant == 0));
    e.mem.set_u8(menu + 0x2a1, u8::from(variant == 3));
    e.mem.set_u32(menu + 0x118, object(107));
    for k in 0..4u32 {
        let callback = match (variant, k) {
            (3, 0) => 0x007e_05f0,
            (3, 3) => 0x007e_07a0,
            _ => CODE + 0x40 + 4 * k,
        };
        e.mem.set_u32(menu + 0x28 + 0x10 * k + 0xc, callback);
    }
    let manager = e.mem.u32(0x011d_8a80);
    if manager != 0 {
        e.mem.set_u32(manager + 0x174, object(108));
        e.mem.set_u8(object(108) + 0x28, u8::from(variant != 1));
    }
    // The fade object's current group (its first word), when there is one.
    let fade = e.mem.u32(0x011d_ace4);
    if fade != 0 {
        e.mem.set_u32(fade, [1u32, 5, 0, 3][variant as usize]);
    }
}

/// The variants of [`put_status_data`]'s world.
const STATUS_DATA_VARIANTS: [u32; 4] = [0, 1, 2, 3];

/// A world for the status data list (object 3, its embedded list at `+4`: an entry of key 2, then a
/// node, object 231, with an entry of key 0x36) and an effect item's words on both pointer values
/// (objects 40 and 3: its actor value `+0x10`, its setting `+0x14` object 233 of archetype 0, so the
/// plain case, object 3 twice, runs every variant). The archetype table's entry 0 has bit 2 but in
/// the variant 2 (the key is then -1). By variant: the effect's key 0x36 (found), 0x37 with the
/// setting's flag 4 (negated, with a duration), 0x37 with flag 2 (no duration), 0x37 with neither
/// and JIP LN's fixes on (the G107 seam asked, none answering).
fn put_status_data(e: &mut Engine, variant: u32) {
    use arena::object;
    let list = object(3);
    e.mem.set_u32(list + 4, object(230));
    e.mem.set_u32(list + 8, object(231));
    e.mem.set_u32(object(230) + 0x10, 2);
    e.mem.set_u32(object(231), object(232));
    e.mem.set_u32(object(231) + 4, 0);
    e.mem.set_u32(object(232) + 0x10, 0x36);
    for effect in [object(40), list] {
        e.mem
            .set_u32(effect + 0x10, if variant == 0 { 0x36 } else { 0x37 });
        e.mem.set_u32(effect + 0x14, object(233));
    }
    e.mem.set_u32(object(233) + 0x98, 0);
    e.mem
        .set_u32(object(233) + 0x58, [0, 4, 2, 0][variant as usize]);
    e.mem.set_u32(0x0118_3328, if variant == 2 { 0 } else { 2 });
    if variant == 3 {
        e.set_jip_patches(engine_core::extend::jip_patches::FIXES);
    }
}

/// The variants of [`put_status_effects`]'s world.
const STATUS_EFFECT_VARIANTS: [u32; 2] = [0, 1];

/// A world for the status effect lists: object 3's lists at `+4` (a `StatusEffectList`'s own) and
/// at `+0x88` (the stats menu's list at its `+0x84`) hold two status effects (objects 240 and 242,
/// the second's node object 241) in the variant 0 and none in the variant 1, and its four status
/// effect words (`+0x74` to `+0x80`) one (object 244) or none; object 40's are all empty. (The
/// arena's own words there would make status effects of small numbers and floats, whose clean-up
/// writes over the zero page.)
fn put_status_effects(e: &mut Engine, variant: u32) {
    use arena::object;
    let full = variant == 0;
    for (k, list) in [(3u32, 4u32), (3, 0x88), (40, 4), (40, 0x88)] {
        let at = object(k) + list;
        let filled = full && k == 3;
        e.mem.set_u32(at, if filled { object(240) } else { 0 });
        e.mem.set_u32(at + 4, if filled { object(241) } else { 0 });
    }
    e.mem.set_u32(object(241), object(242));
    e.mem.set_u32(object(241) + 4, 0);
    for k in [3u32, 40] {
        for slot in [0x74u32, 0x78, 0x7c, 0x80] {
            let one = full && k == 3 && slot == 0x74;
            e.mem
                .set_u32(object(k) + slot, if one { object(244) } else { 0 });
        }
    }
}

/// The variants of [`put_fade_groups`]'s and [`put_archetypes`]'s worlds.
const SMALL_WORLD_VARIANTS: [u32; 3] = [0, 1, 2];

/// The fade object's current group (its first word) on both pointer values (objects 3 and 40): 0
/// and 1, 2 and 3, or 5 and 4 (the arena's word there is a vtable).
fn put_fade_groups(e: &mut Engine, variant: u32) {
    use arena::object;
    let groups = [[0u32, 1], [2, 3], [5, 4]][variant as usize];
    for (k, group) in [3u32, 40].into_iter().zip(groups) {
        e.mem.set_u32(object(k), group);
    }
}

/// An effect setting's archetype (`+0x98`) on both pointer values (objects 3 and 40): 0 and 1, 2
/// and 3, or 0x2b and 0x2c; the archetype flag table's first 0x30 entries with the flag 0x40, the
/// flag 2 or both in turn (the arena's words there are pointers and floats).
fn put_archetypes(e: &mut Engine, variant: u32) {
    use arena::object;
    let first = [0u32, 2, 0x2b][variant as usize];
    for (k, archetype) in [3u32, 40].into_iter().zip([first, first + 1]) {
        e.mem.set_u32(object(k) + 0x98, archetype);
    }
    for archetype in 0..0x30u32 {
        let flags = [0x40u32, 2, 0x42][(archetype % 3) as usize];
        e.mem.set_u32(0x0118_3328 + 16 * archetype, flags);
    }
}

/// The stats menu's worlds, as settings of a Diff.
trait StatsWorlds {
    /// In [`put_stats_menu`]'s world, the vector constructor (`00ec782f`) no stand-in: the fade
    /// object's, which the native code does in place, runs for real on the translation's side.
    fn in_stats(self, args: &'static [Arg]) -> Diff;
    /// [`StatsWorlds::in_stats`] with the translation's plain calls `plain`.
    fn in_stats_with(self, args: &'static [Arg], plain: &[u32]) -> Diff;
    /// In [`put_fade_groups`]'s world: a pointer and a byte.
    fn in_fade_groups(self) -> Diff;
    /// In [`put_archetypes`]'s world: a pointer.
    fn in_archetypes(self) -> Diff;
    /// In [`put_status_data`]'s world.
    fn in_status_data(self, args: &'static [Arg]) -> Diff;
    /// In [`put_status_effects`]'s world, with the translation's plain calls `plain`.
    fn in_status_effects(self, args: &'static [Arg], plain: &[u32]) -> Diff;
}

impl StatsWorlds for Diff {
    fn in_stats(self, args: &'static [Arg]) -> Diff {
        self.in_stats_with(args, &[])
    }

    fn in_fade_groups(self) -> Diff {
        self.in_arena_and(
            &[Arg::Ptr, Arg::Byte],
            CALLEES,
            &[],
            &SMALL_WORLD_VARIANTS,
            put_fade_groups,
        )
    }

    fn in_archetypes(self) -> Diff {
        self.in_arena_and(
            &[Arg::Ptr],
            CALLEES,
            &[],
            &SMALL_WORLD_VARIANTS,
            put_archetypes,
        )
    }

    fn in_stats_with(self, args: &'static [Arg], plain: &[u32]) -> Diff {
        self.in_arena_without_and(
            args,
            CALLEES,
            plain,
            &[0x00ec_782f],
            &STATS_VARIANTS,
            put_stats_menu,
        )
    }

    fn in_status_data(self, args: &'static [Arg]) -> Diff {
        self.in_arena_and(args, CALLEES, &[], &STATUS_DATA_VARIANTS, put_status_data)
    }

    fn in_status_effects(self, args: &'static [Arg], plain: &[u32]) -> Diff {
        self.in_arena_and(
            args,
            CALLEES,
            plain,
            &STATUS_EFFECT_VARIANTS,
            put_status_effects,
        )
    }
}

#[test]
fn stats_menu_fn_00406de0_diff() {
    Diff::new(engine_core::entry!(
        0x0040_6de0,
        interface::stats_menu::fn_00406de0(Ptr)
    ))
    .in_stats(&[Arg::Ptr])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007db1c0_diff() {
    Diff::new(engine_core::entry!(
        0x007d_b1c0,
        interface::stats_menu::fn_007db1c0() -> u32
    ))
    .in_arena_without(&[], CALLEES, &[], &[0x00ec_782f])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007db250_diff() {
    Diff::new(engine_core::entry!(
        0x007d_b250,
        interface::stats_menu::fn_007db250(Ptr, u32, u32)
    ))
    .in_arena(&[Arg::Ptr, Arg::Small, Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007db270_diff() {
    Diff::new(engine_core::entry!(
        0x007d_b270,
        interface::stats_menu::fn_007db270(Ptr, u32, u32, u32)
    ))
    .in_arena(&[Arg::Ptr, Arg::Small, Arg::Small, Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007db2c0_diff() {
    Diff::new(engine_core::entry!(
        0x007d_b2c0,
        interface::stats_menu::fn_007db2c0(Ptr) -> Ptr
    ))
    .in_arena_without(&[Arg::Object], CALLEES, &[], &[0x00ec_782f])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007db340_diff() {
    Diff::new(engine_core::entry!(
        0x007d_b340,
        interface::stats_menu::fn_007db340(Ptr) -> bool
    ))
    .in_stats(&[Arg::Ptr])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007dbfe0_diff() {
    Diff::new(engine_core::entry!(
        0x007d_bfe0,
        interface::stats_menu::fn_007dbfe0(Ptr, u32)
    ))
    .in_fade_groups()
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fade_control_callback_diff() {
    Diff::new(engine_core::entry!(
        0x007d_c040,
        interface::stats_menu::stats_menu_fade_control_callback(u32, u32) -> bool
    ))
    .in_arena(&[Arg::Ptr, Arg::Byte], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007dc0a0_diff() {
    Diff::new(engine_core::entry!(
        0x007d_c0a0,
        interface::stats_menu::fn_007dc0a0(Ptr) -> Ptr
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_do_idle_diff() {
    Diff::new(engine_core::entry!(
        0x007d_c4f0,
        interface::stats_menu::stats_menu_do_idle(Ptr)
    ))
    .in_stats_with(&[Arg::Object], &[0x0070_27b0, 0x0070_2680])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007dcc00_diff() {
    Diff::new(engine_core::entry!(
        0x007d_cc00,
        interface::stats_menu::fn_007dcc00(Ptr) -> i32
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007dcc40_diff() {
    Diff::new(engine_core::entry!(
        0x007d_cc40,
        interface::stats_menu::fn_007dcc40(Ptr) -> i32
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_update_all_actor_values_diff() {
    Diff::new(engine_core::entry!(
        0x007d_cfc0,
        interface::stats_menu::stats_menu_update_all_actor_values(Ptr)
    ))
    .in_stats(&[Arg::Object])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_update_actor_value_diff() {
    Diff::new(engine_core::entry!(
        0x007d_d090,
        interface::stats_menu::stats_menu_update_actor_value(Ptr, u32)
    ))
    .in_stats(&[Arg::Object, Arg::ActorValue])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_update_perks_diff() {
    Diff::new(engine_core::entry!(
        0x007d_d710,
        interface::stats_menu::stats_menu_update_perks()
    ))
    .in_stats(&[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007dd8a0_diff() {
    Diff::new(engine_core::entry!(
        0x007d_d8a0,
        interface::stats_menu::fn_007dd8a0(Ptr) -> u8
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007ddad0_diff() {
    Diff::new(engine_core::entry!(
        0x007d_dad0,
        interface::stats_menu::fn_007ddad0(Ptr, u32)
    ))
    .in_arena(&[Arg::Ptr, Arg::Small], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007ddb00_diff() {
    Diff::new(engine_core::entry!(
        0x007d_db00,
        interface::stats_menu::fn_007ddb00(Ptr)
    ))
    .in_stats(&[Arg::Ptr])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007ddef0_diff() {
    Diff::new(engine_core::entry!(
        0x007d_def0,
        interface::stats_menu::fn_007ddef0() -> f32
    ))
    .in_arena(&[], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007df1b0_diff() {
    Diff::new(engine_core::entry!(
        0x007d_f1b0,
        interface::stats_menu::fn_007df1b0(Ptr) -> bool
    ))
    .in_archetypes()
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007df1d0_diff() {
    Diff::new(engine_core::entry!(
        0x007d_f1d0,
        interface::stats_menu::fn_007df1d0(Ptr) -> u32
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007df210_diff() {
    Diff::new(engine_core::entry!(
        0x007d_f210,
        interface::stats_menu::fn_007df210(Ptr) -> u32
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_update_healing_options_diff() {
    Diff::new(engine_core::entry!(
        0x007d_f230,
        interface::stats_menu::stats_menu_update_healing_options(Ptr, u32)
    ))
    .in_stats(&[Arg::Object, Arg::Small])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_update_player_name_diff() {
    Diff::new(engine_core::entry!(
        0x007d_f4e0,
        interface::stats_menu::stats_menu_update_player_name()
    ))
    .in_stats(&[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_update_player_title_diff() {
    Diff::new(engine_core::entry!(
        0x007d_f5d0,
        interface::stats_menu::stats_menu_update_player_title()
    ))
    .in_stats(&[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_update_healing_mode_diff() {
    Diff::new(engine_core::entry!(
        0x007d_f620,
        interface::stats_menu::stats_menu_update_healing_mode(Ptr)
    ))
    .in_stats(&[Arg::Ptr])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_special_update_fn_diff() {
    Diff::new(engine_core::entry!(
        0x007d_fa30,
        interface::stats_menu::stats_menu_special_update_fn(u32, u32)
    ))
    .in_stats(&[Arg::Ptr, Arg::ActorValue])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_skills_update_fn_diff() {
    Diff::new(engine_core::entry!(
        0x007d_fb00,
        interface::stats_menu::stats_menu_skills_update_fn(u32, u32)
    ))
    .in_stats(&[Arg::Ptr, Arg::ActorValue])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007dfc60_diff() {
    Diff::new(engine_core::entry!(
        0x007d_fc60,
        interface::stats_menu::fn_007dfc60(u32, u32)
    ))
    .in_arena(&[Arg::Ptr, Arg::Small], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007dfcd0_diff() {
    Diff::new(engine_core::entry!(
        0x007d_fcd0,
        interface::stats_menu::fn_007dfcd0() -> u8
    ))
    .in_stats(&[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007dfdf0_diff() {
    Diff::new(engine_core::entry!(
        0x007d_fdf0,
        interface::stats_menu::fn_007dfdf0(Ptr, u32) -> u32
    ))
    .in_arena(&[Arg::Ptr, Arg::ActorValue], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007dfe40_diff() {
    Diff::new(engine_core::entry!(
        0x007d_fe40,
        interface::stats_menu::fn_007dfe40(Ptr) -> i32
    ))
    .in_stats(&[Arg::Ptr])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_set_current_page_diff() {
    Diff::new(engine_core::entry!(
        0x007d_ff40,
        interface::stats_menu::stats_menu_set_current_page(Ptr, i32)
    ))
    .in_stats(&[Arg::Object, Arg::Small])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007e04e0_diff() {
    Diff::new(engine_core::entry!(
        0x007e_04e0,
        interface::stats_menu::fn_007e04e0(u32, u32) -> i32
    ))
    .in_arena(&[Arg::Ptr, Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007e05f0_diff() {
    Diff::new(engine_core::entry!(
        0x007e_05f0,
        interface::stats_menu::fn_007e05f0(Ptr) -> bool
    ))
    .in_stats(&[Arg::Ptr])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007e07a0_diff() {
    Diff::new(engine_core::entry!(
        0x007e_07a0,
        interface::stats_menu::fn_007e07a0(Ptr) -> bool
    ))
    .in_stats(&[Arg::Ptr])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007e07f0_diff() {
    Diff::new(engine_core::entry!(
        0x007e_07f0,
        interface::stats_menu::fn_007e07f0(Ptr)
    ))
    .in_status_effects(&[Arg::Ptr], &[0x0071_f9f0, 0x0071_f570])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007e0850_diff() {
    Diff::new(engine_core::entry!(
        0x007e_0850,
        interface::stats_menu::fn_007e0850(Ptr)
    ))
    .in_stats(&[Arg::Ptr])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007e09b0_diff() {
    Diff::new(engine_core::entry!(
        0x007e_09b0,
        interface::stats_menu::fn_007e09b0(Ptr)
    ))
    .in_arena(&[Arg::Ptr], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_status_data_list_register_diff() {
    Diff::new(engine_core::entry!(
        0x007e_0b20,
        interface::stats_menu::stats_menu_status_data_list_register(Ptr, u32)
    ))
    .in_status_data(&[Arg::Ptr, Arg::Ptr])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007e0ce0_diff() {
    Diff::new(engine_core::entry!(
        0x007e_0ce0,
        interface::stats_menu::fn_007e0ce0(Ptr, u32) -> u32
    ))
    .in_status_data(&[Arg::Ptr, Arg::Small])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007e0e30_diff() {
    Diff::new(engine_core::entry!(
        0x007e_0e30,
        interface::stats_menu::fn_007e0e30(Ptr, u32, u32)
    ))
    .in_arena(&[Arg::Ptr, Arg::Code, Arg::Word], CALLEES, &[])
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007e0e70_diff() {
    Diff::new(engine_core::entry!(
        0x007e_0e70,
        interface::stats_menu::fn_007e0e70(Ptr, u32, u32, u32, u32) -> u32
    ))
    .in_item_list(&[Arg::Ptr, Arg::Word, Arg::Ptr, Arg::Code, Arg::Ptr], 48)
    .run()
    .assert_same();
}

#[test]
fn stats_menu_fn_007e1110_diff() {
    Diff::new(engine_core::entry!(
        0x007e_1110,
        interface::stats_menu::fn_007e1110(Ptr, u32) -> u32
    ))
    .in_item_list(&[Arg::Ptr, Arg::ListTile], 40)
    .run()
    .assert_same();
}

#[test]
fn stats_menu_list_box_misc_stat_id_add_item_diff() {
    Diff::new(engine_core::entry!(
        0x007e_1190,
        interface::stats_menu::list_box_misc_stat_id_add_item(Ptr, u32, u32, u32, u32) -> u32
    ))
    .in_item_list(&[Arg::Ptr, Arg::Word, Arg::Ptr, Arg::Code, Arg::Ptr], 48)
    .run()
    .assert_same();
}
