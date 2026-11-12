//! The runner: where the backend carries the commands out. [`Stream::direct`] on the calling
//! thread, at once; [`Stream::threaded`] on a render thread of its own, at most a frame behind.
//!
//! **Two producers** ([`Stream::cut`]). The game's main thread records its commands through the
//! stream; a drawing thread may record part of a frame (the world's draw) at the same time. Main
//! makes a cut where those commands belong and goes on; the drawing side records through the
//! [`Slot`] the cut gives, and closes it. The backend carries the commands out in the serial
//! order: main's up to the cut, then the slot's, then main's after the cut. The threaded runner
//! takes the slot's chunks as they come once it reaches the cut; the direct runner carries the
//! slot's commands out as they come (main's before the cut are done) and holds main's later ones
//! until the slot is closed; `NV_D3D9_RECORD` writes the same order. While a cut is open, main
//! never waits for the render thread (no full-queue wait; `Present` lets at least one frame be in
//! flight) and makes no sync call: the layer above waits for the slot first, outside its own lock,
//! which the drawing side needs.

use crate::chunk::Chunk;
use crate::cmd::{Backend, Cmd, Data, Reply};
use crate::wire;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// How the threaded runner hands commands over.
#[derive(Debug, Clone)]
pub struct Settings {
    /// Presents the render thread may still have to carry out when `Present` returns: 1 (the
    /// default) lets the game start its next frame while the render thread finishes this one; 0
    /// returns only once this Present is carried out (`NV_RENDER_THREAD_FRAMES`).
    pub frames_in_flight: u64,
    /// A chunk is handed over at this many commands (`NV_RENDER_THREAD_CHUNK`) ...
    pub chunk_commands: usize,
    /// ... or this many bytes, at every Present and before every sync command.
    pub chunk_bytes: usize,
    /// Past this many bytes or chunks waiting, the recording thread waits for the render thread
    /// to carry out half of them (a loading burst of uploads stays bounded).
    pub max_queued_bytes: usize,
    pub max_queued_chunks: usize,
    /// `NV_D3D9_STREAM_CHECK=1`: the commands are hashed as recorded and as carried out, and a
    /// difference stops the run.
    pub check: bool,
    /// `NV_RENDER_THREAD_STATS=1`: a line a second on the hand-overs and the waits.
    pub stats: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            frames_in_flight: 1,
            chunk_commands: 1024,
            chunk_bytes: 256 << 10,
            max_queued_bytes: 64 << 20,
            max_queued_chunks: 64,
            check: false,
            stats: false,
        }
    }
}

impl Settings {
    /// The defaults, with the settings the environment gives.
    pub fn from_env() -> Settings {
        let number = |name: &str| std::env::var(name).ok()?.trim().parse::<u64>().ok();
        let mut s = Settings::default();
        if let Some(n) = number("NV_RENDER_THREAD_FRAMES") {
            s.frames_in_flight = n;
        }
        if let Some(n) = number("NV_RENDER_THREAD_CHUNK") {
            s.chunk_commands = (n as usize).max(1);
        }
        s.check = on("NV_D3D9_STREAM_CHECK");
        s.stats = on("NV_RENDER_THREAD_STATS");
        s
    }
}

fn on(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v.trim() == "1")
}

/// The commands' way to a backend.
pub struct Stream {
    runner: Runner,
    presents: u32,
    record: Option<Arc<Mutex<Recording>>>,
    /// The cuts made so far; the open one, if its slot is not closed yet (or main has not seen
    /// it closed).
    cuts: u64,
    cut: Option<OpenCut>,
    /// The last cut whose slot the backend carried out, and when ([`Stream::last_cut_carried_out`]).
    cut_done: Arc<Mutex<Option<(u64, Instant)>>>,
}

// One stream a run, made once and never moved on the way of a call: kept unboxed, so each
// command reaches the chunk without another pointer to follow.
#[allow(clippy::large_enum_variant)]
enum Runner {
    Direct(Box<dyn Backend>),
    /// The direct runner once a cut was made: the backend shared with the cut's [`Slot`].
    Shared(Arc<Mutex<Box<dyn Backend>>>),
    Threaded(Threaded),
}

/// `NV_D3D9_RECORD`: every command written as [`crate::wire`] encodes it.
struct Recording {
    out: Box<dyn Write + Send>,
    buf: Vec<u8>,
    /// [`Stream::record_repeatable`]: the window handles written as 0, the query reads left out.
    repeatable: bool,
    /// A write failed (printed once): nothing more is written.
    failed: bool,
}

impl Recording {
    /// Encodes `cmd` (its repeatable form where asked) at the end of `into`.
    fn encode(&self, cmd: Cmd<Data<'_>>, into: &mut Vec<u8>) {
        let written = if self.repeatable {
            repeatable_form(cmd)
        } else {
            Some(cmd)
        };
        if let Some(written) = &written {
            wire::encode(written, into);
        }
    }

