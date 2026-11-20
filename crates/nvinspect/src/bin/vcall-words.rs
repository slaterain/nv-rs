//! `vcall-words --exe FILE --map FILE [--crates DIR] [--words ADDR ..]`: every virtual call in the crates whose
//! argument words no vtable of the exe takes at that slot (`nvinspect::vcall_words`). FILE are
//! the contributor's own unpacked `FalloutNV.exe` and a function map (address and size per line,
//! tab-separated); DIR is this repository's `crates` folder unless given. One line per hit on
//! standard output, a summary on standard error.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut exe = None;
    let mut map = None;
    let mut crates = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    let mut words_of = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next().map(PathBuf::from);
        match (arg.as_str(), value) {
            ("--exe", Some(v)) => exe = Some(v),
            ("--map", Some(v)) => map = Some(v),
            ("--crates", Some(v)) => crates = v,
            ("--words", Some(v)) => {
                let Ok(addr) = u32::from_str_radix(&v.to_string_lossy(), 16) else {
                    eprintln!("--words takes a hex address");
                    return ExitCode::from(2);
                };
                words_of.push(addr);
            }
            _ => {
                eprintln!("usage: vcall-words --exe FILE --map FILE [--crates DIR]");
                return ExitCode::from(2);
            }
        }
    }
    let (Some(exe), Some(map)) = (exe, map) else {
        eprintln!("usage: vcall-words --exe FILE --map FILE [--crates DIR]");
        return ExitCode::from(2);
    };
    if !words_of.is_empty() {
        // `--words ADDR` (repeatable): the words each function takes as the check reads them.
        return match nvinspect::vcall_words::words_of(&exe, &map, &words_of) {
            Ok(lines) => {
                for line in lines {
                    println!("{line}");
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("vcall-words: {e}");
                ExitCode::from(2)
            }
        };
    }
    match nvinspect::vcall_words::run(&exe, &map, &crates) {
        Ok((lines, summary)) => {
            for line in lines {
                println!("{line}");
            }
            eprintln!("{summary}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("vcall-words: {e}");
            ExitCode::from(2)
        }
    }
}
