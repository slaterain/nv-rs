//! Function ledger (docs/ENGINE_PORT_PLAN.md, Phase 0).
//!
//! ```text
//! cargo run --release --manifest-path scripts/ledger/Cargo.toml [-- --check] [--tsv <file>]
//! ```
//!
//! Reads the engine map (`research/engine-map/engine_map.tsv`, one row per
//! function of FalloutNV.exe 1.4.0.525) and scans the Rust sources in
//! `crates/` and `viewer/` for executable addresses. Each function gets one
//! status:
//!
//! - `translated`: an address in it is cited on a translation marker line
//!   (ADR-0003: `Translated from <addr>` or `(decompiled, FalloutNV.exe ...)`;
//!   when the marker line itself holds no address, the line above counts).
//! - `traced`: an address in it is cited anywhere else in Rust.
//! - `platform` / `library`: its subsystem is replaced by Rust/Bevy, or is
//!   compiler or runtime-library code (`base_status`).
//! - `open`: none of the above.
//!
//! An address counts for the function whose entry is the nearest at or
//! below it, if the address lies within that function's size. Addresses
//! outside every function (data, or code outside Ghidra's functions) are
//! counted separately.
//!
//! Writes `docs/LEDGER.md`. `--check` writes nothing and fails if the file
//! is out of date. `--tsv <file>` also writes the per-function ledger
//! (address, size, name, subsystem, unit, status, Rust locations).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

mod units;

/// `.text` of FalloutNV.exe 1.4.0.525 (image base 0x400000).
const TEXT: (u32, u32) = (0x0040_1000, 0x00fd_e600);

/// Default status of a subsystem when no Rust cites the function.
/// `library`: compiler-generated code and the C/C++ runtime and zlib.
/// `platform`: replaced by Rust/Bevy (docs/ENGINE_PORT_PLAN.md, "Which
/// parts get translated"): the renderer and shader layer, the Xbox/DirectX
/// libraries, system, file, thread and device layers, movie playback,
/// SpeedTree rendering and the debug transport.
fn base_status(subsystem: &str) -> &'static str {
    match subsystem {
        "compiler: static init/exit" | "LIBCMT" | "libcpmt" | "OLDNAMES" | "zLib" => "library",
        "NiXenonRenderer"
        | "BSShader"
        | "d3d9"
        | "d3dx9"
        | "xgraphics"
        | "XAPILIB"
        | "xaudio2"
        | "x3Daudio"
        | "xonline"
        | "xnet"
        | "xmcore"
        | "XBOXKRNL"
        | "xbdm"
        | "xmp"
        | "binkxenon"
        | "BSSystem"
        | "BSSystemUtilities"
        | "BSDevices"
        | "BSMovie"
        | "SpeedTree"
        | "RemoteLog"
        | "BSDiag"
        | "fallout shared/steam" => "platform",
        _ => "open",
    }
}

const STATUSES: [&str; 5] = ["translated", "traced", "platform", "library", "open"];

struct Func {
    addr: u32,
    size: u32,
    name: String,
    tier: String,
    unit: String,
    subsystem: String,
    placed: String,
    translated: BTreeSet<String>,
    replaced: BTreeSet<String>,
    traced: BTreeSet<String>,
}

impl Func {
    /// Translated or replaced inside `crates/engine`: the engine has it. A
    /// translation elsewhere (an older marker in `crates/world`) still
    /// needs its engine version.
    fn in_engine(&self) -> bool {
        self.translated
            .iter()
            .chain(&self.replaced)
            .any(|l| l.starts_with("crates/engine/"))
    }

    fn status(&self) -> &'static str {
        if !self.translated.is_empty() {
            "translated"
        } else if !self.replaced.is_empty() {
            "platform"
        } else if !self.traced.is_empty() {
            "traced"
        } else {
            base_status(&self.subsystem)
        }
    }
}