    /// Writes `bytes`; a write that fails is printed and ends the recording.
    fn write(&mut self, bytes: &[u8]) {
        if self.failed || bytes.is_empty() {
            return;
        }
        if let Err(error) = self.out.write_all(bytes) {
            eprintln!(
                "NV_D3D9_RECORD: the command file could not be written ({error}); the recording stops"
            );
            self.failed = true;
        }
    }

    fn write_cmd(&mut self, cmd: Cmd<Data<'_>>) {
        let mut buf = std::mem::take(&mut self.buf);
        buf.clear();
        self.encode(cmd, &mut buf);
        self.write(&buf);
        self.buf = buf;
    }
}

/// A cut main has made and not yet seen closed ([`Stream::cut`]).
struct OpenCut {
    id: u64,
    /// The last cut whose slot was closed (shared with the slot).
    closed: Arc<AtomicU64>,
    /// Main's commands after the cut, held for the direct runner until the slot is closed.
    held: Chunk,
    /// ... and their encoding for the recording.
    held_record: Vec<u8>,
}

impl std::fmt::Debug for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stream")
            .field("threaded", &self.is_threaded())
            .field("presents", &self.presents)
            .field("recording", &self.record.is_some())
            .field("cut", &self.cut.as_ref().map(|c| c.id))
            .finish()
    }
}

