//! The differential test of a typed function (docs/ENGINE_CRATE.md, "Typed code"): two
//! [`Engine`]s, the same setup, the translation on one and the typed version on the other, called
//! at the same address with the same generated inputs. What they did is compared exactly: the
//! return value, the memory after the call, the outgoing calls with their words, whether and how
//! the run stopped, the heap, and the control word.
//!
//! ```ignore
//! Diff::new(entry!(0x00d87410, typed_version(Ptr<Thing>, f32) -> f32))
//!     .inputs(&[Input::Word, Input::F32])
//!     .setup(|e, case| {
//!         // Both engines run this: same objects at the same addresses.
//!         let thing = e.mem.alloc(0x40);
//!         e.mem.set_u32(thing + 8, case.word(0));
//!         e.register(0x00abcdef, |_, _| Ret::default()); // a callee's double
//!         vec![thing, case.f32(1).to_bits()]
//!     })
//!     .run()
//!     .assert_same();
//! ```
//!
//! Both engines hold the translations only ([`Engine::new_untyped`]): whatever `NV_TYPED` says,
//! the one function differs and everything it calls is the same. The call log is on, so a
//! `typed_call!` goes by address and every outgoing call shows.
//!
//! Left out of the comparison, because a typed version may differ there without anyone seeing it:
//! the running thread's stack for `with_stack` locals (a local may become a native one), and so a
//! word pointing into that stack in an outgoing call compares as "a stack address", not by value.
//! A callee's double that needs a local's contents reads them itself. Bookkeeping on the engine
//! (the time slice, the clock) isn't compared either: a direct call changes how the slice counts.
//!
//! Fresh engines per case, so nothing a case leaves behind reaches the next. That is the harness's
//! cost: [`Diff::max_cases`] caps it.

use super::super::x87;
use crate::abi::{AbiFn, Ret};
use crate::Engine;
use std::any::Any;
use std::collections::HashSet;
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// The control word of the main thread once Direct3D has made its device: 24-bit precision.
pub const MAIN_THREAD: u16 = 0x007f;
/// The control word of the other threads, and of the main thread before that: 53-bit precision.
pub const OTHER_THREADS: u16 = x87::DEFAULT_CONTROL_WORD;

/// The words tried for an integer or pointer-sized argument, the plainest first.
pub const WORD_VALUES: &[u32] = &[
    1,
    0,
    2,
    0x7f,
    0xff,
    0x100,
    0x7fff_ffff,
    0x8000_0000,
    u32::MAX,
];

/// The bit patterns tried for an `f32`, an ordinary value first (it is what the other arguments
/// hold while one is swept).
pub const F32_VALUES: &[u32] = &[
    0x3f80_0000, // 1
    0xbfc0_0000, // -1.5
    0x3f00_0000, // 0.5
    0x0000_0000, // 0
    0x8000_0000, // -0
    0x0000_0001, // the smallest denormal
    0x8000_0001, // the same, negative
    0x007f_ffff, // the largest denormal
    0x0080_0000, // the smallest normal
    0x4eff_ffff, // just below 2^31
    0x4f00_0000, // 2^31: one past the int range
    0xcf00_0000, // -2^31: the end of it
    0x4f80_0000, // 2^32
    0x5f00_0000, // 2^63
    0x7f7f_ffff, // the largest finite
    0xff7f_ffff, // the same, negative
    0x7f80_0000, // infinity
    0xff80_0000, // negative infinity
    0x7fc0_0000, // a quiet NaN
    0xffc0_0000, // the x87 default NaN
    0x7f80_0001, // a signalling NaN
    0x7fc1_2345, // a quiet NaN with a payload
];

/// The bit patterns tried for an `f64`, an ordinary value first.
pub const F64_VALUES: &[u64] = &[
    0x3ff0_0000_0000_0000, // 1
    0xbff8_0000_0000_0000, // -1.5
    0x3fe0_0000_0000_0000, // 0.5
    0x0000_0000_0000_0000, // 0
    0x8000_0000_0000_0000, // -0
    0x0000_0000_0000_0001, // the smallest denormal
    0x000f_ffff_ffff_ffff, // the largest denormal
    0x0010_0000_0000_0000, // the smallest normal
    0x36a0_0000_0000_0000, // the smallest f32 denormal
    0x3ff0_0000_1000_0000, // needs rounding to fit an f32, halfway
    0x41df_ffff_ffc0_0000, // just below 2^31
    0x41e0_0000_0000_0000, // 2^31
    0xc1e0_0000_0000_0000, // -2^31
    0x43e0_0000_0000_0000, // 2^63
    0x47e0_0000_0000_0000, // 2^127
    0x47f0_0000_0000_0000, // 2^128: past every f32
    0x7fef_ffff_ffff_ffff, // the largest finite
    0xffef_ffff_ffff_ffff, // the same, negative
    0x7ff0_0000_0000_0000, // infinity
    0xfff0_0000_0000_0000, // negative infinity
    0x7ff8_0000_0000_0000, // a quiet NaN
    0xfff8_0000_0000_0000, // the x87 default NaN
    0x7ff0_0000_0000_0001, // a signalling NaN
    0x7ff8_0000_0012_3456, // a quiet NaN with a payload
];

/// One generated argument.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Value {
    Word(u32),
    /// Its bits.
    F32(u32),
    /// Its bits: two words, low word first, as [`crate::abi`] passes it.
    F64(u64),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Word(w) => write!(f, "{w:#x}"),
            Value::F32(b) => write!(f, "{:?}f32({b:#010x})", f32::from_bits(*b)),
            Value::F64(b) => write!(f, "{:?}f64({b:#018x})", f64::from_bits(*b)),
        }
    }
}

