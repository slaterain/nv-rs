//! Kept draw records as the stream's items (`NV_DRAW_ITEMS`, off by default;
//! `engine_core::draw_items`): between the typed draw code's `begin` and `end` of a draw, the
//! commands of the draw's own device calls (`engine_core::real_threads::in_noted_call`) are held
//! here in runs. A setter call (`engine_core::draw_items::interrupt`, at each of the record's
//! setter calls, at the build and at the replay alike) or a command of any other call (a sync
//! command too) ends a run: what it held goes out as it came, so nothing is reordered. The run
//! that ends with the draw is sent as the typed code asked: after a `RecordNext` (the backend
//! carries it out and keeps it: `d3d9_stream::Kept`), or as one `DrawItem` of the record it is
//! again (the same shape: every command but the registers the same; the registers come with the
//! item). A replay's run of another shape goes out as it came. A check run's item (made from the
//! record's replay before the old form that follows it) never sends its draw: its last run is the
//! item or nothing, and its own commands after it are dropped, since the old form draws. A run
//! longer than the backend keeps (`d3d9_stream::kept::MOST_COMMANDS`) goes out as it came too, so
//! the backend never holds a cut record the game's side thinks it has.
//!
//! The hooks run in the typed draw code between device calls, where with real threads the device
//! lock may not be held (the world draw's hold is given up whenever another thread waits for it):
//! each takes it (`engine_core::real_threads::device_guard`) before it reaches the platform's state
//! or the stream, as every such hook of the port does.
//!
//! Off, nothing here runs: [`submit`] is reached only while [`HOLDING`] is set (one thread-local
//! test in `d3d9real`'s sink), and the backend is not wrapped (`d3d9real::backend_of`).

use super::d3d9real::Sink;
use d3d9_stream::kept::Commands;
use d3d9_stream::{Cmd, Data, Reply};
use engine_core::draw_items::{Ask, Done, Hooks};
use engine_core::Engine;
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicU64, Ordering};

/// One draw's held commands.
struct Held {
    ask: Ask,
    /// The device the item and the record are sent for.
    device: u32,
    /// The run being held, and its registers.
    cmds: Commands,
    registers: Vec<u8>,
    done: Option<Done>,
}

impl Held {
    fn check_item(&self) -> bool {
        matches!(self.ask, Ask::Item { check: true, .. })
    }
}

thread_local! {
    static HELD: RefCell<Option<Held>> = const { RefCell::new(None) };
    /// True between `begin` and `end` (the sink's one test).
    pub(super) static HOLDING: Cell<bool> = const { Cell::new(false) };
}

/// The backend's records' epoch: bumped at every device made or reset ([`bump_epoch`]).
static EPOCH: AtomicU64 = AtomicU64::new(0);

/// A device was made or reset: the backend's records are gone (`d3d9_stream::Kept` drops them).
pub(super) fn bump_epoch() {
    EPOCH.fetch_add(1, Ordering::Relaxed);
}

fn epoch() -> u64 {
    EPOCH.load(Ordering::Relaxed)
}

fn begin(e: &mut Engine, ask: Ask) {
    let _device = engine_core::real_threads::device_guard(e);
    let device = super::d3d9real::first_device(e).unwrap_or(0);
    HELD.with(|h| {
        *h.borrow_mut() = Some(Held {
            ask,
            device,
            cmds: Commands::default(),
            registers: Vec::new(),
            done: None,
        });
    });
    HOLDING.with(|h| h.set(device != 0));
}

fn end(e: &mut Engine) -> Done {
    let _device = engine_core::real_threads::device_guard(e);
    HOLDING.with(|h| h.set(false));
    let Some(mut held) = HELD.with(|h| h.borrow_mut().take()) else {
        return Done::Plain;
    };
    if let Some(done) = held.done {
        return done;
    }
    // No draw came: the run held goes out as it came (a check item's is dropped).
    if !held.cmds.is_empty() && !held.check_item() {
        if let Some(mut s) = super::d3d9real::sink_of(e) {
            flush(&mut s, &mut held);
        }
    }
    Done::Plain
}

