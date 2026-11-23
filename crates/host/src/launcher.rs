//! The game's launcher check (`004da570`): a graphics card other than the one `FalloutPrefs.ini`
//! names is named there, as the game's launcher names it, in every preset: only the name (the
//! launcher's other work for a new card, a preset and a size, is not done: the profile's stay).
//! **Not 1:1 with FalloutNV.exe**: the original ends its process and starts the launcher.
//!
//! **Why.** `main` (`0086a850`) asks `004da570` whether the first adapter's name differs from
//! `sD3DDevice:Display` (a new or renamed card, or no such line in `FalloutPrefs.ini`). On "it
//! differs" it releases its renderer (`Renderer::Kill(1)`, `004da4d0`), stops the tasklet
//! manager's workers (`BSTaskletManager` slot 2, `00b01f50`: its quit flag set, every worker woken
//! and waited for) and starts `FalloutNVLauncher.exe` with `ShellExecuteA`. On any answer but 0
//! `main` returns and the process ends; Windows answers 0 only when it is out of memory (a missing
//! file answers 2), so the original never goes on from there. The launcher names the card in
//! `FalloutPrefs.ini`, and the next start's check passes. nv-rs starts no other program: its
//! `ShellExecuteA` answered 0, `main` went on with the workers stopped, and the first cell's
//! navigation mesh update (`NavMeshObstacleManager::ForceUpdate`, `006c39c0`, which spins until
//! the workers' tasks are done) waited for good: New Game stayed on its loading screen after the
//! intro (issue #935, a run started without Play.exe or play.sh, which name the card at every
//! start).
//!
//! **What this does.** The check's translation runs first. When it finds the names differ, the
//! first adapter's name (what the launcher writes) is set in `sD3DDevice:Display` and the
//! translation asks again: it now finds the same card, rewrites the setting with its quotes as at
//! a start after the launcher, and answers "the same card", so `main` keeps its renderer and its
//! workers. The log names both names once. Only the name changes, as Play.exe and play.sh change
//! only the name, in the game's setting, as a start after the launcher would read it from the
//! file. `main` calls the check by address (`e.call`), so the entry registered here runs; it
//! is a row of `crates/engine-core/data/overridable.tsv`.

use engine::prelude::*;
use engine::units::fallout_shared::renderer::{fn_004da570, fn_004da650};
use std::sync::atomic::{AtomicBool, Ordering};

/// The launcher check: true when the first adapter's name is not `sD3DDevice:Display`'s.
pub const LAUNCHER_CHECK: u32 = 0x004d_a570;
/// `sD3DDevice:Display`, a string setting.
const S_D3D_DEVICE: u32 = 0x011c_72c0;
/// `004dcdb0(index)`: an adapter's description (the check asks for index 0).
const ADAPTER_DESCRIPTION: u32 = 0x004d_cdb0;
/// `00403df0(setting)`: a string setting's text.
const STRING_TEXT: u32 = 0x0040_3df0;
/// `005e0190(setting, text)`: sets a string setting to a copy of `text`.
const STRING_SETTING_SET: u32 = 0x005e_0190;

/// Registers the entry over the translation, in every preset.
pub fn install(e: &mut Engine) {
    e.register(LAUNCHER_CHECK, |e, _| {
        u32::from(launcher_check(e, fn_004da570, say)).into_ret()
    });
}

/// The check by `check` (the translation); when the names differ, the card named in the setting
/// and the check made again ([`name_the_card`]). `log` gets the line saying so. False: `main`
/// keeps its renderer and its workers.
fn launcher_check(
    e: &mut Engine,
    mut check: impl FnMut(&mut Engine) -> bool,
    log: impl FnOnce(String),
) -> bool {
    if !check(e) {
        return false;
    }
    let (saved, card) = name_the_card(e);
    let again = check(e);
    log(format!(
        "The game asked for its launcher: sD3DDevice names \"{saved}\", the graphics card is \
         \"{card}\"; the card's name taken, as the launcher writes it{}, the game goes on",
        if again {
            " (the check still found them different: gone on all the same)"
        } else {
            ""
        }
    ));
    false
}