/// What to generate for one argument.
#[derive(Clone, Copy, Debug)]
pub enum Input {
    /// [`WORD_VALUES`].
    Word,
    /// 1 and 0.
    Flag,
    /// [`F32_VALUES`]: NaNs, zeros, denormals and huge values.
    F32,
    /// [`F64_VALUES`].
    F64,
    /// These words: one per branch that matters.
    Of(&'static [u32]),
}

impl Input {
    fn values(self) -> Vec<Value> {
        match self {
            Input::Word => WORD_VALUES.iter().map(|w| Value::Word(*w)).collect(),
            Input::Flag => vec![Value::Word(1), Value::Word(0)],
            Input::F32 => F32_VALUES.iter().map(|b| Value::F32(*b)).collect(),
            Input::F64 => F64_VALUES.iter().map(|b| Value::F64(*b)).collect(),
            Input::Of(words) => words.iter().map(|w| Value::Word(*w)).collect(),
        }
    }
}

/// One generated set of arguments.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Case(pub Vec<Value>);

impl Case {
    /// Argument `i`, a [`Value::Word`].
    pub fn word(&self, i: usize) -> u32 {
        match self.0[i] {
            Value::Word(w) => w,
            other => panic!("argument {i} is {other:?}, not a word"),
        }
    }

    /// Argument `i`, a [`Value::F32`] (NaN bits kept).
    pub fn f32(&self, i: usize) -> f32 {
        match self.0[i] {
            Value::F32(b) => f32::from_bits(b),
            other => panic!("argument {i} is {other:?}, not an f32"),
        }
    }

    /// Argument `i`, a [`Value::F64`] (NaN bits kept).
    pub fn f64(&self, i: usize) -> f64 {
        match self.0[i] {
            Value::F64(b) => f64::from_bits(b),
            other => panic!("argument {i} is {other:?}, not an f64"),
        }
    }

    /// The arguments as call words.
    pub fn words(&self) -> Vec<u32> {
        let mut out = Vec::with_capacity(self.0.len());
        for v in &self.0 {
            match *v {
                Value::Word(w) | Value::F32(w) => out.push(w),
                Value::F64(b) => {
                    out.push(b as u32);
                    out.push((b >> 32) as u32);
                }
            }
        }
        out
    }
}

/// The cases for `inputs`: all combinations when there are at most `max_cases`. Otherwise every
/// value of each argument with the others at their first value, then pseudo-random combinations
/// (a fixed sequence, so a failure repeats) up to `max_cases`.
pub fn cases(inputs: &[Input], max_cases: usize) -> Vec<Case> {
    let lists: Vec<Vec<Value>> = inputs.iter().map(|i| i.values()).collect();
    if lists.iter().any(Vec::is_empty) {
        return Vec::new();
    }
    let all = lists
        .iter()
        .try_fold(1usize, |n, l| n.checked_mul(l.len()))
        .unwrap_or(usize::MAX);
    if all <= max_cases {
        let mut out = vec![Vec::new()];
        for list in &lists {
            out = out
                .into_iter()
                .flat_map(|head: Vec<Value>| {
                    list.iter().map(move |v| {
                        let mut case = head.clone();
                        case.push(*v);
                        case
                    })
                })
                .collect();
        }
        return out.into_iter().map(Case).collect();
    }
    let plain: Vec<Value> = lists.iter().map(|l| l[0]).collect();
    let mut seen: HashSet<Vec<Value>> = HashSet::new();
    let mut out = Vec::new();
    let mut add = |case: Vec<Value>, out: &mut Vec<Case>| {
        if seen.insert(case.clone()) {
            out.push(Case(case));
        }
    };
    add(plain.clone(), &mut out);
    for (i, list) in lists.iter().enumerate() {
        for v in list {
            let mut case = plain.clone();
            case[i] = *v;
            add(case, &mut out);
        }
    }
    // xorshift64, seeded with a fixed odd word; a bounded number of tries so a small space ends.
    let mut x = 0x2545_f491_4f6c_dd1du64;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let mut tries = 16 * max_cases;
    while out.len() < max_cases && tries > 0 {
        tries -= 1;
        let case = lists
            .iter()
            .map(|l| l[(next() % l.len() as u64) as usize])
            .collect();
        add(case, &mut out);
    }
    out
}

/// A setup: runs on each engine with the case, makes what the function reads and the doubles of
/// its callees, and answers the words of the call.
type Setup = Box<dyn Fn(&mut Engine, &Case) -> Vec<u32>>;

/// One function's comparison ([`Diff::run`]).
pub struct Diff {
    addr: u32,
    typed: AbiFn,
    translation: Option<AbiFn>,
    inputs: Vec<Input>,
    setup: Option<Setup>,
    ignored_calls: Vec<u32>,
    control_words: Vec<u16>,
    max_cases: usize,
}

impl Diff {
    /// The typed version as `entry!` gives it: its exe address and its uniform form.
    pub fn new((addr, typed): (u32, AbiFn)) -> Diff {
        Diff {
            addr,
            typed,
            translation: None,
            inputs: Vec::new(),
            setup: None,
            ignored_calls: Vec::new(),
            control_words: vec![OTHER_THREADS, MAIN_THREAD],
            max_cases: 512,
        }
    }

