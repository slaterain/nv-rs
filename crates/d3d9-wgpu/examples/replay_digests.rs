//! A proof tool: `replay_digests FILE [FROM TO]`.
//! An `NV_D3D9_RECORD` file replayed twice into the offscreen wgpu backend in one process: first with
//! `NV_WGPU_DRAW_RECORDS` off (every draw resolved from the state afresh), then on (the last draw's
//! record used again while nothing it came from changed). Each draw's digest of what it gave wgpu
//! (`Device::record_draw_digests`: pipeline key, both stages' bind group keys, the register blocks'
//! places, vertex and index buffers, viewport, scissor, stencil reference, blend factor, sun switch,
//! the draw's arguments) is compared draw by draw. With FROM and TO only the draws of Presents
//! [FROM, TO) are compared (all are replayed). Prints the draws compared and either "alike" or the
//! first draw that differs, with its Present. Exit status 1 when they differ.
//! `NV_WGPU_CACHE=DIR` as the game's host.
fn replay(bytes: &[u8], records: bool, from: u64, to: u64) -> Vec<(u64, u64)> {
    let cache = std::env::var_os("NV_WGPU_CACHE").map(Into::into);
    d3d9_wgpu::replay_draw_digests(bytes, records, from, to, cache)
        .expect("a command file")
        .expect("a GPU adapter")
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(&a[0]).expect("read the command file");
    let from: u64 = a.get(1).map_or(0, |s| s.parse().expect("FROM"));
    let to: u64 = a.get(2).map_or(u64::MAX, |s| s.parse().expect("TO"));
    let off = replay(&bytes, false, from, to);
    let on = replay(&bytes, true, from, to);
    let presents = off.last().map_or(0, |&(_, p)| p + 1);
    println!(
        "records off: {} draws, on: {} draws (Presents {from}..{}), the last in Present {}",
        off.len(),
        on.len(),
        to.min(presents),
        presents.saturating_sub(1)
    );
    match off.iter().zip(&on).position(|(x, y)| x != y) {
        Some(i) => {
            println!(
                "DIFFER at draw {i} (Present {} off, {} on)",
                off[i].1, on[i].1
            );
            std::process::exit(1);
        }
        None if off.len() != on.len() => {
            println!("DIFFER: the draw counts differ");
            std::process::exit(1);
        }
        None => println!("alike: {} draws, the same at each", off.len()),
    }
}
