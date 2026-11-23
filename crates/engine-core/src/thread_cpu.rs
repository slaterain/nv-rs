//! The processor time of the game's other threads, read from another thread: the hitch log's way to tell, when main
//! waited for a lock another game thread held ([`crate::hitch::LockHolder`]), whether the holder was working under
//! the lock or had been pushed off the processor by other programs. Read only around a wait of main's that already
//! passed a millisecond, so its cost (a system call or a small file read) never lands on an ordinary frame.
//!
//! Windows: `GetThreadTimes` on a handle the thread opened for itself at its start. Linux: the first field of
//! `/proc/self/task/<tid>/schedstat` (nanoseconds on the processor), the thread's id read from `/proc/thread-self`
//! at its start. Other systems (macOS): not known, and the line says so.

use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

/// A game thread's entry: its start address, the platform function it is in now ([`enter_call`]) and its
/// processor clock (`None` where the system does not tell it).
struct Clock {
    /// The run the thread is in (`crate::units::platform::KernelRef::id`): thread ids are a run's own, and a test
    /// process runs several runs at once.
    run: usize,
    thread: u32,
    start: u32,
    call: Arc<AtomicU32>,
    /// What the thread is waiting for on the render thread now ([`note_render_wait`]; 0 nothing).
    render_wait: Arc<AtomicU32>,
    os: Option<Os>,
}

#[cfg(windows)]
struct Os(isize);

#[cfg(target_os = "linux")]
struct Os(std::path::PathBuf);

#[cfg(not(any(windows, target_os = "linux")))]
struct Os;

static CLOCKS: Mutex<Vec<Clock>> = Mutex::new(Vec::new());

thread_local! {
    /// This OS thread's current platform function word (its registry entry's), once registered.
    static CALL: std::cell::OnceCell<Arc<AtomicU32>> = const { std::cell::OnceCell::new() };
    /// This OS thread's render-thread wait word (its registry entry's), once registered.
    static RENDER_WAIT: std::cell::OnceCell<Arc<AtomicU32>> = const { std::cell::OnceCell::new() };
}

/// The running OS thread is game thread `thread` of run `run` (its start address `start`): its clock is kept for
/// later reads from other threads. A thread registered again replaces its old entry.
pub fn register(run: usize, thread: u32, start: u32) {
    let call = Arc::new(AtomicU32::new(0));
    CALL.with(|c| {
        let _ = c.set(Arc::clone(&call));
    });
    let render_wait = Arc::new(AtomicU32::new(0));
    RENDER_WAIT.with(|c| {
        let _ = c.set(Arc::clone(&render_wait));
    });
    if let Ok(mut c) = CLOCKS.lock() {
        c.retain(|k| (k.run, k.thread) != (run, thread));
        c.push(Clock {
            run,
            thread,
            start,
            call,
            render_wait,
            os: Os::here(),
        });
    }
}

/// This thread (when registered) enters the platform function at `addr`: answers the one it was in before, for
/// [`leave_call`]. One relaxed store; nothing on an unregistered thread (main, the host, tests).
#[inline]
pub fn enter_call(addr: u32) -> u32 {
    CALL.with(|c| c.get().map_or(0, |w| w.swap(addr, Relaxed)))
}

/// This thread left the platform function [`enter_call`] entered: back to `before`.
#[inline]
pub fn leave_call(before: u32) {
    CALL.with(|c| {
        if let Some(w) = c.get() {
            w.store(before, Relaxed);
        }
    });
}

/// This thread (when registered) starts (`what` > 0: 1 a reply, 2 the frames in flight, 3 room in the queue) or ends
/// (0) a wait for the render thread (`d3d9_stream::hitch::set_wait_hook`). One relaxed store.
pub fn note_render_wait(what: u32) {
    RENDER_WAIT.with(|c| {
        if let Some(w) = c.get() {
            w.store(what, Relaxed);
        }
    });
}

/// What game thread `thread` is waiting for on the render thread now (0: nothing, or not registered).
pub fn render_wait_of(run: usize, thread: u32) -> u32 {
    CLOCKS
        .lock()
        .ok()
        .and_then(|c| {
            c.iter()
                .find(|k| (k.run, k.thread) == (run, thread))
                .map(|k| k.render_wait.load(Relaxed))
        })
        .unwrap_or(0)
}

/// The platform function game thread `thread` is in now (0: none, or not registered).
pub fn call_of(run: usize, thread: u32) -> u32 {
    CLOCKS
        .lock()
        .ok()
        .and_then(|c| {
            c.iter()
                .find(|k| (k.run, k.thread) == (run, thread))
                .map(|k| k.call.load(Relaxed))
        })
        .unwrap_or(0)
}

