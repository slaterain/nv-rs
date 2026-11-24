//! Expectations from owned Tweaks 10.05's timer, pause, queue and rotation functions.
use super::*;

const M: u32 = 0x0200_0000;
const P: u32 = M + 0x1000;
const I: u32 = M + 0x2000;
const PROCESS: u32 = M + 0x3000;
const VTABLE: u32 = M + 0x4000;
const WORLD: u32 = M + 0x5000;
const AIMING: u32 = 0x00fc_0100;

#[derive(Default)]
struct Calls {
    names: Vec<Option<Vec<u8>>>,
    save_ok: bool,
    can_save: bool,
    aiming: bool,
    sky: usize,
    notices: usize,
}
fn base(_: &mut Engine, _: &[u32]) -> Ret {
    panic!("enabled replacement must not call vanilla autosave")
}
fn engine(extra: &str) -> Engine {
    let mut e = Engine::new();
    e.map(0x011d_8000, 0x20000);
    e.map(M, 0x6000);
    e.mem.set_u32(MANAGER, M);
    e.mem.set_u32(PLAYER, P);
    e.mem.set_u32(INTERFACE, I);
    e.mem.set_u32(P + 0x68, PROCESS);
    e.mem.set_u32(PROCESS, VTABLE);
    e.mem.set_u32(VTABLE + 0x404, AIMING);
    e.mem.set_u32(I + 0xc, 1);
    e.mem.set_u32(0x011d_ea10, WORLD);
    e.mem.set_u32(WORLD + 0x68, 0x7654);
    e.mem.set_u32(0x011d_ea30, 0.125f32.to_bits());
    e.ext::<Calls>().save_ok = true;
    e.ext::<Calls>().can_save = true;
    e.register(0x0085_0a40, base);
    e.register(CAN_SAVE, |e, w| {
        assert_eq!(w, [M, 0], "native CanSave(false)");
        u32::from(e.ext::<Calls>().can_save).into_ret()
    });
    e.register(AIMING, |e, w| {
        assert_eq!(w, [PROCESS]);
        u32::from(e.ext::<Calls>().aiming).into_ret()
    });
    e.register(0x0063_ac70, |e, w| {
        assert_eq!(w, [0x7654, 0.125f32.to_bits()]);
        e.ext::<Calls>().sky += 1;
        Ret::default()
    });
    e.register(0x0070_52f0, |e, w| {
        assert_eq!(w, [0x0106_14e0, 0, 0, 0, 1f32.to_bits(), 0]);
        e.ext::<Calls>().notices += 1;
        Ret::default()
    });
    e.register(SAVE, |e, w| {
        assert_eq!(w.len(), 4);
        assert_eq!([w[0], w[2], w[3]], [M, u32::MAX, 0]);
        let name = (w[1] != 0).then(|| e.mem.cstr(w[1]));
        e.ext::<Calls>().names.push(name);
        u32::from(e.ext::<Calls>().save_ok).into_ret()
    });
    let ini = format!("[Tweaks]\nbImprovedAutosave=1\n[Save Manager]\n{extra}");
    install(&mut e, &Settings::new(&[("main", ini.as_bytes())], true));
    message(&mut e, msg::DEFERRED_INIT);
    e
}
fn message(e: &mut Engine, kind: u32) {
    listen(
        e,
        &Message {
            kind,
            data: 0,
            len: 0,
        },
    );
}
fn now(e: &mut Engine, time: u32) {
    e.mem.set_u32(CLOCK, time);
}
fn site(e: &mut Engine, slot: SiteSlot, args: &[u32]) -> Option<u32> {
    e.site(slot.addr(), args)
}
fn save(e: &mut Engine) -> bool {
    e.call(0x0085_0a40, &[M]).bool()
}

