//! For the engine crates' tests of the join points (not for play).

use super::*;

/// The text of the stand-in join's stop ([`joins_first`]).
pub const JOINED: &str = "draw_thread::testing: the join ran";

fn stop_at_join(_: &mut Engine) {
    std::panic::panic_any(JOINED.to_string());
}

fn no_reason(_: &Engine) -> Option<&'static str> {
    None
}

fn no_cut(_: &mut Engine, _: u32, _: u32, _: bool) -> Option<Packet> {
    None
}

fn give_back(_: &mut Engine, p: Packet) -> Result<(), Packet> {
    Err(p)
}

fn nothing_back(_: &mut Engine) -> Option<Packet> {
    None
}

fn drop_packet(_: &mut Engine, _: Packet) {}

fn no_device(_: &mut Engine, _: u32) -> bool {
    false
}

fn nothing(_: &mut Engine) {}

/// Hooks that never cut, with `device_finish` as given.
pub fn hooks_finishing_with(device_finish: fn(&mut Engine)) -> Hooks {
    Hooks {
        serial: no_reason,
        cut: no_cut,
        adopt: give_back,
        hand_back: nothing_back,
        join: drop_packet,
        device_cut: no_device,
        device_close: nothing,
        device_finish,
    }
}

/// Marks `e`'s run (turned on) as having a job posted, with no job: the next join point calls the run's
/// `device_finish` and nothing else.
pub fn mark_posted(e: &Engine) {
    let marked = with_run(e.kernel.id(), |r| {
        if r.pending.is_none() {
            r.pending = Some(Arc::new(Mutex::new(Slot::default())));
            true
        } else {
            false
        }
    });
    if marked == Some(true) {
        POSTED.fetch_add(1, Relaxed);
    }
}

/// Whether calling `addr` with `args` joins the drawing job before anything else it does: the run turned on with a
/// join that stops, a job marked posted, then the call, which must stop with that join's text (a function that read
/// or called anything first would stop otherwise, or return). The run is turned off after.
pub fn joins_first(e: &mut Engine, addr: u32, args: &[u32]) -> bool {
    turn_on(e, hooks_finishing_with(stop_at_join));
    mark_posted(e);
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        e.call(addr, args);
    }));
    e.call_stack.clear();
    // A join that never ran leaves the mark: taken away here.
    if with_run(e.kernel.id(), |r| r.pending.take())
        .flatten()
        .is_some()
    {
        POSTED.fetch_sub(1, Relaxed);
    }
    turn_off(e);
    match out {
        Ok(()) => false,
        Err(payload) => payload
            .downcast_ref::<String>()
            .is_some_and(|s| s == JOINED),
    }
}

/// Whether `e`'s run is in a save's or a load's serial frames (`None`: the run is off).
pub fn save_or_load_state(e: &Engine) -> Option<bool> {
    with_run(e.kernel.id(), |r| r.save_or_load)
}

/// Sets `e`'s run's save-or-load state (the run must be on).
pub fn set_save_or_load_state(e: &Engine, on: bool) {
    with_run(e.kernel.id(), |r| r.save_or_load = on);
}

/// [`joins_first`] for a save's or a load's entry: it joins first, and the run is left in its save-or-load state.
pub fn joins_first_into_save_or_load(e: &mut Engine, addr: u32, args: &[u32]) -> bool {
    joins_first_then(
        e,
        addr,
        args,
        |e| save_or_load_state(e) == Some(true),
        |_| {},
    )
}

/// [`joins_first`] for OnIdle's entry: it joins first, and a save's or a load's serial frames end there.
pub fn joins_first_ending_save_or_load(e: &mut Engine, addr: u32, args: &[u32]) -> bool {
    joins_first_then(
        e,
        addr,
        args,
        |e| save_or_load_state(e) == Some(false),
        |e| set_save_or_load_state(e, true),
    )
}

/// [`joins_first`], then the call again with a join that does not stop (whatever the function does after it may
/// stop: caught), `prepare` run on the turned-on run first, and `check` asked of the run after.
fn joins_first_then(
    e: &mut Engine,
    addr: u32,
    args: &[u32],
    check: impl FnOnce(&Engine) -> bool,
    prepare: impl FnOnce(&Engine),
) -> bool {
    if !joins_first(e, addr, args) {
        return false;
    }
    turn_on(e, hooks_finishing_with(nothing));
    prepare(e);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        e.call(addr, args);
    }));
    e.call_stack.clear();
    let held = check(e);
    turn_off(e);
    held
}
