// Measures this processor's RCPSS and RSQRTSS: proves on all 2^32 inputs that the result depends
// only on the input's exponent (rsqrt: its parity) and the top 12 bits of its mantissa, and writes
// the two tables (the result for 1.m, rsqrt also for 2*1.m) as a Rust source file.
use std::arch::x86_64::*;
use std::fmt::Write;
fn rcp(x: u32) -> u32 { unsafe { _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(f32::from_bits(x)))).to_bits() } }
fn rsq(x: u32) -> u32 { unsafe { _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(f32::from_bits(x)))).to_bits() } }
fn main() {
    let out = std::env::args().nth(1).expect("output file");
    let mut rcp_t = Vec::new();
    let mut rsq_t = Vec::new();
    for i in 0..4096u32 {
        let r = rcp((127 << 23) | (i << 11));
        assert_eq!(r & 0x7ff, 0);
        let e = (r >> 23) & 0xff;
        assert!((120..136).contains(&e), "rcp exp {e}");
        rcp_t.push(((e - 120) << 12) | ((r >> 11) & 0xfff));
    }
    for p in 0..2u32 {
        for i in 0..4096u32 {
            let r = rsq(((127 + p) << 23) | (i << 11));
            assert_eq!(r & 0x7ff, 0);
            let e = (r >> 23) & 0xff;
            assert!((120..136).contains(&e), "rsqrt exp {e}");
            rsq_t.push(((e - 120) << 12) | ((r >> 11) & 0xfff));
        }
    }
    // The low 11 mantissa bits never matter: every input against its index's first one.
    let mut bad = 0u64;
    for x in 0..=u32::MAX {
        if x & 0x7ff != 0 && (x >> 23) & 0xff != 0 && (x >> 23) & 0xff != 0xff {
            let base = x & !0x7ff;
            if rcp(x) != rcp(base) || rsq(x) != rsq(base) { bad += 1; }
        }
    }
    println!("inputs whose low 11 mantissa bits change a result: {bad}");
    let mut s = String::new();
    let brand = processor_brand();
    writeln!(s, "// Measured with scripts/estimate-measure on {brand}: RCPSS and RSQRTSS").unwrap();
    writeln!(s, "// of every 1.m (rsqrt also 2 * 1.m), m's top 12 bits; each entry is the result's exponent - 120").unwrap();
    writeln!(s, "// (bits 12..16) and its mantissa's top 12 bits (bits 0..12). Generated: do not edit by hand.").unwrap();
    for (name, t) in [("RCP", &rcp_t), ("RSQRT", &rsq_t)] {
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
        for w in [r.eax, r.ebx, r.ecx, r.edx] { b.extend_from_slice(&w.to_le_bytes()); }
    }
    String::from_utf8_lossy(&b).trim_end_matches('\0').trim().to_string()
}
