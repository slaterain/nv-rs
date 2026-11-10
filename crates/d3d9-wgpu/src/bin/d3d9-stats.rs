//! What a recording of the Direct3D 9 call stream uses (`d3d9-stats FILE`): the commands and their
//! counts, the device's present parameters, render target and depth surfaces, texture formats,
//! each render and sampler state value with its count, and the `StretchRect`s by the kind of
//! surface on each side (format, samples, size), which says which multisampled targets the game
//! resolves and into what.
use d3d9_stream::Cmd;
use std::collections::{BTreeMap, HashMap};
fn main() {
    let path = std::env::args().nth(1).expect("the recording");
    let bytes = std::fs::read(path).expect("read");
    let mut rs: BTreeMap<(u32, u32), u64> = BTreeMap::new();
    let mut ss: BTreeMap<(u32, u32), u64> = BTreeMap::new();
    let mut tex: BTreeMap<(u32, u32, u32), u64> = BTreeMap::new();
    let mut names: BTreeMap<&'static str, u64> = BTreeMap::new();
    // Each surface's format, samples, width and height; a texture level is "level N".
    let mut kinds: HashMap<u32, String> = HashMap::new();
    let mut stretches: BTreeMap<(String, String, u32), u64> = BTreeMap::new();
    let mut presents = 0;
    for c in d3d9_stream::wire::commands(&bytes).expect("a stream") {
        let c = c.expect("a command");
        *names.entry(c.name()).or_default() += 1;
        match c {
            Cmd::CreateSurface {
                surface,
                depth,
                width,
                height,
                format,
                multisample,
                quality,
                flag,
                ..
            } => {
                println!("CreateSurface {surface:#x} depth {depth} {width}x{height} format {format} ms {multisample} q {quality} flag {flag}");
                kinds.insert(
                    surface,
                    format!("format {format} ms {multisample} {width}x{height}"),
                );
            }
            Cmd::CreateOffscreenSurface {
                surface,
                width,
                height,
                format,
                pool,
                ..
            } => {
                kinds.insert(
                    surface,
                    format!("format {format} offscreen pool {pool} {width}x{height}"),
                );
            }
            Cmd::CreateTexture {
                levels,
                width,
                height,
                usage,
                format,
                pool,
                ..
            } => {
                *tex.entry((format, usage, pool)).or_default() += 1;
                if usage & 1 != 0 {
                    println!("CreateTexture RT {width}x{height} format {format} usage {usage:#x}");
                }
                for (k, &h) in levels.to_words().iter().enumerate() {
                    kinds.insert(
                        h,
                        format!(
                            "format {format} texture usage {usage:#x} {}x{} level {k}",
                            (width >> k).max(1),
                            (height >> k).max(1)
                        ),
                    );
                }
            }
            Cmd::StretchRect {
                source,
                destination,
                filter,
                ..
            } => {
                let kind = |h: u32| {
                    kinds
                        .get(&h)
                        .cloned()
                        .unwrap_or_else(|| "a back buffer or unknown surface".into())
                };
                *stretches
                    .entry((kind(source), kind(destination), filter))
                    .or_default() += 1;
            }
            Cmd::SetRenderState { state, value, .. } => *rs.entry((state, value)).or_default() += 1,
            Cmd::SetSamplerState { kind, value, .. } => *ss.entry((kind, value)).or_default() += 1,
            Cmd::CreateDevice {
                present,
                back_buffers,
                ..
            }
            | Cmd::Reset {
                present,
                back_buffers,
                ..
            } => {
                let p = present.to_words();
                println!("CreateDevice or Reset present {p:?}");
                for &h in back_buffers.to_words().iter() {
                    kinds.insert(
                        h,
                        format!(
                            "back buffer format {} ms {} {}x{}",
                            p.get(2).unwrap_or(&0),
                            p.get(4).unwrap_or(&0),
                            p.first().unwrap_or(&0),
                            p.get(1).unwrap_or(&0)
                        ),
                    );
                }
            }
            Cmd::Present { .. } => presents += 1,
            _ => {}
        }
    }
    println!("presents {presents}");
    for (k, v) in &names {
        println!("cmd {k} {v}");
    }
    for ((f, u, p), v) in &tex {
        println!(
            "texture format {f} ({:?}) usage {u:#x} pool {p}: {v}",
            f.to_le_bytes()
                .map(|b| if b.is_ascii_graphic() { b as char } else { '.' })
        );
    }
    for ((s, v), n) in &rs {
        println!("rs {s} = {v:#x}: {n}");
    }
    for ((s, v), n) in &ss {
        println!("ss {s} = {v:#x}: {n}");
    }
    for ((from, to, filter), n) in &stretches {
        println!("StretchRect {from} -> {to} filter {filter}: {n}");
    }
}