fn lock_any<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Stream {
    fn with(runner: Runner) -> Stream {
        Stream {
            runner,
            presents: 0,
            record: None,
            cuts: 0,
            cut: None,
            cut_done: Arc::new(Mutex::new(None)),
        }
    }

    /// Commands carried out on the calling thread as they come: the calls the backend makes are
    /// the ones a direct call would make, in the same place.
    pub fn direct(backend: Box<dyn Backend>) -> Stream {
        Stream::with(Runner::Direct(backend))
    }

    /// Commands carried out on a render thread (named `nv-render`), which owns `backend`.
    pub fn threaded(backend: Box<dyn Backend>, settings: Settings) -> Stream {
        Stream::with(Runner::Threaded(Threaded::start(backend, settings)))
    }

    pub fn is_threaded(&self) -> bool {
        matches!(self.runner, Runner::Threaded(_))
    }

    /// The presents submitted so far (the last [`Cmd::Present`]'s `frame`).
    pub fn presents(&self) -> u32 {
        self.presents
    }

    /// From now on every command is also written to `out` ([`crate::wire`]'s file: [`wire::MAGIC`]
    /// first). A write that fails is printed and ends the recording.
    pub fn record_to(&mut self, mut out: Box<dyn Write + Send>) -> std::io::Result<()> {
        out.write_all(&wire::header())?;
        self.record = Some(Arc::new(Mutex::new(Recording {
            out,
            buf: Vec::new(),
            repeatable: false,
            failed: false,
        })));
        Ok(())
    }

    /// [`Stream::record_to`] for `NV_REPEATABLE` (a test mode): what differs between two runs of
    /// the same exe with the same input is left out, so their files compare byte for byte: the
    /// real windows' handles ([`Cmd::CreateDevice`], [`Cmd::Reset`], [`Cmd::Present`]) are
    /// written as 0, and [`Cmd::QueryGetData`] (a read, which draws nothing, made as often as the
    /// GPU takes) is not written.
    pub fn record_repeatable(&mut self, out: Box<dyn Write + Send>) -> std::io::Result<()> {
        self.record_to(out)?;
        if let Some(r) = &self.record {
            lock_any(r).repeatable = true;
        }
        Ok(())
    }

    /// Writes out what is recorded so far (`NV_REPEATABLE`'s end, before the process ends).
    pub fn finish_recording(&mut self) {
        if let Some(r) = &self.record {
            if let Err(error) = lock_any(r).out.flush() {
                eprintln!("NV_D3D9_RECORD: the command file could not be written ({error})");
            }
        }
    }

    /// Carries out `cmd` or queues it. A sync command ([`Cmd::is_sync`]) returns its reply once
    /// every command before it is carried out; a [`Cmd::Present`] gets its `frame` number here and
    /// returns once no more than [`Settings::frames_in_flight`] presents are left to carry out
    /// (one at least while a cut is open); any other returns [`Reply::None`] at once.
    ///
    /// # Panics
    /// On a sync command while a cut's slot is open: its reply would come after the slot's
    /// commands, which the drawing side may be waiting to record (see the module's notes).
    pub fn submit(&mut self, mut cmd: Cmd<Data<'_>>) -> Reply {
        #[cfg(debug_assertions)]
        if let Err(why) = cmd.check() {
            panic!("a malformed command: {why}");
        }
        if let Cmd::Present { frame, .. } = &mut cmd {
            self.presents = self.presents.wrapping_add(1);
            *frame = self.presents;
        }
        self.settle_cut();
        let holding = self.cut.is_some();
        if holding && cmd.is_sync() {
            panic!(
                "main's sync call ({}) while the drawing side's slot is open: its reply would come \
                 after the slot, so the caller waits for the slot first (Stream::cut_waiter)",
                cmd.name()
            );
        }
        if let Some(r) = &self.record {
            let mut r = lock_any(r);
            match &mut self.cut {
                Some(open) => r.encode(cmd, &mut open.held_record),
                None => r.write_cmd(cmd),
            }
        }
        match &mut self.runner {
            Runner::Direct(backend) => backend.execute(cmd),
            Runner::Shared(backend) => match &mut self.cut {
                Some(open) => {
                    open.held.push(cmd);
                    Reply::None
                }
                None => lock_any(backend).execute(cmd),
            },
            Runner::Threaded(t) => t.submit(cmd, holding),
        }
    }

    /// A cut of main's stream: what the drawing side records through the returned [`Slot`] is
    /// carried out after every command main submitted before this call and before every command
    /// main submits after it. One cut at a time: the slot must be closed (and main must have
    /// submitted a command since, or called [`Stream::settle`]) before the next.
    ///
    /// # Panics
    /// While another cut is open.
    pub fn cut(&mut self) -> Slot {
        self.settle_cut();
        assert!(
            self.cut.is_none(),
            "a cut is open: one drawing slot at a time"
        );
        self.cuts += 1;
        let id = self.cuts;
        let closed = Arc::new(AtomicU64::new(0));
        if let Runner::Direct(_) = self.runner {
            let Runner::Direct(backend) = std::mem::replace(
                &mut self.runner,
                Runner::Shared(Arc::new(Mutex::new(Box::new(Nothing)))),
            ) else {
                unreachable!()
            };
            self.runner = Runner::Shared(Arc::new(Mutex::new(backend)));
        }
        let side = match &mut self.runner {
            Runner::Direct(_) => unreachable!("made shared above"),
            Runner::Shared(backend) => SlotSide::Direct(Arc::clone(backend)),
            Runner::Threaded(t) => SlotSide::Threaded(t.cut(id)),
        };
        self.cut = Some(OpenCut {
            id,
            closed: Arc::clone(&closed),
            held: Chunk::default(),
            held_record: Vec::new(),
        });
        Slot {
            cut: id,
            side,
            record: self.record.clone(),
            closed,
            cut_done: Arc::clone(&self.cut_done),
            done: false,
        }
    }

    /// Whether a cut is open: its slot not closed yet, or closed and not yet seen here.
    pub fn cut_open(&mut self) -> bool {
        self.settle_cut();
        self.cut.is_some()
    }

    /// What waits for the open cut's slot to be closed, without the stream: the layer above
    /// holds it while its own lock is released ([`CutWaiter::wait`]). `None` without an open cut.
    pub fn cut_waiter(&mut self) -> Option<CutWaiter> {
        self.settle_cut();
        self.cut.as_ref().map(|open| CutWaiter {
            id: open.id,
            closed: Arc::clone(&open.closed),
        })
    }

    /// Main's commands held back for a slot that is closed now are carried out (direct runner)
    /// and written (recording): what [`Stream::submit`] does first anyway.
    pub fn settle(&mut self) {
        self.settle_cut();
    }

    /// The last cut whose slot the backend has carried out, and when (the render thread's clock
    /// on the threaded runner; the slot's close on the direct one).
    pub fn last_cut_carried_out(&self) -> Option<(u64, Instant)> {
        match &self.runner {
            Runner::Threaded(t) => lock(&t.shared.queue).cut_done,
            _ => *lock_any(&self.cut_done),
        }
    }

    /// The open cut's slot closed: main's held commands go on.
    fn settle_cut(&mut self) {
        let Some(open) = &self.cut else {
            return;
        };
        if open.closed.load(Ordering::Acquire) < open.id {
            return;
        }
        let open = self.cut.take().expect("checked above");
        if let Some(r) = &self.record {
            lock_any(r).write(&open.held_record);
        }
        if let Runner::Shared(backend) = &self.runner {
            let mut backend = lock_any(backend);
            for i in 0..open.held.len() {
                backend.execute(open.held.get(i));
            }
        }
    }

    /// Counts an event of the layer above for `NV_RENDER_THREAD_STATS` (an answer it gave without
    /// waiting for the render thread, say): printed a frame, by name. Nothing on the calling thread
    /// or without the stats.
    pub fn note(&mut self, what: &'static str) {
        if let Runner::Threaded(t) = &mut self.runner {
            t.stats.note(what);
        }
    }

    /// Hands what is recorded to the render thread now, without waiting.
    pub fn flush(&mut self) {
        if let Runner::Threaded(t) = &mut self.runner {
            t.hand_over(self.cut.is_some());
        }
    }

    /// The bytes and chunks handed over and not yet carried out (tests).
    #[cfg(test)]
    pub(crate) fn queued(&self) -> (usize, usize) {
        match &self.runner {
            Runner::Direct(_) | Runner::Shared(_) => (0, 0),
            Runner::Threaded(t) => {
                let q = lock(&t.shared.queue);
                (q.queued_bytes, q.chunks.len())
            }
        }
    }

    /// Returns once every command submitted is carried out (after an open cut's slot is closed).
    pub fn finish(&mut self) {
        if let Some(waiter) = self.cut_waiter() {
            waiter.wait();
            self.settle_cut();
        }
        if let Runner::Threaded(t) = &mut self.runner {
            t.hand_over(false);
            drop(t.wait(|q| q.done == q.handed));
        }
    }
}