    /// The function to compare with, for a test whose engine doesn't assemble the translation
    /// (the harness's own tests, or a translation in a crate above). By default the one registered
    /// at the address.
    pub fn translation(mut self, f: AbiFn) -> Diff {
        self.translation = Some(f);
        self
    }

    /// What to generate for each argument.
    pub fn inputs(mut self, inputs: &[Input]) -> Diff {
        self.inputs = inputs.to_vec();
        self
    }

    /// Runs on both engines before the call. Without it the call's words are the case's.
    pub fn setup(mut self, setup: impl Fn(&mut Engine, &Case) -> Vec<u32> + 'static) -> Diff {
        self.setup = Some(Box::new(setup));
        self
    }

    /// Callees left out of the comparison of outgoing calls: ones the typed version inlines or
    /// calls as Rust within its own family, where the translation calls by address.
    pub fn ignore_calls(mut self, addrs: &[u32]) -> Diff {
        self.ignored_calls.extend_from_slice(addrs);
        self
    }

    /// The control words to run each case under. By default both [`OTHER_THREADS`] and
    /// [`MAIN_THREAD`].
    pub fn control_words(mut self, words: &[u16]) -> Diff {
        self.control_words = words.to_vec();
        self
    }

    /// The most cases to generate (512 by default); each runs under each control word.
    pub fn max_cases(mut self, n: usize) -> Diff {
        self.max_cases = n;
        self
    }

    /// Runs every case, and compares.
    pub fn run(&self) -> Report {
        let mut report = Report::default();
        for case in cases(&self.inputs, self.max_cases) {
            for &word in &self.control_words {
                report.runs += 1;
                self.compare(&case, word, &mut report);
            }
        }
        report
    }

    fn compare(&self, case: &Case, word: u16, report: &mut Report) {
        let ours = self.side(case, word, self.translation);
        let theirs = self.side(case, word, Some(self.typed));
        assert_eq!(
            ours.words, theirs.words,
            "the setup gave the two engines different call words: it must do the same on both"
        );
        let mut found = |what: Found| {
            report.found += 1;
            if report.first.len() < KEEP_FOUND {
                report.first.push(Difference {
                    case: case.clone(),
                    control_word: word,
                    what,
                });
            }
        };
        if ours.stop != theirs.stop {
            found(Found::Stop {
                translation: ours.stop.clone(),
                typed: theirs.stop.clone(),
            });
        }
        if !same_ret(&ours.ret, &theirs.ret) {
            found(Found::Return {
                translation: ours.ret,
                typed: theirs.ret,
            });
        }
        let (ours_calls, theirs_calls) = (ours.calls(&self.ignored_calls), {
            theirs.calls(&self.ignored_calls)
        });
        if ours_calls != theirs_calls {
            let at = ours_calls
                .iter()
                .zip(&theirs_calls)
                .position(|(a, b)| a != b)
                .unwrap_or(ours_calls.len().min(theirs_calls.len()));
            found(Found::Call {
                at,
                translation: ours_calls.get(at).cloned(),
                typed: theirs_calls.get(at).cloned(),
            });
        }
        if ours.control_word != theirs.control_word {
            found(Found::ControlWord {
                translation: ours.control_word,
                typed: theirs.control_word,
            });
        }
        let (ours_heap, theirs_heap) = (ours.heap(), theirs.heap());
        if ours_heap != theirs_heap {
            found(Found::Heap {
                translation: ours_heap,
                typed: theirs_heap,
            });
        }
        compare_memory(&ours, &theirs, &mut found);
    }

    /// One engine's run of the case.
    fn side(&self, case: &Case, word: u16, at_address: Option<AbiFn>) -> Side {
        let mut e = Engine::new_untyped();
        // Made first on both sides, so an engine's first local or TLS read doesn't move the heap
        // blocks after it: a typed version with native locals may never make them.
        e.with_stack(8, |_, _| ());
        e.tls();
        let words = match &self.setup {
            Some(setup) => setup(&mut e, case),
            None => case.words(),
        };
        if let Some(f) = at_address {
            e.register(self.addr, f);
        }
        e.call_log = Some(Vec::new());
        x87::set_control_word(word);
        let result = catch_unwind(AssertUnwindSafe(|| e.call(self.addr, &words)));
        let control_word = x87::control_word();
        x87::set_control_word(x87::DEFAULT_CONTROL_WORD);
        let stack = e.guest_stack_region();
        let log = e.call_log.take().unwrap_or_default();
        let (ret, stop) = match result {
            Ok(ret) => (ret, None),
            Err(payload) => (Ret::default(), Some(panic_text(payload.as_ref()))),
        };
        Side {
            engine: e,
            words,
            ret,
            stop,
            log,
            stack,
            control_word,
        }
    }
}

/// How many differences a [`Report`] keeps in full.
const KEEP_FOUND: usize = 16;

fn same_ret(a: &Ret, b: &Ret) -> bool {
    a.eax == b.eax && a.edx == b.edx && a.st0.to_bits() == b.st0.to_bits()
}

fn panic_text(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "a stop with no message (a thrown exception?)".to_string())
}

/// What one engine did.
struct Side {
    engine: Engine,
    words: Vec<u32>,
    ret: Ret,
    stop: Option<String>,
    log: Vec<(u32, Vec<u32>)>,
    stack: Option<(u32, u32)>,
    control_word: u16,
}

/// The word an outgoing call's stack address compares as: both are "a stack address".
const STACK_WORD: u32 = 0xdead_57ac;