/// Executable addresses on a line: 8 hex digits starting `00` (optionally
/// `0x`-prefixed), not part of a longer word, inside `.text`.
fn addresses(line: &str) -> Vec<u32> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 8 <= b.len() {
        let prev_ok = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
        let next_ok = i + 8 == b.len() || !(b[i + 8].is_ascii_alphanumeric() || b[i + 8] == b'_');
        let hex = b[i..i + 8].iter().all(|c| c.is_ascii_hexdigit());
        let mut is_0x = false;
        if i >= 2 && (&b[i - 2..i] == b"0x" || &b[i - 2..i] == b"0X") {
            is_0x = i == 2 || !(b[i - 3].is_ascii_alphanumeric() || b[i - 3] == b'_');
        }
        if hex && (prev_ok || is_0x) && next_ok && &b[i..i + 2] == b"00" {
            if let Ok(v) = u32::from_str_radix(&line[i..i + 8], 16) {
                if (TEXT.0..TEXT.1).contains(&v) {
                    out.push(v);
                    i += 8;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// How a citation counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cite {
    Traced,
    /// On a translation marker (ADR-0003).
    Translated,
    /// On a `Platform: replaces <addr>` marker: Rust platform code stands in
    /// for the function (the allocator, file and thread layers).
    Replaced,
}

fn marker(line: &str) -> Option<Cite> {
    if line.contains("Translated from") || line.contains("decompiled, FalloutNV") {
        Some(Cite::Translated)
    } else if line.contains("Platform: replaces") {
        Some(Cite::Replaced)
    } else {
        None
    }
}

/// (address, kind, line) for every citation in one file.
fn scan(text: &str) -> Vec<(u32, Cite, usize)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let here = addresses(l);
        let m = marker(l);
        if let (Some(k), true, true) = (m, here.is_empty(), i > 0) {
            for a in addresses(lines[i - 1]) {
                out.push((a, k, i));
            }
        }
        for a in here {
            out.push((a, m.unwrap_or(Cite::Traced), i + 1));
        }
    }
    out
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() {
            if name != "target" && !name.starts_with('.') {
                rust_files(&p, out);
            }
        } else if name.ends_with(".rs") {
            out.push(p);
        }
    }
}

fn load_map(path: &Path) -> Result<(Vec<Func>, Vec<String>), String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut header = Vec::new();
    let mut funcs = Vec::new();
    let mut cols: Vec<String> = Vec::new();
    for line in text.lines() {
        if let Some(h) = line.strip_prefix('#') {
            header.push(h.trim().to_string());
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if cols.is_empty() {
            cols = f.iter().map(|s| s.to_string()).collect();
            continue;
        }
        let get = |k: &str| -> String {
            cols.iter()
                .position(|c| c == k)
                .and_then(|i| f.get(i))
                .map(|s| s.to_string())
                .unwrap_or_default()
        };
        let addr = u32::from_str_radix(&get("address"), 16).map_err(|e| format!("{line}: {e}"))?;
        let subsystem = match get("subsystem") {
            s if s.is_empty() => "(unplaced)".to_string(),
            s => s,
        };
        funcs.push(Func {
            addr,
            size: get("size").parse().unwrap_or(0),
            name: get("name"),
            tier: get("name_tier"),
            unit: get("unit"),
            subsystem,
            placed: get("placed"),
            translated: BTreeSet::new(),
            replaced: BTreeSet::new(),
            traced: BTreeSet::new(),
        });
    }
    funcs.sort_by_key(|f| f.addr);
    run_units(&mut funcs);
    Ok((funcs, header))
}

/// Functions the map placed in a subsystem but in no unit get a synthetic
/// unit per run of such functions with the same subsystem:
/// `<subsystem>/run_<first address>`, so they have a file and an owner
/// like the others. Library and platform subsystems are left alone.
fn run_units(funcs: &mut [Func]) {
    let mut current: Option<(String, String)> = None;
    for f in funcs.iter_mut() {
        if !f.unit.is_empty() || base_status(&f.subsystem) != "open" {
            current = None;
            continue;
        }
        match &current {
            Some((sub, unit)) if *sub == f.subsystem => f.unit = unit.clone(),
            _ => {
                let unit = format!("{}/run_{:08x}", f.subsystem, f.addr);
                current = Some((f.subsystem.clone(), unit.clone()));
                f.unit = unit;
            }
        }
    }
}

#[derive(Default, Clone)]
struct Counts {
    n: usize,
    bytes: u64,
    named: usize,
    by: [usize; 5],
}