/// Game thread `thread`'s start address (0 when not registered).
pub fn start_of(run: usize, thread: u32) -> u32 {
    CLOCKS
        .lock()
        .ok()
        .and_then(|c| {
            c.iter()
                .find(|k| (k.run, k.thread) == (run, thread))
                .map(|k| k.start)
        })
        .unwrap_or(0)
}

/// The calling thread's own processor clock and the wall clock, in one unit, so that their two differences over a
/// stretch give the share of it the thread spent on the processor (the hitch log's frame tail: was main working or
/// pushed off the processor?). Windows on x86-64: the thread's cycle count (`QueryThreadCycleTime`, counted in the
/// time-stamp counter's ticks) and the time-stamp counter (`RDTSC`), about a microsecond; Linux and macOS: the
/// thread's CPU-time clock (`clock_gettime(CLOCK_THREAD_CPUTIME_ID)`, ns, no file read) and the monotonic clock (ns).
/// `None` elsewhere (Windows on ARM).
pub fn own_clock() -> Option<(u64, u64)> {
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentThread() -> *mut core::ffi::c_void;
            fn QueryThreadCycleTime(thread: *mut core::ffi::c_void, cycles: *mut u64) -> i32;
        }
        let mut cycles = 0u64;
        // SAFETY: the calling thread's pseudo handle and a local out-word.
        if unsafe { QueryThreadCycleTime(GetCurrentThread(), &mut cycles) } == 0 {
            return None;
        }
        // SAFETY: `RDTSC` reads a register and has no preconditions on this host.
        let ticks = unsafe { core::arch::x86_64::_rdtsc() };
        Some((cycles, ticks))
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        /// `struct timespec` on the 64-bit systems this builds for (Linux and macOS: two 64-bit words).
        #[repr(C)]
        struct Timespec {
            sec: i64,
            nsec: i64,
        }
        extern "C" {
            fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
        }
        /// `CLOCK_THREAD_CPUTIME_ID` (Linux 3, macOS 16).
        const THREAD_CPU: i32 = if cfg!(target_os = "linux") { 3 } else { 16 };
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        let start = *START.get_or_init(std::time::Instant::now);
        let mut t = Timespec { sec: 0, nsec: 0 };
        // SAFETY: the C library's clock read into a local of its own layout.
        if unsafe { clock_gettime(THREAD_CPU, &mut t) } != 0 {
            return None;
        }
        let cpu = (t.sec as u64)
            .wrapping_mul(1_000_000_000)
            .wrapping_add(t.nsec as u64);
        Some((cpu, start.elapsed().as_nanos() as u64))
    }
    #[cfg(not(any(
        all(windows, target_arch = "x86_64"),
        target_os = "linux",
        target_os = "macos"
    )))]
    {
        None
    }
}

/// Game thread `thread`'s processor time so far (ns), when the system tells it.
pub fn cpu_ns(run: usize, thread: u32) -> Option<u64> {
    let c = CLOCKS.lock().ok()?;
    c.iter()
        .find(|k| (k.run, k.thread) == (run, thread))?
        .os
        .as_ref()?
        .read()
}

#[cfg(windows)]
impl Os {
    fn here() -> Option<Os> {
        use windows::Win32::System::Threading::{
            GetCurrentThreadId, OpenThread, THREAD_QUERY_LIMITED_INFORMATION,
        };
        // SAFETY: plain system calls on this thread's own id; the handle is kept for the process's life.
        let handle = unsafe {
            OpenThread(
                THREAD_QUERY_LIMITED_INFORMATION,
                windows::Win32::Foundation::BOOL::from(false),
                GetCurrentThreadId(),
            )
        }
        .ok()?;
        Some(Os(handle.0 as isize))
    }

    fn read(&self) -> Option<u64> {
        use windows::Win32::Foundation::{FILETIME, HANDLE};
        use windows::Win32::System::Threading::GetThreadTimes;
        let mut times = [FILETIME::default(); 4];
        let [created, exited, kernel, user] = &mut times;
        // SAFETY: the handle was opened by `here` and is never closed; the four out pointers are locals.
        unsafe {
            GetThreadTimes(
                HANDLE(self.0 as *mut core::ffi::c_void),
                created,
                exited,
                kernel,
                user,
            )
        }
        .ok()?;
        let ticks = |t: &FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
        // FILETIME counts 100 ns units.
        Some((ticks(kernel) + ticks(user)) * 100)
    }
}

