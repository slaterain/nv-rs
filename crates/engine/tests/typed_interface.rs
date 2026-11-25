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
    0x0044_7080,
    0x0044_a670,
    0x0044_f560,
    0x0045_0f90,
    0x0045_6a20,
    0x0045_7ba0,
    0x0045_a5e0,
    0x0046_9880,
    0x0047_0470,
    0x0047_6b20,
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
    0x004b_cdb0,
    0x004b_d400,
    0x004b_d510,
    0x004b_d570,
    0x004b_d740,
    0x004b_d820,
    0x004b_da70,
    0x004b_e060,
    0x004b_e330,
    0x004b_e380,
    0x004b_f220,
    0x004c_8f30,
    0x004e_ef00,
    0x004f_8a30,
    0x004f_e160,
    0x0050_3a70,
    0x0052_5980,
    0x0052_5b20,
    0x0055_d520,
    0x0056_7790,
    0x0057_85e0,
    0x0057_c730,
    0x0057_dd70,
    0x005e_02b0,
    0x005e_58f0,
    0x0063_3c90,
    0x0064_50f0,
    0x0064_5380,
    0x0064_6360,
    0x0066_29f0,
    0x0066_b0d0,
    0x006b_3eb0,
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
    0x0071_e290,
    0x0071_e420,
    0x0072_6ff0,
    0x0072_8a30,
    0x0072_d250,
    0x0073_0690,
    0x0073_3500,
    0x0074_0f30,
    0x0075_3420,
    0x0075_4cf0,
    0x0075_7b70,
    0x0075_b310,
    0x0075_b750,
    0x0075_c280,
    0x0076_1a20,
    0x0076_3ff0,
    0x0076_4200,
    0x0076_5b80,
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
    0x0077_fc10,
    0x0077_ffd0,
    0x0078_2470,
    0x0078_2a90,
    0x0078_3440,
    0x0078_46a0,
    0x0078_4c80,
    0x0078_8cc0,
    0x0078_db00,
    0x0078_e7c0,
    0x0079_17a0,
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
    0x007a_8ba0,
    0x007a_8e60,
    0x007a_a480,
    0x007a_a530,
    0x007a_a5c0,
    0x007a_a6b0,
    0x007a_b690,
    0x007a_c730,
    0x007b_5520,
    0x007b_6fb0,
    0x007b_7570,
    0x007b_bcf0,
    0x007b_fc30,
    0x007c_01b0,
    0x007c_0580,
    0x007c_19c0,
    0x007c_6120,
    0x007c_b7d0,
    0x007c_e7a0,
    0x007d_a2c0,
    0x007d_cc80,
    0x007d_cfc0,
    0x007d_d090,
    0x007d_d710,
    0x007d_da60,
    0x007d_f230,
    0x007d_f4e0,
    0x007d_f5d0,
    0x007d_fc90,
    0x007d_fcd0,
    0x007d_fd00,
    0x007e_41f0,
    0x007e_47a0,
    0x007e_5890,
    0x007e_9200,
    0x007e_bd50,
    0x007f_6920,
    0x007f_8a80,
    0x007f_a010,
    0x0083_e510,
    0x0084_54f0,
    0x0085_79b0,
    0x0086_4980,
    0x0086_49a0,
    0x0086_5e50,
    0x0087_f4c0,
    0x0087_f6c0,
    0x008c_7aa0,
    0x0090_5820,
    0x0095_3060,
    0x009f_f340,
    0x009f_f690,
    0x00a0_11b0,
    0x00a0_12d0,
    0x00a0_1350,
    0x00a0_1860,
    0x00a0_3c90,
    0x00a0_3da0,
    0x00a0_3eb0,
    0x00a0_4200,
    0x00a0_87d0,
    0x00a0_8b20,
    0x00a0_d9e0,
    0x00a0_f8b0,
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
    0x00ec_623a,
    0x00ec_62c0,
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
        .in_arena(&[Arg::Word], CALLEES, &[])
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

#[test]
fn interface_handle_queued_menu_open_diff() {
    Diff::new(engine_core::entry!(
        0x0070_94f0,
        interface::interface_cpp_p2::interface_handle_queued_menu_open()
    ))
    .in_arena(
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

/// A seam (`extend.rs`'s SITES and the game patches' rows: an instruction inside a translated
/// function where a built-in takes part through `e.site`) inside a member of a family on by default
/// is skipped unless the typed body keeps its `e.site` call there. So no such member holds a seam,
/// except the ones listed, whose typed bodies call `e.site` at it. The function holding a seam is
/// the translated function that starts nearest below it.
#[test]
fn no_member_of_a_family_on_by_default_holds_a_seam() {
    // Each checked in its typed body (2026-11-25): the `e.site` call at the seam, behind the
    // same extender test as the translation's.
    const KEPT: &[(u32, &str)] = &[
        (0x0040_f9d7, "cell_refs"),
        (0x0045_234e, "cell_refs"),
        (0x00c6_5b5b, "collision"),
        (0x00a0_968c, "dispatch"),
        (0x0077_04ae, "hud"),
        (0x0077_0b0e, "hud"),
        (0x0077_14da, "hud"),
        (0x0077_951e, "hud"),
        (0x0077_955d, "hud"),
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
    for family in engine_typed::families()
        .into_iter()
        .chain([&engine_core::typed::leaf::LEAVES])
    {
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
        if let Some(family) = holder.and_then(|start| members.get(&start)) {
            found.push((site.addr, *family));
        }
    }
    let unkept: Vec<_> = found.iter().filter(|f| !KEPT.contains(f)).collect();
    assert!(
        unkept.is_empty(),
        "seams inside members of families on by default (seam, family): {unkept:x?}"
    );
    for kept in KEPT {
        assert!(
            found.contains(kept),
            "{kept:x?} is no member's seam any more"
        );
    }
}