impl Counts {
    fn add(&mut self, f: &Func) {
        self.n += 1;
        self.bytes += f.size as u64;
        self.named += (!f.name.is_empty()) as usize;
        let i = STATUSES.iter().position(|s| *s == f.status()).unwrap();
        self.by[i] += 1;
    }
}

fn pct(a: usize, b: usize) -> String {
    if b == 0 {
        "-".into()
    } else {
        format!("{:.1}%", 100.0 * a as f64 / b as f64)
    }
}

fn render(funcs: &[Func], header: &[String], unmapped: usize, cites: usize) -> String {
    let mut total = Counts::default();
    let mut subs: BTreeMap<&str, Counts> = BTreeMap::new();
    let mut tiers: BTreeMap<&str, usize> = BTreeMap::new();
    let mut placed: BTreeMap<&str, usize> = BTreeMap::new();
    for f in funcs {
        total.add(f);
        subs.entry(&f.subsystem).or_default().add(f);
        *tiers
            .entry(if f.tier.is_empty() { "(none)" } else { &f.tier })
            .or_default() += 1;
        *placed.entry(&f.placed).or_default() += 1;
    }
    let game_fns: Vec<&Func> = funcs
        .iter()
        .filter(|f| base_status(&f.subsystem) == "open")
        .collect();
    let game = game_fns.len();
    let game_done = game_fns
        .iter()
        .filter(|f| matches!(f.status(), "translated" | "traced"))
        .count();
    let mut s = String::new();
    let _ = writeln!(
        s,
        "# Function ledger\n\n\
         Generated by `scripts/ledger` (docs/ENGINE_PORT_PLAN.md, Phase 0); do not edit.\n\
         Regenerate with:\n\n\
         ```sh\ncargo run --release --manifest-path scripts/ledger/Cargo.toml\n```\n\n\
         Every function of FalloutNV.exe 1.4.0.525 from the engine map\n\
         (`research/engine-map/engine_map.tsv`; how it is made: `research/engine-map/README.md`)\n\
         gets one status. Names, units and subsystems come from the Xbox 360 prototype\n\
         (Xbox PDB, ADR-0002), matched to PC functions as described there.\n"
    );
    if !header.is_empty() {
        let _ = writeln!(s, "Map provenance:\n");
        for h in header {
            let _ = writeln!(s, "- {h}");
        }
        let _ = writeln!(s);
    }
    let _ = writeln!(
        s,
        "| Status | Meaning |\n| --- | --- |\n\
         | translated | cited on a translation marker (ADR-0003) in Rust |\n\
         | traced | cited elsewhere in Rust |\n\
         | platform | its subsystem is replaced by Rust/Bevy |\n\
         | library | compiler-generated code, C/C++ runtime, zlib |\n\
         | open | none of the above |\n"
    );
    let _ = writeln!(s, "## Totals\n");
    let _ = writeln!(s, "| | Functions | Share |\n| --- | ---: | ---: |");
    for (i, st) in STATUSES.iter().enumerate() {
        let _ = writeln!(
            s,
            "| {st} | {} | {} |",
            total.by[i],
            pct(total.by[i], total.n)
        );
    }
    let _ = writeln!(s, "| **all** | **{}** | |", total.n);
    let _ = writeln!(
        s,
        "\nGame code (functions in subsystems that are neither platform nor library): \
         **{game}** functions, of which {game_done} ({}) are translated or traced.\n",
        pct(game_done, game)
    );
    let _ = writeln!(
        s,
        "Rust cites {cites} distinct `.text` addresses; {unmapped} of them lie outside every \
         function in the map (mid-instruction data, or code no function covers).\n"
    );
    let _ = writeln!(
        s,
        "Names by tier (see the engine-map README for each tier's evidence):\n"
    );
    let _ = writeln!(s, "| Tier | Functions |\n| --- | ---: |");
    for (k, v) in &tiers {
        let _ = writeln!(s, "| {k} | {v} |");
    }
    let _ = writeln!(s, "\nHow the subsystem was found:\n");
    let _ = writeln!(s, "| Placed by | Functions |\n| --- | ---: |");
    for (k, v) in &placed {
        let _ = writeln!(s, "| {k} | {v} |");
    }
    let _ = writeln!(
        s,
        "\n## By subsystem\n\nLargest first. `Named` counts functions with an Xbox PDB \
         (or, in the runtime libraries, Function ID) name. KB is the size of the PC code.\n"
    );
    let _ = writeln!(
        s,
        "| Subsystem | Functions | KB | Named | translated | traced | platform | library | open | done |\n\
         | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
    );
    let mut rows: Vec<_> = subs.iter().collect();
    rows.sort_by(|a, b| b.1.n.cmp(&a.1.n).then(a.0.cmp(b.0)));
    for (name, c) in rows {
        let done = c.by[0] + c.by[1];
        let base = if base_status(name) == "open" { c.n } else { 0 };
        let _ = writeln!(
            s,
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            c.n,
            c.bytes / 1024,
            c.named,
            c.by[0],
            c.by[1],
            c.by[2],
            c.by[3],
            c.by[4],
            if base == 0 {
                "-".into()
            } else {
                pct(done, base)
            }
        );
    }
    let _ = writeln!(
        s,
        "\n`done` is translated plus traced as a share of the subsystem; it is left out \
         (`-`) for platform and library subsystems."
    );
    s
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    let tsv = args
        .iter()
        .position(|a| a == "--tsv")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (mut funcs, header) = match load_map(&root.join("research/engine-map/engine_map.tsv")) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("ledger: {e}");
            std::process::exit(2);
        }
    };
    let mut files = Vec::new();
    rust_files(&root.join("crates"), &mut files);
    rust_files(&root.join("viewer"), &mut files);
    let mut cited = BTreeSet::new();
    let mut unmapped = BTreeSet::new();
    for path in &files {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        for (a, kind, line) in scan(&text) {
            cited.insert(a);
            let k = funcs.partition_point(|f| f.addr <= a);
            let Some(f) = k.checked_sub(1).map(|k| &mut funcs[k]) else {
                unmapped.insert(a);
                continue;
            };
            if a >= f.addr + f.size.max(1) {
                unmapped.insert(a);
                continue;
            }
            let loc = format!("{rel}:{line}");
            match kind {
                Cite::Translated => f.translated.insert(loc),
                Cite::Replaced => f.replaced.insert(loc),
                Cite::Traced => f.traced.insert(loc),
            };
        }
    }
    let units_dir = root.join("crates/engine/src/units");
    if let Some(a) = args.first() {
        if !matches!(
            a.as_str(),
            "units" | "scaffold" | "queue" | "--check" | "--tsv"
        ) {
            eprintln!("ledger: unknown argument {a} (commands: units, scaffold, queue; options: --check, --tsv <file>)");
            std::process::exit(2);
        }
    }
    match args.first().map(String::as_str) {
        Some("units") => {
            match units::regenerate(&units_dir) {
                Ok(n) => println!("ledger: {n} unit modules"),
                Err(e) => {
                    eprintln!("ledger: {e}");
                    std::process::exit(2);
                }
            }
            return;
        }
        Some("scaffold") => {
            // scaffold <unit or subsystem>...: create unit files for every
            // unit named, or every unit of a subsystem named.
            let want: BTreeSet<&str> = args[1..].iter().map(String::as_str).collect();
            let mut done = BTreeSet::new();
            for f in &funcs {
                if f.unit.is_empty()
                    || base_status(&f.subsystem) != "open"
                    || !(want.contains(f.unit.as_str()) || want.contains(f.subsystem.as_str()))
                {
                    continue;
                }
                if done.insert(f.unit.clone()) {
                    match units::scaffold(&units_dir, &f.unit, &f.subsystem) {
                        Ok(rel) => println!("{}	{rel}", f.unit),
                        Err(e) => {
                            eprintln!("ledger: {e}");
                            std::process::exit(2);
                        }
                    }
                }
            }
            if let Err(e) = units::regenerate(&units_dir) {
                eprintln!("ledger: {e}");
                std::process::exit(2);
            }
            return;
        }
        Some("queue") => {
            queue(&funcs, &args[1..]);
            return;
        }
        _ => {}
    }
    let md = render(&funcs, &header, unmapped.len(), cited.len());
    let out = root.join("docs/LEDGER.md");
    if check {
        let cur = fs::read_to_string(&out).unwrap_or_default();
        if cur.replace("\r\n", "\n") != md {
            eprintln!("ledger: docs/LEDGER.md is out of date; run the ledger");
            std::process::exit(1);
        }
        println!("ledger: docs/LEDGER.md is up to date");
        return;
    }
    fs::write(&out, &md).expect("write docs/LEDGER.md");
    if let Some(t) = tsv {
        let mut s = String::from("address\tsize\tname\tsubsystem\tunit\tstatus\trust\n");
        for f in &funcs {
            let locs: Vec<&String> = f
                .translated
                .iter()
                .chain(&f.replaced)
                .chain(&f.traced)
                .collect();
            let _ = writeln!(
                s,
                "{:08x}\t{}\t{}\t{}\t{}\t{}\t{}",
                f.addr,
                f.size,
                f.name,
                f.subsystem,
                f.unit,
                f.status(),
                locs.iter()
                    .map(|l| l.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
        fs::write(&t, s).expect("write tsv");
    }
    println!(
        "ledger: wrote docs/LEDGER.md ({} functions, {} cited addresses)",
        funcs.len(),
        cited.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_addresses() {
        assert_eq!(
            addresses("`bhkWorld::Update` (`00c6ae70`)"),
            vec![0x00c6ae70]
        );
        assert_eq!(addresses("FUN_00c6ae70 0x00c6ae70"), vec![0x00c6ae70]);
        assert_eq!(addresses("a00c6ae70 00c6ae701"), Vec::<u32>::new());
        // Data (.rdata) and values outside .text are not functions.
        assert_eq!(addresses("01017d00 00001234"), Vec::<u32>::new());
    }

    #[test]
    fn marker_lines_translate() {
        let text = "/// `x` (`00c90e60`) traced\n\
                    // Translated from 00c95a80 and 00c90e60 (decompiled, FalloutNV.exe 1.4.0.525)\n\
                    /// the finder (`00afe220`,\n\
                    /// decompiled, FalloutNV.exe 1.4.0.525): wraps\n";
        let got = scan(text);
        assert!(got.contains(&(0x00c90e60, Cite::Traced, 1)));
        assert!(got.contains(&(0x00c95a80, Cite::Translated, 2)));
        assert!(got.contains(&(0x00c90e60, Cite::Translated, 2)));
        assert!(got
            .iter()
            .any(|&(a, t, _)| a == 0x00afe220 && t == Cite::Translated));
    }

    #[test]
    fn statuses() {
        assert_eq!(base_status("LIBCMT"), "library");
        assert_eq!(base_status("NiXenonRenderer"), "platform");
        assert_eq!(base_status("Havok SDK"), "open");
    }
}

/// `queue`: functions the engine crate does not have yet, per unit
/// (unit, subsystem, functions, bytes, file), largest first. `queue <unit>`: that unit's functions (address, size, status,
/// name), the work list for one translator.
fn queue(funcs: &[Func], args: &[String]) {
    if let Some(unit) = args.first() {
        // Optional address range [lo, hi): one part of a unit split between
        // several translators.
        let hex = |i: usize, d: u32| {
            args.get(i)
                .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                .unwrap_or(d)
        };
        let (lo, hi) = (hex(1, 0), hex(2, u32::MAX));
        for f in funcs
            .iter()
            .filter(|f| &f.unit == unit && f.addr >= lo && f.addr < hi)
        {
            let st = if f.in_engine() { "done" } else { f.status() };
            println!("{:08x}\t{}\t{}\t{}", f.addr, f.size, st, f.name);
        }
        return;
    }
    let mut per: BTreeMap<(&str, &str), (usize, u64)> = BTreeMap::new();
    for f in funcs
        .iter()
        .filter(|f| !f.in_engine() && !f.unit.is_empty() && base_status(&f.subsystem) == "open")
    {
        let e = per.entry((&f.subsystem, &f.unit)).or_default();
        e.0 += 1;
        e.1 += f.size as u64;
    }
    let mut rows: Vec<_> = per.into_iter().collect();
    rows.sort_by(|a, b| b.1 .1.cmp(&a.1 .1).then(a.0.cmp(&b.0)));
    for ((sub, unit), (n, bytes)) in rows {
        let (d, s) = units::module_of(unit, sub);
        println!("{unit}\t{sub}\t{n}\t{bytes}\tunits/{d}/{s}.rs");
    }
}
