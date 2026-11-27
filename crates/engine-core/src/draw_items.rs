//! Kept draw records drawn as items (`NV_DRAW_ITEMS`, port-added, off by default): the hand-over
//! between the typed draw code (`engine_typed::draw::keep`, which decides per draw) and the
//! platform layer (`engine_platform::d3d9real`, which turns the draw's own device calls into the
//! stream's `RecordNext` / `DrawItem` commands, `d3d9_stream::Kept`).
//!
//! The typed draw code brackets a draw's set-up (or its kept record's replay) with [`begin`] and
//! [`end`]. In between, the platform layer holds back the commands of the device calls the draw
//! makes itself (those `crate::real_threads`' recorder notes, or a record's replay makes again:
//! [`crate::real_threads::in_noted_call`]) in runs: a setter call ([`interrupt`]) or a command of
//! any other call ends a run, which goes out as it came, so nothing is ever reordered. The run that
//! ends with the draw is sent as the [`Ask`] says. Without the platform's hooks (tests, scripts) nothing is held:
//! [`end`] answers [`Done::Plain`].
//!
//! The engineers' switches (players see only `sDrawItems`, the `Fallout.ini` row of `NV_DRAW_ITEMS`):
//! - `NV_DRAW_ITEMS=1` / `check` (off by default): the world pass's plain lit geometries (the shadow light shader's
//!   standard set-up, unskinned) drawn from kept records while nothing their set-ups read changed; `check` draws
//!   every one as always and beside it the item it would have been (both forms in one recording; the set-up's work
//!   compared with the record's). Every million draws a line `NV_DRAW_ITEMS: ...` gives the draws replayed, built
//!   and why, resting, checked and differed (with the first difference), the items, and the microseconds a build,
//!   a replay and a plain draw took. The backend's side (`d3d9_stream::Kept`) prints `NV_DRAW_ITEMS backend: ...`
//!   every 1,000 frames and whenever an item found no record, the wrong registers, or a record was cut: those three
//!   must stay 0. Not used while the drawing thread runs. A check run keeps every state set again to its value
//!   (as `NV_D3D9_REPEATS=keep`), so the old form after an item is complete.
//! - `NV_DRAW_ITEMS_AUDIT=1` with `check` (stepped threads only, slow): every 64th build also lists the guest reads
//!   none of its record's inputs, writes, device structures, own bytes or locals covers (`uncovered` in the line).
//! - `NV_REPLAY_ITEMS=items` for `d3d9-replay`, `NV_DIGESTS_ITEMS=1` for `replay_digests`: a check recording
//!   replayed as its items instead of its old form (`record_compare --items` compares the two).

use crate::Engine;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

/// `NV_DRAW_ITEMS`: how the world's still lit draws are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// As the original draws them (the default): nothing of this module runs.
    Off,
    /// Each draw as the original draws it, and beside each that could have been drawn from its
    /// kept record the item it would have been: one recording holds both forms
    /// (`d3d9_stream::kept::Form`), and the records' replays are checked against the set-ups.
    Check,
    /// From kept records while the bytes their set-ups read stay the same.
    On,
}

/// 0 not set yet, else the mode + 1.
static MODE: AtomicU8 = AtomicU8::new(0);

/// The run's mode: [`set_mode`]'s (the host's choice: `NV_DRAW_ITEMS`, else `Fallout.ini`'s
/// `sDrawItems`), else `NV_DRAW_ITEMS` read once (`1` on, `check`; anything else off).
#[inline]
pub fn mode() -> Mode {
    match MODE.load(Ordering::Relaxed) {
        1 => Mode::Off,
        2 => Mode::Check,
        3 => Mode::On,
        _ => {
            let m = parse(std::env::var("NV_DRAW_ITEMS").ok().as_deref());
            let _ = MODE.compare_exchange(0, encode(m), Ordering::Relaxed, Ordering::Relaxed);
            mode()
        }
    }
}

/// A mode's word (`NV_DRAW_ITEMS`, `sDrawItems`): `1` or `on`, `check`; anything else off.
pub fn parse(word: Option<&str>) -> Mode {
    match word.map(str::trim) {
        Some("1" | "on") => Mode::On,
        Some("check") => Mode::Check,
        _ => Mode::Off,
    }
}

fn encode(m: Mode) -> u8 {
    match m {
        Mode::Off => 1,
        Mode::Check => 2,
        Mode::On => 3,
    }
}

