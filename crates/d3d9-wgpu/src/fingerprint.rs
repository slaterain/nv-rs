//! The pipeline cache's source fingerprint, shared by `build.rs` (which includes this file with
//! `#[path]`) and the crate's tests: FNV-1a, 64-bit, over each input file's path relative to a
//! base folder (forward slashes) and its bytes as stored, in path order. The same files give the
//! same value in any checkout; a changed byte, an added, removed or renamed file gives another.

use std::path::{Path, PathBuf};

/// The FNV-1a offset basis: the value of no input at all (a fingerprint equal to it means nothing
/// was hashed, which `build.rs` refuses).
pub const EMPTY: u64 = 0xcbf2_9ce4_8422_2325;

/// Every `.rs`, `.wgsl`, `.toml` or `.lock` file under `dir`, recursively.
pub fn files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            files_under(&path, out);
        } else if path
            .extension()
            .is_some_and(|x| x == "rs" || x == "wgsl" || x == "toml" || x == "lock")
        {
            out.push(path);
        }
    }
}

/// The fingerprint of `files` (absolute paths), named relative to `base`.
pub fn fingerprint(base: &Path, files: &[PathBuf]) -> u64 {
    let mut named: Vec<(String, &PathBuf)> = files
        .iter()
        .map(|p| {
            let rel = p.strip_prefix(base).unwrap_or(p);
            (rel.to_string_lossy().replace('\\', "/"), p)
        })
        .collect();
    named.sort();
    let mut hash = EMPTY;
    let mut feed = |bytes: &[u8]| {
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for (rel, path) in &named {
        feed(rel.as_bytes());
        feed(&[0]);
        if let Ok(bytes) = std::fs::read(path) {
            feed(&bytes);
        }
        feed(&[0]);
    }
    hash
}
