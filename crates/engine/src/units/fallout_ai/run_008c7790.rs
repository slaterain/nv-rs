//! `fallout/ai/run_008c7790` (Xbox PDB source unit), subsystem `fallout/ai`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// The object whose value word `0043d4d0` locates (its `+4`, or a static
/// zero word for null); `*value > 1` enables the whole check.
const GATE_OBJECT: u32 = 0x011c_3ea4;
/// The global that holds the object `0044edb0` is called on (its `+0x10` is
/// the current procedure index, `BaseProcess::GetCurrentProcedureIndex`
/// (Xbox PDB name of `0044edb0`)).
const PROCESS_REFERENCE: u32 = 0x011d_ea0c;

/// `0043d4d0`: address of the word at `+4` of the object (a static zero word
/// for a null object).
const VALUE_WORD_ADDRESS: u32 = 0x0043_d4d0;
/// `00713d90`: a global byte.
const ENABLED_FLAG: u32 = 0x0071_3d90;
/// `004537b0`: a global pointer.
const CURRENT_OBJECT: u32 = 0x0045_37b0;
/// `0087a730` (`__thiscall` on the object): a bool.
const OBJECT_QUALIFIES: u32 = 0x0087_a730;
/// `0040fc90`: `GetCurrentThreadId` wrapper.
const THREAD_ID: u32 = 0x0040_fc90;
/// `0044edb0` (`__thiscall`): the word at `+0x10`.
const PROCEDURE_INDEX: u32 = 0x0044_edb0;
/// `008c7bc0`: a global byte.
const SECOND_FLAG: u32 = 0x008c_7bc0;
/// `00713d80`: the address of a static table.
const TABLE: u32 = 0x0071_3d80;
/// `008c7a30` (`__thiscall` on the table, `RET 4`): the table entry for an
/// index below 2, else 0.
const TABLE_LOOKUP: u32 = 0x008c_7a30;
/// `0084e3a0`: the word at `+0xc` of the object.
const OBJECT_VALUE: u32 = 0x0084_e3a0;

/// Entry `index` of the static table.
fn table_entry(e: &mut Engine, index: u32) -> u32 {
    let table = e.call(TABLE, &args![]).u32();
    e.call(TABLE_LOOKUP, &args![table, index]).u32()
}

