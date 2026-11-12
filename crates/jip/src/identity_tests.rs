//! Tests of JIP's identity: its answers with the flag on, xNVSE's without it, and the console
//! echo. The numbers are what JIP 57.30's source computes and its DLL loads; none has been run in
//! the real game here.

use super::*;
use crate::test_support::*;
use std::cell::RefCell;

const CONSOLE: u32 = 0x0200_f000;

thread_local! {
    static LINES: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// `EvaluateLine` as JIP's echo sees it: the handler (the first word) called from it with the
/// eight words after.
fn evaluate_line(e: &mut Engine, w: &[u32]) -> Ret {
    e.call(w[0], &w[1..9])
}

/// The table with JIP installed (answering as `installed` says), the console's print kept, and
/// a stand-in `EvaluateLine`.
fn setup(installed: bool) -> Engine {
    LINES.with(|l| l.borrow_mut().clear());
    let mut e = engine();
    crate::install_as(&mut e, installed);
    e.register(0x0071_b160, |_, _| eax(CONSOLE));
    e.register(0x0071_d0a0, |e, a| {
        assert_eq!((a[0], a[1]), (CONSOLE, a[2]), "the line is its own format");
        let line = nvse::table::guest_text(e, a[1]);
        LINES.with(|l| l.borrow_mut().push(line));
        eax(0)
    });
    e.register(0x005e_1550, evaluate_line);
    e
}

fn run(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> f64 {
    let (ok, result) = call(e, opcode, args);
    assert_eq!(ok, 1, "{opcode:04x}");
    result
}

/// Runs `opcode` as a line of its own, through the stand-in `EvaluateLine`.
fn run_as_line(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> f64 {
    let words = line(e, opcode, args);
    let mut w = vec![execute_of(e, opcode)];
    w.extend_from_slice(&words);
    assert_eq!(e.call(0x005e_1550, &w).eax & 0xff, 1);
    e.mem.f64(RESULT)
}

fn lines() -> Vec<String> {
    LINES
        .with(|l| std::mem::take(&mut *l.borrow_mut()))
        .into_iter()
        .map(|l| String::from_utf8(l).unwrap())
        .collect()
}

/// The numbers: JIP's version is the double its DLL loads, 57.3 (`0x404CA66666666666`), 5730 as
/// an integer; xNVSE 6.4.9's full version as JIP computes it, 6.49 (`0x4019F5C28F5C28F6`).
#[test]
fn the_versions() {
    assert_eq!(version().to_bits(), 0x404c_a666_6666_6666);
    assert_eq!(version(), 57.3);
    assert_eq!(VERSION_INT, 5730);
    assert_eq!(nvse_version_full().to_bits(), 0x4019_f5c2_8f5c_28f6);
    assert_eq!(nvse_version_full(), 6.49);
}

// JIP answers "not installed" until the plan's step 3.10 turns the flag on (and this line off).
const _: () = assert!(!crate::ANSWERS_INSTALLED);

/// JIP's names: `JIP NVSE Plugin` and `lutana_nvse` without case, and every name beginning with
/// `JIP LN NVSE`; nothing else.
#[test]
fn jips_own_names() {
    for name in [
        "JIP LN NVSE",
        "jip ln nvse",
        "JIP LN NVSE Plugin",
        "JIP LN NVSE_whatever",
        "JIP NVSE Plugin",
        "jip nvse plugin",
        "lutana_nvse",
        "LUTANA_NVSE",
    ] {
        assert!(is_jip_alias(name.as_bytes()), "{name}");
    }
    for name in [
        "JIP LN NVS",
        "JIP NVSE Plugin2",
        "lutana",
        "lutana_nvse ",
        "JIP",
        "nvse",
        "",
        "JIP LN NVSE\u{e9}",
    ] {
        assert!(!is_jip_alias(name.as_bytes()), "{name}");
    }
    assert!(
        !is_jip_alias(b"JIP LN NVSE\xe9"),
        "a byte from 0x80 after the name"
    );
}

/// Not answering "installed" (as built today): `GetLNVersion` and `GetNVSEVersionFull` have no
/// handler, xNVSE's own `IsPluginInstalled` and `GetPluginVersion` stay in their records and
/// answer that JIP is not installed.
#[test]
fn not_installed_jip_is_not_there() {
    let mut e = setup(false);
    assert_eq!(execute_of(&e, GET_LN_VERSION), 0);
    assert_eq!(execute_of(&e, GET_NVSE_VERSION_FULL), 0);
    for opcode in [XNVSE_IS_PLUGIN_INSTALLED, XNVSE_GET_PLUGIN_VERSION] {
        assert_eq!(execute_of(&e, opcode), nvse::table::execute_address(opcode));
    }
    for name in ["JIP LN NVSE", "JIP NVSE Plugin", "lutana_nvse"] {
        assert_eq!(
            run(&mut e, XNVSE_IS_PLUGIN_INSTALLED, &[s(name.as_bytes())]),
            0.0
        );
        assert_eq!(
            run(&mut e, XNVSE_GET_PLUGIN_VERSION, &[s(name.as_bytes())]),
            -1.0
        );
    }
}

/// Answering "installed": `GetLNVersion` 57.3, `GetNVSEVersionFull` 6.49; JIP's
/// `IsPluginInstalled` 1 and `GetPluginVersion` 57.3 for its names, and for other names what
/// xNVSE's plugin list says (no other plugin is built in: the result as xNVSE left it, and -1).
#[test]
fn installed_jip_answers() {
    let mut e = setup(true);
    assert_eq!(run(&mut e, GET_LN_VERSION, &[]), 57.3);
    assert_eq!(run(&mut e, GET_NVSE_VERSION_FULL, &[]), 6.49);
    assert_eq!(
        execute_of(&e, XNVSE_IS_PLUGIN_INSTALLED),
        IS_PLUGIN_INSTALLED
    );
    assert_eq!(execute_of(&e, XNVSE_GET_PLUGIN_VERSION), GET_PLUGIN_VERSION);
    for name in [
        "JIP LN NVSE",
        "jip nvse plugin",
        "LUTANA_NVSE",
        "JIP LN NVSE v57",
    ] {
        assert_eq!(
            run(&mut e, XNVSE_IS_PLUGIN_INSTALLED, &[s(name.as_bytes())]),
            1.0
        );
        assert_eq!(
            run(&mut e, XNVSE_GET_PLUGIN_VERSION, &[s(name.as_bytes())]),
            57.3
        );
    }
    for name in ["JohnnyGuitar NVSE", "ShowOffNVSE", "nvse", ""] {
        assert_eq!(
            run(&mut e, XNVSE_IS_PLUGIN_INSTALLED, &[s(name.as_bytes())]),
            0.0
        );
        assert_eq!(
            run(&mut e, XNVSE_GET_PLUGIN_VERSION, &[s(name.as_bytes())]),
            -1.0
        );
    }
    // JIP's `IsPluginInstalled` leaves a result it does not set as it was.
    let words = line(&mut e, XNVSE_IS_PLUGIN_INSTALLED, &[s(b"none")]);
    e.mem.set_f64(RESULT, 7.0);
    e.call(IS_PLUGIN_INSTALLED, &words);
    assert_eq!(e.mem.f64(RESULT), 7.0);
}

/// A name of 128 bytes or more runs past JIP's buffer: without the list guard row the port
/// stops; with it the name is read whole. 127 bytes and the terminator fit.
#[test]
fn a_long_plugin_name() {
    let long = "x".repeat(0x80);
    let mut e = setup(true);
    for opcode in [XNVSE_IS_PLUGIN_INSTALLED, XNVSE_GET_PLUGIN_VERSION] {
        let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            call(&mut e, opcode, &[s(long.as_bytes())])
        }));
        assert!(stop.is_err(), "{opcode:04x}");
    }
    let mut e = setup(true);
    assert_eq!(
        run(
            &mut e,
            XNVSE_GET_PLUGIN_VERSION,
            &[s(&long.as_bytes()[1..])]
        ),
        -1.0,
        "127 bytes fit"
    );
    nvse::commands::array::set_fix(&mut e, LIST_GUARD, true);
    let jip_long = format!("JIP LN NVSE{}", "x".repeat(0x80));
    assert_eq!(
        run(&mut e, XNVSE_IS_PLUGIN_INSTALLED, &[s(jip_long.as_bytes())]),
        1.0
    );
    assert_eq!(
        run(&mut e, XNVSE_IS_PLUGIN_INSTALLED, &[s(long.as_bytes())]),
        0.0
    );
}

