// Measures this x86-64 processor's RCPSS and RSQRTSS and writes engine-core's estimate tables
// (crates/engine-core/src/estimate/tables.rs) as the file given as the argument.
//
// The tables hold the result for 1.m (RSQRTSS also for 2 * 1.m), m's top 12 bits. Before writing,
// the model engine-core builds from them (copied below: `rcp_model`, `rsqrt_model`, the same
// rules) is compared with the instruction on ALL 2^32 inputs: that proves at once that the low 11
// mantissa bits never matter, that the exponent only moves the result's exponent, and that the
// special inputs (zero, denormal, infinity, NaN, negative) answer as the model says. Any
// difference stops the tool and nothing is written. The output is byte for byte the committed
// file when run on the processor named in its header.
use std::arch::x86_64::*;
use std::fmt::Write;

fn rcp(x: u32) -> u32 {
    unsafe { _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(f32::from_bits(x)))).to_bits() }
}

fn rsq(x: u32) -> u32 {
    unsafe { _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(f32::from_bits(x)))).to_bits() }
}

const DEFAULT_NAN: u32 = 0xffc0_0000;
const SIGN: u32 = 0x8000_0000;
const EXPONENT: u32 = 0x7f80_0000;
const MANTISSA: u32 = 0x007f_ffff;
const QUIET: u32 = 0x0040_0000;

fn entry_of(r: u32) -> u16 {
    assert_eq!(r & 0x7ff, 0, "a result with bits below the table's 12");
    let e = (r >> 23) & 0xff;
    assert!((120..136).contains(&e), "a result exponent outside the table's range: {e}");
    (((e - 120) << 12) | ((r >> 11) & 0xfff)) as u16
}

// engine-core estimate.rs, the same rules.
fn from_entry(entry: u16, sign: u32, shift: i32) -> u32 {
    let entry = u32::from(entry);
    let exponent = (entry >> 12) as i32 + 120 + shift;
    if exponent < 1 {
        return sign;
    }
    sign | ((exponent as u32) << 23) | ((entry & 0xfff) << 11)
}

fn rcp_model(t: &[u16], x: u32) -> u32 {
    let sign = x & SIGN;
    let exponent = x & EXPONENT;
    if exponent == EXPONENT {
        return if x & MANTISSA != 0 { x | QUIET } else { sign };
    }
    if exponent == 0 {
        return sign | EXPONENT;
    }
    let field = (exponent >> 23) as i32;
    from_entry(t[((x & MANTISSA) >> 11) as usize], sign, 127 - field)
}

fn rsqrt_model(t: &[u16], x: u32) -> u32 {
    let sign = x & SIGN;
    let exponent = x & EXPONENT;
    if exponent == EXPONENT {
        return if x & MANTISSA != 0 {
            x | QUIET
        } else if sign != 0 {
            DEFAULT_NAN
        } else {
            0
        };
    }
    if exponent == 0 {
        return sign | EXPONENT;
    }
    if sign != 0 {
        return DEFAULT_NAN;
    }
    let field = (exponent >> 23) as i32;
    let (half, reference) = if field & 1 == 1 { (0, 127) } else { (4096, 128) };
    from_entry(t[half + ((x & MANTISSA) >> 11) as usize], 0, (reference - field) / 2)
}

fn main() {
    let out = std::env::args().nth(1).expect("the output file");
    let rcp_t: Vec<u16> = (0..4096u32).map(|i| entry_of(rcp((127 << 23) | (i << 11)))).collect();
    let rsq_t: Vec<u16> = (0..2u32)
        .flat_map(|p| (0..4096u32).map(move |i| entry_of(rsq(((127 + p) << 23) | (i << 11)))))
        .collect();
    let (mut rcp_bad, mut rsq_bad) = (0u64, 0u64);
    for x in 0..=u32::MAX {
        if rcp_model(&rcp_t, x) != rcp(x) {
            if rcp_bad < 5 {
                eprintln!("RCPSS {x:08x}: the model {:08x}, the instruction {:08x}", rcp_model(&rcp_t, x), rcp(x));
            }
            rcp_bad += 1;
        }
        if rsqrt_model(&rsq_t, x) != rsq(x) {
            if rsq_bad < 5 {
                eprintln!("RSQRTSS {x:08x}: the model {:08x}, the instruction {:08x}", rsqrt_model(&rsq_t, x), rsq(x));
            }
            rsq_bad += 1;
        }
    }
    println!("of all 2^32 inputs, the model differs from the instruction on: RCPSS {rcp_bad}, RSQRTSS {rsq_bad}");
    assert!(rcp_bad == 0 && rsq_bad == 0, "the model is not this processor's: nothing written");
    let mut s = String::new();
    let brand = processor_brand();
    writeln!(s, "// Measured with scripts/estimate-measure on {brand}: RCPSS and RSQRTSS").unwrap();
    writeln!(s, "// of every 1.m (rsqrt also 2 * 1.m), m's top 12 bits; each entry is the result's exponent - 120").unwrap();
    writeln!(s, "// (bits 12..16) and its mantissa's top 12 bits (bits 0..12). Generated: do not edit by hand.").unwrap();
    for (name, t) in [("RCP", &rcp_t), ("RSQRT", &rsq_t)] {
        // Kept as written: rustfmt would rewrap the rows, and the file must stay the tool's output.
        writeln!(s, "#[rustfmt::skip]").unwrap();
        writeln!(s, "pub(super) const {name}: [u16; {}] = [", t.len()).unwrap();
        for row in t.chunks(16) {
            let line: Vec<String> = row.iter().map(|v| format!("0x{v:04x}")).collect();
            writeln!(s, "    {},", line.join(", ")).unwrap();
        }
        writeln!(s, "];").unwrap();
    }
    std::fs::write(&out, s).unwrap();
    println!("wrote {out}");
}

fn processor_brand() -> String {
    let mut b = Vec::new();
    for leaf in 0x8000_0002u32..=0x8000_0004 {
        let r = unsafe { __cpuid(leaf) };
        for w in [r.eax, r.ebx, r.ecx, r.edx] {
            b.extend_from_slice(&w.to_le_bytes());
        }
    }
    String::from_utf8_lossy(&b).trim_end_matches('\0').trim().to_string()
}