#[test]
fn enable_and_seconds_match_native_integer_single_precision_conversion() {
    let mut off = Engine::new();
    install(&mut off, &Settings::new(&[], true));
    assert!(off.ext_get::<State>().is_none());
    assert!(off.wraps_at(0x0085_0a40).is_empty());
    for seconds in [0, 1, 2, -1, 2_147_483, 2_147_484, i32::MIN, i32::MAX] {
        let ms = milliseconds(seconds);
        assert_eq!(
            ms,
            match seconds {
                0 => 0,
                1 => 1000,
                2 => 2000,
                -1 => -1000,
                2_147_483 => 2_147_483_008,
                _ => i32::MIN,
            }
        );
    }
    for seconds in [0, 1, 2] {
        let mut e = engine(&format!("iAutoSaveTimer={seconds}\n"));
        assert_eq!(e.ext::<nvse::Nvse>().plugins.per_frame, seconds > 1);
        assert_eq!(e.wraps_at(0x0085_0a40), [Owner::Stewie]);
        assert_eq!(e.site_owners(0x0085_1dae), [Owner::Stewie]);
    }
}

#[test]
fn timer_preserves_gate_order_aiming_retry_and_five_frame_request() {
    let mut e = engine("iAutoSaveTimer=10\nbPreventAutosaveInCombat=1\nbPreventAutosaveInTGM=1\n");
    now(&mut e, 9999);
    timer(&mut e);
    assert_eq!(e.mem.u8(M + 0xf), 0);
    now(&mut e, 10000);
    e.mem.set_u8(M + 0xf, 9);
    timer(&mut e);
    assert_eq!(e.mem.u8(M + 0xf), 9);
    e.mem.set_u8(M + 0xf, 0);
    for (offset, value) in [(0xc, 2), (0x4bc, 1)] {
        e.mem.set_u32(I + offset, value);
        timer(&mut e);
        assert_eq!(e.mem.u8(M + 0xf), 0);
        e.mem.set_u32(I + offset, if offset == 0xc { 1 } else { 0 });
    }
    e.ext::<Calls>().can_save = false;
    timer(&mut e);
    assert_eq!(e.ext::<State>().last, 0);
    e.ext::<Calls>().can_save = true;
    e.ext::<Calls>().aiming = true;
    timer(&mut e);
    assert_eq!(e.mem.u8(M + 0xf), 0);
    e.ext::<Calls>().aiming = false;
    e.mem.set_u8(P + 0xdf0, 1);
    timer(&mut e);
    assert_eq!(e.ext::<State>().last, 3000, "native 3-second retry");
    now(&mut e, 13000);
    e.mem.set_u8(P + 0xdf1, 1);
    e.mem.set_u8(0x011e_0898, 1);
    timer(&mut e);
    assert_eq!(e.ext::<State>().last, 6000);
    now(&mut e, 16000);
    e.mem.set_u8(0x011e_0898, 0);
    timer(&mut e);
    assert_eq!(e.mem.u8(M + 0xf), 5);
    assert!(e.ext::<State>().timed);
    assert_eq!(
        e.ext::<State>().last,
        6000,
        "queueing does not reset deadline"
    );
    assert!(e.ext::<Calls>().names.is_empty());
}

#[test]
fn deadline_uses_native_unsigned_wrapping_sum_and_cached_process() {
    let mut e = engine("iAutoSaveTimer=2\n");
    e.ext::<State>().last = u32::MAX - 1000;
    now(&mut e, 998);
    timer(&mut e);
    assert_eq!(e.mem.u8(M + 0xf), 0);
    now(&mut e, 999);
    e.mem.set_u32(P + 0x68, 0);
    timer(&mut e);
    assert_eq!(e.mem.u8(M + 0xf), 5, "uses original cached process");
}