/// Stands in for the backend while it moves into its shared place ([`Stream::cut`]).
struct Nothing;

impl Backend for Nothing {
    fn execute(&mut self, _: Cmd<Data<'_>>) -> Reply {
        Reply::None
    }
}

/// Waits for a cut's slot to be closed ([`Stream::cut_waiter`]).
#[derive(Clone)]
pub struct CutWaiter {
    id: u64,
    closed: Arc<AtomicU64>,
}

impl CutWaiter {
    /// Whether the slot is closed.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire) >= self.id
    }

    /// Returns once the slot is closed (a short sleep between looks: the drawing side's whole draw
    /// is waited for, a millisecond's lag does not matter).
    pub fn wait(&self) {
        while !self.is_closed() {
            std::thread::sleep(Duration::from_micros(200));
        }
    }
}

/// The drawing side's place in main's stream ([`Stream::cut`]): its commands are carried out
/// after main's before the cut and before main's after it. [`Slot::close`] (or dropping it) ends it.
pub struct Slot {
    cut: u64,
    side: SlotSide,
    record: Option<Arc<Mutex<Recording>>>,
    closed: Arc<AtomicU64>,
    cut_done: Arc<Mutex<Option<(u64, Instant)>>>,
    done: bool,
}

// One slot a cut, made once and kept where it was made: unboxed, as [`Runner`].
#[allow(clippy::large_enum_variant)]
enum SlotSide {
    /// The direct runner's backend: the slot's commands are carried out as they come.
    Direct(Arc<Mutex<Box<dyn Backend>>>),
    Threaded(SlotThreaded),
}

impl std::fmt::Debug for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slot").field("cut", &self.cut).finish()
    }
}

impl Slot {
    /// The cut's number (1, 2, ... in the run).
    pub fn cut(&self) -> u64 {
        self.cut
    }

    /// Carries out `cmd` in the slot or queues it, as [`Stream::submit`] does for main. A sync
    /// command's reply comes after every command before it in the serial order.
    ///
    /// # Panics
    /// On a [`Cmd::Present`]: the drawing side does not present.
    pub fn submit(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        #[cfg(debug_assertions)]
        if let Err(why) = cmd.check() {
            panic!("a malformed command: {why}");
        }
        assert!(
            !matches!(cmd, Cmd::Present { .. }),
            "a Present from the drawing side"
        );
        assert!(!self.done, "a command after the slot was closed");
        if let Some(r) = &self.record {
            lock_any(r).write_cmd(cmd);
        }
        match &mut self.side {
            SlotSide::Direct(backend) => lock_any(backend).execute(cmd),
            SlotSide::Threaded(t) => t.submit(cmd),
        }
    }

    /// The slot is complete: main's commands after the cut follow.
    pub fn close(mut self) {
        self.end();
    }

    fn end(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        match &mut self.side {
            SlotSide::Direct(_) => *lock_any(&self.cut_done) = Some((self.cut, Instant::now())),
            SlotSide::Threaded(t) => t.close(),
        }
        self.closed.store(self.cut, Ordering::Release);
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.end();
    }
}

