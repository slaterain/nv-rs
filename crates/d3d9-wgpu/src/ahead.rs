//! Work done ahead on a few threads below normal priority ([`Queue`]): the shader modules the
//! device makes when the game creates its shaders, before any draw needs them (FAST-LINK step
//! 1A). A job waits in the queue under its key. A draw that needs one before a thread got to it
//! takes it back and does it itself; one a thread is doing is waited for, alone. Nothing is
//! dropped while the queue lives: every job pushed is done once, by a thread or by the caller
//! that took it back, and its result handed out once.

use std::collections::{HashSet, VecDeque};
use std::hash::Hash;
use std::sync::{mpsc, Arc, Condvar, Mutex};

/// A job: done once, on a worker thread or on the thread that takes it back.
pub(crate) type Job<R> = Box<dyn FnOnce() -> R + Send>;

/// What [`Queue::take`] found for a key.
pub(crate) enum Taken<R> {
    /// The job was still queued: taken back, for the caller to do now.
    Job(Job<R>),
    /// A thread did it (or was doing it, and the caller waited for that one alone).
    Done(R),
    /// Not pushed, or its result already handed out; or its job panicked on a thread.
    None,
}

struct Shared<K, R> {
    queued: Mutex<Queued<K, R>>,
    work: Condvar,
    /// The threads at normal priority, not below it ([`Queue::boost`]).
    boosted: std::sync::atomic::AtomicBool,
}

struct Queued<K, R> {
    jobs: VecDeque<(K, Job<R>)>,
    stop: bool,
    /// No job started while set ([`Queue::hold`]; tests).
    held: bool,
}

