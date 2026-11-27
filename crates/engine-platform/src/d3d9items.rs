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
use engine_core::real_threads::{Deferred, DeferredCall};
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
    /// A run was sent as it came for being as long as the backend keeps: the record is a cut-off tail of the draw's
    /// own commands, so no patched item draws it.
    capped: bool,
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
    /// The last draw's buffers, emptied, for the next one (no allocation per draw).
    static SPARE: RefCell<Option<(Commands, Vec<u8>)>> = const { RefCell::new(None) };
    /// The device the items go to, as of a records' epoch ([`device_now`]).
    static DEVICE: Cell<(u64, u32)> = const { Cell::new((u64::MAX, 0)) };
}

/// The device items and records are sent for: the platform's first device, looked up (under the device lock) once
/// per records' epoch, which every device made or reset bumps ([`bump_epoch`]), so a draw's `begin` takes no lock.
fn device_now(e: &mut Engine) -> u32 {
    let now = epoch();
    let (of, device) = DEVICE.with(Cell::get);
    if of == now {
        return device;
    }
    let _device = engine_core::real_threads::device_guard(e);
    let device = super::d3d9real::first_device(e).unwrap_or(0);
    DEVICE.with(|d| d.set((now, device)));
    device
}

/// A finished draw's buffers kept for the next ([`SPARE`]).
fn spare(mut held: Held) {
    held.cmds.clear();
    held.registers.clear();
    SPARE.with(|s| *s.borrow_mut() = Some((held.cmds, held.registers)));
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
    let device = device_now(e);
    let (cmds, registers) = SPARE.with(|s| s.borrow_mut().take()).unwrap_or_default();
    HELD.with(|h| {
        *h.borrow_mut() = Some(Held {
            ask,
            device,
            cmds,
            registers,
            done: None,
            capped: false,
        });
    });
    HOLDING.with(|h| h.set(device != 0));
}

/// The device lock is taken only to send what is still held (a draw that never came); a draw that came was sent
/// by the sink, under its lock.
fn end(e: &mut Engine) -> Done {
    HOLDING.with(|h| h.set(false));
    let Some(mut held) = HELD.with(|h| h.borrow_mut().take()) else {
        return Done::Plain;
    };
    if let Some(done) = held.done {
        spare(held);
        return done;
    }
    // No draw came: the run held goes out as it came (a check item's is dropped).
    if !held.cmds.is_empty() && !held.check_item() {
        let _device = engine_core::real_threads::device_guard(e);
        if let Some(mut s) = super::d3d9real::sink_of(e) {
            flush(&mut s, &mut held);
        }
    }
    spare(held);
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
    patched,
};

thread_local! {
    /// A patched item's patches as they are built (no allocation per item).
    static PATCHES: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// The `SetConstants` kind of a constant upload's device slot.
fn constant_kind(slot: u32) -> Option<u8> {
    match slot {
        0x178 => Some(0),
        0x180 => Some(1),
        0x188 => Some(2),
        0x1b4 => Some(3),
        0x1bc => Some(4),
        0x1c4 => Some(5),
        _ => None,
    }
}

/// The command a deferred device call is when the device takes it (`engine_core::real_threads::DeferredCall`, its
/// structure's bytes): as the per-call path records it (`super::d3d9real`'s setters); None for a call the item form
/// does not carry.
fn command<'a>(c: &DeferredCall, bytes: &'a [u8]) -> Option<Cmd<Data<'a>>> {
    let a = c.args();
    let device = c.this;
    Some(match (c.slot, a.len()) {
        (0xe4, 2) => Cmd::SetRenderState {
            device,
            state: a[0],
            value: a[1],
        },
        (0x10c, 3) => Cmd::SetTextureStageState {
            device,
            stage: a[0],
            kind: a[1],
            value: a[2],
        },
        (0x114, 3) => Cmd::SetSamplerState {
            device,
            sampler: a[0],
            kind: a[1],
            value: a[2],
        },
        (0x104, 2) => Cmd::SetTexture {
            device,
            sampler: a[0],
            texture: a[1],
        },
        (0x15c, 1) => Cmd::SetVertexDeclaration {
            device,
            declaration: a[0],
        },
        (0x164, 1) => Cmd::SetFvf { device, fvf: a[0] },
        (0x170 | 0x1ac, 1) => Cmd::SetShader {
            device,
            pixel: c.slot == 0x1ac,
            shader: a[0],
        },
        (0x190, 4) => Cmd::SetStreamSource {
            device,
            stream: a[0],
            buffer: a[1],
            offset: a[2],
            stride: a[3],
        },
        (0x1a0, 1) => Cmd::SetIndices {
            device,
            buffer: a[0],
        },
        (0xdc, 2) if bytes.len() == 16 => Cmd::SetClipPlane {
            device,
            index: a[0],
            plane: std::array::from_fn(|i| {
                u32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().expect("4 bytes"))
            }),
        },
        (0x144, 3) => Cmd::DrawPrimitive {
            device,
            kind: a[0],
            start: a[1],
            count: a[2],
        },
        (0x148, 6) => Cmd::DrawIndexedPrimitive {
            device,
            kind: a[0],
            base_vertex: a[1] as i32,
            min_index: a[2],
            vertices: a[3],
            start_index: a[4],
            count: a[5],
        },
        (slot, 3) => Cmd::SetConstants {
            device,
            kind: constant_kind(slot)?,
            start: a[0],
            count: a[2],
            data: Data::Bytes(bytes),
        },
        _ => return None,
    })
}