impl Side {
    /// The outgoing calls in order (the call itself is first), the ignored ones left out.
    fn calls(&self, ignored: &[u32]) -> Vec<(u32, Vec<u32>)> {
        let on_stack = |w: u32| self.stack.is_some_and(|(lo, hi)| (lo..hi).contains(&w));
        self.log
            .iter()
            .filter(|(addr, _)| !ignored.contains(addr))
            .map(|(addr, words)| {
                let words = words
                    .iter()
                    .map(|w| if on_stack(*w) { STACK_WORD } else { *w })
                    .collect();
                (*addr, words)
            })
            .collect()
    }

    /// (the top of the heap, the bytes it has left): the same on both unless one leaked or
    /// allocated more.
    fn heap(&self) -> (u32, u64) {
        (self.engine.mem.heap_top(), self.engine.mem.heap_available())
    }

    fn on_stack(&self, a: u32) -> bool {
        self.stack.is_some_and(|(lo, hi)| (lo..hi).contains(&a))
    }
}

/// Reports the bytes that differ between the two engines' memories, a run of them at a time,
/// outside the stack for locals, and the pages only one has.
fn compare_memory(ours: &Side, theirs: &Side, found: &mut impl FnMut(Found)) {
    let (a, b) = (&ours.engine.mem, &theirs.engine.mem);
    let (pages_a, pages_b) = (a.mapped_pages(), b.mapped_pages());
    let (mut i, mut j) = (0, 0);
    while i < pages_a.len() || j < pages_b.len() {
        match (pages_a.get(i), pages_b.get(j)) {
            (Some(&pa), Some(&pb)) if pa == pb => {
                compare_page(pa, a.page_bytes(pa), b.page_bytes(pa), ours, found);
                i += 1;
                j += 1;
            }
            (Some(&pa), pb) if pb.map_or(true, |&pb| pa < pb) => {
                if !ours.on_stack(pa) {
                    found(Found::Mapped {
                        page: pa,
                        by_typed: false,
                    });
                }
                i += 1;
            }
            (_, Some(&pb)) => {
                if !ours.on_stack(pb) {
                    found(Found::Mapped {
                        page: pb,
                        by_typed: true,
                    });
                }
                j += 1;
            }
            _ => unreachable!("the loop's condition says one list has a page left"),
        }
    }
}

fn compare_page(page: u32, a: &[u8], b: &[u8], side: &Side, found: &mut impl FnMut(Found)) {
    if a == b {
        return;
    }
    let mut k = 0;
    while k < a.len() {
        let start = k;
        while k < a.len() && a[k] != b[k] && !side.on_stack(page + k as u32) {
            k += 1;
        }
        if k > start {
            found(Found::Memory {
                addr: page + start as u32,
                translation: a[start..k].to_vec(),
                typed: b[start..k].to_vec(),
            });
        } else {
            k += 1;
        }
    }
}

/// One thing the two engines did differently.
#[derive(Clone, Debug, PartialEq)]
pub enum Found {
    /// One stopped and the other didn't, or they stopped with different messages.
    Stop {
        translation: Option<String>,
        typed: Option<String>,
    },
    /// The return value, bit for bit.
    Return { translation: Ret, typed: Ret },
    /// The `at`th call in the log (the call itself is 0): address and words.
    Call {
        at: usize,
        translation: Option<(u32, Vec<u32>)>,
        typed: Option<(u32, Vec<u32>)>,
    },
    /// A run of bytes after the call.
    Memory {
        addr: u32,
        translation: Vec<u8>,
        typed: Vec<u8>,
    },
    /// A page only one side has mapped.
    Mapped { page: u32, by_typed: bool },
    /// The heap's top and free bytes after the call: one side kept or freed more.
    Heap {
        translation: (u32, u64),
        typed: (u32, u64),
    },
    /// The control word after the call.
    ControlWord { translation: u16, typed: u16 },
}

/// A difference and the input that showed it.
#[derive(Clone, Debug, PartialEq)]
pub struct Difference {
    pub case: Case,
    pub control_word: u16,
    pub what: Found,
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "case {:?}, control word {:04x}: ",
            self.case.0, self.control_word
        )?;
        match &self.what {
            Found::Stop { translation, typed } => {
                write!(f, "stop: translation {translation:?}, typed {typed:?}")
            }
            Found::Return { translation, typed } => write!(
                f,
                "return: translation eax {:08x} edx {:08x} st0 {:?} ({:016x}), \
                 typed eax {:08x} edx {:08x} st0 {:?} ({:016x})",
                translation.eax,
                translation.edx,
                translation.st0,
                translation.st0.to_bits(),
                typed.eax,
                typed.edx,
                typed.st0,
                typed.st0.to_bits()
            ),
            Found::Call {
                at,
                translation,
                typed,
            } => write!(
                f,
                "outgoing call {at}: translation {translation:08x?}, typed {typed:08x?}"
            ),
            Found::Memory {
                addr,
                translation,
                typed,
            } => write!(
                f,
                "memory at {addr:08x}: translation {translation:02x?}, typed {typed:02x?}"
            ),
            Found::Mapped { page, by_typed } => write!(
                f,
                "page {page:08x} mapped only by the {}",
                if *by_typed {
                    "typed version"
                } else {
                    "translation"
                }
            ),
            Found::Heap { translation, typed } => write!(
                f,
                "heap (top, free): translation {translation:x?}, typed {typed:x?}"
            ),
            Found::ControlWord { translation, typed } => write!(
                f,
                "control word after: translation {translation:04x}, typed {typed:04x}"
            ),
        }
    }
}