#[test]
fn pause_overlap_zero_sentinel_load_and_save_messages_preserve_clock_contract() {
    let mut e = engine("bAllSavesResetAutosaveTimer=0\n");
    e.ext::<State>().last = 10;
    now(&mut e, 100);
    site(&mut e, MENU_OPEN, &[]);
    now(&mut e, 200);
    site(&mut e, CONSOLE_OPEN, &[]);
    now(&mut e, 300);
    site(&mut e, MENU_CLOSE, &[]);
    assert_eq!(e.ext::<State>().last, 10);
    now(&mut e, 400);
    site(&mut e, CONSOLE_CLOSE, &[]);
    assert_eq!(e.ext::<State>().last, 310);
    assert_eq!(e.ext::<State>().pause_start, 0);
    now(&mut e, 0);
    site(&mut e, MENU_OPEN, &[]);
    now(&mut e, 50);
    site(&mut e, MENU_CLOSE, &[]);
    assert_eq!(
        e.ext::<State>().last,
        310,
        "zero remains native unset sentinel"
    );
    now(&mut e, 500);
    site(&mut e, MENU_OPEN, &[]);
    now(&mut e, 600);
    e.mem.set_u8(M + 0xf, 5);
    message(&mut e, msg::POST_LOAD_GAME);
    assert_eq!(e.ext::<State>().last, 600);
    assert_eq!(e.ext::<State>().pause_start, 600);
    assert!(e.ext::<State>().menu);
    assert_eq!(e.mem.u8(M + 0xf), 0);
    now(&mut e, 700);
    site(&mut e, MENU_CLOSE, &[]);
    assert_eq!(e.ext::<State>().last, 700);
    now(&mut e, 800);
    message(&mut e, msg::SAVE_GAME);
    assert_eq!(e.ext::<State>().last, 700, "disabled all-saves reset");
    e.ext::<State>().timed = true;
    message(&mut e, msg::SAVE_GAME);
    assert_eq!(e.ext::<State>().last, 800);
    assert!(!e.ext::<State>().timed);
    now(&mut e, 900);
    message(&mut e, msg::NEW_GAME);
    assert_eq!(e.ext::<State>().last, 900);
}

#[test]
fn queue_hooks_preserve_travel_countdown_dialog_and_loading_contract() {
    let mut e = engine("");
    assert_eq!(site(&mut e, TRAVEL, &[M]), Some(0));
    assert_eq!(e.mem.u8(M + 0xf), 32);
    site(&mut e, COUNTDOWN, &[M]);
    assert_eq!(e.mem.u8(M + 0xf), 31);
    for global in [0x011d_a0c0, 0x011d_9514] {
        e.mem.set_u32(global, 1);
        site(&mut e, COUNTDOWN, &[M]);
        assert_eq!(e.mem.u8(M + 0xf), 255);
        e.mem.set_u32(global, 0);
    }
    assert_eq!(site(&mut e, LOADING, &[M, 1]), None);
    assert_eq!(e.mem.u8(M + 0xf), 120);
    site(&mut e, LOADING, &[M, 0]);
    assert_eq!(e.mem.u8(M + 0xf), 120);
}

#[test]
fn rotation_writes_only_successful_numbered_indices_through_portable_file_layer() {
    crate::tests::with_writable_mod_file(|| {
        let mut e = engine("");
        for index in [1, 2, 3, 1] {
            assert!(save(&mut e));
            assert_eq!(e.ext::<State>().index, index);
        }
        let writes = crate::tests::mod_file_writes();
        assert_eq!(writes.len(), 4);
        assert!(writes.iter().all(|(name, _)| name == crate::INI));
        assert_eq!(
            e.ext::<Calls>().names,
            [
                Some(b"Autosave - 1".to_vec()),
                Some(b"Autosave - 2".to_vec()),
                Some(b"Autosave - 3".to_vec()),
                Some(b"Autosave - 1".to_vec()),
            ]
        );
        assert_eq!(
            mod_files::profile_string(&e, crate::INI, b"Unrelated", b"kept"),
            Some(b"1".to_vec())
        );
        assert_eq!(
            mod_files::profile_string(&e, crate::INI, b"Save Manager", b"_iAutoSaveIndex"),
            Some(b"1".to_vec())
        );
        e.ext::<Calls>().save_ok = false;
        assert!(!save(&mut e));
        assert_eq!(
            e.ext::<State>().index,
            2,
            "failure does not roll runtime index back"
        );
        assert_eq!(crate::tests::mod_file_writes().len(), 4);
        e.ext::<Calls>().save_ok = true;
        crate::tests::allow_mod_file_writes(false);
        assert!(
            save(&mut e),
            "INI write failure does not change game's success"
        );
        assert_eq!(e.ext::<State>().index, 3);
        assert_eq!(crate::tests::mod_file_writes().len(), 4);
    });
}

