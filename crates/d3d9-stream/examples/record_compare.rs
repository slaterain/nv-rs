//! `cargo run --release -p d3d9-stream --example record_compare -- [--ends-deferred | --digests | --counts] A.d3d9s B.d3d9s`
//! (or `--items CHECK.d3d9s`: a check run's recording as its old form and as its items, draw by draw; or
//! `--save-digests REC.d3d9s OUT.dig`: a recording's per-draw digests saved, which `--digests` then takes for A or B
//! in place of the recording, so one stepped recording at a time is on disk):
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
    let old = d3d9_stream::kept::Form::Old;
    describe_forms((a, old), (b, old), i, open);
}

/// [`describe`] with each file's check-run draws in a form of their own (`--items`: one check recording as its old
/// form against its items).
fn describe_forms(
    (a, fa): (&str, d3d9_stream::kept::Form),
    (b, fb): (&str, d3d9_stream::kept::Form),
    i: usize,
    open: impl Fn(&str) -> BufReader<File>,
) {
    let snap = |path: &str, form| match d3d9_stream::wire::draw_snapshot_as(open(path), i, form) {
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
    let (sa, sb) = (snap(a, fa), snap(b, fb));
    println!(
        "  draw {i}: after present {} (draw {} of that frame) in A; after present {} (draw {}) in B",
        sa.presents, sa.in_frame, sb.presents, sb.in_frame
    );
    let ma: std::collections::BTreeMap<_, _> = sa.state.iter().cloned().collect();
    let mb: std::collections::BTreeMap<_, _> = sb.state.iter().cloned().collect();
    // Which draw it is: its shaders in A, alike or not.
    for (k, v) in &ma {
        if k.starts_with("At(\"SetShader\"") {
            println!("  A's {k}: {}", hex(v));
        }
    }
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
    let frames = args.first().is_some_and(|a| a == "--frames");
    if args.first().is_some_and(|a| a == "--items") {
        items_check(&args[1..]);
    }
    if args.first().is_some_and(|a| a == "--save-digests") {
        save_digests(&args[1..]);
    }
    if deferred || digests || counts || frames {
        args.remove(0);
    }
    let [a, b] = args.as_slice() else {
        eprintln!("usage: record_compare [--ends-deferred | --digests | --counts | --frames] A.d3d9s B.d3d9s");
        std::process::exit(2);
    };
    let open = |path: &str| match File::open(path) {
        Ok(f) => BufReader::with_capacity(1 << 20, f),
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(2);
        }
    };
    if frames {
        let read = |path: &str| match d3d9_stream::wire::frame_draws(open(path)) {
            Ok(list) => list,
            Err(why) => {
                eprintln!("{path}: {why}");
                std::process::exit(2);
            }
        };
        let (fa, fb) = (read(a), read(b));
        let differ: Vec<usize> = (0..fa.len().max(fb.len()))
            .filter(|&i| fa.get(i) != fb.get(i))
            .collect();
        println!(
            "frames A {} B {}; draws A {} B {}; frames whose draws differ: {}",
            fa.len(),
            fb.len(),
            fa.iter().map(|&n| u64::from(n)).sum::<u64>(),
            fb.iter().map(|&n| u64::from(n)).sum::<u64>(),
            differ.len()
        );
        for &i in differ.iter().take(40) {
            println!("frame {i}: A {:?} B {:?}", fa.get(i), fb.get(i));
        }
        std::process::exit(i32::from(!differ.is_empty()));
    }
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
        // A `.dig` file holds a recording's digests (`--save-digests`): its recording may be gone (disk).
        let read = |path: &str| match if is_saved(path) {
            read_saved(path)
        } else {
            d3d9_stream::wire::draw_digests(open(path))
        } {
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
                differing(&da, &db);
                match (is_saved(a), is_saved(b)) {
                    (false, false) => describe(a, b, i, open),
                    (true, false) => describe_one(b, i, open),
                    (false, true) => describe_one(a, i, open),
                    (true, true) => {}
                }
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

/// `--items FILE`: one check run's recording (`NV_DRAW_ITEMS=check`) replayed into the state digest
/// twice, as its old form and as its items (`d3d9_stream::Kept`): alike when every item gave the
/// device what its draw gave it. Exits as `main` does.
/// Whether `path` is a file of saved digests (`--save-digests`), not a recording.
fn is_saved(path: &str) -> bool {
    path.ends_with(".dig")
}

/// A file of saved digests: each draw's as a little-endian `u64`.
fn read_saved(path: &str) -> Result<Vec<u64>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() % 8 != 0 {
        return Err(format!("{} bytes, not whole digests", bytes.len()));
    }
    Ok(bytes
        .chunks_exact(8)
        .map(|c| u64::from_le_bytes(c.try_into().expect("8 bytes")))
        .collect())
}

/// `--save-digests REC OUT.dig`: the recording's per-draw digests (as `--digests` makes them) saved, so the
/// recording can be deleted before the next run is recorded (two stepped recordings are 2 GB each). Exit 0 saved,
/// 2 not.
fn save_digests(args: &[String]) -> ! {
    let [rec, out] = args else {
        eprintln!("usage: record_compare --save-digests REC.d3d9s OUT.dig");
        std::process::exit(2);
    };
    let digests = File::open(rec)
        .map_err(|e| e.to_string())
        .and_then(|f| d3d9_stream::wire::draw_digests(BufReader::with_capacity(1 << 20, f)));
    let digests = digests.unwrap_or_else(|why| {
        eprintln!("{rec}: {why}");
        std::process::exit(2);
    });
    let bytes: Vec<u8> = digests.iter().flat_map(|d| d.to_le_bytes()).collect();
    if let Err(e) = std::fs::write(out, bytes) {
        eprintln!("{out}: {e}");
        std::process::exit(2);
    }
    println!("{} draws' digests saved to {out}", digests.len());
    std::process::exit(0);
}

/// The first differing draw `i` described from one recording only (the other's digests were saved, its recording
/// deleted): where it is, its shaders, and the draw.
fn describe_one(path: &str, i: usize, open: impl Fn(&str) -> BufReader<File>) {
    match d3d9_stream::wire::draw_snapshot_as(open(path), i, d3d9_stream::kept::Form::Old) {
        Ok(Some(s)) => {
            println!(
                "  draw {i} in {path}: after present {} (draw {} of that frame); the other file's recording is not \
                 at hand, so the states that differ are not named",
                s.presents, s.in_frame
            );
            for (k, v) in &s.state {
                if k.starts_with("At(\"SetShader\"") {
                    println!("  its {k}: {}", hex(v));
                }
            }
            println!("  the draw: {}", s.draw);
        }
        Ok(None) => println!("  {path}: no draw {i}"),
        Err(why) => println!("  {path}: {why}"),
    }
}

/// How many draws of two equally long digest lists differ, and the first of them: one stale register or a whole
/// class of draws.
fn differing(a: &[u64], b: &[u64]) {
    let at: Vec<usize> = (0..a.len().min(b.len())).filter(|&i| a[i] != b[i]).collect();
    let first: Vec<String> = at.iter().take(12).map(usize::to_string).collect();
    println!(
        "  {} of {} draws differ; the first at draws [{}]",
        at.len(),
        a.len().min(b.len()),
        first.join(", ")
    );
}

fn items_check(args: &[String]) -> ! {
    let [path] = args else {
        eprintln!("usage: record_compare --items CHECK.d3d9s");
        std::process::exit(2);
    };
    let read = |form| {
        let file = File::open(path).unwrap_or_else(|e| {
            eprintln!("{path}: {e}");
            std::process::exit(2);
        });
        d3d9_stream::wire::draw_digests_as(BufReader::with_capacity(1 << 20, file), form)
            .unwrap_or_else(|why| {
                eprintln!("{path}: {why}");
                std::process::exit(2);
            })
    };
    let old = read(d3d9_stream::kept::Form::Old);
    let items = read(d3d9_stream::kept::Form::Items);
    match old.iter().zip(&items).position(|(a, b)| a != b) {
        Some(i) => {
            println!("old form and items: DIFFER at draw {i} of {}", old.len());
            differing(&old, &items);
            let open = |path: &str| BufReader::with_capacity(1 << 20, File::open(path).expect("read"));
            describe_forms(
                (path, d3d9_stream::kept::Form::Old),
                (path, d3d9_stream::kept::Form::Items),
                i,
                open,
            );
            std::process::exit(1);
        }
        None if old.len() != items.len() => {
            println!(
                "old form and items: DIFFER: {} and {} draws",
                old.len(),
                items.len()
            );
            std::process::exit(1);
        }
        None => {
            println!("old form and items: alike: {} draws", old.len());
            std::process::exit(0);
        }
    }
}
