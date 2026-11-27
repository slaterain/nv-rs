//! Replays a recording of the Direct3D 9 call stream (`d3d9_stream::wire`, recorded once from a
//! game run) into an offscreen wgpu backend (`d3d9-replay FILE`): `NV_SHOT` with `NV_SHOT_FRAMES`
//! or `NV_SHOT_LIST` captures its presents as the game run captured them, so `shot-diff` compares
//! the backend with the run's real Direct3D 9 frames without running the game again.
//! `NV_WGPU_CACHE=DIR` keeps the driver's compiled pipelines there between replays, as the game's
//! host does (`host::wgpu_cache_path`).
//! A check run's recording (`NV_DRAW_ITEMS=check`) holds two forms of the draws it could have
//! drawn from kept records (`d3d9_stream::Kept`): replayed as its old form (the default) or, with
//! `NV_REPLAY_ITEMS=items`, as its items; `shot-diff` between the two sets of captures shows what
//! the items change in the picture.
fn main() {
    let path = std::env::args().nth(1).expect("the recording");
    let bytes = std::fs::read(path).expect("read");
    let mut b = d3d9_wgpu::WgpuBackend::new(d3d9_wgpu::Output::Offscreen);
    if let Some(dir) = std::env::var_os("NV_WGPU_CACHE") {
        b = b.with_cache_dir(dir.into());
    }
    let form = match std::env::var("NV_REPLAY_ITEMS").as_deref() {
        Ok("items") => d3d9_stream::kept::Form::Items,
        _ => d3d9_stream::kept::Form::Old,
    };
    let mut b = d3d9_stream::Kept::with_form(b, form);
    let start = std::time::Instant::now();
    let n = d3d9_stream::wire::replay(&bytes, &mut b).expect("a recording");
    let c = b.counts();
    eprintln!("{n} commands in {:?} ({form:?}: {c:?})", start.elapsed());
}
