//! Typed code's calls to the synchronisation imports, made as Rust calls in play.
//!
//! The typed families call `InterlockedIncrement`, `InterlockedDecrement` and
//! `InterlockedCompareExchange` (reference counts of scene objects, lights and shaders' objects),
//! `EnterCriticalSection` and `LeaveCriticalSection` (the tile and render locks),
//! `TlsGetValue`, `TlsSetValue` and `GetCurrentThreadId` (the lock-free map's thread slot, the
//! particles' owner) tens of thousands of times a frame on the street. By address each such call
//! packs its words, finds the import's slot in the call cache, calls the registered function
//! through the one indirect call every call by address shares, and that function unpacks the words
//! again.
//!
//! With real threads (play) the functions here call the real threads' own import
//! (`crate::real_threads::SYNC`) as a plain Rust call where the call by address would have run
//! that very function: the slot still holds that registration (anything registered over the slot,
//! a test's double or a hook, is called by address as before) and no per-call diagnostic is on.
//! The time slice, its hook and the engine's call stack go as for the call by address
//! ([`Engine::call_typed`]). So the same function runs with the same arguments in the same order;
//! only the look-up's indirect call and the packing of the words go. Everything else calls by
//! address exactly as before: stepped threads (the reference mode, scripted and repeatable runs,
//! the differential tests) and an engine without real threads.

use crate::ptr::Ptr;
use crate::Engine;

/// `InterlockedIncrement` (import slot).
pub const INTERLOCKED_INCREMENT: u32 = 0x00fd_f1c0;
/// `InterlockedDecrement` (import slot).
pub const INTERLOCKED_DECREMENT: u32 = 0x00fd_f1c4;
/// `InterlockedCompareExchange` (import slot).
pub const INTERLOCKED_COMPARE_EXCHANGE: u32 = 0x00fd_f1bc;
/// `EnterCriticalSection` (import slot).
pub const ENTER_CRITICAL_SECTION: u32 = 0x00fd_f05c;
/// `LeaveCriticalSection` (import slot).
pub const LEAVE_CRITICAL_SECTION: u32 = 0x00fd_f100;
/// `TlsGetValue` (import slot).
pub const TLS_GET_VALUE: u32 = 0x00fd_f044;
/// `TlsSetValue` (import slot).
pub const TLS_SET_VALUE: u32 = 0x00fd_f04c;
/// `GetCurrentThreadId` (import slot).
pub const GET_CURRENT_THREAD_ID: u32 = 0x00fd_f1d0;

