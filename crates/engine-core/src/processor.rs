//! What the game is told about the processor where it cannot ask a real x86 one (Apple silicon
//! and any other non-x86 machine; decision 3 of the Linux and macOS track): a fixed answer, the
//! one every x86-64 PC made since about 2008 gives for the bits the game tests, so the game takes
//! the same paths as on players' PCs. On x86 and x86-64 the real processor answers (the machine
//! table's `Cpuid` lines, `engine-gamebryo`'s probe) and this table is not used.
//!
//! Every place the game asks (FalloutNV.exe 1.4.0.525, every `CPUID` instruction in `.text`):
//! - `__get_sse2_info` (`00ed1927`): the `ID` flag test (`00ed193c`), then `CPUID` leaf 0
//!   (`00ed1951`; its four words are stored and never read) and leaf 1 (`00ed1964`; only `EDX` bit
//!   26, SSE2, is tested). With SSE2 the C runtime's SSE2 maths is chosen.
//! - NiMain's probe `00a74a10` (`00a74a56`, `00a74aa4`, `00a74af7`): leaf `0x80000000` (twice) and,
//!   only when its `EAX` lacks bit 31, leaf 1 (`EDX` bits 15 CMOV, 23 MMX, 25 SSE). Bit 31 set gives
//!   mode 6, which every x86-64 PC answers (an x86-64 processor reports extended leaves up to at
//!   least `0x80000008`, the address sizes long mode needs).
//!
//! The answer sets exactly the tested bits; every other word is 0 (nothing reads them), except
//! leaf 0's `EAX` (the highest leaf), 1, so the leaf the game asks next exists. A declared
//! difference from a real PC: no vendor string, no family/model, no other feature bits. The
//! processor COUNT is not here: `GetSystemInfo` reports the real machine's.

/// The `ID` flag test's answer (`EFLAGS` read back after flipping bit 21, minus the entry flags):
/// `0x0020_0000`, a processor with `CPUID` whose `ID` bit is clear at the start, as a process
/// starts on Windows (measured on this PC's capture, `CpuIdFlagTest 200000`).
pub const FIXED_ID_FLAG_TEST: u32 = 0x0020_0000;

/// Leaf 1's `EDX` bits the game tests: 15 CMOV, 23 MMX, 25 SSE, 26 SSE2 (all on every x86-64
/// processor).
pub const LEAF_1_EDX: u32 = (1 << 15) | (1 << 23) | (1 << 25) | (1 << 26);

/// The fixed `CPUID` answer (`EAX`, `EBX`, `ECX`, `EDX`) for the leaves the game asks; `None` for
/// any other leaf (the game asks none).
pub fn fixed_cpuid(leaf: u32) -> Option<[u32; 4]> {
    match leaf {
        0 => Some([1, 0, 0, 0]),
        1 => Some([0, 0, 0, LEAF_1_EDX]),
        0x8000_0000 => Some([0x8000_0008, 0, 0, 0]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tested bits as an x86-64 PC answers them: SSE2 for `__get_sse2_info`; bit 31 of the
    /// extended leaf (mode 6 in NiMain's probe), and MMX, CMOV and SSE should leaf 1 be read.
    #[test]
    fn the_fixed_answer_has_the_bits_the_game_tests() {
        assert_eq!(fixed_cpuid(1).unwrap()[3] & 0x0400_0000, 0x0400_0000);
        assert_eq!(fixed_cpuid(1).unwrap()[3], 0x0680_8000);
        assert_eq!(
            fixed_cpuid(0x8000_0000).unwrap()[0] & 0x8000_0000,
            0x8000_0000
        );
        assert!(fixed_cpuid(0).unwrap()[0] >= 1);
        assert_eq!(fixed_cpuid(2), None);
        assert_eq!(fixed_cpuid(0x8000_0001), None);
        assert_eq!(FIXED_ID_FLAG_TEST, 0x0020_0000);
    }
}