#[test]
fn periodic_fullsave_zero_max_and_minimum_interval_keep_native_edge_cases() {
    crate::tests::with_writable_mod_file(|| {
        let mut e = engine("iMaxAutoSaveCount=0\nbPeriodicFullsave=1\n");
        assert!(save(&mut e));
        assert_eq!(e.ext::<Calls>().names, [None]);
        assert_eq!(e.ext::<Calls>().notices, 1);
        assert!(crate::tests::mod_file_writes().is_empty());
        e.ext::<State>().config.periodic_fullsave = false;
        assert!(save(&mut e));
        assert!(save(&mut e));
        assert_eq!(
            e.ext::<Calls>().names[1..],
            [
                Some(b"Autosave - 1".to_vec()),
                Some(b"Autosave - 1".to_vec())
            ]
        );
        e.ext::<State>().config.min_interval = 1000;
        e.ext::<State>().last = 100;
        e.ext::<State>().timed = true;
        now(&mut e, 1099);
        e.mem.set_u16(M + 0xe, 0x0501);
        let sky = e.ext::<Calls>().sky;
        assert!(!save(&mut e));
        assert_eq!(
            e.mem.u16(M + 0xe),
            0,
            "cleared before minimum-interval test"
        );
        assert_eq!(e.ext::<Calls>().sky, sky);
        assert!(
            e.ext::<State>().timed,
            "only actual save message clears pending"
        );
        now(&mut e, 1100);
        assert!(save(&mut e));
    });
}

#[test]
fn manual_save_keys_use_game_pressed_edges_and_preserve_full_then_incremental_order() {
    crate::tests::with_writable_mod_file(|| {
        let mut e = engine("iCreateSaveKey=60\niIncrementalSaveKey=61\nbHideAutosaveMessage=1\n");
        e.mem.set_u32(0x011f_35cc, 0x8877);
        e.register(0x00a2_4180, |_, words| {
            assert_eq!(words[0], 0x8877);
            assert_eq!(words[2], 1);
            assert!([60, 61].contains(&words[1]));
            1u32.into_ret()
        });
        e.register(0x0055_d520, |e, words| {
            assert_eq!(words, &[P]);
            nvse::table::guest_str(e, b"Cour\xe9er").into_ret()
        });
        now(&mut e, 1);
        assert_eq!(e.site(0x0094_0c78, &[]), None);
        assert_eq!(
            e.ext::<Calls>().names,
            [None, Some(b"Cour\xe9er - Save 1".to_vec())]
        );
        assert_eq!(
            e.ext::<Calls>().notices,
            2,
            "manual messages are not gated by autosave-message setting"
        );
        e.ext::<Calls>().save_ok = false;
        e.site(0x0094_0c78, &[]);
        assert_eq!(e.ext::<Calls>().notices, 2);
        assert_eq!(crate::tests::mod_file_writes().len(), 1);
    });
}

#[test]
fn incremental_slots_use_strict_signed_interval_and_persist_only_success() {
    crate::tests::with_writable_mod_file(|| {
        let mut e = engine("iMaxIncrementalSaveCount=2\niIncrementalSaveSlotChangeInterval=1\n");
        e.register(0x0055_d520, |e, _| {
            nvse::table::guest_str(e, b"Courier").into_ret()
        });
        now(&mut e, 1000);
        assert!(incremental_save(&mut e));
        assert_eq!(e.ext::<State>().incremental_index, 0);
        now(&mut e, 1001);
        assert!(incremental_save(&mut e));
        assert_eq!(e.ext::<State>().incremental_index, 1);
        now(&mut e, 2001);
        assert!(incremental_save(&mut e));
        assert_eq!(e.ext::<State>().incremental_index, 1);
        now(&mut e, 2002);
        e.ext::<Calls>().save_ok = false;
        assert!(!incremental_save(&mut e));
        assert_eq!(e.ext::<State>().incremental_index, 2);
        assert_eq!(e.ext::<State>().last_slot_change, 2002);
        assert_eq!(
            mod_files::profile_string(&e, crate::INI, b"Save Manager", b"_iIncrementalSaveIndex"),
            Some(b"1".to_vec())
        );
        now(&mut e, 3003);
        e.ext::<Calls>().save_ok = true;
        assert!(incremental_save(&mut e));
        assert_eq!(e.ext::<State>().incremental_index, 1);
        message(&mut e, msg::NEW_GAME);
        assert_eq!(e.ext::<State>().last_slot_change, 3003);
        assert_eq!(e.ext::<State>().incremental_index, 1);
        now(&mut e, 3003u32.wrapping_add(0x80000001));
        assert!(incremental_save(&mut e));
        assert_eq!(
            e.ext::<State>().incremental_index,
            1,
            "signed negative elapsed does not advance"
        );
    });
}

