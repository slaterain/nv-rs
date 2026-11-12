//! The pipeline cache's source fingerprint: a hash of every source file that decides what the driver
//! compiles (this crate's `src`, with the shader converter and the pipeline layouts, and the vendored
//! `wgpu-hal`'s `src`), given to the crate as `D3D9_WGPU_PIPELINE_SOURCES`.
//!
//! The pipeline cache file's name carries it ([`cache::PipelineCacheFile::open`]), so a build whose
//! converted shaders or layouts may differ starts with a new cache: its first start makes every
//! shader's library behind the start screen (a cold start), instead of finding a cache wgpu accepts
//! but whose libraries no longer match (#514's lesson: a "warm" start then left the unlisted shaders'
//! libraries to be compiled on their first draw, in play).

use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            files(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs" || x == "wgsl") {
            out.push(path);
        }
    }
}

fn main() {
    let here = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("set by cargo"));
    let roots = [here.join("src"), here.join("../../vendor/wgpu-hal/src")];
    let mut all = Vec::new();
    for root in &roots {
        if root.is_dir() {
            println!("cargo:rerun-if-changed={}", root.display());
            files(root, &mut all);
        }
    }
    all.sort();
    // FNV-1a, 64-bit: over each file's path (relative to the crate) and its bytes, line endings as
    // stored, so the same sources give the same fingerprint on any machine checked out alike.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |bytes: &[u8]| {
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for path in &all {
        let rel = path.strip_prefix(&here).unwrap_or(path);
        feed(rel.to_string_lossy().replace('\\', "/").as_bytes());
        feed(&[0]);
        if let Ok(bytes) = std::fs::read(path) {
            feed(&bytes);
        }
        feed(&[0]);
    }
    println!("cargo:rustc-env=D3D9_WGPU_PIPELINE_SOURCES={hash:016x}");
}