/// What [`Stream::record_repeatable`] writes for `cmd`: the window handles as 0, a query read not
/// at all.
fn repeatable_form(cmd: Cmd<Data<'_>>) -> Option<Cmd<Data<'_>>> {
    Some(match cmd {
        Cmd::CreateDevice {
            device,
            behavior,
            present,
            back_buffers,
            depth_stencil,
            ..
        } => Cmd::CreateDevice {
            device,
            focus: 0,
            window: 0,
            behavior,
            present,
            back_buffers,
            depth_stencil,
        },
        Cmd::Reset {
            device,
            present,
            back_buffers,
            depth_stencil,
            ..
        } => Cmd::Reset {
            device,
            window: 0,
            present,
            back_buffers,
            depth_stencil,
        },
        Cmd::Present {
            device,
            source,
            dest,
            dirty,
            model,
            frame,
            ..
        } => Cmd::Present {
            device,
            source,
            dest,
            window: 0,
            dirty,
            model,
            frame,
        },
        Cmd::QueryGetData { .. } => return None,
        other => other,
    })
}

impl Drop for Stream {
    fn drop(&mut self) {
        // A slot closed meanwhile: main's held commands are carried out and written first.
        self.settle_cut();
        if let Some(r) = &self.record {
            let _ = lock_any(r).out.flush();
        }
    }
}

/// The recording side of the threaded runner.
struct Threaded {
    shared: Arc<Shared>,
    chunk: Chunk,
    settings: Settings,
    presents: u64,
    /// `check`: the hash of every command main recorded.
    hash: u64,
    encoded: Vec<u8>,
    thread: Option<std::thread::JoinHandle<()>>,
    stats: Stats,
}

/// What both sides share.
struct Shared {
    /// The render thread stopped (`Queue::dead` says why): read at every command, so the
    /// recording side stops at its next call.
    dead: AtomicBool,
    queue: Mutex<Queue>,
    /// The render thread waits on it for chunks.
    work: Condvar,
    /// The recording threads wait on it for the render thread's progress.
    progress: Condvar,
}

#[derive(Default)]
struct Queue {
    chunks: VecDeque<Chunk>,
    /// Emptied chunks to reuse.
    free: Vec<Chunk>,
    queued_bytes: usize,
    /// Chunks handed over, and carried out.
    handed: u64,
    done: u64,
    presents_done: u64,
    /// The last sync command's reply, until taken.
    reply: Option<Reply>,
    /// The cuts' slots not yet carried out, oldest first ([`Stream::cut`]): a slot closed but not
    /// yet reached may wait behind the next cut's.
    slots: VecDeque<SlotQueue>,
    /// The last cut whose slot was carried out, and when.
    cut_done: Option<(u64, Instant)>,
    /// The render thread's time carrying commands out.
    busy: Duration,
    closing: bool,
    /// Why the render thread stopped (its backend panicked).
    dead: Option<String>,
    renderer_sleeps: bool,
    /// Recording threads waiting for progress (main, the drawing side).
    waiters: u32,
}

/// One cut's slot on the queue: its chunks as the drawing side hands them over, their bytes,
/// whether it is closed, and the reply to its last sync command.
#[derive(Default)]
struct SlotQueue {
    cut: u64,
    chunks: VecDeque<Chunk>,
    bytes: usize,
    closed: bool,
    reply: Option<Reply>,
}

impl Queue {
    /// Cut `cut`'s slot (it is on the queue from the cut until it is carried out).
    fn slot(&mut self, cut: u64) -> &mut SlotQueue {
        self.slots
            .iter_mut()
            .find(|s| s.cut == cut)
            .expect("a slot is on the queue until it is carried out")
    }
}

/// Emptied chunks kept for reuse.
const FREE_CHUNKS: usize = 4;

/// How long the render thread sleeps between turns of a backend's idle work while it has more
/// ([`Backend::idle`]): a GPU answer is seen this soon after it comes in.
pub const IDLE_POLL: Duration = Duration::from_millis(1);

fn lock(m: &Mutex<Queue>) -> MutexGuard<'_, Queue> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The queue, once `ready` holds for it (panics when the render thread stopped).
fn wait_for(shared: &Shared, mut ready: impl FnMut(&Queue) -> bool) -> MutexGuard<'_, Queue> {
    let mut q = lock(&shared.queue);
    loop {
        stopped(&q);
        if ready(&q) {
            return q;
        }
        q.waiters += 1;
        q = shared
            .progress
            .wait(q)
            .unwrap_or_else(PoisonError::into_inner);
        q.waiters -= 1;
    }
}

impl Threaded {
    fn start(backend: Box<dyn Backend>, settings: Settings) -> Threaded {
        let shared = Arc::new(Shared {
            dead: AtomicBool::new(false),
            queue: Mutex::new(Queue::default()),
            work: Condvar::new(),
            progress: Condvar::new(),
        });
        let theirs = Arc::clone(&shared);
        let check = settings.check;
        let thread = std::thread::Builder::new()
            .name("nv-render".into())
            .spawn(move || render(&theirs, backend, check))
            .expect("the render thread starts");
        Threaded {
            shared,
            chunk: Chunk::default(),
            stats: Stats {
                on: settings.stats,
                ..Stats::default()
            },
            settings,
            presents: 0,
            hash: wire::HASH_START,
            encoded: Vec::new(),
            thread: Some(thread),
        }
    }

    /// `holding`: a cut is open (no full-queue wait, at least a frame in flight).
    fn submit(&mut self, cmd: Cmd<Data<'_>>, holding: bool) -> Reply {
        if self.shared.dead.load(Ordering::Relaxed) {
            stopped(&lock(&self.shared.queue));
        }
        if self.settings.check {
            self.encoded.clear();
            wire::encode(&cmd, &mut self.encoded);
            self.hash = wire::hash(self.hash, &self.encoded);
        }
        self.stats.commands += 1;
        let sync = cmd.is_sync();
        let present = matches!(cmd, Cmd::Present { .. });
        let name = cmd.name();
        self.chunk.push(cmd);
        if sync {
            self.chunk.reply_wanted = true;
            self.hand_over(holding);
            let t = Instant::now();
            let reply = self
                .wait(|q| q.reply.is_some())
                .reply
                .take()
                .expect("the reply waited for");
            self.stats.reply(name, t.elapsed());
            reply
        } else if present {
            self.presents += 1;
            self.hand_over(holding);
            // With a cut open, the render thread may be waiting at the cut for the drawing side:
            // this Present must not wait for itself.
            let in_flight = if holding {
                self.settings.frames_in_flight.max(1)
            } else {
                self.settings.frames_in_flight
            };
            let need = self.presents.saturating_sub(in_flight);
            let t = Instant::now();
            drop(self.wait(|q| q.presents_done >= need));
            self.stats.present.add(t.elapsed());
            self.stats.frame(&self.shared);
            Reply::None
        } else {
            if self.chunk.len() >= self.settings.chunk_commands
                || self.chunk.bytes >= self.settings.chunk_bytes
            {
                self.hand_over(holding);
            }
            Reply::None
        }
    }

