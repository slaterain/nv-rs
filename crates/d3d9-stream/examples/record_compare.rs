//! `cargo run --release -p d3d9-stream --example record_compare -- [--ends-deferred] A.d3d9s B.d3d9s`:
//! two `NV_D3D9_RECORD` files compared command by command with object handles renamed by first use
//! (`wire::compare_renamed`): what a run whose game heap was laid out otherwise (a typed family's
//! objects moved out of it, ADR-0014 stage 2) is compared by, where the plain `cmp` would see every
//! modelled object at another address. Prints how many commands agreed and the first difference.
//! With `--ends-deferred` (A the serial run, B one with the world's draw on the drawing thread) an
//! object's `Destroy` may come later in B, where the drawing side's release leaves it to the join
//! (`wire::compare_renamed_ends_deferred`, which says why that changes no picture and what it still
//! checks). Exit 0: alike; 1: a difference; 2: a file could not be read.

use std::fs::File;
use std::io::BufReader;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let deferred = args.first().is_some_and(|a| a == "--ends-deferred");
    if deferred {
        args.remove(0);
    }
    let [a, b] = args.as_slice() else {
        eprintln!("usage: record_compare [--ends-deferred] A.d3d9s B.d3d9s");
        std::process::exit(2);
    };
    let open = |path: &str| match File::open(path) {
        Ok(f) => BufReader::with_capacity(1 << 20, f),
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(2);
        }
    };
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