/// Jobs by key, done on worker threads in the order pushed.
pub(crate) struct Queue<K, R> {
    shared: Arc<Shared<K, R>>,
    results: mpsc::Receiver<(K, Option<R>)>,
    /// Results received while waiting for another key, handed out by [`Queue::finished`].
    early: Vec<(K, Option<R>)>,
    /// Every key pushed whose result has not been handed out.
    pending: HashSet<K>,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl<K, R> Queue<K, R>
where
    K: Eq + Hash + Clone + Send + 'static,
    R: Send + 'static,
{
    /// A queue with `threads` worker threads (at least one), below normal priority, named
    /// `name`.
    pub(crate) fn new(threads: usize, name: &str) -> Queue<K, R> {
        let shared = Arc::new(Shared {
            queued: Mutex::new(Queued {
                jobs: VecDeque::new(),
                stop: false,
                held: false,
            }),
            work: Condvar::new(),
            boosted: std::sync::atomic::AtomicBool::new(false),
        });
        let (send, results) = mpsc::channel();
        let threads = (0..threads.max(1))
            .filter_map(|_| {
                let shared = shared.clone();
                let send = send.clone();
                std::thread::Builder::new()
                    .name(name.to_string())
                    .spawn(move || work(&shared, &send))
                    .ok()
            })
            .collect();
        Queue {
            shared,
            results,
            early: Vec::new(),
            pending: HashSet::new(),
            threads,
        }
    }

    /// Queues `job` under `key`, unless a job of that key is pending; whether it was queued.
    pub(crate) fn push(&mut self, key: K, job: Job<R>) -> bool {
        if self.threads.is_empty() || !self.pending.insert(key.clone()) {
            return false;
        }
        lock(&self.shared.queued).jobs.push_back((key, job));
        self.shared.work.notify_one();
        true
    }

    /// Whether a job of `key` was pushed and its result not handed out yet.
    pub(crate) fn has(&self, key: &K) -> bool {
        self.pending.contains(key)
    }

    /// How many jobs were pushed whose results are not handed out yet.
    pub(crate) fn pending(&self) -> usize {
        self.pending.len()
    }

    /// The job of `key` for the caller to do now, if no thread has started it; else its result,
    /// waiting for that job alone if a thread is still doing it (results that come in meanwhile
    /// are kept for [`Queue::finished`]).
    pub(crate) fn take(&mut self, key: &K) -> Taken<R> {
        if !self.pending.contains(key) {
            return Taken::None;
        }
        if let Some(job) = self.take_queued(key) {
            return Taken::Job(job);
        }
        let result = match self.early.iter().position(|(k, _)| k == key) {
            Some(i) => self.early.swap_remove(i).1,
            None => {
                self.boost(true);
                let r = loop {
                    match self.results.recv() {
                        Ok((k, r)) if &k == key => break r,
                        Ok(other) => self.early.push(other),
                        // Every thread gone: no result will come.
                        Err(_) => break None,
                    }
                };
                self.boost(false);
                r
            }
        };
        self.pending.remove(key);
        result.map_or(Taken::None, Taken::Done)
    }

    /// No thread starts a job while `on` (the jobs a thread is doing go on); then they go on
    /// (tests: a caller that needs a job no thread has started takes it back).
    pub(crate) fn hold(&self, on: bool) {
        lock(&self.shared.queued).held = on;
        if !on {
            self.shared.work.notify_all();
        }
    }

    /// The job of `key` if no thread has started it, taken back for the caller to do; else
    /// none, without waiting (a thread's job stays with it).
    pub(crate) fn take_queued(&mut self, key: &K) -> Option<Job<R>> {
        let mut q = lock(&self.shared.queued);
        let i = q.jobs.iter().position(|(k, _)| k == key)?;
        let (_, job) = q.jobs.remove(i)?;
        drop(q);
        self.pending.remove(key);
        Some(job)
    }

    /// Every job no thread has started, taken back for the caller to do (the game's start does
    /// them on every core).
    pub(crate) fn take_all_queued(&mut self) -> Vec<(K, Job<R>)> {
        let jobs: Vec<(K, Job<R>)> = lock(&self.shared.queued).jobs.drain(..).collect();
        for (k, _) in &jobs {
            self.pending.remove(k);
        }
        jobs
    }

    /// The results done so far, without waiting (a job that panicked on a thread gives none).
    pub(crate) fn finished(&mut self) -> Vec<(K, R)> {
        let mut out = Vec::new();
        let early = std::mem::take(&mut self.early);
        for (k, r) in early.into_iter().chain(self.results.try_iter()) {
            self.pending.remove(&k);
            if let Some(r) = r {
                out.push((k, r));
            }
        }
        out
    }

    /// Every result, waiting until no job is pending.
    pub(crate) fn wait_all(&mut self) -> Vec<(K, R)> {
        let mut out = self.finished();
        if self.pending.is_empty() {
            return out;
        }
        self.boost(true);
        while !self.pending.is_empty() {
            match self.results.recv() {
                Ok((k, r)) => {
                    self.pending.remove(&k);
                    if let Some(r) = r {
                        out.push((k, r));
                    }
                }
                Err(_) => break,
            }
        }
        self.boost(false);
        out
    }

    /// The threads at normal priority while a caller waits on them (`true`), or back below it:
    /// a waiting caller (a draw, the game's start) is not held to their low priority. A thread
    /// doing a job changes at once; one waiting for a job, from its next.
    pub(crate) fn boost(&self, on: bool) {
        self.shared
            .boosted
            .store(on, std::sync::atomic::Ordering::Relaxed);
        #[cfg(windows)]
        for t in &self.threads {
            use std::os::windows::io::AsRawHandle;
            set_priority(Some(t.as_raw_handle() as isize), on);
        }
    }

    /// Each thread's priority now (`THREAD_PRIORITY_*`: 0 normal, -1 below normal; tests).
    #[cfg(windows)]
    pub(crate) fn priorities(&self) -> Vec<i32> {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetThreadPriority(thread: isize) -> i32;
        }
        use std::os::windows::io::AsRawHandle;
        self.threads
            .iter()
            // SAFETY: a handle of a thread the queue holds (joined only when it is dropped).
            .map(|t| unsafe { GetThreadPriority(t.as_raw_handle() as isize) })
            .collect()
    }
}

