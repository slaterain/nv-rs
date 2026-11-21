//! `cargo run --release -p d3d9-stream --example record_compare -- [--digests] A.d3d9s B.d3d9s`: two
//! `NV_D3D9_RECORD` files compared command by command with object handles renamed by first use
//! (`wire::compare_renamed`): what a run whose game heap was laid out otherwise (a typed family's
//! objects moved out of it, ADR-0014 stage 2) is compared by, where the plain `cmp` would see every
//! modelled object at another address. Prints how many commands agreed and the first difference.
//! With `--digests`, each draw's device state instead (`wire::draw_digests`): runs whose streams
//! differ only in states set again to the value they had (`NV_D3D9_REPEATS=keep` against the
//! default) give the same digest at every draw. Exit 0: alike; 1: a difference; 2: a file could
//! not be read.

use std::fs::File;
use std::io::BufReader;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let digests = args.first().is_some_and(|a| a == "--digests");
    if digests {
        args.remove(0);
    }
    let [a, b] = args.as_slice() else {
        eprintln!("usage: record_compare [--digests] A.d3d9s B.d3d9s");
        std::process::exit(2);
    };
    let open = |path: &str| match File::open(path) {
        Ok(f) => BufReader::with_capacity(1 << 20, f),
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(2);
        }
    };
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
                std::process::exit(1);
            }
        }
        return;
    }
    match d3d9_stream::wire::compare_renamed(open(a), open(b)) {
        Ok((n, None)) => println!("alike: {n} commands, handles renamed by first use"),
        Ok((n, Some(d))) => {
            println!("{n} commands alike, then command {} differs:", d.index);
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