fn interrupt(e: &mut Engine) {
    if !HOLDING.with(Cell::get) {
        return;
    }
    let Some(mut held) = HELD.with(|h| h.borrow_mut().take()) else {
        return;
    };
    if held.done.is_none() && !held.cmds.is_empty() {
        let _device = engine_core::real_threads::device_guard(e);
        if let Some(mut s) = super::d3d9real::sink_of(e) {
            flush(&mut s, &mut held);
        }
    }
    HELD.with(|h| *h.borrow_mut() = Some(held));
}

fn forget(e: &mut Engine, ids: &[u32]) {
    let _device = engine_core::real_threads::device_guard(e);
    let Some(device) = super::d3d9real::first_device(e) else {
        return;
    };
    if let Some(mut s) = super::d3d9real::sink_of(e) {
        s.submit_now(Cmd::ForgetRecords {
            device,
            ids: Data::from_words(ids),
        });
    }
}

/// The hooks the typed draw code reaches (`engine_core::draw_items::install`).
pub(super) static HOOKS: Hooks = Hooks {
    begin,
    end,
    interrupt,
    forget,
    epoch,
};

/// The run held goes out as it came; holding goes on.
fn flush(s: &mut Sink<'_>, held: &mut Held) {
    for c in held.cmds.iter() {
        s.submit_now(c);
    }
    held.cmds.clear();
    held.registers.clear();
}

/// The draw came: the run that ends with it sent as asked.
fn finish(s: &mut Sink<'_>, held: &mut Held) {
    let device = held.device;
    let done = match held.ask {
        Ask::Record { id, forget, check } => {
            s.submit_now(Cmd::RecordNext {
                device,
                id,
                forget,
                check: u32::from(check),
            });
            for c in held.cmds.iter() {
                s.submit_now(c);
            }
            Done::Recorded(held.cmds.shape())
        }
        Ask::Item { id, hash, check } if held.cmds.shape() == hash => {
            s.submit_now(Cmd::DrawItem {
                device,
                id,
                check: u32::from(check),
                constants: Data::Bytes(&held.registers),
            });
            Done::Drawn
        }
        // Not the record's: as it came (a check item's dropped: its old form draws).
        Ask::Item { check, .. } => {
            if !check {
                flush(s, held);
            }
            Done::Other
        }
    };
    held.cmds.clear();
    held.registers.clear();
    held.done = Some(done);
}

/// A command for the sink while a draw's commands may be held ([`HOLDING`]).
pub(super) fn submit(s: &mut Sink<'_>, cmd: Cmd<Data<'_>>) -> Reply {
    let Some(mut held) = HELD.with(|h| h.borrow_mut().take()) else {
        return s.submit_now(cmd);
    };
    let own = engine_core::real_threads::in_noted_call() && !cmd.is_sync();
    let reply = if !own {
        // Another call's command ends the run.
        if held.done.is_none() && !held.cmds.is_empty() {
            flush(s, &mut held);
        }
        s.submit_now(cmd)
    } else if held.done.is_some() {
        // After the draw: as it came, but a check item's (its old form makes them).
        if held.check_item() {
            Reply::None
        } else {
            s.submit_now(cmd)
        }
    } else {
        // A run the backend would cut goes out as it came, and a new one starts (a replay cuts at the same place).
        if held.cmds.len() >= d3d9_stream::kept::MOST_COMMANDS {
            flush(s, &mut held);
        }
        held.cmds.keep(cmd);
        if let Cmd::SetConstants { data, .. } = cmd {
            held.registers.extend_from_slice(&data.bytes());
        }
        if matches!(
            cmd,
            Cmd::DrawPrimitive { .. } | Cmd::DrawIndexedPrimitive { .. }
        ) {
            finish(s, &mut held);
        }
        Reply::None
    };
    HELD.with(|h| *h.borrow_mut() = Some(held));
    reply
}

#[cfg(test)]
mod tests {
    use super::*;
    use d3d9_stream::{Backend, Stream};
    use std::sync::{Arc, Mutex};

    /// TEST DATA: a backend that logs each command's name (and a `SetConstants`' first word).
    struct Names(Arc<Mutex<Vec<String>>>);