impl<K, R> Drop for Queue<K, R> {
    /// The jobs not started are dropped; the threads finish the ones they are doing and end.
    fn drop(&mut self) {
        {
            let mut q = lock(&self.shared.queued);
            q.stop = true;
            q.jobs.clear();
        }
        self.shared.work.notify_all();
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// A worker thread: jobs in the order pushed until the queue stops. A job that panics gives
/// no result (its key's taker then makes its own).
fn work<K, R>(shared: &Shared<K, R>, send: &mpsc::Sender<(K, Option<R>)>) {
    lower_priority();
    let mut normal = false;
    loop {
        let (key, job) = {
            let mut q = lock(&shared.queued);
            loop {
                if q.stop {
                    return;
                }
                if !q.held {
                    if let Some(next) = q.jobs.pop_front() {
                        break next;
                    }
                }
                q = shared.work.wait(q).unwrap_or_else(|e| e.into_inner());
            }
        };
        let boosted = shared.boosted.load(std::sync::atomic::Ordering::Relaxed);
        if boosted != normal {
            set_priority(None, boosted);
            normal = boosted;
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)).ok();
        if send.send((key, result)).is_err() {
            return;
        }
    }
}

/// The calling thread below normal priority, so the game's own threads keep theirs.
pub(crate) fn lower_priority() {
    set_priority(None, false);
}

/// A thread at normal priority (`normal`) or below it: the thread of `handle`, or the calling
/// thread.
fn set_priority(handle: Option<isize>, normal: bool) {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentThread() -> isize;
            fn SetThreadPriority(thread: isize, priority: i32) -> i32;
        }
        // `THREAD_PRIORITY_NORMAL` and `THREAD_PRIORITY_BELOW_NORMAL`.
        let value = if normal { 0 } else { -1 };
        // SAFETY: a handle of a thread the queue holds (joined only when it is dropped) or the
        // calling thread's pseudo handle, and a documented priority value.
        unsafe {
            let thread = match handle {
                Some(h) => h,
                None => GetCurrentThread(),
            };
            SetThreadPriority(thread, value);
        }
    }
    #[cfg(not(windows))]
    let _ = (handle, normal);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Every job pushed is done once on the threads and its result handed out once; a key
    /// pushed again while pending is not queued twice.
    #[test]
    fn every_job_is_done_once() {
        let runs = Arc::new(AtomicU32::new(0));
        let mut q: Queue<u32, u32> = Queue::new(3, "test");
        for k in 0..50 {
            let runs = runs.clone();
            assert!(q.push(
                k,
                Box::new(move || {
                    runs.fetch_add(1, Ordering::Relaxed);
                    k * 2
                })
            ));
        }
        assert!(!q.push(7, Box::new(|| 0)), "7 is pending");
        let mut got = q.wait_all();
        got.sort_unstable();
        assert_eq!(got, (0..50).map(|k| (k, k * 2)).collect::<Vec<_>>());
        assert_eq!(runs.load(Ordering::Relaxed), 50);
        assert!(!q.has(&7));
        assert!(matches!(q.take(&7), Taken::None));
    }

    /// Every job no thread has started is taken back at once, each once, the one a thread is
    /// doing left to it.
    #[test]
    fn every_job_no_thread_started_is_taken_back() {
        let gate = gate();
        let started = Arc::new(AtomicU32::new(0));
        let mut q: Queue<u32, u32> = Queue::new(1, "test");
        let (g, s) = (gate.clone(), started.clone());
        q.push(
            1,
            Box::new(move || {
                s.store(1, Ordering::SeqCst);
                wait_at(&g);
                10
            }),
        );
        for k in 2..6 {
            q.push(k, Box::new(move || k * 10));
        }
        while started.load(Ordering::SeqCst) == 0 {
            std::thread::yield_now();
        }
        let mut back: Vec<(u32, u32)> = q
            .take_all_queued()
            .into_iter()
            .map(|(k, job)| (k, job()))
            .collect();
        back.sort_unstable();
        assert_eq!(back, vec![(2, 20), (3, 30), (4, 40), (5, 50)]);
        assert_eq!(q.pending(), 1, "job 1 still with its thread");
        assert!(q.take_all_queued().is_empty());
        open(&gate);
        assert_eq!(q.wait_all(), vec![(1, 10)]);
    }

    /// A gate a job waits at until it is opened.
    type Gate = Arc<(Mutex<bool>, Condvar)>;

    fn gate() -> Gate {
        Arc::new((Mutex::new(false), Condvar::new()))
    }

    fn wait_at(g: &Gate) {
        let (open, cv) = &**g;
        let mut o = open.lock().unwrap();
        while !*o {
            o = cv.wait(o).unwrap();
        }
    }

    fn open(g: &Gate) {
        let (open, cv) = &**g;
        *open.lock().unwrap() = true;
        cv.notify_all();
    }

    /// `f` of the queue on a helper thread, the queue handed back: a call that waits for a job
    /// held at a gate, or for a result that was lost, fails the test after 10 s instead of
    /// hanging it.
    fn in_time<T: Send + 'static>(
        q: Queue<u32, u32>,
        what: &str,
        f: fn(&mut Queue<u32, u32>) -> T,
    ) -> (Queue<u32, u32>, T) {
        let (send, back) = mpsc::channel();
        std::thread::spawn(move || {
            let mut q = q;
            let r = f(&mut q);
            let _ = send.send((q, r));
        });
        back.recv_timeout(std::time::Duration::from_secs(10))
            .unwrap_or_else(|_| panic!("{what}: still waiting after 10 s"))
    }

    /// A result that comes in while `take` waits for another key is kept: `has` still names
    /// it, and `take` of its key and `finished` hand it out later, once each. And `take` waits
    /// for its own job alone: job 1's result comes while job 4 is still held.
    #[test]
    fn results_that_come_while_take_waits_are_kept_for_later() {
        let (a, b) = (gate(), gate());
        let started = Arc::new(AtomicU32::new(0));
        let mut q: Queue<u32, u32> = Queue::new(2, "test");
        // One thread does job 1 (held at gate A); the other 2, 5, then 3 (which opens gate A),
        // then 4 (held at gate B).
        let (a1, s1) = (a.clone(), started.clone());
        q.push(
            1,
            Box::new(move || {
                s1.store(1, Ordering::SeqCst);
                wait_at(&a1);
                10
            }),
        );
        q.push(2, Box::new(|| 20));
        q.push(5, Box::new(|| 50));
        let a3 = a.clone();
        q.push(
            3,
            Box::new(move || {
                open(&a3);
                30
            }),
        );
        let b4 = b.clone();
        q.push(
            4,
            Box::new(move || {
                wait_at(&b4);
                40
            }),
        );
        // Job 1 has left the queue: `take` waits for it rather than taking it back.
        while started.load(Ordering::SeqCst) == 0 {
            std::thread::yield_now();
        }
        let (q, one) = in_time(q, "take(&1) with job 4 held", |q| match q.take(&1) {
            Taken::Done(r) => Some(r),
            _ => None,
        });
        // Job 4 goes on now, so a failed check below drops the queue (which waits for its
        // threads) without waiting for gate B.
        open(&b);
        assert_eq!(one, Some(10));
        assert!(q.has(&2), "job 2's result is kept");
        let (mut q, two) = in_time(q, "take(&2) of a kept result", |q| match q.take(&2) {
            Taken::Done(r) => Some(r),
            _ => None,
        });
        assert_eq!(two, Some(20));
        let mut rest = q.finished();
        assert!(rest.contains(&(5, 50)), "job 5's kept result: {rest:?}");
        let (mut q, all) = in_time(q, "wait_all", Queue::wait_all);
        rest.extend(all);
        rest.extend(q.finished());
        rest.sort_unstable();
        assert_eq!(rest, vec![(3, 30), (4, 40), (5, 50)], "each once");
        assert_eq!(q.pending(), 0);
    }

    /// While a caller waits on the threads (`take` of a job a thread is doing, `wait_all`), they
    /// run at normal priority; below it again after.
    #[cfg(windows)]
    #[test]
    fn the_threads_a_caller_waits_on_run_at_normal_priority() {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentThread() -> isize;
            fn GetThreadPriority(thread: isize) -> i32;
        }
        fn now() -> i32 {
            // SAFETY: the calling thread's pseudo handle.
            unsafe { GetThreadPriority(GetCurrentThread()) }
        }
        // A job that waits (10 s at most) until its thread runs at normal priority, and gives
        // the priority it ends with.
        fn until_normal() -> i32 {
            let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while now() != 0 && std::time::Instant::now() < end {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            now()
        }
        let started = Arc::new(AtomicU32::new(0));
        let mut q: Queue<u32, i32> = Queue::new(1, "test");
        let s = started.clone();
        q.push(
            1,
            Box::new(move || {
                s.store(1, Ordering::SeqCst);
                until_normal()
            }),
        );
        while started.load(Ordering::SeqCst) == 0 {
            std::thread::yield_now();
        }
        assert!(matches!(q.take(&1), Taken::Done(0)), "boosted for take");
        q.push(2, Box::new(now));
        let mut got = Vec::new();
        while got.is_empty() {
            got = q.finished();
            std::thread::yield_now();
        }
        assert_eq!(got, vec![(2, -1)], "below normal after");
        q.push(3, Box::new(until_normal));
        assert_eq!(q.wait_all(), vec![(3, 0)], "boosted for wait_all");
    }

    /// A job still queued (its one thread busy) is taken back for the caller to do; one a thread
    /// is doing is waited for alone, and a result that comes in meanwhile is kept for later.
    #[test]
    fn a_needed_job_is_taken_back_or_waited_for_alone() {
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let mut q: Queue<u32, u32> = Queue::new(1, "test");
        // Job 1 holds the one thread until the gate opens.
        let g = gate.clone();
        q.push(
            1,
            Box::new(move || {
                let (open, cv) = &*g;
                let mut o = open.lock().unwrap();
                while !*o {
                    o = cv.wait(o).unwrap();
                }
                10
            }),
        );
        q.push(2, Box::new(|| 20));
        q.push(3, Box::new(|| 30));
        // Job 3 has not started (the thread is in job 1): taken back.
        match q.take(&3) {
            Taken::Job(job) => assert_eq!(job(), 30),
            _ => panic!("job 3 should come back"),
        }
        assert!(!q.has(&3));
        // Open the gate from another thread a little later; job 1 is then waited for alone
        // (job 2, done after it, is handed out by `finished`).
        let g = gate.clone();
        let opener = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let (open, cv) = &*g;
            *open.lock().unwrap() = true;
            cv.notify_all();
        });
        // Job 1 is running on the thread now (or about to): not in the queue.
        while lock(&q.shared.queued).jobs.iter().any(|(k, _)| *k == 1) {
            std::thread::yield_now();
        }
        match q.take(&1) {
            Taken::Done(r) => assert_eq!(r, 10),
            _ => panic!("job 1's result"),
        }
        opener.join().unwrap();
        assert_eq!(q.wait_all(), vec![(2, 20)]);
    }

    /// A job that panics on a thread gives no result: its key is no longer pending and a taker
    /// gets nothing (and makes its own); the thread goes on with the next job.
    #[test]
    fn a_job_that_panics_gives_no_result() {
        let mut q: Queue<u32, u32> = Queue::new(1, "test");
        q.push(
            1,
            Box::new(|| panic!("a job's panic (expected by the test)")),
        );
        q.push(2, Box::new(|| 2));
        let got = q.wait_all();
        assert_eq!(got, vec![(2, 2)]);
        assert!(!q.has(&1));
    }

    /// Dropping the queue ends its threads promptly, with jobs left in it.
    #[test]
    fn dropping_the_queue_ends_its_threads() {
        let mut q: Queue<u32, u32> = Queue::new(2, "test");
        for k in 0..100 {
            q.push(
                k,
                Box::new(move || {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    k
                }),
            );
        }
        let t = std::time::Instant::now();
        drop(q);
        assert!(t.elapsed() < std::time::Duration::from_secs(1));
    }
}