/// The run's mode from now on (the host, before the game's device is made).
pub fn set_mode(m: Mode) {
    MODE.store(encode(m), Ordering::Relaxed);
}

/// What to do with a draw's own commands ([`begin`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// Send them after a `RecordNext` of record `id` (record `forget` dropped, 0: none): the
    /// backend carries them out and keeps them. `check`: they are a check run's old form (its item
    /// was sent just before).
    Record { id: u32, forget: u32, check: bool },
    /// Send one `DrawItem` of record `id` in their place, with their registers, when they are the
    /// record's (`hash`, [`Done::Recorded`]'s). `check`: a check run's item, made beside the old form
    /// that follows: when it cannot be made the commands are dropped, not sent (the old form draws).
    Item { id: u32, hash: u64, check: bool },
}

/// What came of a [`begin`]'s draw ([`end`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Done {
    /// The record was sent ([`Ask::Record`]): its commands' hash (not counting the registers).
    Recorded(u64),
    /// The item was sent ([`Ask::Item`]).
    Drawn,
    /// Neither: the commands went out as they came (or, for a check item, were dropped), or there
    /// were none.
    Plain,
    /// [`Ask::Item`] only: the draw's commands were not its record's (another hash): sent as they
    /// came (dropped for a check item).
    Other,
}

/// The platform layer's side ([`install`]).
pub struct Hooks {
    pub begin: fn(&mut Engine, Ask),
    pub end: fn(&mut Engine) -> Done,
    /// A setter call of the draw's set-up or of its record's replay ([`interrupt`]).
    pub interrupt: fn(&mut Engine),
    /// Records dropped (`ForgetRecords`).
    pub forget: fn(&mut Engine, &[u32]),
    /// Bumped whenever the backend's records are gone (a device made or reset): the typed draw
    /// code then drops its record numbers.
    pub epoch: fn() -> u64,
}

static HOOKS: OnceLock<&'static Hooks> = OnceLock::new();

/// The platform layer's hooks for every engine of the run (the first call wins).
pub fn install(hooks: &'static Hooks) {
    let _ = HOOKS.set(hooks);
}

/// `f` with this thread's device call recorder paused: a hook reaches the device's state outside any noted call
/// (it sends what it held), which is no call of the draw the recorder could have missed
/// ([`crate::real_threads::note_device_state`]).
fn recorder_paused<R>(f: impl FnOnce() -> R) -> R {
    struct Resume;
    impl Drop for Resume {
        fn drop(&mut self) {
            crate::real_threads::device_record_pause(false);
        }
    }
    crate::real_threads::device_record_pause(true);
    let _resume = Resume;
    f()
}

/// The draw's own commands from here are held for `ask` ([`end`]).
pub fn begin(e: &mut Engine, ask: Ask) {
    if let Some(h) = HOOKS.get() {
        recorder_paused(|| (h.begin)(e, ask));
    }
}

/// The draw is done: what came of its [`begin`].
pub fn end(e: &mut Engine) -> Done {
    HOOKS
        .get()
        .map_or(Done::Plain, |h| recorder_paused(|| (h.end)(e)))
}

/// A setter call of the draw's set-up (at the build) or of its record's replay: the run of the draw's own commands
/// held so far goes out as it came, so a record and its items begin at the same place whatever the setter sends.
#[inline]
pub fn interrupt(e: &mut Engine) {
    if let Some(h) = HOOKS.get() {
        recorder_paused(|| (h.interrupt)(e));
    }
}

/// Records `ids` dropped in the backend.
pub fn forget(e: &mut Engine, ids: &[u32]) {
    if let Some(h) = HOOKS.get() {
        recorder_paused(|| (h.forget)(e, ids));
    }
}

/// The backend's records' epoch ([`Hooks::epoch`]); 0 without the hooks.
pub fn epoch() -> u64 {
    HOOKS.get().map_or(0, |h| (h.epoch)())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_name_the_modes() {
        assert_eq!(parse(Some("1")), Mode::On);
        assert_eq!(parse(Some(" on ")), Mode::On);
        assert_eq!(parse(Some("check")), Mode::Check);
        for off in [None, Some("0"), Some(""), Some("yes"), Some("off")] {
            assert_eq!(parse(off), Mode::Off, "{off:?}");
        }
    }
}