#[test]
fn reload_hotkey_replaces_only_native_quickload_reads_and_defers_load_to_main_phase() {
    let mut e = engine("iReloadCurrentSaveKey=62\n");
    e.map(0x011a_2000, 0x2000);
    e.mem.set_u32(0x011a_2fe0, 1);
    e.register(0x00a2_4180, |_, words| {
        assert_eq!(words, &[0x8877, 62, 1]);
        1u32.into_ret()
    });
    e.register(0x00a2_4660, |_, words| {
        assert_eq!(words, &[0x8877, 0x1a, 1]);
        1u32.into_ret()
    });
    e.register(0x0085_12f0, |e, words| {
        assert_eq!(words, &[M]);
        assert_eq!(e.mem.u8(0x011e_0be5), 1);
        e.ext::<Calls>().notices += 1;
        Ret::default()
    });
    e.register(0x0085_09f0, |e, _| {
        e.ext::<Calls>().sky += 1;
        Ret::default()
    });
    assert_eq!(e.site(0x0094_039b, &[0x8877, 0x1a, 1]), Some(0));
    assert!(e.ext::<State>().reload_requested);
    // Another source input read in the same tick ORs in vanilla quickload.
    assert_eq!(e.site(0x0094_2869, &[0x8877, 0x1a, 1]), Some(0));
    assert!(e.ext::<State>().quickload_requested);
    e.mem.set_u8(0x011d_ea2e, 1);
    assert_eq!(e.site(0x0086_eac2, &[]), Some(0x011d_effc));
    assert_eq!(e.ext::<Calls>().notices, 0);
    e.mem.set_u8(0x011d_ea2e, 0);
    e.site(0x0086_eac2, &[]);
    assert_eq!(e.ext::<Calls>().notices, 1);
    assert_eq!(e.ext::<Calls>().sky, 0, "reload wins over quickload");
    assert!(!e.ext::<State>().reload_requested);
    assert!(!e.ext::<State>().quickload_requested);
    e.register(0x00a2_4180, |_, _| 0u32.into_ret());
    e.site(0x0094_7ab8, &[0x8877, 0x1a, 1]);
    e.site(0x0086_eac2, &[]);
    assert_eq!(e.ext::<Calls>().sky, 0, "one load per tick");
    e.mem.set_u32(0x011a_2fe0, 2);
    e.site(0x0086_eac2, &[]);
    assert_eq!(e.ext::<Calls>().sky, 1);
    assert_eq!(e.mem.u32(0x011a_3b34), (-1f32).to_bits());
    assert_eq!(e.mem.u8(0x011e_0be5), 0);
}