/// Sets `sD3DDevice:Display` to the first adapter's name, as the launcher writes it; returns the
/// setting's text before and the name.
fn name_the_card(e: &mut Engine) -> (String, String) {
    let text = e.call(STRING_TEXT, &args![S_D3D_DEVICE]).u32();
    let saved = String::from_utf8_lossy(&e.mem.cstr(text)).into_owned();
    let adapter = e.call(ADAPTER_DESCRIPTION, &args![0u32]).u32();
    let name = fn_004da650(e, Ptr::new(adapter));
    let card = String::from_utf8_lossy(&e.mem.cstr(name.addr())).into_owned();
    e.call(STRING_SETTING_SET, &args![S_D3D_DEVICE, name.addr()]);
    (saved, card)
}

/// The log line, once a run.
fn say(line: String) {
    static SAID: AtomicBool = AtomicBool::new(false);
    if !SAID.swap(true, Ordering::Relaxed) {
        eprintln!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// The adapter list (`004dce30`): non-zero when there is one.
    const ADAPTER_LIST: u32 = 0x004d_ce30;
    /// `00717e50(adapter)`: the adapter's second name; the check does not read it.
    const ADAPTER_SECOND_NAME: u32 = 0x0071_7e50;
    const STRICMP: u32 = 0x00ec_68e4;
    const STRING_COPY: u32 = 0x0040_46f0;
    const STRCAT: u32 = 0x00ec_6380;
    /// The text the check puts around the name: the exe's `"` at `01021184`.
    const DELIMITER: u32 = 0x0102_1184;
    /// Where a string setting keeps its text.
    const VALUE: u32 = 4;

    /// The check's callees as the game's: a first adapter named `card` (none for `None`), the
    /// setting's text `saved`, and the C library's string calls.
    fn engine(card: Option<&str>, saved: &str) -> Engine {
        let mut e = Engine::new();
        e.mem.map(S_D3D_DEVICE & !0xfff, 0x1000);
        e.mem.map(DELIMITER & !0xfff, 0x1000);
        e.mem.set_cstr(DELIMITER, b"\"");
        let adapter = match card {
            Some(name) => {
                let adapter = e.mem.alloc(0x300);
                e.mem.set_cstr(adapter + 0x204, name.as_bytes());
                adapter
            }
            None => 0,
        };
        let text = e.mem.alloc(0x100);
        e.mem.set_cstr(text, saved.as_bytes());
        e.mem.set_u32(S_D3D_DEVICE + VALUE, text);
        e.register(ADAPTER_LIST, |_, _| 1u32.into_ret());
        e.register_double(ADAPTER_DESCRIPTION, move |_, a| {
            assert_eq!(a[0], 0, "the first adapter");
            adapter.into_ret()
        });
        e.register(ADAPTER_SECOND_NAME, |_, _| 0u32.into_ret());
        e.register(STRING_TEXT, |e, a| e.mem.u32(a[0] + VALUE).into_ret());
        e.register(STRING_SETTING_SET, |e, a| {
            let text = e.mem.cstr(a[1]);
            let copy = e.mem.alloc(text.len() as u32 + 1);
            e.mem.set_cstr(copy, &text);
            e.mem.set_u32(a[0] + VALUE, copy);
            a[0].into_ret()
        });
        e.register(STRICMP, |e, a| {
            let left = e.mem.cstr(a[0]).to_ascii_lowercase();
            let right = e.mem.cstr(a[1]).to_ascii_lowercase();
            (left.cmp(&right) as i32).into_ret()
        });
        e.register(STRING_COPY, |e, a| {
            let text = e.mem.cstr(a[1]);
            e.mem.set_cstr(a[0], &text);
            a[0].into_ret()
        });
        e.register(STRCAT, |e, a| {
            let mut text = e.mem.cstr(a[0]);
            text.extend(e.mem.cstr(a[1]));
            e.mem.set_cstr(a[0], &text);
            a[0].into_ret()
        });
        e
    }

    /// The setting's text now.
    fn setting(e: &Engine) -> String {
        String::from_utf8_lossy(&e.mem.cstr(e.mem.u32(S_D3D_DEVICE + VALUE))).into_owned()
    }

    /// The installed entry's answer and the lines it said.
    fn ask(e: &mut Engine) -> (u32, Vec<String>) {
        let lines = Rc::new(RefCell::new(Vec::new()));
        let said = Rc::clone(&lines);
        e.register_double(LAUNCHER_CHECK, move |e, _| {
            let said = Rc::clone(&said);
            u32::from(launcher_check(e, fn_004da570, move |line| {
                said.borrow_mut().push(line)
            }))
            .into_ret()
        });
        let answer = e.call(LAUNCHER_CHECK, &args![]).u32();
        let lines = lines.borrow().clone();
        (answer, lines)
    }

    /// Issue #935: a saved name that is not the card's is answered "the same card" (so `main`
    /// neither releases its renderer nor stops its workers), with the card's name in the setting
    /// between the quotes the check writes, as at a start after the launcher; the log names both.
    #[test]
    fn another_card_is_named_and_the_game_goes_on() {
        let mut e = engine(
            Some("NVIDIA GeForce GTX 1650 Ti"),
            "nv-rs test: deliberately mismatched adapter",
        );
        // The translation alone asks for the launcher.
        assert!(fn_004da570(&mut e));
        assert_eq!(setting(&e), "nv-rs test: deliberately mismatched adapter");
        let (answer, lines) = ask(&mut e);
        assert_eq!(answer, 0);
        assert_eq!(setting(&e), "\"NVIDIA GeForce GTX 1650 Ti\"");
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("\"nv-rs test: deliberately mismatched adapter\""));
        assert!(lines[0].contains("\"NVIDIA GeForce GTX 1650 Ti\""));
        assert!(!lines[0].contains("still found"), "{}", lines[0]);
        // The quotes are for the file (the game's settings write puts `sD3DDevice="..."` there):
        // the next start reads the name without them, finds the same card and says nothing.
        let read_back = e.mem.alloc(0x40);
        e.mem.set_cstr(read_back, b"NVIDIA GeForce GTX 1650 Ti");
        e.mem.set_u32(S_D3D_DEVICE + VALUE, read_back);
        let (answer, lines) = ask(&mut e);
        assert_eq!((answer, lines.len()), (0, 0));
        assert_eq!(setting(&e), "\"NVIDIA GeForce GTX 1650 Ti\"");
    }

    /// No `sD3DDevice` line (an empty text) is another card too.
    #[test]
    fn no_saved_name_is_named_too() {
        let mut e = engine(Some("AMD Radeon RX 7800 XT"), "");
        let (answer, lines) = ask(&mut e);
        assert_eq!(answer, 0);
        assert_eq!(setting(&e), "\"AMD Radeon RX 7800 XT\"");
        assert_eq!(lines.len(), 1);
    }

    /// The same card (the check ignores case) and no adapter are the translation's own: "the
    /// same card", the setting rewritten with its quotes, nothing said.
    #[test]
    fn the_same_card_and_no_card_are_the_checks_own() {
        let mut e = engine(Some("NVIDIA GeForce"), "nvidia geforce");
        assert_eq!(ask(&mut e), (0, vec![]));
        assert_eq!(setting(&e), "\"nvidia geforce\"");
        let mut e = engine(None, "anything");
        assert_eq!(ask(&mut e), (0, vec![]));
        assert_eq!(setting(&e), "\"anything\"");
    }

    /// A check that still finds them different after the name is set (it never should) is
    /// answered "the same card" all the same, and the log says so: the game never waits.
    #[test]
    fn a_check_that_still_differs_goes_on() {
        let mut e = engine(Some("Card"), "Old card");
        let mut lines = Vec::new();
        let mut asked = 0;
        let answer = launcher_check(
            &mut e,
            |_| {
                asked += 1;
                true
            },
            |line| lines.push(line),
        );
        assert!(!answer);
        assert_eq!(asked, 2);
        assert_eq!(setting(&e), "Card");
        assert!(lines[0].contains("still found"), "{lines:?}");
    }

    /// `install` puts the entry at the check's address, the one `main` calls.
    #[test]
    fn the_entry_is_at_the_checks_address() {
        let mut e = engine(Some("Card"), "Old card");
        install(&mut e);
        assert_eq!(e.call(LAUNCHER_CHECK, &args![]).u32(), 0);
        assert_eq!(setting(&e), "\"Card\"");
    }
}