// Translated from 008c7aa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the current thread has to be treated specially (the result is an
/// `OR` of two conditions). Nothing happens unless the gate value is above 1
/// and the enabled flag is set. Then:
///
/// - `b` is whether the current object qualifies (`0087a730`);
/// - `a` is decided by comparing the thread id with the current procedure
///   index. Equal: unless the second flag is set, `a` holds when table entry
///   0 is below 11 or table entry 1 is below 7. Different: `a` holds when the
///   second flag is set or table entry 1 is below 6.
/// - if `a` is still false, it holds when the current object's value word
///   (`0084e3a0`) is non-zero and differs from the thread id.
///
/// Returns `a || b`. The names of the flags and table entries are not known.
pub fn fn_008c7aa0(e: &mut Engine) -> bool {
    let mut a = false;
    let mut b = false;
    let gate = e.call(VALUE_WORD_ADDRESS, &args![GATE_OBJECT]).u32();
    if e.mem.i32(gate) > 1 && e.call(ENABLED_FLAG, &args![]).bool() {
        let object = e.call(CURRENT_OBJECT, &args![]).u32();
        if e.call(OBJECT_QUALIFIES, &args![object]).bool() {
            b = true;
        }
        let thread = e.call(THREAD_ID, &args![]).u32();
        let process = e.mem.u32(PROCESS_REFERENCE);
        let procedure = e.call(PROCEDURE_INDEX, &args![process]).u32();
        if thread == procedure {
            if !e.call(SECOND_FLAG, &args![]).bool() {
                a = table_entry(e, 0) < 0xb || table_entry(e, 1) < 7;
            }
        } else if e.call(SECOND_FLAG, &args![]).bool() || table_entry(e, 1) < 6 {
            a = true;
        }
        if !a {
            let object = e.call(CURRENT_OBJECT, &args![]).u32();
            let value = e.call(OBJECT_VALUE, &args![object]).u32();
            if value != 0 && thread != value {
                a = true;
            }
        }
    }
    a || b
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x008c7aa0, fn_008c7aa0() -> bool)]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the callees answer.
    #[derive(Clone, Copy)]
    struct Setup {
        gate: i32,
        enabled: bool,
        qualifies: bool,
        thread: u32,
        procedure: u32,
        second_flag: bool,
        entries: [u32; 2],
        object_value: u32,
    }

    /// Everything off: the result is false and the thread (5) differs from
    /// the procedure (6), with large table entries.
    const QUIET: Setup = Setup {
        gate: 2,
        enabled: true,
        qualifies: false,
        thread: 5,
        procedure: 6,
        second_flag: false,
        entries: [100, 100],
        object_value: 0,
    };

    fn run(s: Setup) -> bool {
        let mut e = Engine::new();
        e.map(0x011d_e000, 0x1000);
        let gate = e.mem.alloc(4);
        e.mem.set_i32(gate, s.gate);
        e.register_double(VALUE_WORD_ADDRESS, move |_, a| {
            assert_eq!(a[0], GATE_OBJECT);
            Ret {
                eax: gate,
                ..Ret::default()
            }
        });
        e.register_double(ENABLED_FLAG, move |_, _| (s.enabled as u32).into_ret_u32());
        e.register_double(CURRENT_OBJECT, |_, _| 0x7777u32.into_ret_u32());
        e.register_double(OBJECT_QUALIFIES, move |_, a| {
            assert_eq!(a[0], 0x7777);
            (s.qualifies as u32).into_ret_u32()
        });
        e.register_double(THREAD_ID, move |_, _| s.thread.into_ret_u32());
        e.mem.set_u32(PROCESS_REFERENCE, 0x8888);
        e.register_double(PROCEDURE_INDEX, move |_, a| {
            assert_eq!(a[0], 0x8888);
            s.procedure.into_ret_u32()
        });
        e.register_double(SECOND_FLAG, move |_, _| {
            (s.second_flag as u32).into_ret_u32()
        });
        e.register_double(TABLE, |_, _| 0x9999u32.into_ret_u32());
        e.register_double(TABLE_LOOKUP, move |_, a| {
            assert_eq!(a[0], 0x9999);
            s.entries[a[1] as usize].into_ret_u32()
        });
        e.register_double(OBJECT_VALUE, move |_, a| {
            assert_eq!(a[0], 0x7777);
            s.object_value.into_ret_u32()
        });
        e.call(0x008c_7aa0, &args![]).bool()
    }

    trait IntoRetU32 {
        fn into_ret_u32(self) -> Ret;
    }
    impl IntoRetU32 for u32 {
        fn into_ret_u32(self) -> Ret {
            Ret {
                eax: self,
                ..Ret::default()
            }
        }
    }

    #[test]
    fn nothing_happens_without_the_gate() {
        assert!(!run(Setup {
            gate: 1,
            qualifies: true,
            ..QUIET
        }));
        assert!(!run(Setup {
            enabled: false,
            qualifies: true,
            ..QUIET
        }));
    }

    #[test]
    fn a_qualifying_object_is_enough() {
        assert!(!run(QUIET));
        assert!(run(Setup {
            qualifies: true,
            ..QUIET
        }));
    }

    #[test]
    fn same_thread_and_procedure_checks_both_table_entries() {
        let same = Setup {
            procedure: 5,
            ..QUIET
        };
        assert!(!run(same));
        assert!(run(Setup {
            entries: [10, 100],
            ..same
        }));
        assert!(!run(Setup {
            entries: [11, 7],
            ..same
        }));
        assert!(run(Setup {
            entries: [11, 6],
            ..same
        }));
        // The second flag skips the table check altogether.
        assert!(!run(Setup {
            second_flag: true,
            entries: [0, 0],
            ..same
        }));
    }

    #[test]
    fn different_procedure_checks_the_flag_or_the_second_entry() {
        assert!(run(Setup {
            second_flag: true,
            ..QUIET
        }));
        assert!(run(Setup {
            entries: [100, 5],
            ..QUIET
        }));
        assert!(!run(Setup {
            entries: [0, 6],
            ..QUIET
        }));
    }

    #[test]
    fn a_foreign_owner_of_the_object_value_counts() {
        assert!(run(Setup {
            object_value: 9,
            ..QUIET
        }));
        // Owned by this very thread: no.
        assert!(!run(Setup {
            object_value: 5,
            ..QUIET
        }));
    }
}
