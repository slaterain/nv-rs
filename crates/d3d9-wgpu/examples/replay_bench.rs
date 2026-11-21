//! A measuring tool: `replay_bench FILE FROM TO [REPEAT]`.
//! An `NV_D3D9_RECORD` file replayed into the offscreen wgpu backend (as `d3d9-replay`), Presents before FROM as a
//! warm-up, then each frame of [FROM, TO) timed: decoding alone (a first pass over the same bytes), and the backend's
//! `execute` per kind of command (draws, constant sets, other state sets, resource uploads, Present = the submit).
//! `NV_WGPU_CACHE=DIR` as the game's host. Prints the means per frame and the frame times' median and 99th percentile.
//! With REPEAT > 1 the window's frames are replayed REPEAT times over (the same commands again: state and objects as
//! the window left them, so only the timing is meaningful), for a sampler to see more of them.
use d3d9_stream::{Backend, Cmd};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

fn kind(c: &Cmd<d3d9_stream::Data<'_>>) -> &'static str {
    match c {
        Cmd::DrawIndexedPrimitive { .. }
        | Cmd::DrawPrimitive { .. }
        | Cmd::DrawPrimitiveUp { .. } => "draw",
        Cmd::SetConstants { .. } => "constants",
        Cmd::SetSamplerState { .. }
        | Cmd::SetRenderState { .. }
        | Cmd::SetTextureStageState { .. } => "states",
        Cmd::SetTexture { .. } => "textures",
        Cmd::SetStreamSource { .. }
        | Cmd::SetIndices { .. }
        | Cmd::SetVertexDeclaration { .. }
        | Cmd::SetFvf { .. } => "streams",
        Cmd::SetShader { .. } => "shaders",
        Cmd::UploadBuffer { .. } | Cmd::UploadSurface { .. } => "uploads",
        Cmd::Present { .. } => "present",
        Cmd::SetRenderTarget { .. }
        | Cmd::SetDepthStencilSurface { .. }
        | Cmd::BeginScene { .. }
        | Cmd::EndScene { .. }
        | Cmd::Clear { .. }
        | Cmd::StretchRect { .. } => "targets",
        _ => "other",
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(&a[0]).expect("read");
    let (from, to): (u64, u64) = (a[1].parse().unwrap(), a[2].parse().unwrap());
    let repeat: u32 = a.get(3).map_or(1, |s| s.parse().unwrap());
    // Decoding alone over the window.
    let mut present = 0u64;
    let mut decode = Duration::ZERO;
    let mut t = Instant::now();
    let mut n_win = 0u64;
    for c in d3d9_stream::wire::commands(&bytes).expect("stream") {
        let c = c.expect("command");
        let p = matches!(c, Cmd::Present { .. });
        if present >= from && present < to {
            n_win += 1;
        }
        if p {
            present += 1;
            if present == from {
                t = Instant::now();
            } else if present == to {
                decode = t.elapsed();
            }
        }
    }
    let mut b = d3d9_wgpu::WgpuBackend::new(d3d9_wgpu::Output::Offscreen);
    if let Some(dir) = std::env::var_os("NV_WGPU_CACHE") {
        b = b.with_cache_dir(dir.into());
    }
    let mut by: BTreeMap<&'static str, (Duration, u64)> = BTreeMap::new();
    let mut frames: Vec<f64> = Vec::new();
    let mut present = 0u64;
    let mut frame_start = Instant::now();
    for c in d3d9_stream::wire::commands(&bytes).expect("stream") {
        let c = c.expect("command");
        let p = matches!(c, Cmd::Present { .. });
        if present >= from && present < to {
            let k = kind(&c);
            let s = Instant::now();
            b.execute(c);
            let e = by.entry(k).or_default();
            e.0 += s.elapsed();
            e.1 += 1;
        } else {
            b.execute(c);
        }
        if p {
            present += 1;
            if present > from && present <= to {
                frames.push(frame_start.elapsed().as_secs_f64() * 1e3);
            }
            frame_start = Instant::now();
            if present == to && repeat > 1 {
                break;
            }
        }
    }
    // Repeats: the window's commands again (decoded again each time).
    for _ in 1..repeat {
        let mut present = 0u64;
        for c in d3d9_stream::wire::commands(&bytes).expect("stream") {
            let c = c.expect("command");
            let p = matches!(c, Cmd::Present { .. });
            if present >= from && present < to {
                match c {
                    // Objects already made: creations and destructions are not repeated.
                    Cmd::CreateTexture { .. }
                    | Cmd::CreateCubeTexture { .. }
                    | Cmd::CreateSurface { .. }
                    | Cmd::CreateOffscreenSurface { .. }
                    | Cmd::CreateBuffer { .. }
                    | Cmd::CreateShader { .. }
                    | Cmd::CreateDeclaration { .. }
                    | Cmd::CreateQuery { .. }
                    | Cmd::Destroy { .. } => {}
                    c => {
                        b.execute(c);
                    }
                }
            }
            if p {
                present += 1;
                if present >= to {
                    break;
                }
            }
        }
    }
    let nf = frames.len().max(1) as f64;
    let total: f64 = frames.iter().sum::<f64>() / nf;
    let mut sorted = frames.clone();
    sorted.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let pct = |q: f64| {
        sorted
            .get(((sorted.len() as f64 - 1.0) * q) as usize)
            .copied()
            .unwrap_or(0.0)
    };
    println!(
        "window {from}..{to}: {} frames, {:.0} commands a frame; frame {total:.3} ms (median {:.3}, 99% {:.3}); decode alone {:.3} ms",
        frames.len(),
        n_win as f64 / nf,
        pct(0.5),
        pct(0.99),
        decode.as_secs_f64() * 1e3 / nf
    );
    let mut sum = 0.0;
    for (k, (d, n)) in &by {
        let ms = d.as_secs_f64() * 1e3 / nf;
        sum += ms;
        println!(
            "  {k:10} {ms:7.3} ms a frame, {:7.0} commands, {:6.2} us each",
            *n as f64 / nf,
            d.as_secs_f64() * 1e6 / (*n).max(1) as f64
        );
    }
    println!("  executes   {sum:7.3} ms a frame (with the timer around each command)");
}