/// What a [`Diff`] found.
#[derive(Default, Debug)]
pub struct Report {
    /// Cases run, each control word counted.
    pub runs: usize,
    /// Differences found.
    pub found: usize,
    /// The first of them in full.
    pub first: Vec<Difference>,
}

impl Report {
    /// Stops the test with the differences when there are any.
    #[track_caller]
    pub fn assert_same(&self) {
        assert!(self.found == 0, "{self}");
    }

    /// The test's guard against a comparison that ran nothing.
    pub fn is_same(&self) -> bool {
        self.runs > 0 && self.found == 0
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} difference(s) in {} run(s):", self.found, self.runs)?;
        for d in &self.first {
            writeln!(f, "  {d}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    const ADDR: u32 = 0x0040_2000;
    const CALLEE: u32 = 0x0040_2100;
    const OTHER_CALLEE: u32 = 0x0040_2110;

    // ---- The generator ------------------------------------------------------

    /// Every float class the plan names is in the lists.
    #[test]
    fn the_float_lists_hold_nan_zero_denormal_and_huge() {
        let f32s: Vec<f32> = F32_VALUES.iter().map(|b| f32::from_bits(*b)).collect();
        assert!(f32s.iter().any(|f| f.is_nan()));
        assert!(F32_VALUES.contains(&0));
        assert!(F32_VALUES.contains(&0x8000_0000));
        assert!(f32s.iter().any(|f| f.is_subnormal()));
        assert!(f32s.iter().any(|f| f.is_infinite()));
        assert!(F32_VALUES.contains(&f32::MAX.to_bits()));
        // A signalling NaN: quiet bit clear.
        assert!(f32s
            .iter()
            .any(|f| f.is_nan() && f.to_bits() & 0x0040_0000 == 0));
        let f64s: Vec<f64> = F64_VALUES.iter().map(|b| f64::from_bits(*b)).collect();
        assert!(f64s.iter().any(|f| f.is_nan()));
        assert!(F64_VALUES.contains(&0));
        assert!(f64s.iter().any(|f| f.is_subnormal()));
        assert!(F64_VALUES.contains(&f64::MAX.to_bits()));
        // Past the f32 range but finite in f64.
        assert!(f64s
            .iter()
            .any(|f| f.is_finite() && f.abs() > f64::from(f32::MAX)));
    }

    #[test]
    fn small_spaces_are_run_whole() {
        assert_eq!(cases(&[], 10), vec![Case(vec![])]);
        assert_eq!(cases(&[Input::Flag], 10).len(), 2);
        let both = cases(&[Input::Flag, Input::Of(&[7, 8, 9])], 10);
        assert_eq!(both.len(), 6);
        assert_eq!(both[0].0, [Value::Word(1), Value::Word(7)]);
        assert_eq!(both[5].0, [Value::Word(0), Value::Word(9)]);
        assert!(cases(&[Input::Of(&[])], 10).is_empty());
    }

    /// Past the cap every value of each argument still appears, and no case twice.
    #[test]
    fn large_spaces_sweep_each_argument_then_sample() {
        let inputs = [Input::F32, Input::Word, Input::F64];
        let list = cases(&inputs, 200);
        assert_eq!(list.len(), 200);
        let unique: HashSet<&Case> = list.iter().collect();
        assert_eq!(unique.len(), 200);
        for b in F32_VALUES {
            assert!(list.iter().any(|c| c.0[0] == Value::F32(*b)), "{b:08x}");
        }
        for b in F64_VALUES {
            assert!(list.iter().any(|c| c.0[2] == Value::F64(*b)), "{b:016x}");
        }
        // The same sequence each time, so a failure repeats.
        assert_eq!(list, cases(&inputs, 200));
    }

    #[test]
    fn case_words_pass_doubles_low_word_first() {
        let c = Case(vec![
            Value::Word(5),
            Value::F32(0x7f80_0001),
            Value::F64(0x1122_3344_5566_7788),
        ]);
        assert_eq!(c.words(), [5, 0x7f80_0001, 0x5566_7788, 0x1122_3344]);
        // A signalling NaN arrives with its bits.
        assert_eq!(c.f32(1).to_bits(), 0x7f80_0001);
    }

    // ---- Things that are the same pass -----------------------------------------

    fn truncate(_: &mut Engine, x: f32) -> i32 {
        crate::x87::fistp_chop(f64::from(x))
    }

    fn truncate_again(_: &mut Engine, x: f32) -> i32 {
        crate::x87::fistp_chop(f64::from(x))
    }

    #[test]
    fn the_same_behaviour_passes_every_input() {
        let report = Diff::new(crate::entry!(0x0040_2000, truncate_again(f32) -> i32))
            .translation(crate::entry!(0x0040_2000, truncate(f32) -> i32).1)
            .inputs(&[Input::F32])
            .run();
        report.assert_same();
        assert!(report.is_same());
        // Every value, under both control words.
        assert_eq!(report.runs, 2 * F32_VALUES.len());
    }

    /// A translation's `with_stack` local may become a native local: the stack for locals isn't
    /// compared, nor does the local move a later heap block, and an outgoing pointer to a local
    /// compares as "a stack address".
    #[test]
    fn a_local_may_leave_the_guest_stack() {
        fn on_guest_stack(e: &mut Engine, a: u32) -> u32 {
            e.with_stack(16, |e, local| {
                e.mem.set_u32(local.addr(), a);
                let block = e.mem.alloc(4);
                e.mem.set_u32(block, e.mem.u32(local.addr()) + 1);
                e.call(CALLEE, &[local.addr()]);
                block
            })
        }
        fn fewer_locals(e: &mut Engine, a: u32) -> u32 {
            let local = a;
            let block = e.mem.alloc(4);
            e.mem.set_u32(block, local + 1);
            // Its own, shallower local for the callee.
            e.with_stack(8, |e, out| e.call(CALLEE, &[out.addr()]));
            block
        }
        let report = Diff::new(crate::entry!(0x0040_2000, fewer_locals(u32) -> u32))
            .translation(crate::entry!(0x0040_2000, on_guest_stack(u32) -> u32).1)
            .inputs(&[Input::Word])
            .setup(|e, case| {
                e.register(CALLEE, |_, _| Ret::default());
                case.words()
            })
            .run();
        report.assert_same();
        assert!(report.is_same());
    }

    /// A callee the typed version inlines is ignored once it says so.
    #[test]
    fn ignored_callees_are_not_compared() {
        fn calling(e: &mut Engine, a: u32) -> u32 {
            e.call(CALLEE, &[a]).u32() + 1
        }
        fn inlined(_: &mut Engine, a: u32) -> u32 {
            a * 2 + 1
        }
        let diff = |ignore: &[u32]| {
            Diff::new(crate::entry!(0x0040_2000, inlined(u32) -> u32))
                .translation(crate::entry!(0x0040_2000, calling(u32) -> u32).1)
                .inputs(&[Input::Of(&[1, 2])])
                .setup(|e, case| {
                    e.register(CALLEE, |_, a| Ret {
                        eax: a[0] * 2,
                        ..Ret::default()
                    });
                    case.words()
                })
                .ignore_calls(ignore)
                .run()
        };
        assert!(diff(&[CALLEE]).is_same());
        assert!(!diff(&[]).is_same());
    }

    // ---- The stand-ins that are deliberately wrong: the harness catches each --------

    /// The class the plan expects (compare-1106's float-as-int): a typed version with Rust's
    /// saturating cast where the exe's `FISTP` gives the integer indefinite. It agrees on ordinary
    /// values and differs on NaN and past the int range, and the harness names those inputs.
    #[test]
    fn a_wrong_float_to_int_is_caught_on_nan_and_huge_only() {
        fn wrong_cast(_: &mut Engine, x: f32) -> i32 {
            x as i32
        }
        let report = Diff::new(crate::entry!(0x0040_2000, wrong_cast(f32) -> i32))
            .translation(crate::entry!(0x0040_2000, truncate(f32) -> i32).1)
            .inputs(&[Input::F32])
            .control_words(&[OTHER_THREADS])
            .max_cases(1000)
            .run();
        assert!(!report.is_same());
        assert!(report.first.len() <= KEEP_FOUND);
        for d in &report.first {
            assert!(matches!(d.what, Found::Return { .. }), "{d}");
            // Never on a value inside the int range.
            let x = d.case.f32(0);
            assert!(
                x.is_nan() || f64::from(x).abs() >= 2_147_483_648.0,
                "not an edge: {d}"
            );
        }
        let flagged = |bits: u32| report.first.iter().any(|d| d.case.0[0] == Value::F32(bits));
        assert!(flagged(0x7fc0_0000), "a quiet NaN:\n{report}");
        assert!(flagged(0x7f80_0001), "a signalling NaN:\n{report}");
        assert!(flagged(0x4f00_0000), "2^31:\n{report}");
        assert!(flagged(0x7f80_0000), "infinity:\n{report}");
        // -2^31 fits: both give it.
        assert!(!flagged(0xcf00_0000));
        // And the message names the input and the two answers.
        let text = report.to_string();
        assert!(text.contains("return: translation eax 80000000"), "{text}");
        assert!(text.contains("typed eax 00000000"), "{text}");
    }

    /// A wrong float result by bits: a NaN's sign, or a flushed denormal, compare unequal though
    /// `==` on values would call NaNs different and -0 equal to 0.
    #[test]
    fn a_wrong_float_return_is_caught_by_bits() {
        fn exact(_: &mut Engine, x: f64) -> f64 {
            x
        }
        fn flushes(_: &mut Engine, x: f64) -> f64 {
            if x.is_subnormal() || x == 0.0 {
                0.0
            } else {
                x
            }
        }
        let report = Diff::new(crate::entry!(0x0040_2000, flushes(f64) -> f64))
            .translation(crate::entry!(0x0040_2000, exact(f64) -> f64).1)
            .inputs(&[Input::F64])
            .control_words(&[OTHER_THREADS])
            .run();
        let cases: Vec<u64> = report
            .first
            .iter()
            .map(|d| d.case.f64(0).to_bits())
            .collect();
        // -0 and both denormals, and no NaN is called different from itself.
        assert_eq!(
            cases,
            [
                0x8000_0000_0000_0000,
                0x0000_0000_0000_0001,
                0x000f_ffff_ffff_ffff
            ],
            "{report}"
        );
    }

    /// x87 rounding: a typed version without the helper agrees at 53 bits and differs at the
    /// main thread's 24, so each control word has to run.
    #[test]
    fn a_missing_precision_step_shows_under_the_main_threads_control_word() {
        fn with_helper(_: &mut Engine, a: f64, b: f64) -> f64 {
            x87_add(a, b)
        }
        fn plain_add(_: &mut Engine, a: f64, b: f64) -> f64 {
            a + b
        }
        let report = Diff::new(crate::entry!(0x0040_2000, plain_add(f64, f64) -> f64))
            .translation(crate::entry!(0x0040_2000, with_helper(f64, f64) -> f64).1)
            .inputs(&[Input::F64, Input::Of(&[])])
            .run();
        assert_eq!(report.runs, 0, "an empty input list has no cases");
        let report = Diff::new(crate::entry!(0x0040_2000, plain_add(f64, f64) -> f64))
            .translation(crate::entry!(0x0040_2000, with_helper(f64, f64) -> f64).1)
            .inputs(&[Input::F64, Input::F64])
            .max_cases(600)
            .run();
        assert!(report.found > 0);
        assert!(
            report.first.iter().any(|d| d.control_word == MAIN_THREAD),
            "{report}"
        );
        assert!(
            report.first.iter().all(|d| d.control_word == MAIN_THREAD),
            "53 bits needs no helper: {report}"
        );
    }

    /// Memory written: a stand-in that stores the same value at the wrong offset, one that skips a
    /// store on one input, and one that writes a word too many.
    #[test]
    fn wrong_memory_is_caught() {
        fn stores(e: &mut Engine, p: Ptr, v: u32) {
            e.mem.set_u32(p.addr() + 8, v);
        }
        fn wrong_offset(e: &mut Engine, p: Ptr, v: u32) {
            e.mem.set_u32(p.addr() + 12, v);
        }
        fn skips_zero(e: &mut Engine, p: Ptr, v: u32) {
            if v != 0 {
                e.mem.set_u32(p.addr() + 8, v);
            }
        }
        fn one_too_many(e: &mut Engine, p: Ptr, v: u32) {
            e.mem.set_u32(p.addr() + 8, v);
            e.mem.set_u32(p.addr() + 4, 1);
        }
        let translation = crate::entry!(0x0040_2000, stores(Ptr, u32)).1;
        let run = |typed: (u32, AbiFn)| {
            Diff::new(typed)
                .translation(translation)
                .inputs(&[Input::Of(&[0, 0x1234])])
                .control_words(&[OTHER_THREADS])
                .setup(|e, case| {
                    let block = e.mem.alloc(0x20);
                    e.mem.set_u32(block + 8, 0xaaaa_aaaa);
                    vec![block, case.word(0)]
                })
                .run()
        };

        let report = run(crate::entry!(0x0040_2000, wrong_offset(Ptr, u32)));
        assert_eq!(report.found, 2, "{report}");
        // Storing 0: the old word stays where the translation cleared it.
        let Found::Memory {
            ref translation,
            ref typed,
            ..
        } = report.first[0].what
        else {
            panic!("{report}");
        };
        assert_eq!(
            (&translation[..], &typed[..]),
            (&[0; 4][..], &[0xaa; 4][..])
        );
        // Storing 0x1234: one run from the word the translation wrote into the next one.
        let Found::Memory {
            ref translation,
            ref typed,
            ..
        } = report.first[1].what
        else {
            panic!("{report}");
        };
        assert_eq!(translation[..], [0x34, 0x12, 0, 0, 0, 0]);
        assert_eq!(typed[..], [0xaa, 0xaa, 0xaa, 0xaa, 0x34, 0x12]);

        let report = run(crate::entry!(0x0040_2000, skips_zero(Ptr, u32)));
        assert_eq!(report.found, 1, "{report}");
        assert_eq!(report.first[0].case.word(0), 0);
        assert!(matches!(report.first[0].what, Found::Memory { .. }));

        let report = run(crate::entry!(0x0040_2000, one_too_many(Ptr, u32)));
        assert_eq!(report.found, 2, "{report}");
    }

    /// Outgoing calls: the wrong word, the wrong order, one missing, one extra.
    #[test]
    fn wrong_outgoing_calls_are_caught() {
        fn two_calls(e: &mut Engine, a: u32) {
            e.call(CALLEE, &[a]);
            e.call(OTHER_CALLEE, &[a + 1]);
        }
        fn wrong_word(e: &mut Engine, a: u32) {
            e.call(CALLEE, &[a]);
            e.call(OTHER_CALLEE, &[a + 2]);
        }
        fn wrong_order(e: &mut Engine, a: u32) {
            e.call(OTHER_CALLEE, &[a + 1]);
            e.call(CALLEE, &[a]);
        }
        fn missing(e: &mut Engine, a: u32) {
            e.call(CALLEE, &[a]);
        }
        fn extra(e: &mut Engine, a: u32) {
            e.call(CALLEE, &[a]);
            e.call(OTHER_CALLEE, &[a + 1]);
            e.call(CALLEE, &[a]);
        }
        let translation = crate::entry!(0x0040_2000, two_calls(u32)).1;
        let run = |typed: (u32, AbiFn)| {
            let report = Diff::new(typed)
                .translation(translation)
                .inputs(&[Input::Of(&[5])])
                .control_words(&[OTHER_THREADS])
                .setup(|e, case| {
                    e.register(CALLEE, |_, _| Ret::default());
                    e.register(OTHER_CALLEE, |_, _| Ret::default());
                    case.words()
                })
                .run();
            assert_eq!(report.found, 1, "{report}");
            report.first[0].what.clone()
        };
        assert_eq!(
            run(crate::entry!(0x0040_2000, wrong_word(u32))),
            Found::Call {
                at: 2,
                translation: Some((OTHER_CALLEE, vec![6])),
                typed: Some((OTHER_CALLEE, vec![7])),
            }
        );
        assert_eq!(
            run(crate::entry!(0x0040_2000, wrong_order(u32))),
            Found::Call {
                at: 1,
                translation: Some((CALLEE, vec![5])),
                typed: Some((OTHER_CALLEE, vec![6])),
            }
        );
        assert_eq!(
            run(crate::entry!(0x0040_2000, missing(u32))),
            Found::Call {
                at: 2,
                translation: Some((OTHER_CALLEE, vec![6])),
                typed: None,
            }
        );
        assert_eq!(
            run(crate::entry!(0x0040_2000, extra(u32))),
            Found::Call {
                at: 3,
                translation: None,
                typed: Some((CALLEE, vec![5])),
            }
        );
    }

    /// A stop is behaviour: the translation stops on a bad pointer where a stand-in tests it first.
    #[test]
    fn a_missing_stop_is_caught() {
        fn reads(e: &mut Engine, p: u32) -> u32 {
            e.mem.u32(p)
        }
        fn tests_first(e: &mut Engine, p: u32) -> u32 {
            if e.mem.is_mapped(p) {
                e.mem.u32(p)
            } else {
                0
            }
        }
        let report = Diff::new(crate::entry!(0x0040_2000, tests_first(u32) -> u32))
            .translation(crate::entry!(0x0040_2000, reads(u32) -> u32).1)
            .inputs(&[Input::Of(&[0x0300_0000])])
            .control_words(&[OTHER_THREADS])
            .run();
        assert_eq!(report.found, 1, "{report}");
        assert_eq!(
            report.first[0].what,
            Found::Stop {
                translation: Some("read of unmapped memory at 03000000".to_string()),
                typed: None,
            }
        );
        // Both stopping alike is the same behaviour.
        Diff::new(crate::entry!(0x0040_2000, reads(u32) -> u32))
            .translation(crate::entry!(0x0040_2000, reads(u32) -> u32).1)
            .inputs(&[Input::Of(&[0x0300_0000])])
            .run()
            .assert_same();
    }

    /// A block the translation frees and a stand-in keeps.
    #[test]
    fn a_leak_is_caught() {
        fn frees(e: &mut Engine) {
            let block = e.mem.alloc(0x100);
            e.mem.free(block);
        }
        fn leaks(e: &mut Engine) {
            e.mem.alloc(0x100);
        }
        let report = Diff::new(crate::entry!(0x0040_2000, leaks()))
            .translation(crate::entry!(0x0040_2000, frees()).1)
            .control_words(&[OTHER_THREADS])
            .run();
        assert!(
            report
                .first
                .iter()
                .any(|d| matches!(d.what, Found::Heap { .. })),
            "{report}"
        );
    }

    /// A control word left changed.
    #[test]
    fn a_changed_control_word_is_caught() {
        fn leaves_it(_: &mut Engine) {}
        fn changes_it(_: &mut Engine) {
            crate::x87::set_control_word(MAIN_THREAD);
        }
        let report = Diff::new(crate::entry!(0x0040_2000, changes_it()))
            .translation(crate::entry!(0x0040_2000, leaves_it()).1)
            .control_words(&[OTHER_THREADS])
            .run();
        assert_eq!(report.found, 1, "{report}");
        assert_eq!(
            report.first[0].what,
            Found::ControlWord {
                translation: OTHER_THREADS,
                typed: MAIN_THREAD,
            }
        );
    }

    /// The harness stands against `NV_TYPED`, and both engines hold translations: the callee is
    /// never a typed version, and the target's address is registered only with what was asked.
    #[test]
    fn the_engines_are_untyped_and_the_call_is_at_the_address() {
        let report = Diff::new((ADDR, |e, a| Ret {
            eax: e.call_stack.last().copied().unwrap_or(0) + a[0],
            ..Ret::default()
        }))
        .translation(|e, a| Ret {
            eax: e.call_stack.last().copied().unwrap_or(0) + a[0],
            ..Ret::default()
        })
        .inputs(&[Input::Of(&[1])])
        .run();
        report.assert_same();
    }

    #[test]
    #[should_panic(expected = "different call words")]
    fn a_setup_that_differs_between_engines_is_refused() {
        use std::cell::Cell;
        use std::rc::Rc;
        let n = Rc::new(Cell::new(0));
        let shared = n.clone();
        Diff::new((ADDR, |_, _| Ret::default()))
            .translation(|_, _| Ret::default())
            .inputs(&[Input::Of(&[1])])
            .setup(move |_, _| {
                shared.set(shared.get() + 1);
                vec![shared.get()]
            })
            .run();
    }

    /// A report says what it found, and doesn't grow without bound.
    #[test]
    fn a_report_keeps_the_first_differences() {
        fn one(_: &mut Engine, _: u32) -> u32 {
            1
        }
        fn two(_: &mut Engine, _: u32) -> u32 {
            2
        }
        let report = Diff::new(crate::entry!(0x0040_2000, two(u32) -> u32))
            .translation(crate::entry!(0x0040_2000, one(u32) -> u32).1)
            .inputs(&[Input::Word])
            .max_cases(1000)
            .control_words(&[OTHER_THREADS, MAIN_THREAD])
            .run();
        assert_eq!(report.runs, 2 * WORD_VALUES.len());
        assert_eq!(report.found, report.runs);
        assert_eq!(report.first.len(), KEEP_FOUND);
        let text = report.to_string();
        assert!(
            text.starts_with("18 difference(s) in 18 run(s):\n"),
            "{text}"
        );
        assert!(
            text.contains("case [0x1], control word 027f: return"),
            "{text}"
        );
    }
}
