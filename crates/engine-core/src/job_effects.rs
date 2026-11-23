//! Effects of the game's physics callbacks recorded inside Havok's parallel jobs and replayed in a
//! fixed order on the stepping thread (the multithreaded physics step; `engine-havok`'s
//! `hkCpuAgentSectorJob` and step functions, `engine-fallout-env`'s collision listener).
//!
//! With the multithreaded step Havok runs the narrow phase in jobs (`hkCpuAgentSectorJob`,
//! `00d93380`) on its worker threads and on the stepping thread at once, and every new contact
//! point calls the world's collision listeners from inside its job. The game's listener
//! (`FOCollisionListener`, `00623cb0`) decides there whether Havok keeps the point (the event's
//! reject word), which has to stay in the job, and starts effects that were never made for several
//! threads: the collision sound (`ImpactMixer::PlayCollisionSound`, `00837550`, which drops a sound
//! heard again within a short interval, so the first collision of a kind wins), the collision damage
//! queued on the listener's list (`006238b0`), and a projectile's impact (its slot `0x32c`, where the
//! first impact decides the hit).
//!
//! Inside a job those effects are recorded here with the job's place in the step ([`JobKey`]: the
//! island's position among the world's active islands, the job's kind and its first sector or part)
//! and their order in the job. The stepping thread replays them ([`replay`]) once every job of the
//! batch has ended, sorted by that key: the continuous step works the active islands in that order
//! and each island's agents sector by sector, so the replayed order is the continuous step's for the
//! contacts both steps find, and it is the same on every run whichever thread ran which job.
//! Outside a job (the shipped continuous step, and the new pairs the multithreaded step runs on the
//! stepping thread) nothing is recorded and effects run at once, as before.
//!
//! What stays in the job only reads the game's objects (the bodies' references, kinds, materials,
//! the hit reference's answers) and writes the job's own event and stack; what is recorded writes
//! the sound mixer, the damage list and the projectile, which no decision in a job reads.

use crate::Engine;
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// A job's place in the step: the island's position among the world's active islands (its
/// halfword `+0x20`), the job's kind (its type byte: 0xD the sectors of an island's agents, 0xF a
/// list of agents), and its first sector (0xD) or its part of the list (0xF). Sorted in that order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobKey {
    pub island: u16,
    pub kind: u8,
    pub part: u32,
}

/// One recorded effect: where it was made, its place among its job's effects, and the work.
struct Recorded {
    key: JobKey,
    seq: u32,
    effect: Box<dyn FnOnce(&mut Engine) + Send>,
}

thread_local! {
    /// The job the calling thread is running (with the number of effects it has recorded so far).
    static CURRENT: Cell<Option<(JobKey, u32)>> = const { Cell::new(None) };
}

/// Every run's recorded effects, by the run's kernel id (`KernelRef::id`: the same in every real
/// thread's engine of one run).
static RECORDED: Mutex<Vec<(usize, Vec<Recorded>)>> = Mutex::new(Vec::new());
/// How many effects wait in [`RECORDED`] over all runs (so a replay with none takes no lock).
static WAITING: AtomicUsize = AtomicUsize::new(0);

/// The calling thread runs the job at `key` until the returned guard is dropped.
#[must_use = "the job ends when the guard is dropped"]
pub fn enter(key: JobKey) -> JobScope {
    let before = CURRENT.with(|c| c.replace(Some((key, 0))));
    JobScope { before }
}

/// A job in progress on the calling thread ([`enter`]); dropping it ends the job.
pub struct JobScope {
    before: Option<(JobKey, u32)>,
}

impl Drop for JobScope {
    fn drop(&mut self) {
        CURRENT.with(|c| c.set(self.before));
    }
}

/// Whether the calling thread is inside a job.
pub fn in_job() -> bool {
    CURRENT.with(|c| c.get()).is_some()
}

/// Runs `effect` now, or, inside a job, records it to run at the next [`replay`] on the stepping
/// thread (after the effects its job recorded before it).
pub fn run_or_record(e: &mut Engine, effect: impl FnOnce(&mut Engine) + Send + 'static) {
    if !in_job() {
        effect(e);
        return;
    }
    record_in_job(e.kernel.id(), Box::new(effect));
}

/// Records `effect` for the run `run` under the calling thread's job (which there is).
fn record_in_job(run: usize, effect: Box<dyn FnOnce(&mut Engine) + Send>) {
    let Some((key, seq)) = CURRENT.with(|c| c.get()) else {
        return;
    };
    CURRENT.with(|c| c.set(Some((key, seq.wrapping_add(1)))));
    let mut recorded = RECORDED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let entry = match recorded.iter().position(|(id, _)| *id == run) {
        Some(at) => &mut recorded[at].1,
        None => {
            recorded.push((run, Vec::new()));
            &mut recorded.last_mut().expect("just pushed").1
        }
    };
    entry.push(Recorded { key, seq, effect });
    WAITING.fetch_add(1, Ordering::Release);
}

