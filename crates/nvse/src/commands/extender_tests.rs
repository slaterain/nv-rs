//! Tests of xNVSE's version and plugin commands through their records. The answers pinned are
//! xNVSE 6.4.9's as its source and the shipped DLL give them (the plugin ones over nv-rs's built-in
//! plugins: none yet); none has been run in the real game yet.

use super::super::test_support::*;
use super::*;
use std::cell::RefCell;

const CONSOLE: u32 = 0x0200_f000;

thread_local! {
    static LINES: RefCell<Vec<(Vec<u8>, u32)>> = const { RefCell::new(Vec::new()) };
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// The table and a console whose lines are kept (format, first word).
fn setup() -> Engine {
    LINES.with(|l| l.borrow_mut().clear());
    let mut e = engine_with_table();
    e.register(0x0071_b160, |_, _| eax(CONSOLE));
    e.register(0x0071_d0a0, |e, a| {
        let format = crate::table::guest_text(e, a[1]);
        let word = e.mem.u32(a[2]);
        LINES.with(|l| l.borrow_mut().push((format, word)));
        eax(0)
    });
    e
}

fn run(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> f64 {
    let (ok, result) = call(e, opcode, 0, args);
    assert_eq!(ok, 1);
    result
}

/// The release the built-in matches is xNVSE 6.4.9 (`nvse_version.h`), in one place: the co-save
/// header takes its version from it.
#[test]
fn the_version_is_xnvse_6_4_9_in_one_place() {
    assert_eq!(crate::VERSION, (6, 4, 9));
    assert_eq!(crate::cosave::NVSE_VERSION, (6, 4));
}

/// `GetNVSEVersion`, `GetNVSERevision` and `GetNVSEBeta` answer 6, 4 and 9 as commands and as
/// conditions (condition-capable records), and print the number in the console.
#[test]
fn the_version_commands_answer_6_4_9() {
    let mut e = setup();
    for (opcode, want, line) in [
        (0x1400, 6.0, "NVSE version: %d"),
        (0x1401, 4.0, "NVSE revision: %d"),
        (0x1402, 9.0, "NVSE beta: %d"),
    ] {
        assert_eq!(run(&mut e, opcode, &[]), want);
        assert_eq!(eval(&mut e, opcode, 0, 0, 0), want);
        assert!(LINES.with(|l| l.borrow().is_empty()), "no console");
        console_on(&mut e);
        run(&mut e, opcode, &[]);
        eval(&mut e, opcode, 0, 0, 0);
        let lines = LINES.with(|l| std::mem::take(&mut *l.borrow_mut()));
        assert_eq!(lines.len(), 2);
        for (format, word) in lines {
            assert_eq!(format, line.as_bytes());
            assert_eq!(f64::from(word), want);
        }
        console_off(&mut e);
    }
}

fn console_on(e: &mut Engine) {
    e.mem.set_u8(0x011d_ea2e, 1);
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
}

fn console_off(e: &mut Engine) {
    e.mem.set_u8(0x011d_ea2e, 0);
}

/// No plugin is built in, so every name answers "not installed": `IsPluginInstalled` 0,
/// `GetPluginVersion` -1, JIP LN NVSE's own names included (they answer installed once JIP's
/// commands are built in, I-13). Neither prints anything.
#[test]
fn no_plugin_is_installed() {
    let mut e = setup();
    console_on(&mut e);
    for name in ["JIP NVSE Plugin", "lutana_nvse", "JIP LN NVSE", "nvse", ""] {
        assert_eq!(run(&mut e, 0x15df, &[s(name)]), 0.0, "{name}");
        assert_eq!(run(&mut e, 0x15e0, &[s(name)]), -1.0, "{name}");
    }
    assert!(LINES.with(|l| l.borrow().is_empty()));
}

/// The lookup over a list of plugins: the first whose name matches without case (A to Z only),
/// its version unsigned.
#[test]
fn plugins_are_found_by_name_without_case() {
    let list = [
        Plugin {
            name: "Some Plugin",
            version: 0xffff_ffff,
        },
        Plugin {
            name: "SOME PLUGIN",
            version: 2,
        },
    ];
    assert_eq!(installed(&list, b"some plugin"), 1.0);
    assert_eq!(
        version_of(&list, b"sOME pLUGIN"),
        Some(4_294_967_295.0),
        "the first"
    );
    assert_eq!(installed(&list, b"Some Plugin "), 0.0);
    assert_eq!(installed(&list, b"Some_Plugin"), 0.0);
    assert_eq!(version_of(&list, b"other"), None);
    assert_eq!(installed(&[], b"some plugin"), 0.0);
}

/// A plugin name of 256 bytes or more runs past xNVSE's buffer (the saved frame is right above
/// it): without the script list and scan guard row the port stops the game; with it the name is
/// read whole. 255 bytes and the terminator fit.
#[test]
fn a_long_plugin_name() {
    let long = "x".repeat(256);
    let mut e = setup();
    for opcode in [0x15df, 0x15e0] {
        let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            call(&mut e, opcode, 0, &[s(&long)])
        }));
        assert!(stop.is_err(), "{opcode:04x}");
    }
    assert_eq!(run(&mut e, 0x15e0, &[s(&long[1..])]), -1.0, "255 bytes fit");
    let mut e = setup();
    super::super::array::set_fix(&mut e, super::super::array::LIST_GUARD, true);
    assert_eq!(run(&mut e, 0x15df, &[s(&long)]), 0.0);
    assert_eq!(run(&mut e, 0x15e0, &[s(&long)]), -1.0);
}

/// The records as the DLL has them: the version commands condition-capable with no parameters,
/// the plugin commands with one string; the help texts.
#[test]
fn the_records_match_the_dll() {
    let e = engine_with_table();
    for (opcode, help, params, condition) in [
        (0x1400, "returns the installed version of NVSE", 0, 1),
        (
            0x1401,
            "returns the numbered revision of the installed version of NVSE",
            0,
            1,
        ),
        (
            0x1402,
            "returns the numbered beta of the installed version of NVSE",
            0,
            1,
        ),
        (
            0x15df,
            "returns 1 if the specified plugin is installed, else 0",
            1,
            0,
        ),
        (
            0x15e0,
            "returns the version of the specified plugin, or -1 if the plugin is not installed",
            1,
            0,
        ),
    ] {
        let record = crate::table::record_of(&e, opcode);
        assert_eq!(
            crate::table::guest_text(&e, e.mem.u32(record + 0xc)),
            help.as_bytes()
        );
        assert_eq!(e.mem.u16(record + 0x12), params);
        assert_eq!(e.mem.u32(record + 0x24), condition, "{opcode:04x}");
    }
}