#[cfg(target_os = "linux")]
impl Os {
    fn here() -> Option<Os> {
        let link = std::fs::read_link("/proc/thread-self").ok()?;
        let tid = link.file_name()?.to_owned();
        Some(Os(std::path::Path::new("/proc/self/task")
            .join(tid)
            .join("schedstat")))
    }

    fn read(&self) -> Option<u64> {
        schedstat_ns(&std::fs::read_to_string(&self.0).ok()?)
    }
}

/// The processor time (ns) in a `schedstat` line: its first field.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn schedstat_ns(text: &str) -> Option<u64> {
    text.split_whitespace().next()?.parse().ok()
}

#[cfg(not(any(windows, target_os = "linux")))]
impl Os {
    fn here() -> Option<Os> {
        None
    }

    fn read(&self) -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A thread that works shows its processor time growing, read from another thread; one that sleeps does not
    /// (on the systems that tell it; elsewhere nothing is known and nothing is claimed).
    #[test]
    fn a_working_thread_s_time_grows_and_a_sleeping_one_s_does_not() {
        let (tx, rx) = std::sync::mpsc::channel();
        let (go_tx, go_rx) = std::sync::mpsc::channel::<bool>();
        let worker = std::thread::spawn(move || {
            register(7, 0x7fff_0001, 0x00aa_8430);
            tx.send(()).unwrap();
            while let Ok(work) = go_rx.recv() {
                let t = std::time::Instant::now();
                if work {
                    let mut x = 0u64;
                    while t.elapsed() < std::time::Duration::from_millis(60) {
                        x = std::hint::black_box(
                            x.wrapping_mul(6364136223846793005).wrapping_add(1),
                        );
                    }
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(60));
                }
                tx.send(()).unwrap();
            }
        });
        rx.recv().unwrap();
        assert_eq!(start_of(7, 0x7fff_0001), 0x00aa_8430);
        assert_eq!(
            start_of(8, 0x7fff_0001),
            0,
            "another run's thread of that id"
        );
        assert_eq!(start_of(7, 0x7fff_0002), 0);
        let measure = |work: bool| {
            let before = cpu_ns(7, 0x7fff_0001);
            go_tx.send(work).unwrap();
            rx.recv().unwrap();
            before.zip(cpu_ns(7, 0x7fff_0001)).map(|(b, a)| a - b)
        };
        let worked = measure(true);
        let slept = measure(false);
        drop(go_tx);
        worker.join().unwrap();
        if cfg!(any(windows, target_os = "linux")) {
            let (worked, slept) = (worked.unwrap(), slept.unwrap());
            assert!(worked > 20_000_000, "60 ms of work showed {worked} ns");
            assert!(slept < 20_000_000, "60 ms of sleep showed {slept} ns");
        } else {
            assert_eq!((worked, slept), (None, None));
        }
    }

    /// A thread's own clock against the wall, where the system tells it: a busy stretch is (nearly) all on the
    /// processor and never more, a sleeping one nearly none, so the two clocks are in one unit (a loaded machine may
    /// take some of a busy stretch away: the best of five tries counts).
    #[test]
    fn a_thread_s_own_clock_tells_working_from_sleeping() {
        if own_clock().is_none() {
            if cfg!(any(
                all(windows, target_arch = "x86_64"),
                target_os = "linux",
                target_os = "macos"
            )) {
                panic!("this system tells a thread its own processor clock");
            }
            return;
        }
        let share = |work: bool| {
            let (cpu0, wall0) = own_clock().expect("told");
            let t = std::time::Instant::now();
            if work {
                let mut x = 0u64;
                while t.elapsed() < std::time::Duration::from_millis(40) {
                    x = std::hint::black_box(x.wrapping_add(1));
                }
            } else {
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            let (cpu1, wall1) = own_clock().expect("still told");
            (cpu1 - cpu0) as f64 / (wall1 - wall0) as f64
        };
        let busy: Vec<f64> = (0..5).map(|_| share(true)).collect();
        assert!(
            busy.iter().any(|s| (0.6..=1.05).contains(s)),
            "busy shares {busy:?}"
        );
        assert!(
            busy.iter().all(|s| *s <= 1.05),
            "never above the wall: {busy:?}"
        );
        let slept = share(false);
        assert!(slept < 0.2, "a sleeping thread's share {slept}");
    }

    #[test]
    fn schedstat_s_first_field_is_the_time() {
        assert_eq!(schedstat_ns("123456789 2000 17\n"), Some(123_456_789));
        assert_eq!(schedstat_ns(""), None);
    }
}