    /// The chunk being recorded goes to the render thread (waking it if it sleeps); past the
    /// queue's limit, waits until the render thread has carried out half of it, except while a
    /// cut is open (`holding`): main's chunks after the cut wait for the drawing side, which must
    /// not wait for main.
    fn hand_over(&mut self, holding: bool) {
        // A stopped render thread stops the caller here even with nothing to hand over (a caller
        // polling for a result the render thread will never give would spin otherwise).
        if self.shared.dead.load(Ordering::Relaxed) {
            stopped(&lock(&self.shared.queue));
        }
        if self.chunk.is_empty() {
            return;
        }
        if self.settings.check {
            self.chunk.check = Some(self.hash);
        }
        let (wake, full) = {
            let mut q = lock(&self.shared.queue);
            stopped(&q);
            let next = q.free.pop().unwrap_or_default();
            let chunk = std::mem::replace(&mut self.chunk, next);
            q.queued_bytes += chunk.bytes;
            q.chunks.push_back(chunk);
            q.handed += 1;
            (
                q.renderer_sleeps,
                q.queued_bytes > self.settings.max_queued_bytes
                    || q.chunks.len() > self.settings.max_queued_chunks,
            )
        };
        self.stats.chunks += 1;
        if wake {
            self.shared.work.notify_one();
        }
        if full && !holding {
            let (bytes, chunks) = (
                self.settings.max_queued_bytes / 2,
                self.settings.max_queued_chunks / 2,
            );
            let t = Instant::now();
            drop(self.wait(|q| q.queued_bytes <= bytes && q.chunks.len() <= chunks));
            self.stats.full.add(t.elapsed());
        }
    }

    /// Main's cut: what is recorded goes over, then the cut's mark, which the render thread
    /// stops at to carry out the slot.
    fn cut(&mut self, id: u64) -> SlotThreaded {
        self.hand_over(false);
        {
            let mut q = lock(&self.shared.queue);
            stopped(&q);
            let mut mark = q.free.pop().unwrap_or_default();
            mark.cut = Some(id);
            if self.settings.check {
                mark.check = Some(self.hash);
            }
            q.chunks.push_back(mark);
            q.handed += 1;
            q.slots.push_back(SlotQueue {
                cut: id,
                ..SlotQueue::default()
            });
        }
        self.shared.work.notify_one();
        SlotThreaded {
            cut: id,
            shared: Arc::clone(&self.shared),
            chunk: Chunk::default(),
            settings: self.settings.clone(),
            hash: wire::HASH_START,
            encoded: Vec::new(),
        }
    }

    fn wait(&self, ready: impl FnMut(&Queue) -> bool) -> MutexGuard<'_, Queue> {
        wait_for(&self.shared, ready)
    }
}

/// The drawing side's recording into the threaded runner's slot.
struct SlotThreaded {
    cut: u64,
    shared: Arc<Shared>,
    chunk: Chunk,
    settings: Settings,
    /// `check`: the hash of the slot's commands (from the cut on).
    hash: u64,
    encoded: Vec<u8>,
}

impl SlotThreaded {
    fn submit(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        if self.shared.dead.load(Ordering::Relaxed) {
            stopped(&lock(&self.shared.queue));
        }
        if self.settings.check {
            self.encoded.clear();
            wire::encode(&cmd, &mut self.encoded);
            self.hash = wire::hash(self.hash, &self.encoded);
        }
        let sync = cmd.is_sync();
        self.chunk.push(cmd);
        if sync {
            self.chunk.reply_wanted = true;
            self.hand_over();
            let cut = self.cut;
            wait_for(&self.shared, |q| {
                q.slots.iter().any(|s| s.cut == cut && s.reply.is_some())
            })
            .slot(cut)
            .reply
            .take()
            .expect("the reply waited for")
        } else {
            if self.chunk.len() >= self.settings.chunk_commands
                || self.chunk.bytes >= self.settings.chunk_bytes
            {
                self.hand_over();
            }
            Reply::None
        }
    }

    /// The slot's chunk goes to the render thread; past the queue's limit, waits until the render
    /// thread has carried out half of the slot's (it does once it reaches the cut, which needs
    /// nothing of main's after it).
    fn hand_over(&mut self) {
        if self.shared.dead.load(Ordering::Relaxed) {
            stopped(&lock(&self.shared.queue));
        }
        if self.chunk.is_empty() {
            return;
        }
        if self.settings.check {
            self.chunk.check = Some(self.hash);
        }
        let (wake, full) = {
            let mut q = lock(&self.shared.queue);
            stopped(&q);
            let next = q.free.pop().unwrap_or_default();
            let chunk = std::mem::replace(&mut self.chunk, next);
            let renderer_sleeps = q.renderer_sleeps;
            let slot = q.slot(self.cut);
            slot.bytes += chunk.bytes;
            slot.chunks.push_back(chunk);
            (
                renderer_sleeps,
                slot.bytes > self.settings.max_queued_bytes
                    || slot.chunks.len() > self.settings.max_queued_chunks,
            )
        };
        if wake {
            self.shared.work.notify_one();
        }
        if full {
            let (bytes, chunks) = (
                self.settings.max_queued_bytes / 2,
                self.settings.max_queued_chunks / 2,
            );
            let cut = self.cut;
            drop(wait_for(&self.shared, |q| {
                q.slots
                    .iter()
                    .find(|s| s.cut == cut)
                    .map_or(true, |s| s.bytes <= bytes && s.chunks.len() <= chunks)
            }));
        }
    }

