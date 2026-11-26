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
//! item or nothing, and its own commands after it are dropped, since the old form draws.
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
        if let Some(mut s) = super::d3d9real::sink_of(e) {
            flush(&mut s, &mut held);
        }
    }
    HELD.with(|h| *h.borrow_mut() = Some(held));
}

fn forget(e: &mut Engine, ids: &[u32]) {
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
}
