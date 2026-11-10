//! Replays a recording of the Direct3D 9 call stream (`d3d9_stream::wire`, recorded once from a
//! game run) into an offscreen wgpu backend (`d3d9-replay FILE`): `NV_SHOT` with `NV_SHOT_FRAMES`
//! or `NV_SHOT_LIST` captures its presents as the game run captured them, so `shot-diff` compares
//! the backend with the run's real Direct3D 9 frames without running the game again.
//! `NV_WGPU_CACHE=DIR` keeps the driver's compiled pipelines there between replays, as the game's
//! host does (`host::wgpu_cache_path`).
fn main() {
    let path = std::env::args().nth(1).expect("the recording");
    let bytes = std::fs::read(path).expect("read");
    let mut b = d3d9_wgpu::WgpuBackend::new(d3d9_wgpu::Output::Offscreen);
    if let Some(dir) = std::env::var_os("NV_WGPU_CACHE") {
        b = b.with_cache_dir(dir.into());
    }
    let start = std::time::Instant::now();
    let n = d3d9_stream::wire::replay(&bytes, &mut b).expect("a recording");
    eprintln!("{n} commands in {:?}", start.elapsed());
}
