//! The hitch log's counts on the drawing side (`host::hitch_log`; the game side's are `engine_core::hitch`): how
//! long main waited for the render thread, and the pipelines and shader modules the backend made, with their time.
//! Each is one relaxed atomic add where the stream already reads the clock (its waits) or where the backend makes a
//! pipeline (milliseconds of work each). The host reads them after each of main's frames ([`snapshot`]).

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// What is counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Count {
    /// Main's wait in `Present` for the render thread (the frames in flight), ns.
    PresentWaitNs,
    /// Main's waits for a reply (a read-back, a query's answer, a lock): ns and how many.
    ReplyWaitNs,
    ReplyWaits,
    /// Main's waits for room in the command queue (the render thread behind), ns.
    QueueFullNs,
    /// Render pipelines the backend made (on any of its threads) and their time (ns), shader modules made and
    /// their time (ns, the conversion included).
    Pipelines,
    PipelineNs,
    Modules,
    ModuleNs,
}

/// How many counts there are.
pub const COUNTS: usize = Count::ModuleNs as usize + 1;

#[allow(clippy::declare_interior_mutable_const)]
const ZERO: AtomicU64 = AtomicU64::new(0);
static COUNTERS: [AtomicU64; COUNTS] = [ZERO; COUNTS];

/// Adds `n` to `c`.
#[inline]
pub fn add(c: Count, n: u64) {
    COUNTERS[c as usize].fetch_add(n, Relaxed);
}

/// Adds the duration `d` in ns to `c`.
#[inline]
pub fn add_time(c: Count, d: std::time::Duration) {
    add(c, d.as_nanos().min(u128::from(u64::MAX)) as u64);
}

/// Every count now (the host subtracts the one before).
pub fn snapshot() -> [u64; COUNTS] {
    std::array::from_fn(|i| COUNTERS[i].load(Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_land_in_their_count() {
        let before = snapshot();
        add(Count::Pipelines, 3);
        add_time(Count::PipelineNs, std::time::Duration::from_millis(18));
        let now = snapshot();
        assert!(now[Count::Pipelines as usize] - before[Count::Pipelines as usize] >= 3);
        assert!(now[Count::PipelineNs as usize] - before[Count::PipelineNs as usize] >= 18_000_000);
    }
}
