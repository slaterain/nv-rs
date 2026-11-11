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
}

struct Queued<K, R> {
    jobs: VecDeque<(K, Job<R>)>,
    stop: bool,
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
            }),
            work: Condvar::new(),
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
        {
            let mut q = lock(&self.shared.queued);
            if let Some(i) = q.jobs.iter().position(|(k, _)| k == key) {
                let (_, job) = q.jobs.remove(i).expect("found above");
                drop(q);
                self.pending.remove(key);
                return Taken::Job(job);
            }
        }
        let result = match self.early.iter().position(|(k, _)| k == key) {
            Some(i) => self.early.swap_remove(i).1,
            None => loop {
                match self.results.recv() {
                    Ok((k, r)) if &k == key => break r,
                    Ok(other) => self.early.push(other),
                    // Every thread gone: no result will come.
                    Err(_) => break None,
                }
            },
        };
        self.pending.remove(key);
        result.map_or(Taken::None, Taken::Done)
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

    /// Every result, waiting until no job is pending (tests).
    pub(crate) fn wait_all(&mut self) -> Vec<(K, R)> {
        let mut out = self.finished();
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
        out
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
    loop {
        let (key, job) = {
            let mut q = lock(&shared.queued);
            loop {
                if q.stop {
                    return;
                }
                if let Some(next) = q.jobs.pop_front() {
                    break next;
                }
                q = shared.work.wait(q).unwrap_or_else(|e| e.into_inner());
            }
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)).ok();
        if send.send((key, result)).is_err() {
            return;
        }
    }
}

/// The calling thread below normal priority, so the game's own threads keep theirs.
pub(crate) fn lower_priority() {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentThread() -> isize;
            fn SetThreadPriority(thread: isize, priority: i32) -> i32;
        }
        /// `THREAD_PRIORITY_BELOW_NORMAL`.
        const BELOW_NORMAL: i32 = -1;
        // SAFETY: the current thread's pseudo handle and a documented priority value.
        unsafe {
            SetThreadPriority(GetCurrentThread(), BELOW_NORMAL);
        }
    }
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
