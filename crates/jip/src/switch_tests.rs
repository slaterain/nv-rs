//! Step 3.10's switch: the host's row `NV_JIP_INSTALLED` ([`crate::set_answers_installed`]),
//! read by [`crate::install`]. Off (every preset), JIP is put in exactly as before the row
//! existed (`install_as(e, false)`); on, as `install_as(e, true)`.

use crate::test_support::engine;
use crate::{install, install_as, set_answers_installed, Jip, DEFAULT_ANSWERS_INSTALLED};
use engine_core::extend::{jip_patches, Owner};
use engine_core::prelude::*;

/// The guest pages JIP writes vtable slots in once installed (`test_support`'s mapped pages:
/// the processes', `ExtraTimeLeft`'s and the queued references' vtables).
const VTABLE_PAGES: [(u32, u32); 3] = [
    (0x0108_7000, 0x4000),
    (0x0101_5000, 0x1000),
    (0x0101_6000, 0x1000),
];

/// What JIP's install leaves behind that a run can see.
#[derive(Debug, PartialEq)]
struct Snapshot {
    /// Every JIP opcode's record (10 words: name, ..., execute, ..., eval) and whether a handler
    /// is registered at its execute and eval addresses.
    records: Vec<(u32, Vec<u32>, bool, bool)>,
    /// The owners of every seam and wrap row in the engine's tables.
    sites: Vec<(u32, Vec<Owner>)>,
    wraps: Vec<(u32, Vec<Owner>)>,
    /// The game patches' word.
    patches: u32,
    /// The vtable pages' bytes.
    vtables: Vec<Vec<u8>>,
    /// xNVSE's built-ins: file, name, version, listed; the per-frame message wanted.
    plugins: Vec<(&'static str, &'static str, u32, bool)>,
    per_frame: bool,
    /// The names `IsPluginInstalled` finds.
    loaded: Vec<&'static str>,
    answers_installed: bool,
}

fn snapshot(e: &mut Engine) -> Snapshot {
    let records = (crate::table::FIRST_OPCODE..=crate::table::LAST_OPCODE)
        .map(|op| {
            let at = nvse::table::record_of(e, op);
            let words = (0..10).map(|i| e.mem.u32(at + 4 * i)).collect();
            (
                op,
                words,
                e.is_registered(nvse::table::execute_address(op)),
                e.is_registered(nvse::table::eval_address(op)),
            )
        })
        .collect();
    let sites = engine_core::extend::all_sites()
        .map(|r| (r.addr, e.site_owners(r.addr)))
        .collect();
    let wraps = engine_core::extend::all_wraps()
        .map(|r| (r.addr, e.wraps_at(r.addr)))
        .collect();
    let vtables = VTABLE_PAGES
        .iter()
        .map(|&(at, len)| (0..len).map(|i| e.mem.u8(at + i)).collect())
        .collect();
    let plugins = nvse::plugins::built_ins(e)
        .iter()
        .map(|p| (p.file, p.name, p.version, p.listed))
        .collect();
    let per_frame = e
        .ext_get::<nvse::Nvse>()
        .is_some_and(|n| n.plugins.per_frame);
    let loaded = nvse::commands::extender::loaded_plugins(e)
        .iter()
        .map(|p| p.name)
        .collect();
    Snapshot {
        records,
        sites,
        wraps,
        patches: e.jip_patches(),
        vtables,
        plugins,
        per_frame,
        loaded,
        answers_installed: e.ext::<Jip>().answers_installed,
    }
}

/// JIP put in by `put_in` on a fresh test engine.
fn installed_by(put_in: impl FnOnce(&mut Engine)) -> Snapshot {
    let mut e = engine();
    put_in(&mut e);
    snapshot(&mut e)
}

/// "Not installed" unless a run switches it on: the default, and a fresh JIP state.
#[test]
fn the_default_is_not_installed() {
    // (`DEFAULT_ANSWERS_INSTALLED` is held false by a `const` assert in lib.rs.)
    assert!(!Jip::default().row_on);
    assert_eq!(Jip::default().row_on, DEFAULT_ANSWERS_INSTALLED);
    // A fresh engine's state, as the host has it before its row runs.
    let mut e = engine();
    assert_eq!(e.ext::<Jip>().row_on, DEFAULT_ANSWERS_INSTALLED);
}

/// The row off (never set, or set off): exactly the path every run took before the row, JIP
/// answering "not installed": the same records, handlers, seams, wraps, vtable slots, plugin
/// list; the patch word 0; JIP not among the loaded plugins; no per-frame message.
#[test]
fn row_off_is_the_path_before_the_row() {
    let before = installed_by(|e| install_as(e, false));
    let never_set = installed_by(install);
    let set_off = installed_by(|e| {
        set_answers_installed(e, false);
        install(e);
    });
    assert_eq!(never_set, before);
    assert_eq!(set_off, before);
    assert_eq!(before.patches, 0);
    assert!(!before.answers_installed);
    assert!(!before.per_frame);
    assert!(!before.loaded.contains(&"JIP LN NVSE"));
    assert!(before
        .plugins
        .iter()
        .any(|&(file, _, _, listed)| file == "jip_nvse.dll" && !listed));
    // No built command answers: no execute, no eval, nothing registered (`commands::tests`
    // checks every one, names too).
    let rec = before.records.iter().find(|r| r.0 == 0x266b).unwrap();
    assert_eq!((rec.1[6], rec.1[8], rec.2, rec.3), (0, 0, false, false));
}

/// The row on: exactly `install_as(e, true)`: JIP listed with its version, its built commands'
/// handlers, the `INSTALLED` bit of the patch word, the per-frame message.
#[test]
fn row_on_is_the_installed_path() {
    let installed = installed_by(|e| install_as(e, true));
    let row_on = installed_by(|e| {
        set_answers_installed(e, true);
        install(e);
    });
    assert_eq!(row_on, installed);
    assert!(row_on.answers_installed);
    assert!(row_on.per_frame);
    assert!(row_on.loaded.contains(&"JIP LN NVSE"));
    assert!(row_on
        .plugins
        .iter()
        .any(|&(file, _, version, listed)| file == "jip_nvse.dll"
            && listed
            && version == crate::identity::VERSION_INT));
    assert_ne!(row_on.patches & jip_patches::INSTALLED, 0);
    let rec = row_on.records.iter().find(|r| r.0 == 0x266b).unwrap();
    assert_eq!(rec.1[6], nvse::table::execute_address(0x266b));
    assert!(rec.2);
    // And it is not the "not installed" path.
    assert_ne!(row_on, installed_by(|e| install_as(e, false)));
}

/// The row is read once, at install: set after it, it changes nothing.
#[test]
fn the_row_counts_only_before_install() {
    let mut e = engine();
    install(&mut e);
    set_answers_installed(&mut e, true);
    install(&mut e);
    assert_eq!(snapshot(&mut e), installed_by(|e| install_as(e, false)));
}