#[test]
fn event_triggers_queue_source_delays_and_preserve_prelevel_unsigned_boundary() {
    let mut e = engine(
        "bSaveOnLocationDiscovered=1\nbSaveOnQuestCompleted=1\nbSavePreLevelUp=1\nbSaveOnPickpocket=1\nbSaveOnCraft=1\n",
    );
    for address in [0x00779529, 0x0060ca6c] {
        e.mem.set_u8(M + 0xf, 99);
        e.site(address, &[]);
        assert_eq!(e.mem.u8(M + 0xf), 1);
    }
    e.ext::<State>().last = 20;
    now(&mut e, 10020);
    e.mem.set_u8(M + 0xf, 0);
    e.site(0x0077d6de, &[]);
    assert_eq!(e.mem.u8(M + 0xf), 0);
    now(&mut e, 10021);
    e.site(0x0077d6de, &[]);
    assert_eq!(e.mem.u8(M + 0xf), 1);
    now(&mut e, 0);
    e.mem.set_u8(M + 0xf, 0);
    e.site(0x0077d6de, &[]);
    assert_eq!(
        e.mem.u8(M + 0xf),
        1,
        "source wrapping(now-10000) unsigned comparison"
    );
    for (pick, caught, expected) in [(0, 0, 0), (1, 1, 0), (1, 0, 1)] {
        e.mem.set_u8(P + 0x88, pick);
        e.mem.set_u8(P + 0x89, caught);
        e.mem.set_u8(M + 0xf, 0);
        e.site(0x0075b24a, &[P]);
        assert_eq!(e.mem.u8(M + 0xf), expected);
    }
    e.map(0x011c6000, 0x1000);
    e.mem.set_u32(0x011c6dd0, WORLD);
    e.mem.set_u32(WORLD + 4, 10);
    e.site(0x007272f2, &[0]);
    e.site(0x007272f2, &[1]);
    e.mem.set_u8(M + 0xf, 0);
    e.site(0x00726f2a, &[0]);
    assert_eq!(e.mem.u8(M + 0xf), 0);
    e.mem.set_u32(WORLD + 4, 11);
    e.site(0x00726f2a, &[0]);
    assert_eq!(e.mem.u8(M + 0xf), 20);
    e.site(0x00726f2a, &[1]);
}

#[test]
fn exit_rotation_bypasses_min_interval_but_forces_original_menu_checks() {
    crate::tests::with_writable_mod_file(|| {
        let mut e = engine("bSaveOnExitGame=1\niMinAutosaveInterval=999\n");
        assert_eq!(e.site(0x00851050, &[1]), None);
        e.site(0x005b6cb0, &[P]);
        assert_eq!(e.site(0x00851050, &[1]), Some(0));
        assert_eq!(e.ext::<Calls>().names, [Some(b"Autosave - 1".to_vec())]);
        assert_eq!(e.ext::<Calls>().sky, 0, "exit calls rotation directly");
    });
}

#[test]
fn script_suppression_preserves_console_echo_rule_and_native_autosave_gate_write() {
    let mut e = engine("bPreventScriptedSaves=1\nbHideAutosaveMessage=1\n");
    assert_eq!(e.mem.u32(0x011de2a0), 0);
    assert_eq!(e.site(0x00850415, &[1]), Some(0));
    assert_eq!(e.site(0x005c54b7, &[]), Some(0));
    e.mem.set_u8(0x011dea2e, 1);
    let tls = e.tls();
    assert_eq!(e.site(0x005c54b7, &[]), Some(0));
    e.mem.set_u8(tls + 0x268, 1);
    assert_eq!(e.site(0x005c54b7, &[]), Some(1));
    e.mem.set_u8(0x011dea2e, 0);
    assert_eq!(e.site(0x005c54b7, &[]), Some(0));
}

