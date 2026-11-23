//! `cargo run --release -p d3d9-stream --example record_compare -- [--ends-deferred | --digests | --counts] A.d3d9s B.d3d9s`:
//! two `NV_D3D9_RECORD` files compared command by command with object handles renamed by first use
//! (`wire::compare_renamed`): what a run whose game heap was laid out otherwise (a typed family's
//! objects moved out of it, ADR-0014 stage 2) is compared by, where the plain `cmp` would see every
//! modelled object at another address. Prints how many commands agreed and the first difference.
//! With `--ends-deferred` (A the serial run, B one with the world's draw on the drawing thread) an
//! object's `Destroy` may come later in B, where the drawing side's release leaves it to the join
//! (`wire::compare_renamed_ends_deferred`, which says why that changes no picture and what it still
//! checks). With `--digests`, each draw's device state instead (`wire::draw_digests`): runs whose
//! streams differ only in states set again to the value they had (`NV_D3D9_REPEATS=keep` against
//! the default) give the same digest at every draw. With `--counts`, how many of each command each
//! file holds and their bytes (`wire::command_counts`), largest byte difference first: why one stream
//! is bigger (exit 1 when any count or size differs). Exit 0: alike; 1: a difference; 2: a file could not
//! be read.

use std::fs::File;
use std::io::BufReader;

/// Up to 64 bytes in hex words, for a state's value.
fn hex(bytes: &[u8]) -> String {
    let shown: Vec<String> = bytes
        .chunks(4)
        .take(16)
        .map(|w| w.iter().rev().map(|b| format!("{b:02x}")).collect())
        .collect();
    let more = if bytes.len() > 64 { " ..." } else { "" };
    format!("<{}: {}{more}>", bytes.len(), shown.join(" "))
}

/// The first differing draw `i` of the two files in the open: which present it comes after and its place among the
/// draws since, which states differ (both values), whether the other commands so far differ (with the last of them),
/// and the two draw commands.
fn describe(a: &str, b: &str, i: usize, open: impl Fn(&str) -> BufReader<File>) {
    let snap = |path: &str| match d3d9_stream::wire::draw_snapshot(open(path), i) {
        Ok(Some(s)) => s,
        Ok(None) => {
            println!("  {path}: no draw {i}");
            std::process::exit(1);
        }
        Err(why) => {
            eprintln!("{path}: {why}");
            std::process::exit(2);
        }
    };
    let (sa, sb) = (snap(a), snap(b));
    println!(
        "  draw {i}: after present {} (draw {} of that frame) in A; after present {} (draw {}) in B",
        sa.presents, sa.in_frame, sb.presents, sb.in_frame
    );
    let ma: std::collections::BTreeMap<_, _> = sa.state.iter().cloned().collect();
    let mb: std::collections::BTreeMap<_, _> = sb.state.iter().cloned().collect();
    let keys: std::collections::BTreeSet<_> = ma.keys().chain(mb.keys()).cloned().collect();
    let mut shown = 0;
    for k in keys {
        let (va, vb) = (ma.get(&k), mb.get(&k));
        if va == vb {
            continue;
        }
        shown += 1;
        if shown > 20 {
            println!("  ... more states differ");
            break;
        }
        let v = |x: Option<&Vec<u8>>| x.map_or("(never set)".to_string(), |b| hex(b));
        println!("  state {k}: A {}  B {}", v(va), v(vb));
    }
    if shown == 0 {
        println!("  every state alike");
    }
    if sa.events != sb.events {
        println!("  the other commands so far differ (creations, uploads, clears, presents); the last in A:");
        for r in &sa.recent {
            println!("    A {r}");
        }
        for r in &sb.recent {
            println!("    B {r}");
        }
    }
    if sa.draw != sb.draw {
        println!("  the draws differ:\n    A {}\n    B {}", sa.draw, sb.draw);
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let deferred = args.first().is_some_and(|a| a == "--ends-deferred");
    let digests = args.first().is_some_and(|a| a == "--digests");
    let counts = args.first().is_some_and(|a| a == "--counts");
    if deferred || digests || counts {
        args.remove(0);
    }
    let [a, b] = args.as_slice() else {
        eprintln!("usage: record_compare [--ends-deferred | --digests | --counts] A.d3d9s B.d3d9s");
        std::process::exit(2);
    };
    let open = |path: &str| match File::open(path) {
        Ok(f) => BufReader::with_capacity(1 << 20, f),
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(2);
        }
    };
    if counts {
        let read = |path: &str| match d3d9_stream::wire::command_counts(open(path)) {
            Ok(list) => list,
            Err(why) => {
                eprintln!("{path}: {why}");
                std::process::exit(2);
            }
        };
        let (ca, cb) = (read(a), read(b));
        let mut rows: std::collections::BTreeMap<&str, [u64; 4]> = Default::default();
        for (name, n, bytes) in ca {
            let r = rows.entry(name).or_default();
            (r[0], r[1]) = (n, bytes);
        }
        for (name, n, bytes) in cb {
            let r = rows.entry(name).or_default();
            (r[2], r[3]) = (n, bytes);
        }
        let mut rows: Vec<_> = rows.into_iter().collect();
        rows.sort_by_key(|(_, r)| std::cmp::Reverse((r[3] as i64 - r[1] as i64).abs()));
        println!("command\tA count\tA bytes\tB count\tB bytes\tB-A count\tB-A bytes");
        let differ = rows.iter().any(|(_, r)| r[0] != r[2] || r[1] != r[3]);
        for (name, r) in rows {
            println!(
                "{name}\t{}\t{}\t{}\t{}\t{}\t{}",
                r[0],
                r[1],
                r[2],
                r[3],
                r[2] as i64 - r[0] as i64,
                r[3] as i64 - r[1] as i64
            );
        }
        std::process::exit(i32::from(differ));
    }
    if digests {
        let read = |path: &str| match d3d9_stream::wire::draw_digests(open(path)) {
            Ok(list) => list,
            Err(why) => {
                eprintln!("{path}: {why}");
                std::process::exit(2);
            }
        };
        let (da, db) = (read(a), read(b));
        match da.iter().zip(&db).position(|(x, y)| x != y) {
            None if da.len() == db.len() => {
                println!("alike: {} draws, the same device state at each", da.len());
            }
            None => {
                println!(
                    "{} draws alike, then one file ends: {a} has {}, {b} {}",
                    da.len().min(db.len()),
                    da.len(),
                    db.len()
                );
                std::process::exit(1);
            }
            Some(i) => {
                println!("{i} draws alike, then draw {i} sees another device state");
                describe(a, b, i, open);
                std::process::exit(1);
            }
        }
        return;
    }
    let found = if deferred {
        d3d9_stream::wire::compare_renamed_ends_deferred(open(a), open(b)).map(|(n, d, ends)| {
            println!("objects B ended later than A: {}", ends.later);
            (n, d)
        })
    } else {
        d3d9_stream::wire::compare_renamed(open(a), open(b))
    };
    let what = if deferred {
        "commands other than Destroy"
    } else {
        "commands"
    };
    match found {
        Ok((n, None)) => println!("alike: {n} {what}, handles renamed by first use"),
        Ok((n, Some(d))) => {
            println!("{n} {what} alike, then command {} differs:", d.index);
            println!("  {a}: {}", d.a.as_deref().unwrap_or("(the file ends)"));
            println!("  {b}: {}", d.b.as_deref().unwrap_or("(the file ends)"));
            std::process::exit(1);
        }
        Err(why) => {
            eprintln!("{why}");
            std::process::exit(2);
        }
    }
}
