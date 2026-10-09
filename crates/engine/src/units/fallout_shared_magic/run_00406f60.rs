//! `fallout shared/magic/run_00406f60` (Xbox PDB source unit), subsystem `fallout shared/magic`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// The `va_list` form of the formatter: `__thiscall` on the string object,
/// arguments (format, `va_list`), `RET 8`. It formats into a 1 KiB buffer
/// and stores the text in the string (`004037f0`).
const FORMAT_VA_LIST: u32 = 0x0040_6f90;

// Translated from 00406f60 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `printf`-style string formatter (`cdecl`): `string` is the string
/// object that receives the text, `format` the format string, and
/// `variable_arguments` the words the caller pushed after it. They are copied
/// to memory because the `va_list` handed to `00406f90` is the address of
/// the first one. Returns what `00406f90` returns (`EAX`).
pub fn fn_00406f60(e: &mut Engine, string: Ptr, format: Ptr, variable_arguments: &[u32]) -> u32 {
    let list = e.mem.alloc(4 * variable_arguments.len().max(1) as u32);
    for (i, word) in variable_arguments.iter().enumerate() {
        e.mem.set_u32(list + 4 * i as u32, *word);
    }
    let result = e.call(FORMAT_VA_LIST, &args![string, format, list]).u32();
    e.mem.free(list);
    result
}

/// The uniform form of [`fn_00406f60`]: it takes any number of argument
/// words, so it cannot go through `entry!`. The first two words are the
/// string and the format; the rest are the variable arguments.
fn fn_00406f60_uniform(e: &mut Engine, a: &[u32]) -> Ret {
    fn_00406f60(e, Ptr::new(a[0]), Ptr::new(a[1]), &a[2..]).into_ret()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![(0x00406f60, fn_00406f60_uniform as AbiFn)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_a_va_list_over_the_extra_words() {
        let mut e = Engine::new();
        // The double reads the words the va_list points at.
        e.register(FORMAT_VA_LIST, |e, a| {
            assert_eq!((a[0], a[1]), (0x1000, 0x2000));
            Ret {
                eax: e.mem.u32(a[2]) + e.mem.u32(a[2] + 8),
                ..Ret::default()
            }
        });
        let r = e.call(0x0040_6f60, &args![0x1000u32, 0x2000u32, 5u32, 6u32, 7u32]);
        assert_eq!(r.u32(), 12);
    }

    #[test]
    fn without_variable_arguments_the_list_is_still_a_valid_address() {
        let mut e = Engine::new();
        e.register(FORMAT_VA_LIST, |_, a| Ret {
            eax: a[2],
            ..Ret::default()
        });
        assert_ne!(e.call(0x0040_6f60, &args![1u32, 2u32]).u32(), 0);
    }
}