#[test]
fn pause_option_precedes_continue_and_only_pause_save_uses_native_save_gate() {
    let mut e = engine("bReplaceContinueWithQuicksave=1\n");
    e.map(0x011d3000, 0x1000);
    e.mem.set_u32(0x011d3280, 0x12345678);
    let original = P + 0x100;
    e.mem.set_u32(original + 8, 0x007d0440);
    e.mem.set_u32(original + 0xc, 3);
    e.register(0x00aa3e40, |e, w| {
        assert_eq!(w, &[0x011f6238, 16]);
        e.mem.alloc(16).into_ret()
    });
    e.register(0x007cb2e0, |e, w| {
        assert_eq!(w[0], 0x011dab88);
        let option = e.mem.u32(w[1]);
        let record = [
            e.mem.u32(option),
            e.mem.u32(option + 4),
            e.mem.u32(option + 8),
            e.mem.u32(option + 0xc),
        ];
        e.ext::<Vec<[u32; 4]>>().push(record);
        if record[2] == PAUSE_CHOOSE {
            assert_eq!(e.site(0x007cbdaf, &[option]), Some(1));
        }
        Ret::default()
    });
    assert_eq!(e.site(0x007cc7c2, &[0x011dab88, original]), Some(1));
    let records = e.ext::<Vec<[u32; 4]>>().clone();
    assert_eq!(records[0], [0x01076ee0, 0x12345678, PAUSE_CHOOSE, 2]);
    assert_eq!(records[1][2..], [0x007d0440, 1]);
    assert_eq!(e.site(0x007cbdaf, &[original]), None);
    e.mem.set_u32(0x011daac0, P);
    assert_eq!(e.site(0x0070f7e2, &[P]), Some(1));
    assert_eq!(e.site(0x0070f7e2, &[P + 4]), Some(0));
    e.register(0x00a012d0, |_, w| {
        assert_eq!(w, &[P, 0x1000, 1f32.to_bits(), 1]);
        Ret::default()
    });
    assert_eq!(e.site(0x007cfd3c, &[0, 0x1000, 1]), Some(1));
    assert_eq!(e.site(0x007cfd3c, &[P, 0x1000, 1]), Some(1));
}

#[test]
fn pause_choose_closes_only_after_success_and_plays_native_cancel_on_failure() {
    crate::tests::with_writable_mod_file(|| {
        let mut e = engine("bReplaceContinueWithQuicksave=1\n");
        e.register(0x0055d520, |e, _| {
            nvse::table::guest_str(e, b"Courier").into_ret()
        });
        e.register(0x007ce7a0, |e, w| {
            assert_eq!(w, &[1]);
            e.ext::<Calls>().sky += 1;
            Ret::default()
        });
        now(&mut e, 1);
        e.call(PAUSE_CHOOSE, &[]);
        assert_eq!(e.ext::<Calls>().sky, 1);
        e.ext::<Calls>().save_ok = false;
        e.mem.set_u32(0x011f6d98, 0x8877);
        e.register(0x00ad7550, |e, w| {
            assert_eq!(w[0], 0x8877);
            assert_eq!(e.mem.cstr(w[2]), b"UIMenuCancel");
            assert_eq!(w[3], 0x121);
            0x9988u32.into_ret()
        });
        e.register(0x00ad8830, |e, w| {
            assert_eq!(w, &[0x9988, 0]);
            e.ext::<Calls>().sky += 10;
            Ret::default()
        });
        e.call(PAUSE_CHOOSE, &[]);
        assert_eq!(
            e.ext::<Calls>().sky,
            11,
            "no second close after failed save"
        );
    });
}

#[test]
fn active_save_highlight_is_case_sensitive_and_keeps_initialized_trait() {
    let mut e = engine("");
    let current = nvse::table::guest_str(&mut e, b"Courier - Save 1.fos");
    let entry_name = nvse::table::guest_str(&mut e, b"Courier - Save 1.fos");
    let entry = P + 0x100;
    e.mem.set_u32(M + 0x14, current);
    e.mem.set_u32(entry, entry_name);
    e.register(0x00a012d0, |e, w| {
        e.ext::<Vec<Vec<u32>>>().push(w.to_vec());
        Ret::default()
    });
    e.site(0x007d6931, &[P, entry, 0x1000, 0x101]);
    assert_eq!(
        e.ext::<Vec<Vec<u32>>>().as_slice(),
        &[vec![P, 0x1000, 1f32.to_bits(), 1], vec![P, 0xff4, 0, 1]]
    );
    e.ext::<Vec<Vec<u32>>>().clear();
    let different_case = nvse::table::guest_str(&mut e, b"courier - Save 1.fos");
    e.mem.set_u32(entry, different_case);
    e.site(0x007d6931, &[P, entry, 0x1000, 1]);
    assert_eq!(e.ext::<Vec<Vec<u32>>>().len(), 1);
}