    fn close(&mut self) {
        // A stopped render thread: nothing more to hand over (and no panic in a drop).
        if !self.shared.dead.load(Ordering::Relaxed) {
            self.hand_over();
        }
        // Not found only after the render thread stopped (its slots dropped).
        if let Some(slot) = lock(&self.shared.queue)
            .slots
            .iter_mut()
            .find(|s| s.cut == self.cut)
        {
            slot.closed = true;
        }
        self.shared.work.notify_one();
    }
}

/// Stops the recording thread with the render thread's reason when it stopped.
fn stopped(q: &Queue) {
    if let Some(why) = &q.dead {
        panic!("the render thread stopped: {why}");
    }
}

impl Drop for Threaded {
    /// What is recorded is carried out, then the render thread ends.
    fn drop(&mut self) {
        {
            let mut q = lock(&self.shared.queue);
            if q.dead.is_none() && !self.chunk.is_empty() {
                if self.settings.check {
                    self.chunk.check = Some(self.hash);
                }
                let chunk = std::mem::take(&mut self.chunk);
                q.queued_bytes += chunk.bytes;
                q.chunks.push_back(chunk);
                q.handed += 1;
            }
            q.closing = true;
        }
        self.shared.work.notify_one();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Carries `chunk`'s commands out on `backend`; its reply (the last command's). `hash`: the
/// carried-out hash of its side, checked against the chunk's recorded one.
fn carry_out(chunk: &Chunk, backend: &mut dyn Backend, check: bool, hash: &mut u64) -> Reply {
    let mut encoded = Vec::new();
    let mut last = Reply::None;
    for i in 0..chunk.len() {
        let cmd = chunk.get(i);
        if check {
            encoded.clear();
            wire::encode(&cmd, &mut encoded);
            *hash = wire::hash(*hash, &encoded);
        }
        last = backend.execute(cmd);
    }
    if chunk.check.is_some_and(|recorded| recorded != *hash) {
        panic!("NV_D3D9_STREAM_CHECK: the commands carried out differ from the commands recorded");
    }
    last
}

/// The render thread stops: `panic`'s message is the recording side's reason.
fn die(shared: &Shared, mut q: MutexGuard<'_, Queue>, panic: &(dyn std::any::Any + Send)) {
    q.dead = Some(panic_message(panic));
    q.chunks.clear();
    q.slots.clear();
    shared.dead.store(true, Ordering::Relaxed);
    drop(q);
    shared.progress.notify_all();
}

/// The render thread: carries the chunks out in order until the stream closes, a cut's slot where
/// its mark is. A panic of the backend is kept as the reason the recording side stops with.
fn render(shared: &Shared, mut backend: Box<dyn Backend>, check: bool) {
    let mut hash = wire::HASH_START;
    loop {
        let mut chunk = {
            let mut q = lock(&shared.queue);
            loop {
                if let Some(chunk) = q.chunks.pop_front() {
                    break chunk;
                }
                if q.closing {
                    return;
                }
                // Nothing queued: the backend's idle work (outside the lock), then a sleep until
                // the next chunk, or a short one while the backend has more of it.
                drop(q);
                let idle =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| backend.idle()));
                q = lock(&shared.queue);
                let more = match idle {
                    Ok(again) => again,
                    Err(panic) => return die(shared, q, &*panic),
                };
                if !q.chunks.is_empty() || q.closing {
                    continue;
                }
                q.renderer_sleeps = true;
                q = if more {
                    shared
                        .work
                        .wait_timeout(q, IDLE_POLL)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0
                } else {
                    shared.work.wait(q).unwrap_or_else(PoisonError::into_inner)
                };
                q.renderer_sleeps = false;
            }
        };
        if let Some(cut) = chunk.cut {
            if !carry_out_slot(shared, &mut *backend, check, cut) {
                return;
            }
        }
        let started = Instant::now();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            carry_out(&chunk, &mut *backend, check, &mut hash)
        }));
        let busy = started.elapsed();
        let mut q = lock(&shared.queue);
        q.busy += busy;
        match outcome {
            Ok(reply) => {
                q.queued_bytes -= chunk.bytes;
                q.presents_done += chunk.presents;
                if chunk.reply_wanted {
                    q.reply = Some(reply);
                }
                q.done += 1;
                chunk.clear();
                if q.free.len() < FREE_CHUNKS {
                    q.free.push(chunk);
                }
            }
            Err(panic) => return die(shared, q, &*panic),
        }
        let wake = q.waiters > 0;
        drop(q);
        if wake {
            shared.progress.notify_all();
        }
    }
}

