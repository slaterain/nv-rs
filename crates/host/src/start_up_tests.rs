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

/// The log says when JIP is on with real threads, and when its row was on but JIP did not read
/// it; nothing when JIP is off as asked, or on with stepped threads.
#[test]
fn jip_start_lines_say_what_jip_is() {
    assert!(jip_start_lines(false, false, "real").is_empty());
    assert!(jip_start_lines(false, false, "stepped").is_empty());
    assert!(jip_start_lines(true, true, "stepped").is_empty());
    assert!(jip_start_lines(true, true, " Stepped ").is_empty());
    let real = jip_start_lines(true, true, "real");
    assert_eq!(real.len(), 1);
    assert!(real[0].contains("real threads"), "{real:?}");
    let unread = jip_start_lines(true, false, "stepped");
    assert_eq!(unread.len(), 1);
    assert!(unread[0].contains("not installed"), "{unread:?}");
}