/// Runs this run's recorded effects on the calling thread, sorted by their jobs' keys and, within
/// a job, in the order it recorded them; answers how many ran. Called by the stepping thread once
/// the jobs of a batch have ended; inside a job it does nothing (a job never replays).
pub fn replay(e: &mut Engine) -> usize {
    if WAITING.load(Ordering::Acquire) == 0 || in_job() {
        return 0;
    }
    let run = e.kernel.id();
    let mut list = {
        let mut recorded = RECORDED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match recorded.iter().position(|(id, _)| *id == run) {
            Some(at) => recorded.swap_remove(at).1,
            None => return 0,
        }
    };
    WAITING.fetch_sub(list.len(), Ordering::AcqRel);
    list.sort_by_key(|r| (r.key, r.seq));
    let count = list.len();
    for recorded in list {
        (recorded.effect)(e);
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex as StdMutex};

    fn key(island: u16, kind: u8, part: u32) -> JobKey {
        JobKey { island, kind, part }
    }

    /// A log the effects append to (shared with the closures).
    fn log() -> Arc<StdMutex<Vec<u32>>> {
        Arc::new(StdMutex::new(Vec::new()))
    }

    fn push(
        log: &Arc<StdMutex<Vec<u32>>>,
        value: u32,
    ) -> impl FnOnce(&mut Engine) + Send + 'static {
        let log = log.clone();
        move |_| log.lock().unwrap().push(value)
    }

    #[test]
    fn outside_a_job_an_effect_runs_at_once() {
        let mut e = Engine::new();
        let seen = log();
        assert!(!in_job());
        run_or_record(&mut e, push(&seen, 7));
        assert_eq!(*seen.lock().unwrap(), [7]);
        assert_eq!(replay(&mut e), 0);
    }

    #[test]
    fn inside_a_job_effects_wait_for_the_replay() {
        let mut e = Engine::new();
        let seen = log();
        {
            let _job = enter(key(0, 0xd, 0));
            assert!(in_job());
            run_or_record(&mut e, push(&seen, 1));
            run_or_record(&mut e, push(&seen, 2));
            // A job never replays.
            assert_eq!(replay(&mut e), 0);
        }
        assert!(!in_job());
        assert!(seen.lock().unwrap().is_empty());
        assert_eq!(replay(&mut e), 2);
        assert_eq!(*seen.lock().unwrap(), [1, 2]);
        // Nothing is left.
        assert_eq!(replay(&mut e), 0);
    }

    #[test]
    fn the_replay_follows_island_then_kind_then_part_then_order_in_the_job() {
        let mut e = Engine::new();
        let seen = log();
        // Recorded out of order, as jobs on several threads finish in any order.
        let jobs = [
            (key(2, 0xd, 0), [20, 21]),
            (key(0, 0xd, 4), [4, 5]),
            (key(1, 0xd, 0), [10, 11]),
            (key(0, 0xf, 0), [8, 9]),
            (key(0, 0xd, 0), [0, 1]),
        ];
        for (k, values) in jobs {
            let _job = enter(k);
            for v in values {
                run_or_record(&mut e, push(&seen, v));
            }
        }
        assert_eq!(replay(&mut e), 10);
        assert_eq!(*seen.lock().unwrap(), [0, 1, 4, 5, 8, 9, 10, 11, 20, 21]);
    }

    #[test]
    fn jobs_on_several_threads_replay_in_key_order_on_the_calling_thread() {
        let mut e = Engine::new();
        let seen = log();
        let threads: Vec<u32> = std::thread::scope(|s| {
            // Each worker records for the same run (as real threads' engines share one kernel).
            let run = e.kernel.id();
            let handles: Vec<_> = (0..4u16)
                .rev()
                .map(|island| {
                    let seen = seen.clone();
                    s.spawn(move || {
                        let _job = enter(key(island, 0xd, 0));
                        for n in 0..3 {
                            record_in_job(run, Box::new(push(&seen, u32::from(island) * 10 + n)));
                        }
                        u32::from(island)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        assert_eq!(threads.len(), 4);
        assert!(seen.lock().unwrap().is_empty());
        let main = std::thread::current().id();
        let ran_on = Arc::new(StdMutex::new(Vec::new()));
        {
            let _job = enter(key(9, 0xd, 0));
            let ran_on = ran_on.clone();
            run_or_record(&mut e, move |_| {
                ran_on.lock().unwrap().push(std::thread::current().id())
            });
        }
        assert_eq!(replay(&mut e), 13);
        assert_eq!(
            *seen.lock().unwrap(),
            [0, 1, 2, 10, 11, 12, 20, 21, 22, 30, 31, 32]
        );
        assert_eq!(*ran_on.lock().unwrap(), [main]);
    }

    #[test]
    fn runs_keep_their_own_records() {
        let (mut a, mut b) = (Engine::new(), Engine::new());
        assert_ne!(a.kernel.id(), b.kernel.id());
        let seen = log();
        {
            let _job = enter(key(0, 0xd, 0));
            run_or_record(&mut a, push(&seen, 1));
            run_or_record(&mut b, push(&seen, 2));
        }
        assert_eq!(replay(&mut b), 1);
        assert_eq!(*seen.lock().unwrap(), [2]);
        assert_eq!(replay(&mut a), 1);
        assert_eq!(*seen.lock().unwrap(), [2, 1]);
    }

    #[test]
    fn an_effect_replayed_runs_outside_any_job() {
        let mut e = Engine::new();
        let inside = Arc::new(StdMutex::new(None));
        {
            let _job = enter(key(0, 0xd, 0));
            let inside = inside.clone();
            run_or_record(&mut e, move |_| *inside.lock().unwrap() = Some(in_job()));
        }
        replay(&mut e);
        assert_eq!(*inside.lock().unwrap(), Some(false));
    }
}
