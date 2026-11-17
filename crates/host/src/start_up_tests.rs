//! The start-up's order of the rows and the extenders, and its log lines on JIP.

use super::*;

/// The rows go in before xNVSE, xNVSE before JIP, JIP before UIO: `NV_JIP_INSTALLED` is a flag
/// `jip::install` reads, and the extenders' wraps go over the rows. A textual check of
/// [`install_rows_and_extenders`] (running it needs FalloutNV.exe's tables), and
/// `Host::start_with` calls it.
#[test]
fn the_rows_go_in_before_the_extenders() {
    let text = include_str!("lib.rs");
    let body = text
        .split("pub(crate) fn install_rows_and_extenders(")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("install_rows_and_extenders in lib.rs");
    let at = |call: &str| {
        body.find(call)
            .unwrap_or_else(|| panic!("{call} in install_rows_and_extenders"))
    };
    let order = [
        at("fixes::install(engine, choice)"),
        at("nvse::install(engine)"),
        at("jip::install(engine)"),
        at("mcm::install(engine)"),
        at("uio::install(engine)"),
    ];
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
    let start_with = text
        .split("pub fn start_with(")
        .nth(1)
        .expect("Host::start_with");
    assert!(start_with.contains("install_rows_and_extenders(&mut engine, choice);"));
    for call in ["fixes::install(", "nvse::install(", "jip::install("] {
        assert!(
            !start_with
                .split("install_rows_and_extenders(&mut engine, choice);")
                .next()
                .unwrap()
                .contains(call),
            "{call} before the ordered start-up"
        );
    }
}

/// The log says when JIP's row was on but JIP did not read it; nothing when JIP is off as asked
/// or on as asked (with real threads too: JIP's own data no longer makes a thread wait; a mod's
/// event script run from another thread still waits for the main thread a moment, which
/// docs/FIXES.md and the INI row say, not the log).
#[test]
fn jip_start_lines_say_what_jip_is() {
    assert!(jip_start_lines(false, false).is_empty());
    assert!(jip_start_lines(true, true).is_empty());
    let unread = jip_start_lines(true, false);
    assert_eq!(unread.len(), 1);
    assert!(unread[0].contains("not installed"), "{unread:?}");
}
