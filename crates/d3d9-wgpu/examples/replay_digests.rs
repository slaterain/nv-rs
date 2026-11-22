//! A proof tool: `replay_digests FILE [FROM TO]`.
//! An `NV_D3D9_RECORD` file replayed twice into the offscreen wgpu backend in one process: first with
//! `NV_WGPU_DRAW_RECORDS` off (every draw resolved from the state afresh), then on (the last draw's
//! record used again while nothing it came from changed). Each draw's digest of what it gave wgpu
//! (`Device::record_draw_digests`: pipeline key, both stages' bind group keys, the register blocks'
//! places, vertex and index buffers, viewport, scissor, stencil reference, blend factor, sun switch,
//! the draw's arguments) is compared draw by draw. With FROM and TO only the draws of Presents
//! [FROM, TO) are compared (all are replayed). Prints the draws compared and either "alike" or the
//! first draw that differs, with its Present. Exit status 1 when they differ.
//! Across two builds (a change that has no switch): `NV_DIGESTS_SAVE=FILE` writes the digests to
//! FILE, and `NV_DIGESTS_REF=FILE` compares both replays' digests with FILE's (one build's saved
//! list against another's), each line printed as above. The saved list is read before anything is
//! written, and one file named for both is refused.
//! `NV_DIGESTS_STORAGE=1` adds a third replay, records on and the registers read from storage
//! (`Device::set_storage_regs`), compared with the first.
//! `NV_WGPU_CACHE=DIR` as the game's host.
fn replay(bytes: &[u8], records: bool, storage: bool, from: u64, to: u64) -> Vec<(u64, u64)> {
    let cache = std::env::var_os("NV_WGPU_CACHE").map(Into::into);
    d3d9_wgpu::replay_draw_digests_with(bytes, records, storage, from, to, cache)
        .expect("a command file")
        .expect("a GPU adapter (that can read the registers from storage, when asked)")
}

/// Prints the first draw where `a` and `b` part (`what` names the pair), or that they are alike;
/// true when they are.
fn compare(what: &str, a: &[(u64, u64)], b: &[(u64, u64)]) -> bool {
    match a.iter().zip(b).position(|(x, y)| x != y) {
        Some(i) => {
            println!(
                "{what}: DIFFER at draw {i} (Present {} and {})",
                a[i].1, b[i].1
            );
            false
        }
        None if a.len() != b.len() => {
            println!(
                "{what}: DIFFER: the draw counts differ ({}, {})",
                a.len(),
                b.len()
            );
            false
        }
        None => {
            println!("{what}: alike: {} draws, the same at each", a.len());
            true
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(&a[0]).expect("read the command file");
    let from: u64 = a.get(1).map_or(0, |s| s.parse().expect("FROM"));
    let to: u64 = a.get(2).map_or(u64::MAX, |s| s.parse().expect("TO"));
    let save = std::env::var_os("NV_DIGESTS_SAVE");
    let reference = std::env::var_os("NV_DIGESTS_REF");
    // The saved list is read before anything is written: one file named for both would compare
    // this build with itself and always say alike, so that is refused.
    if save.is_some() && save == reference {
        eprintln!("NV_DIGESTS_SAVE and NV_DIGESTS_REF name the same file: give two");
        std::process::exit(2);
    }
    let saved: Option<Vec<(u64, u64)>> = reference.map(|path| {
        std::fs::read(path)
            .expect("read the saved digests")
            .chunks_exact(16)
            .map(|c| {
                let w = |k: usize| u64::from_le_bytes(c[k..k + 8].try_into().expect("8 bytes"));
                (w(0), w(8))
            })
            .collect()
    });
    let off = replay(&bytes, false, false, from, to);
    let on = replay(&bytes, true, false, from, to);
    let presents = off.last().map_or(0, |&(_, p)| p + 1);
    println!(
        "records off: {} draws, on: {} draws (Presents {from}..{}), the last in Present {}",
        off.len(),
        on.len(),
        to.min(presents),
        presents.saturating_sub(1)
    );
    let mut ok = compare("records off and on", &off, &on);
    if std::env::var_os("NV_DIGESTS_STORAGE").is_some() {
        let stored = replay(&bytes, true, true, from, to);
        ok &= compare("records off and registers from storage", &off, &stored);
    }
    if let Some(saved) = saved {
        ok &= compare("saved and records off", &saved, &off);
        ok &= compare("saved and records on", &saved, &on);
    }
    if let Some(path) = save {
        let out: Vec<u8> = off
            .iter()
            .flat_map(|&(d, p)| d.to_le_bytes().into_iter().chain(p.to_le_bytes()))
            .collect();
        std::fs::write(&path, out).expect("write the digests");
        println!("saved {} digests", off.len());
    }
    if !ok {
        std::process::exit(1);
    }
}