    impl Backend for Names {
        fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
            let name = match cmd {
                Cmd::SetConstants { data, .. } => format!("SetConstants {}", data.to_words()[0]),
                other => other.name().to_string(),
            };
            self.0.lock().unwrap().push(name);
            Reply::None
        }
    }

    /// A stream into kept records in front of a [`Names`]: the stream and its log.
    fn kept_stream() -> (Stream, Arc<Mutex<Vec<String>>>) {
        let log = Arc::new(Mutex::new(Vec::new()));
        let names = Names(Arc::clone(&log));
        (Stream::direct(Box::new(d3d9_stream::Kept::new(names))), log)
    }

    /// One draw's commands through the sink with `ask` held (each marked whether it is the draw's own, a noted
    /// call's, and with its first word): what came of it. The log is taken.
    fn run(
        stream: &mut Stream,
        log: &Mutex<Vec<String>>,
        ask: Ask,
        cmds: &[(bool, Cmd<()>, u32)],
    ) -> (Vec<String>, Option<Done>) {
        HELD.with(|h| {
            *h.borrow_mut() = Some(Held {
                ask,
                device: 1,
                cmds: Commands::default(),
                registers: Vec::new(),
                done: None,
            });
        });
        for &(own, cmd, word) in cmds {
            let words = [word, 0, 0, 0];
            let cmd = cmd.map(|()| Data::from_words(&words));
            let mut sink = Sink::Main(stream);
            if own {
                engine_core::real_threads::as_noted_call_for_tests(|| submit(&mut sink, cmd));
            } else {
                submit(&mut sink, cmd);
            }
        }
        let done = HELD.with(|h| h.borrow_mut().take()).and_then(|h| h.done);
        (std::mem::take(&mut *log.lock().unwrap()), done)
    }

    fn stream_source() -> Cmd<()> {
        Cmd::SetStreamSource {
            device: 1,
            stream: 0,
            buffer: 0x50,
            offset: 0,
            stride: 16,
        }
    }

    fn constants() -> Cmd<()> {
        Cmd::SetConstants {
            device: 1,
            kind: 0,
            start: 0,
            count: 1,
            data: (),
        }
    }

    fn texture() -> Cmd<()> {
        Cmd::SetTexture {
            device: 1,
            sampler: 0,
            texture: 0x60,
        }
    }

    fn draw() -> Cmd<()> {
        Cmd::DrawPrimitive {
            device: 1,
            kind: 4,
            start: 0,
            count: 2,
        }
    }

    /// The draw's own commands with its register word `word`, after a setter's send.
    fn one_draw(word: u32) -> [(bool, Cmd<()>, u32); 4] {
        [
            (false, texture(), 0),
            (true, stream_source(), 0),
            (true, constants(), word),
            (true, draw(), 0),
        ]
    }

    /// A draw's own commands, together up to the draw, go after a `RecordNext` (a setter's send before them goes
    /// first, as it came); again with other registers and the same shape, one `DrawItem` stands for them, which the
    /// backend carries out as the record's commands with the item's registers; of another shape, they go as they
    /// came.
    #[test]
    fn a_draws_own_commands_become_a_record_and_then_one_item() {
        let (mut stream, log) = kept_stream();
        let record = Ask::Record {
            id: 4,
            forget: 2,
            check: false,
        };
        let (got, done) = run(&mut stream, &log, record, &one_draw(7));
        let Some(Done::Recorded(hash)) = done else {
            panic!("{done:?}");
        };
        let want = |w: u32| {
            vec![
                "SetTexture".to_string(),
                "SetStreamSource".to_string(),
                format!("SetConstants {w}"),
                "DrawPrimitive".to_string(),
            ]
        };
        assert_eq!(got, want(7));
        let item = |hash| Ask::Item {
            id: 4,
            hash,
            check: false,
        };
        let (got, done) = run(&mut stream, &log, item(hash), &one_draw(9));
        assert_eq!(done, Some(Done::Drawn));
        assert_eq!(
            got,
            want(9),
            "the record's commands with the item's registers"
        );
        let (got, done) = run(&mut stream, &log, item(hash ^ 1), &one_draw(9));
        assert_eq!(done, Some(Done::Other), "another shape");
        assert_eq!(got, want(9), "sent as they came");
    }

    /// A send from outside the draw's own calls ends a run: what it held goes first, so the order is the one they
    /// came in, and the run that ends with the draw is the record; a check run's item sends its earlier runs as
    /// they came too, but drops a last run that is not its record's (its old form draws).
    #[test]
    fn a_send_between_the_draws_own_commands_keeps_their_order() {
        let (mut stream, log) = kept_stream();
        let cmds = [
            (true, stream_source(), 0),
            (false, texture(), 0),
            (true, constants(), 7),
            (true, draw(), 0),
        ];
        let record = Ask::Record {
            id: 1,
            forget: 0,
            check: false,
        };
        let (got, done) = run(&mut stream, &log, record, &cmds);
        assert!(matches!(done, Some(Done::Recorded(_))), "{done:?}");
        assert_eq!(
            got,
            [
                "SetStreamSource",
                "SetTexture",
                "SetConstants 7",
                "DrawPrimitive"
            ]
        );
        let item = Ask::Item {
            id: 1,
            hash: 0,
            check: true,
        };
        let (got, done) = run(&mut stream, &log, item, &cmds);
        assert_eq!(done, Some(Done::Other));
        assert_eq!(
            got,
            ["SetStreamSource", "SetTexture"],
            "the check item's last run dropped"
        );
    }

    /// `cmd` (with its first word `word`) sent as one of the draw's own device calls sends it: inside the device
    /// lock and a noted call.
    fn own(e: &mut Engine, cmd: Cmd<()>, word: u32) {
        let words = [word, 0, 0, 0];
        let _device = engine_core::real_threads::device_guard(e);
        engine_core::real_threads::as_noted_call_for_tests(|| {
            let mut s = super::super::d3d9real::sink_of(e).expect("the test stream");
            submit(&mut s, cmd.map(|()| Data::from_words(&words)));
        });
    }

    /// The shape of `cmds` (each with its first word) as a held run's.
    fn shape(cmds: &[(Cmd<()>, u32)]) -> u64 {
        let mut c = Commands::default();
        for &(cmd, word) in cmds {
            let words = [word, 0, 0, 0];
            c.keep(cmd.map(|()| Data::from_words(&words)));
        }
        c.shape()
    }

    /// A device made by the Direct3D 9 model with a kept records' stream (its log) and the hooks installed.
    fn device_with_stream() -> (Engine, Arc<Mutex<Vec<String>>>) {
        let (mut e, device) = super::super::d3d9device::tests::device_with("", 0x40);
        let (stream, log) = kept_stream();
        super::super::d3d9real::test_stream(&mut e, device, stream);
        engine_core::draw_items::install(&HOOKS);
        (e, log)
    }

    /// The real hooks with real threads and the device lock not held, as between the world draw's device calls
    /// when another thread waits for the lock (review of #1108): each hook takes the lock itself (a debug build's
    /// check stops the test otherwise) and gives it back; a run's flush at a setter call while a record is being
    /// built is not counted as a device call the recorder missed; the record is the run after the setter.
    #[test]
    fn the_hooks_take_the_device_lock_and_leave_the_recorder_alone() {
        use engine_core::draw_items as items;
        use engine_core::real_threads as rt;
        let (mut e, log) = device_with_stream();
        rt::start(&mut e).expect("real threads on");
        assert!(!rt::device_lock_held(&e));
        let record = Ask::Record {
            id: 3,
            forget: 0,
            check: false,
        };
        items::begin(&mut e, record);
        assert!(HOLDING.with(Cell::get), "a device to hold for");
        rt::device_record_begin();
        own(&mut e, stream_source(), 0);
        items::interrupt(&mut e);
        own(&mut e, stream_source(), 0);
        own(&mut e, constants(), 7);
        own(&mut e, draw(), 0);
        assert!(
            !rt::device_record_unseen(),
            "the flush is no device call the recorder missed"
        );
        rt::device_record_end();
        let want = shape(&[(stream_source(), 0), (constants(), 7), (draw(), 0)]);
        assert_eq!(items::end(&mut e), Done::Recorded(want));
        items::forget(&mut e, &[3]);
        assert!(!rt::device_lock_held(&e));
        assert_eq!(
            *log.lock().unwrap(),
            [
                "SetStreamSource",
                "SetStreamSource",
                "SetConstants 7",
                "DrawPrimitive"
            ]
        );
    }

    /// A draw that never came: what was held goes out at the end, as it came.
    #[test]
    fn a_run_without_its_draw_goes_out_at_the_end() {
        use engine_core::draw_items as items;
        let (mut e, log) = device_with_stream();
        let record = Ask::Record {
            id: 5,
            forget: 0,
            check: false,
        };
        items::begin(&mut e, record);
        own(&mut e, stream_source(), 0);
        own(&mut e, constants(), 4);
        assert!(log.lock().unwrap().is_empty(), "held");
        assert_eq!(items::end(&mut e), Done::Plain);
        assert_eq!(*log.lock().unwrap(), ["SetStreamSource", "SetConstants 4"]);
        assert!(!HOLDING.with(Cell::get));
    }

    /// Without a device the backend has, nothing is held.
    #[test]
    fn nothing_is_held_without_a_device() {
        let (mut e, _) = super::super::d3d9device::tests::device_with("", 0x40);
        assert_eq!(super::super::d3d9real::first_device(&mut e), None);
        engine_core::draw_items::install(&HOOKS);
        let record = Ask::Record {
            id: 1,
            forget: 0,
            check: false,
        };
        engine_core::draw_items::begin(&mut e, record);
        assert!(!HOLDING.with(Cell::get));
        assert_eq!(engine_core::draw_items::end(&mut e), Done::Plain);
    }

    /// A check run's item draws nothing of its own: its record's own commands after its draw are dropped too
    /// (its old form sends them).
    #[test]
    fn a_check_items_own_commands_after_its_draw_are_dropped() {
        let (mut stream, log) = kept_stream();
        let record = Ask::Record {
            id: 6,
            forget: 0,
            check: false,
        };
        let (_, done) = run(&mut stream, &log, record, &one_draw(7));
        let Some(Done::Recorded(hash)) = done else {
            panic!("{done:?}");
        };
        let item = Ask::Item {
            id: 6,
            hash,
            check: true,
        };
        let mut cmds = one_draw(9).to_vec();
        cmds.push((true, stream_source(), 0));
        let (got, done) = run(&mut stream, &log, item, &cmds);
        assert_eq!(done, Some(Done::Drawn));
        assert_eq!(got, ["SetTexture"], "only the setter's send");
    }

    /// A run longer than the backend keeps goes out as it came when the next own command comes, and a new run
    /// starts: the record is the run up to the draw after it (review of #1108).
    #[test]
    fn a_run_longer_than_the_backend_keeps_goes_out_as_it_came() {
        let (mut stream, log) = kept_stream();
        let most = d3d9_stream::kept::MOST_COMMANDS;
        let mut cmds = vec![(true, constants(), 1); most + 3];
        cmds.push((true, draw(), 0));
        let record = Ask::Record {
            id: 8,
            forget: 0,
            check: false,
        };
        let (got, done) = run(&mut stream, &log, record, &cmds);
        assert_eq!(got.len(), most + 4, "every command went out once");
        let mut last = vec![(constants(), 1); 3];
        last.push((draw(), 0));
        assert_eq!(done, Some(Done::Recorded(shape(&last))));
    }

    /// A check run keeps the states set again to their value (review of #1108), as `NV_D3D9_REPEATS=keep` does.
    #[test]
    fn a_check_run_keeps_repeated_sets() {
        use super::super::d3d9device::repeats_kept;
        use engine_core::draw_items::Mode;
        assert!(repeats_kept(false, Mode::Check));
        assert!(repeats_kept(true, Mode::Off));
        assert!(!repeats_kept(false, Mode::Off));
        assert!(!repeats_kept(false, Mode::On));
    }
}
