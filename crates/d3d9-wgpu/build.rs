//! The pipeline cache's source fingerprint (`src/fingerprint.rs`), given to the crate as
//! `D3D9_WGPU_PIPELINE_SOURCES`: every source that decides what the driver compiles. That is this
//! crate's `src` (the shader converter and the pipeline layouts) and `Cargo.toml`, the vendored
//! `wgpu-hal` (its `src`, `Cargo.toml` and `build.rs`), and the workspace's `Cargo.lock`, which pins
//! naga, wgpu-core and the other crates that turn the converted shaders into SPIR-V.
//!
//! The pipeline cache file's name carries it (`cache::PipelineCacheFile::open`), so a build whose
//! converted shaders or layouts may differ starts with a new cache: its first start makes every
//! shader's library behind the start screen (a cold start), instead of finding a cache wgpu accepts
//! but whose libraries no longer match (#514's lesson: a "warm" start then left the unlisted shaders'
//! libraries to be compiled on their first draw, in play). Any dependency update also gives one cold
//! start: the safe side.

#[path = "src/fingerprint.rs"]
mod fingerprint;

use std::path::PathBuf;

fn main() {
    // Canonical paths, so every input is named relative to the workspace (the same in any checkout).
    let here = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("set by cargo"))
        .canonicalize()
        .expect("the crate's folder");
    let workspace = here
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/d3d9-wgpu sits two folders below the workspace")
        .to_path_buf();
    let roots = [here.join("src"), workspace.join("vendor/wgpu-hal/src")];
    let single = [
        here.join("Cargo.toml"),
        workspace.join("Cargo.lock"),
        workspace.join("vendor/wgpu-hal/Cargo.toml"),
        workspace.join("vendor/wgpu-hal/build.rs"),
    ];
    let mut all = Vec::new();
    for root in &roots {
        // A missing root would silently drop what it decides from the fingerprint: refuse.
        assert!(
            root.is_dir(),
            "the pipeline fingerprint's folder {} is missing",
            root.display()
        );
        println!("cargo:rerun-if-changed={}", root.display());
        fingerprint::files_under(root, &mut all);
    }
    for file in &single {
        assert!(
            file.is_file(),
            "the pipeline fingerprint's file {} is missing",
            file.display()
        );
        println!("cargo:rerun-if-changed={}", file.display());
        all.push(file.clone());
    }
    let hash = fingerprint::fingerprint(&workspace, &all);
    assert_ne!(hash, fingerprint::EMPTY, "nothing was hashed");
    println!("cargo:rustc-env=D3D9_WGPU_PIPELINE_SOURCES={hash:016x}");
}