/// At a cut's mark: the slot's chunks carried out as the drawing side hands them over, until it
/// is closed. False when the render thread stopped (a panic) or the stream closed meanwhile.
fn carry_out_slot(shared: &Shared, backend: &mut dyn Backend, check: bool, cut: u64) -> bool {
    let mut hash = wire::HASH_START;
    loop {
        let mut chunk = {
            let mut q = lock(&shared.queue);
            loop {
                let closing = q.closing;
                let slot = q
                    .slots
                    .front_mut()
                    .filter(|s| s.cut == cut)
                    .expect("the cut's slot is the oldest on the queue at its mark");
                if let Some(chunk) = slot.chunks.pop_front() {
                    break chunk;
                }
                if slot.closed || closing {
                    q.slots.pop_front();
                    q.cut_done = Some((cut, Instant::now()));
                    let wake = q.waiters > 0;
                    drop(q);
                    if wake {
                        shared.progress.notify_all();
                    }
                    return true;
                }
                q.renderer_sleeps = true;
                q = shared.work.wait(q).unwrap_or_else(PoisonError::into_inner);
                q.renderer_sleeps = false;
            }
        };
        let started = Instant::now();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            carry_out(&chunk, backend, check, &mut hash)
        }));
        let busy = started.elapsed();
        let mut q = lock(&shared.queue);
        q.busy += busy;
        match outcome {
            Ok(reply) => {
                let slot = q.slot(cut);
                slot.bytes -= chunk.bytes;
                if chunk.reply_wanted {
                    slot.reply = Some(reply);
                }
                chunk.clear();
                if q.free.len() < FREE_CHUNKS {
                    q.free.push(chunk);
                }
            }
            Err(panic) => {
                die(shared, q, &*panic);
                return false;
            }
        }
        let wake = q.waiters > 0;
        drop(q);
        if wake {
            shared.progress.notify_all();
        }
    }
}

/// A panic's message.
pub(crate) fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "a panic without a message".to_string()
    }
}

/// `NV_RENDER_THREAD_STATS`: counted on the recording side, printed once a second.
#[derive(Default)]
struct Stats {
    on: bool,
    since: Option<Instant>,
    busy_before: Duration,
    frames: u64,
    commands: u64,
    chunks: u64,
    present: Waits,
    /// The waits for replies, by command.
    replies: Vec<(&'static str, Waits)>,
    /// The layer's events ([`Stream::note`]), by name.
    events: Vec<(&'static str, u64)>,
    full: Waits,
}

/// Time spent waiting, and how many waits.
#[derive(Default)]
struct Waits {
    count: u64,
    time: Duration,
}

impl Waits {
    fn add(&mut self, t: Duration) {
        self.count += 1;
        self.time += t;
    }

    fn per_frame(&self, frames: f64) -> String {
        format!(
            "{:.3} ms ({:.2})",
            self.time.as_secs_f64() * 1e3 / frames,
            self.count as f64 / frames
        )
    }
}

impl Stats {
    fn note(&mut self, what: &'static str) {
        if !self.on {
            return;
        }
        match self.events.iter_mut().find(|(n, _)| *n == what) {
            Some((_, count)) => *count += 1,
            None => self.events.push((what, 1)),
        }
    }

    fn reply(&mut self, name: &'static str, t: Duration) {
        if !self.on {
            return;
        }
        match self.replies.iter_mut().find(|(n, _)| *n == name) {
            Some((_, w)) => w.add(t),
            None => {
                let mut w = Waits::default();
                w.add(t);
                self.replies.push((name, w));
            }
        }
    }

    fn frame(&mut self, shared: &Shared) {
        if !self.on {
            return;
        }
        self.frames += 1;
        let now = Instant::now();
        let since = *self.since.get_or_insert(now);
        let elapsed = now - since;
        if elapsed < Duration::from_secs(1) {
            return;
        }
        let busy = lock(&shared.queue).busy;
        let frames = self.frames as f64;
        let replies = if self.replies.is_empty() {
            "none".to_string()
        } else {
            self.replies
                .iter()
                .map(|(n, w)| format!("{n} {}", w.per_frame(frames)))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let events: String = self
            .events
            .iter()
            .map(|(n, count)| format!("; {n} {:.2} a frame", *count as f64 / frames))
            .collect();
        eprintln!(
            "NV_RENDER_THREAD: {:.1} frames/s; a frame: {:.0} commands in {:.1} chunks, render \
             thread busy {:.3} ms; the recording thread waited a frame: at Present {}, on a full \
             queue {}, for replies: {replies}{events}",
            frames / elapsed.as_secs_f64(),
            self.commands as f64 / frames,
            self.chunks as f64 / frames,
            (busy - self.busy_before).as_secs_f64() * 1e3 / frames,
            self.present.per_frame(frames),
            self.full.per_frame(frames),
        );
        *self = Stats {
            on: true,
            since: Some(now),
            busy_before: busy,
            ..Stats::default()
        };
    }
}