/// The console echo: a command run as a line of its own with the console open prints
/// `<name> >> <result>`; run another way (inside an expression), or with the console closed, it
/// prints nothing. `GetNVSEVersionFull` never prints.
#[test]
fn the_console_echo() {
    let mut e = setup(true);
    run_as_line(&mut e, GET_LN_VERSION, &[]);
    assert!(lines().is_empty(), "the console is closed");
    console(&mut e, true);
    assert_eq!(run_as_line(&mut e, GET_LN_VERSION, &[]), 57.3);
    run_as_line(&mut e, XNVSE_IS_PLUGIN_INSTALLED, &[s(b"JIP LN NVSE")]);
    run_as_line(&mut e, XNVSE_IS_PLUGIN_INSTALLED, &[s(b"other")]);
    run_as_line(&mut e, XNVSE_GET_PLUGIN_VERSION, &[s(b"lutana_nvse")]);
    run_as_line(&mut e, XNVSE_GET_PLUGIN_VERSION, &[s(b"other")]);
    run_as_line(&mut e, GET_NVSE_VERSION_FULL, &[]);
    assert_eq!(
        lines(),
        [
            "GetLNVersion >> 57.3",
            "IsPluginInstalled >> 1",
            "IsPluginInstalled >> 0",
            "GetPluginVersion >> 57.3",
            "GetPluginVersion >> -1",
        ]
    );
    // Called other than by `EvaluateLine`: nothing.
    run(&mut e, GET_LN_VERSION, &[]);
    run(&mut e, XNVSE_IS_PLUGIN_INSTALLED, &[s(b"JIP LN NVSE")]);
    assert!(lines().is_empty());
}