#[cfg(test)]
thread_local! {
    /// The calls this thread made directly (tests).
    static DIRECT_CALLS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// One direct call made (tests count them).
#[inline(always)]
fn note_direct() {
    #[cfg(test)]
    DIRECT_CALLS.with(|c| c.set(c.get() + 1));
}

/// `InterlockedIncrement(target)`: the word's new value.
#[inline]
pub fn interlocked_increment(e: &mut Engine, target: u32) -> i32 {
    if e.real.is_some() {
        let (expect, f) = crate::real_threads::SYNC.interlocked_increment;
        return e
            .call_typed(INTERLOCKED_INCREMENT, expect, &[target], |e| {
                note_direct();
                f(e, Ptr::new(target)) as u32
            })
            .i32();
    }
    e.call(INTERLOCKED_INCREMENT, &[target]).i32()
}

/// `InterlockedDecrement(target)`: the word's new value.
#[inline]
pub fn interlocked_decrement(e: &mut Engine, target: u32) -> i32 {
    if e.real.is_some() {
        let (expect, f) = crate::real_threads::SYNC.interlocked_decrement;
        return e
            .call_typed(INTERLOCKED_DECREMENT, expect, &[target], |e| {
                note_direct();
                f(e, Ptr::new(target)) as u32
            })
            .i32();
    }
    e.call(INTERLOCKED_DECREMENT, &[target]).i32()
}

/// `InterlockedCompareExchange(target, exchange, comparand)`: the word's old value.
#[inline]
pub fn interlocked_compare_exchange(
    e: &mut Engine,
    target: u32,
    exchange: i32,
    comparand: i32,
) -> i32 {
    let words = crate::args![target, exchange, comparand];
    if e.real.is_some() {
        let (expect, f) = crate::real_threads::SYNC.interlocked_compare_exchange;
        return e
            .call_typed(INTERLOCKED_COMPARE_EXCHANGE, expect, &words, |e| {
                note_direct();
                f(e, Ptr::new(target), exchange, comparand) as u32
            })
            .i32();
    }
    e.call(INTERLOCKED_COMPARE_EXCHANGE, &words).i32()
}

/// `EnterCriticalSection(section)`.
#[inline]
pub fn enter_critical_section(e: &mut Engine, section: u32) {
    if e.real.is_some() {
        let (expect, f) = crate::real_threads::SYNC.enter_critical_section;
        e.call_typed(ENTER_CRITICAL_SECTION, expect, &[section], |e| {
            note_direct();
            f(e, Ptr::new(section));
            0
        });
        return;
    }
    e.call(ENTER_CRITICAL_SECTION, &[section]);
}

/// `LeaveCriticalSection(section)`.
#[inline]
pub fn leave_critical_section(e: &mut Engine, section: u32) {
    if e.real.is_some() {
        let (expect, f) = crate::real_threads::SYNC.leave_critical_section;
        e.call_typed(LEAVE_CRITICAL_SECTION, expect, &[section], |e| {
            note_direct();
            f(e, Ptr::new(section));
            0
        });
        return;
    }
    e.call(LEAVE_CRITICAL_SECTION, &[section]);
}

/// `TlsGetValue(index)`: the calling thread's value.
#[inline]
pub fn tls_get_value(e: &mut Engine, index: u32) -> u32 {
    if e.real.is_some() {
        let (expect, f) = crate::real_threads::SYNC.tls_get_value;
        return e
            .call_typed(TLS_GET_VALUE, expect, &[index], |e| {
                note_direct();
                f(e, index)
            })
            .u32();
    }
    e.call(TLS_GET_VALUE, &[index]).u32()
}

/// `TlsSetValue(index, value)`: nonzero when stored.
#[inline]
pub fn tls_set_value(e: &mut Engine, index: u32, value: u32) -> i32 {
    if e.real.is_some() {
        let (expect, f) = crate::real_threads::SYNC.tls_set_value;
        return e
            .call_typed(TLS_SET_VALUE, expect, &[index, value], |e| {
                note_direct();
                f(e, index, value) as u32
            })
            .i32();
    }
    e.call(TLS_SET_VALUE, &[index, value]).i32()
}

/// `GetCurrentThreadId()`.
#[inline]
pub fn get_current_thread_id(e: &mut Engine) -> u32 {
    if e.real.is_some() {
        let (expect, f) = crate::real_threads::SYNC.get_current_thread_id;
        return e
            .call_typed(GET_CURRENT_THREAD_ID, expect, &[], |e| {
                note_direct();
                f(e)
            })
            .u32();
    }
    e.call(GET_CURRENT_THREAD_ID, &[]).u32()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::Ret;
    use crate::args;
    use crate::thread_host::MAIN_THREAD_ID;

    /// The eight imports above at their exe slots.
    const AT: [(&str, u32); 8] = [
        ("InterlockedIncrement", INTERLOCKED_INCREMENT),
        ("InterlockedDecrement", INTERLOCKED_DECREMENT),
        ("InterlockedCompareExchange", INTERLOCKED_COMPARE_EXCHANGE),
        ("EnterCriticalSection", ENTER_CRITICAL_SECTION),
        ("LeaveCriticalSection", LEAVE_CRITICAL_SECTION),
        ("TlsGetValue", TLS_GET_VALUE),
        ("TlsSetValue", TLS_SET_VALUE),
        ("GetCurrentThreadId", GET_CURRENT_THREAD_ID),
    ];

    /// An engine with the platform's imports, as `real_threads`' tests make one, but the eight
    /// above at their exe slots (the rest after them), and a look-up of a slot by name.
    fn engine_at_exe_slots() -> (Engine, impl Fn(&str) -> u32) {
        let mut e = Engine::new();
        e.mem.map(0x00fd_f000, 0x2000);
        e.mem.map(0x00fe_0000, 0x2000);
        let imports: Vec<crate::exe::Import> = crate::units::platform::imports()
            .iter()
            .enumerate()
            .map(|(i, (dll, name, _))| crate::exe::Import {
                dll: dll.to_string(),
                name: name.to_string(),
                slot: AT
                    .iter()
                    .find(|(n, _)| n == name)
                    .map_or(0x00fe_0000 + 4 * i as u32, |&(_, slot)| slot),
            })
            .collect();
        let slots: Vec<(String, u32)> = imports.iter().map(|i| (i.name.clone(), i.slot)).collect();
        e.register_imports(&imports);
        e.start_real_clock();
        let slot = move |name: &str| {
            slots
                .iter()
                .find(|(n, _)| n == name)
                .unwrap_or_else(|| panic!("no import {name}"))
                .1
        };
        (e, slot)
    }

    fn direct_calls() -> u64 {
        DIRECT_CALLS.with(std::cell::Cell::get)
    }

    /// With real threads each function calls the real threads' import directly and answers as the
    /// call by address does, the two mixed on one word, one TLS index and one critical section.
    #[test]
    fn with_real_threads_the_imports_run_directly_and_answer_as_by_address() {
        let (mut e, slot) = engine_at_exe_slots();
        crate::real_threads::start(&mut e).expect("real threads on");
        let word = e.mem.alloc(4);
        let index = e.call(slot("TlsAlloc"), &[]).u32();
        let section = e.mem.alloc(24);
        e.call(slot("InitializeCriticalSection"), &args![section]);
        // Each slot called by address first: a direct call whose slot the call cache does not
        // hold yet goes by address (which puts it there).
        for (at, words) in [
            (INTERLOCKED_INCREMENT, vec![word]),
            (INTERLOCKED_DECREMENT, vec![word]),
            (INTERLOCKED_COMPARE_EXCHANGE, vec![word, 0, 0]),
            (TLS_GET_VALUE, vec![index]),
            (TLS_SET_VALUE, vec![index, 0]),
            (GET_CURRENT_THREAD_ID, vec![]),
            (ENTER_CRITICAL_SECTION, vec![section]),
            (LEAVE_CRITICAL_SECTION, vec![section]),
        ] {
            e.call(at, &words);
        }
        let before = direct_calls();
        e.mem.set_u32(word, 5);
        assert_eq!(interlocked_increment(&mut e, word), 6);
        assert_eq!(e.call(INTERLOCKED_INCREMENT, &args![word]).i32(), 7);
        assert_eq!(interlocked_decrement(&mut e, word), 6);
        assert_eq!(e.call(INTERLOCKED_DECREMENT, &args![word]).i32(), 5);
        assert_eq!(interlocked_decrement(&mut e, word), 4);
        assert_eq!(
            interlocked_compare_exchange(&mut e, word, 9, 4),
            4,
            "swapped"
        );
        assert_eq!(
            interlocked_compare_exchange(&mut e, word, 1, 4),
            9,
            "not swapped"
        );
        assert_eq!(e.mem.u32(word), 9);
        assert_eq!(tls_set_value(&mut e, index, 0x1234), 1);
        assert_eq!(e.call(TLS_GET_VALUE, &args![index]).u32(), 0x1234);
        e.call(TLS_SET_VALUE, &args![index, 0x5678u32]);
        assert_eq!(tls_get_value(&mut e, index), 0x5678);
        assert_eq!(get_current_thread_id(&mut e), MAIN_THREAD_ID);
        assert_eq!(e.call(GET_CURRENT_THREAD_ID, &[]).u32(), MAIN_THREAD_ID);
        enter_critical_section(&mut e, section);
        e.call(ENTER_CRITICAL_SECTION, &args![section]);
        leave_critical_section(&mut e, section);
        assert_eq!(e.mem.u32(section + 8), 1, "held once more");
        e.call(LEAVE_CRITICAL_SECTION, &args![section]);
        assert_eq!(e.mem.u32(section + 8), 0, "free");
        assert_eq!(direct_calls() - before, 10, "every call above went direct");
    }

    /// A game thread of its own: the thread id and TLS value it reads directly are its own, and a
    /// critical section main entered directly holds it off until main leaves directly.
    #[test]
    fn a_game_thread_gets_its_own_id_and_value_and_waits_for_mains_section() {
        // `p`: +0 the TLS index, +4 the section, +8 the event the thread sets, +12 its id read
        // directly, +16 by address, +20 its TLS value before and +24 after it set one, +28 its try
        // of the section while main holds it, +32 1 once it entered the section, +36 and +40 the
        // slots of TryEnterCriticalSection and SetEvent.
        fn thread(e: &mut Engine, a: &[u32]) -> Ret {
            let p = a[0];
            let (index, section) = (e.mem.u32(p), e.mem.u32(p + 4));
            let id = get_current_thread_id(e);
            e.mem.set_u32(p + 12, id);
            let by_address = e.call(GET_CURRENT_THREAD_ID, &[]).u32();
            e.mem.set_u32(p + 16, by_address);
            let before = tls_get_value(e, index);
            e.mem.set_u32(p + 20, before);
            tls_set_value(e, index, 0x4444);
            let after = e.call(TLS_GET_VALUE, &args![index]).u32();
            e.mem.set_u32(p + 24, after);
            let try_enter = e.mem.u32(p + 36);
            let tried = e.call(try_enter, &args![section]).u32();
            e.mem.set_u32(p + 28, tried);
            let set_event = e.mem.u32(p + 40);
            let event = e.mem.u32(p + 8);
            e.call(set_event, &args![event]);
            enter_critical_section(e, section);
            e.mem.set_u32(p + 32, 1);
            leave_critical_section(e, section);
            Ret::default()
        }
        let (mut e, slot) = engine_at_exe_slots();
        crate::real_threads::start(&mut e).expect("real threads on");
        e.register(0x0040_5000, thread);
        let p = e.mem.alloc(0x30);
        let index = e.call(slot("TlsAlloc"), &[]).u32();
        tls_set_value(&mut e, index, 0x1111);
        let section = e.mem.alloc(24);
        e.call(slot("InitializeCriticalSection"), &args![section]);
        let event = e
            .call(slot("CreateEventA"), &args![0u32, 0u32, 0u32, 0u32])
            .u32();
        for (at, word) in [
            (0u32, index),
            (4, section),
            (8, event),
            (36, slot("TryEnterCriticalSection")),
            (40, slot("SetEvent")),
        ] {
            e.mem.set_u32(p + at, word);
        }
        enter_critical_section(&mut e, section);
        let h = e
            .call(
                slot("CreateThread"),
                &args![0u32, 0u32, 0x0040_5000u32, p, 0u32, 0u32],
            )
            .u32();
        e.call(slot("WaitForSingleObject"), &args![event, 0xffff_ffffu32]);
        assert_eq!(
            e.mem.u32(p + 32),
            0,
            "held off while main holds the section"
        );
        leave_critical_section(&mut e, section);
        e.call(slot("WaitForSingleObject"), &args![h, 0xffff_ffffu32]);
        let id = e.mem.u32(p + 12);
        assert_ne!(id, MAIN_THREAD_ID);
        assert_eq!(e.mem.u32(p + 16), id, "the same id by address");
        assert_eq!(e.mem.u32(p + 20), 0, "main's value is not the thread's");
        assert_eq!(e.mem.u32(p + 24), 0x4444);
        assert_eq!(e.mem.u32(p + 28), 0, "main held the section");
        assert_eq!(e.mem.u32(p + 32), 1, "entered once main left");
        assert_eq!(tls_get_value(&mut e, index), 0x1111, "main's own value");
    }

    /// Anything registered over a slot, and any per-call diagnostic, sends the call by address;
    /// so does an engine without real threads.
    #[test]
    fn a_double_a_diagnostic_or_stepped_threads_call_by_address() {
        fn double(_: &mut Engine, _: &[u32]) -> Ret {
            Ret {
                eax: 77,
                ..Ret::default()
            }
        }
        let (mut e, _) = engine_at_exe_slots();
        let word = e.mem.alloc(4);
        let before = direct_calls();
        assert_eq!(
            interlocked_increment(&mut e, word),
            1,
            "stepped: the platform's"
        );
        assert_eq!(get_current_thread_id(&mut e), MAIN_THREAD_ID);
        crate::real_threads::start(&mut e).expect("real threads on");
        e.call_log = Some(Vec::new());
        assert_eq!(interlocked_increment(&mut e, word), 2);
        let logged = e.call_log.take().expect("the log");
        assert_eq!(
            logged,
            vec![(INTERLOCKED_INCREMENT, vec![word])],
            "by address"
        );
        e.register(INTERLOCKED_DECREMENT, double);
        assert_eq!(interlocked_decrement(&mut e, word), 77, "the double");
        assert_eq!(e.mem.u32(word), 2);
        assert_eq!(direct_calls(), before, "none went direct");
        assert_eq!(
            interlocked_increment(&mut e, word),
            3,
            "the slot not replaced: direct"
        );
        assert_eq!(direct_calls(), before + 1);
    }
}