/// Record `id` drawn as a patched item from this thread's deferred device calls
/// (`engine_core::draw_items::patched`). Refused on the drawing side and while a drawing cut is open on the device
/// (the bulk update's refusal): with the drawing thread on, its calls are made as they came (review of #1227; the
/// typed draw code asks for none while the drawing thread's packets are on). Refused too without a device, or when
/// [`patched_from`] refuses.
fn patched(e: &mut Engine, id: u32) -> bool {
    if super::d3d9draw::drawing_thread() {
        return false;
    }
    let device = device_now(e);
    if device == 0 {
        return false;
    }
    let _device = engine_core::real_threads::device_guard(e);
    if super::d3d9real::sink_of(e).is_none() {
        return false;
    }
    engine_core::real_threads::with_deferred(|d| patched_from(e, device, id, d))
}

/// [`patched`] under the device lock. The calls (`d`) must all be the device's and carried by the item form, with the
/// draw last and alone. The item is then, in order: the state sets of the setters and camera set-ups (and of the
/// draw's own calls made before the record's commands begin: [`engine_core::real_threads::Deferred::mark`]) sent as
/// they are; the record's own commands, which the backend keeps (their state sets are the own calls' after the mark,
/// with the same words); the patches, every constant upload of the calls in their order (the record's own uploads
/// among them, so every register the record sets is set again with the replay's values, and a register set twice
/// ends with its last value: obligations a and b of the review of #1220); its draw. The model is brought by one
/// [`bulk_update`](super::d3d9device::bulk_update) to where exactly those sets leave it (the record's own uploads
/// left out of its list: the patches set the same registers after them; review of #1227), and the objects it let go
/// of end after the item. False, with nothing sent or changed, when a call is not one of those or the update is
/// refused.
fn patched_from(e: &mut Engine, device: u32, id: u32, d: &Deferred) -> bool {
    let n = d.len();
    let carried = d.calls().enumerate().all(|(i, (c, bytes))| {
        c.this == device
            && command(c, bytes).is_some_and(|cmd| {
                matches!(
                    cmd,
                    Cmd::DrawPrimitive { .. } | Cmd::DrawIndexedPrimitive { .. }
                ) == (i + 1 == n)
            })
    });
    if n == 0 || !carried {
        return false;
    }
    let mark = d.mark();
    let constant = |c: &DeferredCall| constant_kind(c.slot).is_some();
    let sets = d.calls().take(n - 1).enumerate();
    let ordinary = sets
        .clone()
        .filter(move |(i, (c, _))| !constant(c) && !(c.own && *i >= mark));
    let own = sets
        .clone()
        .filter(move |(i, (c, _))| !constant(c) && c.own && *i >= mark);
    let uploads = sets.filter(move |(_, (c, _))| constant(c));
    fn cmd<'a>((_, (c, bytes)): (usize, (&DeferredCall, &'a [u8]))) -> Cmd<Data<'a>> {
        command(c, bytes).expect("checked above")
    }
    let list = ordinary.clone().chain(own).chain(uploads.clone()).map(cmd);
    let Ok(ending) = super::d3d9device::bulk_update(e, device, list) else {
        return false;
    };
    PATCHES.with(|p| {
        let mut p = p.borrow_mut();
        p.clear();
        for (_, (c, bytes)) in uploads {
            let a = c.args();
            let kind = u32::from(constant_kind(c.slot).expect("an upload"));
            for word in [kind, a[0], a[2]] {
                p.extend_from_slice(&word.to_le_bytes());
            }
            p.extend_from_slice(bytes);
        }
        let mut s = super::d3d9real::sink_of(e).expect("checked by the caller");
        for c in ordinary.map(cmd) {
            s.submit_now(c);
        }
        s.submit_now(Cmd::DrawPatched {
            device,
            id,
            check: 0,
            patches: Data::Bytes(&p),
        });
    });
    super::d3d9device::end_after_bulk(e, ending);
    true
}

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
            // A record of state sets and its draw, whole: its kept states (none: a patched item's own sets go before
            // it as ordinary ones, [`patched`]) let a `DrawPatched` draw it. Never one holding an upload, a creation
            // or a clear (review of #1220).
            let last = held.cmds.len().saturating_sub(1);
            let sets = held
                .cmds
                .iter()
                .take(last)
                .all(|c| d3d9_stream::kept::is_state(&c));
            if !check && sets && !held.capped {
                s.submit_now(Cmd::KeepStates {
                    device,
                    id,
                    count: 0,
                });
                Done::Patchable(held.cmds.shape())
            } else {
                Done::Recorded(held.cmds.shape())
            }
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
            held.capped = true;
        }
        held.cmds.keep(cmd);
        if let Cmd::SetConstants { data, .. } = cmd {
            let at = held.registers.len();
            held.registers.resize(at + data.len() as usize, 0);
            data.copy_to(0, &mut held.registers[at..]);
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
                capped: false,
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
        let Some(Done::Patchable(hash)) = done else {
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
        assert!(matches!(done, Some(Done::Patchable(_))), "{done:?}");
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
        assert_eq!(items::end(&mut e), Done::Patchable(want));
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

    /// A draw that never came: what was held goes out at the end, as it came; with real threads and the device lock
    /// not held, the end takes the lock itself for that flush (a debug build's check stops the test otherwise) and
    /// gives it back (review of #1108).
    #[test]
    fn a_run_without_its_draw_goes_out_at_the_end() {
        use engine_core::draw_items as items;
        use engine_core::real_threads as rt;
        for real in [false, true] {
            let (mut e, log) = device_with_stream();
            if real {
                rt::start(&mut e).expect("real threads on");
                assert!(!rt::device_lock_held(&e));
            }
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
            assert!(!real || !rt::device_lock_held(&e), "the lock given back");
        }
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
        let Some(Done::Patchable(hash)) = done else {
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
        assert_eq!(
            done,
            Some(Done::Recorded(shape(&last))),
            "a cut-off tail: no patched item's"
        );
    }

    /// The texture lines the patched items' tests' device takes (A8R8G8B8 managed textures; TEST DATA).
    const TEXTURES: &str = "\nDomain\td1\t15,31545844\nCheckDeviceFormat\t0 1 16 0 3\t0\t\td1";

    /// A device with a kept records' stream (its log), the hooks, and `n` textures, each forwarded (its end is a
    /// command): (engine, device, textures, log).
    fn patched_world(n: usize) -> (Engine, u32, Vec<u32>, Arc<Mutex<Vec<String>>>) {
        use super::super::d3d9device::tests::{device_with, new_texture};
        let (mut e, device) = device_with(TEXTURES, 0x40);
        let textures: Vec<u32> = (0..n)
            .map(|_| new_texture(&mut e, device, [4, 4, 1, 0, 21, 1]))
            .collect();
        // The cull mode the tests read set first (the model answers no default for it).
        assert_eq!(e.vcall(device, 4 * 57, &[22, 2]).u32(), 0);
        let (stream, log) = kept_stream();
        super::super::d3d9real::test_stream(&mut e, device, stream);
        for t in &textures {
            super::super::d3d9real::test_forwarded(&mut e, *t);
        }
        engine_core::draw_items::install(&HOOKS);
        (e, device, textures, log)
    }

    /// Record `id` built from the draw's own commands: a stream source of no buffer, an upload of first word 7, the
    /// draw.
    fn build_record(e: &mut Engine, id: u32) -> Done {
        use engine_core::draw_items as items;
        let record = Ask::Record {
            id,
            forget: 0,
            check: false,
        };
        items::begin(e, record);
        let unbound = Cmd::SetStreamSource {
            device: 1,
            stream: 0,
            buffer: 0,
            offset: 0,
            stride: 16,
        };
        own(e, unbound, 0);
        own(e, constants(), 7);
        own(e, draw(), 0);
        items::end(e)
    }

    /// One of the draw's own calls as a record keeps it (no structure kept: one is read where it lies).
    fn own_call(this: u32, slot: u32, args: &[u32]) -> engine_core::real_threads::DeviceCall {
        engine_core::real_threads::DeviceCall {
            this,
            slot,
            args: args.to_vec(),
            data: Vec::new(),
            data_arg: None,
        }
    }

    /// The record's own calls made again while deferring (after the mark): its stream source, its upload (of
    /// first word `word`, from guest memory) and its draw.
    fn replay_own(e: &mut Engine, device: u32, word: u32) {
        use engine_core::real_threads as rt;
        rt::device_defer_mark();
        rt::device_replay(e, &own_call(device, 0x190, &[0, 0, 0, 16]));
        let registers = e.mem.alloc(16);
        e.mem.set_u32(registers, word);
        rt::device_replay(e, &own_call(device, 0x178, &[0, registers, 1]));
        rt::device_replay(e, &own_call(device, 0x144, &[4, 0, 2]));
    }

    /// A word the device model answers for `Get*(args.., out)` at vtable index `index`.
    fn get(e: &mut Engine, device: u32, index: u32, args: &[u32]) -> u32 {
        let out = e.mem.alloc(16);
        let args: Vec<u32> = args.iter().copied().chain([out]).collect();
        assert_eq!(e.vcall(device, 4 * index, &args).u32(), 0);
        e.mem.u32(out)
    }

    /// A record of state sets and its draw is patchable (`KeepStates` of none after it, so the backend can draw it as
    /// a patched item). Its replay with every device call deferred reaches nothing until the draw; then one patched
    /// item carries them: the setter's set as it is, the record's own commands (its upload with the build's
    /// registers), the patches (every upload, with the replay's registers), the draw. The model holds what the calls
    /// would have left.
    #[test]
    fn a_patched_item_is_the_deferred_calls_at_once() {
        use engine_core::draw_items as items;
        use engine_core::real_threads as rt;
        let (mut e, device, _, log) = patched_world(0);
        assert!(matches!(build_record(&mut e, 3), Done::Patchable(_)));
        log.lock().unwrap().clear();
        rt::device_defer_begin();
        rt::device_vcall(&mut e, device, 0xe4, &[22, 1]);
        replay_own(&mut e, device, 9);
        assert_eq!(rt::device_defer_end(), rt::Deferral::Kept);
        assert!(log.lock().unwrap().is_empty(), "nothing before the draw");
        assert_ne!(get(&mut e, device, 58, &[22]), 1, "the model not yet");
        assert!(items::patched(&mut e, 3));
        assert_eq!(
            *log.lock().unwrap(),
            [
                "SetRenderState",
                "SetStreamSource",
                "SetConstants 7",
                "SetConstants 9",
                "DrawPrimitive"
            ]
        );
        assert_eq!(get(&mut e, device, 58, &[22]), 1, "the setter's state");
        let out = e.mem.alloc(16);
        assert_eq!(e.vcall(device, 4 * 95, &[0, out, 1]).u32(), 0);
        assert_eq!(e.mem.u32(out), 9, "the replay's register");
    }

    /// No patched item (nothing sent, the model as it was, the caller makes the calls): with a drawing cut open,
    /// for a call of another device, for a draw that is not last and alone, for no record's build being patchable
    /// (a check run's).
    #[test]
    fn a_patched_item_is_refused_when_it_could_differ() {
        use engine_core::draw_items as items;
        use engine_core::real_threads as rt;
        let (mut e, device, _, log) = patched_world(0);
        assert!(matches!(build_record(&mut e, 3), Done::Patchable(_)));
        log.lock().unwrap().clear();
        let refused = |e: &mut Engine, why: &str| {
            assert_eq!(rt::device_defer_end(), rt::Deferral::Kept, "{why}");
            assert!(!items::patched(e, 3), "{why}");
            assert!(log.lock().unwrap().is_empty(), "{why}: nothing sent");
            assert_ne!(get(e, device, 58, &[22]), 1, "{why}: the model as it was");
        };
        assert!(super::super::d3d9draw::draw_cut(&mut e, device));
        rt::device_defer_begin();
        rt::device_vcall(&mut e, device, 0xe4, &[22, 1]);
        replay_own(&mut e, device, 9);
        refused(&mut e, "a cut open");
        super::super::d3d9draw::draw_join(&mut e);
        log.lock().unwrap().clear();
        rt::device_defer_begin();
        rt::device_vcall(&mut e, device + 4, 0xe4, &[22, 1]);
        rt::device_vcall(&mut e, device, 0xe4, &[22, 1]);
        replay_own(&mut e, device, 9);
        refused(&mut e, "another device");
        rt::device_defer_begin();
        rt::device_vcall(&mut e, device, 0xe4, &[22, 1]);
        replay_own(&mut e, device, 9);
        rt::device_replay(&mut e, &own_call(device, 0x144, &[4, 0, 2]));
        refused(&mut e, "two draws");
        rt::device_defer_begin();
        rt::device_vcall(&mut e, device, 0xe4, &[22, 1]);
        rt::device_defer_mark();
        refused(&mut e, "no draw");
        // A check run's record is no patched item's.
        let record = Ask::Record {
            id: 4,
            forget: 0,
            check: true,
        };
        items::begin(&mut e, record);
        own(&mut e, draw(), 0);
        assert!(matches!(items::end(&mut e), Done::Recorded(_)));
    }

    /// A texture the program released while a sampler holds it, set again by a setter and then replaced: its end goes
    /// after the patched item (review of #1227), so the backend never gets a set of a texture it has ended.
    #[test]
    fn an_object_a_patched_item_lets_go_of_ends_after_it() {
        use engine_core::draw_items as items;
        use engine_core::real_threads as rt;
        let (mut e, device, textures, log) = patched_world(2);
        let (b, c) = (textures[0], textures[1]);
        assert_eq!(e.vcall(device, 4 * 65, &[0, b]).u32(), 0);
        assert_eq!(e.vcall(b, 8, &[]).u32(), 0, "released, still bound");
        assert!(matches!(build_record(&mut e, 5), Done::Patchable(_)));
        log.lock().unwrap().clear();
        rt::device_defer_begin();
        rt::device_vcall(&mut e, device, 0x104, &[0, b]);
        rt::device_vcall(&mut e, device, 0x104, &[0, c]);
        replay_own(&mut e, device, 9);
        assert_eq!(rt::device_defer_end(), rt::Deferral::Kept);
        assert!(items::patched(&mut e, 5));
        assert_eq!(
            *log.lock().unwrap(),
            [
                "SetTexture",
                "SetTexture",
                "SetStreamSource",
                "SetConstants 7",
                "SetConstants 9",
                "DrawPrimitive",
                "Destroy"
            ]
        );
        assert_eq!(get(&mut e, device, 64, &[0]), c, "the sampler's texture");
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

    /// The device items go to is looked up once per records' epoch, so a draw's `begin` takes no device lock; a device
    /// made moves the epoch on, so the next `begin` looks again (a stale number of the old epoch is not taken). A
    /// finished draw's buffers are kept for the next.
    #[test]
    fn a_begin_looks_up_the_device_once_per_epoch() {
        use engine_core::draw_items as items;
        let (mut e, _log) = device_with_stream();
        let record = Ask::Record {
            id: 9,
            forget: 0,
            check: false,
        };
        items::begin(&mut e, record);
        let device = HELD.with(|h| h.borrow().as_ref().map(|h| h.device));
        assert!(matches!(device, Some(d) if d != 0));
        own(&mut e, stream_source(), 0);
        own(&mut e, draw(), 0);
        assert!(matches!(items::end(&mut e), Done::Patchable(_)));
        assert_eq!(
            DEVICE.with(Cell::get).1,
            device.unwrap(),
            "kept for the epoch"
        );
        let room = SPARE.with(|s| s.borrow().as_ref().map(|(c, _)| c.is_empty()));
        assert_eq!(room, Some(true), "the buffers kept, emptied");
        // A device made (the epoch moved on): looked up again.
        DEVICE.with(|d| d.set((epoch(), 0x1234)));
        bump_epoch();
        items::begin(&mut e, record);
        assert_eq!(HELD.with(|h| h.borrow().as_ref().map(|h| h.device)), device);
        items::end(&mut e);
        // The device destroyed: the epoch moves on, and the next begin finds no device (nothing held).
        let before = epoch();
        let gone = device.unwrap();
        super::super::d3d9real::forget(&mut e, gone);
        assert_ne!(epoch(), before, "a destroyed device moves the epoch");
        items::begin(&mut e, record);
        assert_eq!(
            HELD.with(|h| h.borrow().as_ref().map(|h| h.device)),
            Some(0)
        );
        assert!(!HOLDING.with(Cell::get), "nothing held without a device");
        items::end(&mut e);
    }
}
