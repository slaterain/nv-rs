//! The occlusion counts the game's threads read without waiting for the render thread
//! ([`QueryAnswers`]).
//!
//! With the render thread, a `GetData` of a query used to be a sync command: the game's thread
//! waited until the render thread had carried out every command before it (0.6-0.7 ms a frame in
//! the crowded street, more when the render thread is further behind), only to hear "not yet" most
//! of the time. Now the render thread posts each count here once the GPU has it, and the game's
//! threads read it at once (`engine-platform`'s `d3d9real::query_get_data`): the count of the
//! game's last `END`, or "not yet", as Direct3D 9 answers until the GPU is done.

use d3d9_stream::Handle;
use std::collections::HashMap;
use std::sync::Mutex;

/// One query as the backend has carried it out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Answer {
    /// How many queries the backend made at this handle (one destroyed and made again there is
    /// another), as the game's side counts its `CreateQuery`s.
    pub made: u64,
    /// An occlusion query (the others are answered by the device model, not here).
    pub occlusion: bool,
    /// The `D3DISSUE_END`s the backend carried out on it.
    pub ends: u64,
    /// The count of the last of them, once the GPU has it.
    pub count: Option<u32>,
}

/// The queries' answers by handle, shared by the backend (which posts) and the game's side
/// (which reads). The backend's side is [`QueryAnswers::made`], [`QueryAnswers::ended`] and
/// [`QueryAnswers::post`]; the game's side reads with [`QueryAnswers::get`].
#[derive(Debug, Default)]
pub struct QueryAnswers(Mutex<HashMap<Handle, Answer>>);

impl QueryAnswers {
    fn with<R>(&self, f: impl FnOnce(&mut HashMap<Handle, Answer>) -> R) -> R {
        f(&mut self.0.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// The query at `query` as the backend has it now.
    pub fn get(&self, query: Handle) -> Option<Answer> {
        self.with(|m| m.get(&query).copied())
    }

    /// A query was made at `query`.
    pub fn made(&self, query: Handle, occlusion: bool) {
        self.with(|m| {
            let a = m.entry(query).or_default();
            *a = Answer {
                made: a.made + 1,
                occlusion,
                ends: 0,
                count: None,
            };
        });
    }

    /// An `END` of `query` was carried out: its number (a count comes later).
    pub fn ended(&self, query: Handle) -> u64 {
        self.with(|m| {
            let a = m.entry(query).or_default();
            a.ends += 1;
            a.count = None;
            a.ends
        })
    }

    /// Whether `END` number `ends` is still the last of `query`.
    pub fn awaits(&self, query: Handle, ends: u64) -> bool {
        self.with(|m| m.get(&query).is_some_and(|a| a.ends == ends))
    }

    /// The count of `END` number `ends` of `query`, unless a later one came.
    pub fn post(&self, query: Handle, ends: u64, count: u32) {
        self.with(|m| {
            if let Some(a) = m.get_mut(&query) {
                if a.ends == ends {
                    a.count = Some(count);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A count belongs to the `END` it was read for: a later `END` waits for its own, and a query
    /// made again at the same handle starts over (its `made` tells the two apart).
    #[test]
    fn a_count_belongs_to_its_end_and_its_query() {
        let a = QueryAnswers::default();
        a.made(0x10, true);
        assert_eq!(a.ended(0x10), 1);
        assert!(a.awaits(0x10, 1));
        a.post(0x10, 1, 42);
        assert_eq!(a.get(0x10).and_then(|q| q.count), Some(42));
        // A second END: the first's count is no longer the answer, and a late post of it is
        // dropped.
        assert_eq!(a.ended(0x10), 2);
        assert!(!a.awaits(0x10, 1));
        a.post(0x10, 1, 7);
        assert_eq!(a.get(0x10).and_then(|q| q.count), None);
        a.post(0x10, 2, 9);
        assert_eq!(
            a.get(0x10),
            Some(Answer {
                made: 1,
                occlusion: true,
                ends: 2,
                count: Some(9)
            })
        );
        // Made again at the same handle.
        a.made(0x10, true);
        assert_eq!(
            a.get(0x10),
            Some(Answer {
                made: 2,
                occlusion: true,
                ends: 0,
                count: None
            })
        );
        assert_eq!(a.get(0x20), None);
    }
}
