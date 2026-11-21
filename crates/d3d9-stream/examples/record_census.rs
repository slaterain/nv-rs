//! A measuring tool: `record_census FILE A B [C D ...]`.
//! Frame-to-frame census of an `NV_D3D9_RECORD` file: for every draw in the windows of Presents [A, B), the same
//! object's draw in the frame before (matched by its streams, indices and draw arguments, and its place among
//! draws with the same key in its frame) is compared: (a) the whole device state the same, shader constants and
//! transforms included; (b) everything but some shader constants / transforms the same (which registers differ is
//! counted); (c) other state differs (shaders, render or sampler states, textures, targets...); (n) no match in the
//! frame before. Also per frame: draws, distinct whole states (without constants), distinct pipeline-ish states
//! (shaders, declaration, render states, targets), distinct texture sets, constant bytes set.
use d3d9_stream::Cmd;
use std::collections::{BTreeMap, HashMap};

fn h64(mut h: u64, words: &[u32]) -> u64 {
    for &w in words {
        h ^= u64::from(w);
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= h >> 29;
    }
    h
}

#[derive(Clone)]
struct DrawRec {
    pipe: u64,
    textures: u64,
    streams: u64,
    consts: Vec<[u32; 4]>,
    transforms: u64,
    own: Vec<(usize, [u32; 4])>,
}

/// A pass's sums: draws, commands, draws again, frames, the last viewport, target and depth surface.
type PassSums = (u64, u64, u64, u64, [u32; 2], u32, u32);

#[derive(Default)]
struct Win {
    frames: u64,
    draws: u64,
    same: u64,
    consts_only: u64,
    state: u64,
    new: u64,
    regs: BTreeMap<(u8, u32), u64>,
    distinct_state: u64,
    distinct_pipe: u64,
    distinct_tex: u64,
    const_bytes: u64,
    transforms_only: u64,
    own_same: u64,
    own_regs: u64,
    pipe_changes: u64,
    tex_changes: u64,
    stream_changes: u64,
    /// By pass (BeginScene ordinal in the frame): draws, commands, draws of an object the frame's biggest pass also
    /// draws, frames seen, the last viewport (width, height), target and depth surface.
    /// The draws' own constant registers (16 bytes each) by kind: the same value in every draw of the frame that sets
    /// it; else the same as this draw's last frame; else changing.
    own_frame_wide: u64,
    own_kept: u64,
    own_changing: u64,
    own_change_regs: BTreeMap<usize, u64>,
    own_kept_regs: BTreeMap<usize, u64>,
    /// ModelViewProj rows (vs c0-c3) that changed: by the largest difference of their four floats in ulps:
    /// <= 4, <= 256, <= 65536, more.
    mvp_ulps: [u64; 4],
    passes: BTreeMap<u32, PassSums>,
}

/// The constants the draw itself set (since the draw before it) the same as last frame's same draw's.
fn w_own(w: &mut Win, prev: Option<&DrawRec>, rec: &DrawRec) {
    w.own_regs += rec.own.len() as u64;
    if prev.is_some_and(|p| p.own == rec.own) {
        w.own_same += 1;
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(&a[0]).expect("read");
    let ranges: Vec<(u64, u64)> = a[1..]
        .chunks(2)
        .map(|c| (c[0].parse().unwrap(), c[1].parse().unwrap()))
        .collect();
    let mut wins: Vec<Win> = ranges.iter().map(|_| Win::default()).collect();
    // Current state.
    let mut rs: BTreeMap<u32, u32> = BTreeMap::new();
    let mut ss: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    let mut tss: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    let mut tex: BTreeMap<u32, u32> = BTreeMap::new();
    let mut streams: BTreeMap<u32, (u32, u32, u32)> = BTreeMap::new();
    let mut indices = 0u32;
    let mut shaders = [0u32; 2];
    let mut decl = 0u32;
    let mut targets: BTreeMap<u32, u32> = BTreeMap::new();
    let mut depth = 0u32;
    let mut viewport = [0u32; 6];
    let mut transforms: BTreeMap<u32, u64> = BTreeMap::new();
    // Constants: 6 kinds x 256 registers.
    let mut consts = vec![[0u32; 4]; 6 * 256];
    let mut present = 0u64;
    let mut pending: Vec<(usize, [u32; 4])> = Vec::new();
    let mut prev = (0u64, 0u64, 0u64);
    let mut pass = 0u32;
    let mut pass_keys: BTreeMap<u32, Vec<u64>> = BTreeMap::new();
    let mut pass_cmds: BTreeMap<u32, u64> = BTreeMap::new();
    let mut pass_at: BTreeMap<u32, ([u32; 2], u32, u32)> = BTreeMap::new();
    let mut last: HashMap<(u64, u32), DrawRec> = HashMap::new();
    let mut this: HashMap<(u64, u32), DrawRec> = HashMap::new();
    let mut seen: HashMap<u64, u32> = HashMap::new();
    let mut frame_states: std::collections::HashSet<u64> = Default::default();
    let mut frame_pipes: std::collections::HashSet<u64> = Default::default();
    let mut frame_tex: std::collections::HashSet<u64> = Default::default();
    let win_of = |p: u64| ranges.iter().position(|&(lo, hi)| p >= lo && p < hi);
    for cmd in d3d9_stream::wire::commands(&bytes).expect("stream") {
        let cmd = cmd.expect("command");
        if matches!(cmd, Cmd::BeginScene { .. }) {
            pass += 1;
        }
        *pass_cmds.entry(pass).or_default() += 1;
        match &cmd {
            Cmd::SetRenderState { state, value, .. } => {
                rs.insert(*state, *value);
            }
            Cmd::SetSamplerState {
                sampler,
                kind,
                value,
                ..
            } => {
                ss.insert((*sampler, *kind), *value);
            }
            Cmd::SetTextureStageState {
                stage, kind, value, ..
            } => {
                tss.insert((*stage, *kind), *value);
            }
            Cmd::SetTexture {
                sampler, texture, ..
            } => {
                tex.insert(*sampler, *texture);
            }
            Cmd::SetStreamSource {
                stream,
                buffer,
                offset,
                stride,
                ..
            } => {
                streams.insert(*stream, (*buffer, *offset, *stride));
            }
            Cmd::SetIndices { buffer, .. } => indices = *buffer,
            Cmd::SetShader { pixel, shader, .. } => shaders[*pixel as usize] = *shader,
            Cmd::SetVertexDeclaration { declaration, .. } => decl = *declaration,
            Cmd::SetFvf { fvf, .. } => decl = 0x8000_0000 | *fvf,
            Cmd::SetRenderTarget { index, surface, .. } => {
                targets.insert(*index, *surface);
            }
            Cmd::SetDepthStencilSurface { surface, .. } => depth = *surface,
            Cmd::SetViewport { viewport: v, .. } => viewport = *v,
            Cmd::SetTransform { state, matrix, .. } => {
                let w = matrix.to_words();
                transforms.insert(*state, h64(1, &w));
            }
            Cmd::SetConstants {
                kind,
                start,
                count,
                data,
                ..
            } => {
                let w = data.to_words();
                let per = if kind % 3 == 2 { 1 } else { 4 };
                for r in 0..*count as usize {
                    let slot = usize::from(*kind) * 256 + *start as usize + r;
                    if slot < consts.len() {
                        let mut v = [0u32; 4];
                        for (i, x) in v.iter_mut().enumerate().take(per) {
                            *x = w.get(r * per + i).copied().unwrap_or(0);
                        }
                        consts[slot] = v;
                        pending.push((slot, v));
                    }
                }
                if let Some(i) = win_of(present) {
                    wins[i].const_bytes += u64::from(data.len());
                }
            }
            Cmd::DrawIndexedPrimitive { .. } | Cmd::DrawPrimitive { .. } => {
                if let Some(i) = win_of(present) {
                    let args = match &cmd {
                        Cmd::DrawIndexedPrimitive {
                            kind,
                            base_vertex,
                            min_index,
                            vertices,
                            start_index: start,
                            count,
                            ..
                        } => [
                            *kind,
                            *base_vertex as u32,
                            *min_index,
                            *vertices,
                            *start,
                            *count,
                        ],
                        Cmd::DrawPrimitive {
                            kind, start, count, ..
                        } => [*kind, 0, 0, 0, *start, *count],
                        _ => unreachable!(),
                    };
                    let mut sw: Vec<u32> = streams
                        .iter()
                        .flat_map(|(k, v)| [*k, v.0, v.1, v.2])
                        .collect();
                    sw.push(indices);
                    let streams_h = h64(7, &sw);
                    let key0 = h64(streams_h, &args);
                    let n = seen.entry(key0).or_insert(0);
                    let key = (key0, *n);
                    *n += 1;
                    let mut pw: Vec<u32> = vec![shaders[0], shaders[1], decl, depth];
                    pw.extend(rs.iter().flat_map(|(k, v)| [*k, *v]));
                    pw.extend(targets.iter().flat_map(|(k, v)| [*k, *v]));
                    pw.extend(viewport);
                    let pipe = h64(3, &pw);
                    let mut tw: Vec<u32> = tex.iter().flat_map(|(k, v)| [*k, *v]).collect();
                    tw.extend(ss.iter().flat_map(|((a, b), v)| [*a, *b, *v]));
                    tw.extend(tss.iter().flat_map(|((a, b), v)| [*a, *b, *v]));
                    let textures = h64(5, &tw);
                    let tf: Vec<u32> = transforms
                        .iter()
                        .flat_map(|(k, v)| [*k, *v as u32, (*v >> 32) as u32])
                        .collect();
                    let rec = DrawRec {
                        pipe,
                        textures,
                        streams: streams_h,
                        consts: consts.clone(),
                        transforms: h64(9, &tf),
                        own: std::mem::take(&mut pending),
                    };
                    w_own(&mut wins[i], last.get(&key), &rec);
                    let w = &mut wins[i];
                    w.draws += 1;
                    w.pipe_changes += u64::from(prev.0 != pipe);
                    w.tex_changes += u64::from(prev.1 != textures);
                    w.stream_changes += u64::from(prev.2 != streams_h);
                    prev = (pipe, textures, streams_h);
                    frame_states.insert(h64(
                        pipe,
                        &[textures as u32, (textures >> 32) as u32, streams_h as u32],
                    ));
                    frame_pipes.insert(pipe);
                    frame_tex.insert(textures);
                    match last.get(&key) {
                        None => w.new += 1,
                        Some(p) => {
                            if p.pipe != rec.pipe
                                || p.textures != rec.textures
                                || p.streams != rec.streams
                            {
                                w.state += 1;
                            } else {
                                let diff: Vec<usize> = (0..consts.len())
                                    .filter(|&s| p.consts[s] != rec.consts[s])
                                    .collect();
                                if diff.is_empty() && p.transforms == rec.transforms {
                                    w.same += 1;
                                } else {
                                    if diff.is_empty() {
                                        w.transforms_only += 1;
                                    }
                                    w.consts_only += 1;
                                    for s in diff {
                                        *w.regs
                                            .entry(((s / 256) as u8, (s % 256) as u32))
                                            .or_default() += 1;
                                    }
                                }
                            }
                        }
                    }
                    pass_keys.entry(pass).or_default().push(key0);
                    pass_at.entry(pass).or_insert((
                        [viewport[2], viewport[3]],
                        targets.get(&0).copied().unwrap_or(0),
                        depth,
                    ));
                    this.insert(key, rec);
                }
            }
            Cmd::Present { .. } => {
                if let Some(i) = win_of(present) {
                    let w = &mut wins[i];
                    let mut values: HashMap<usize, std::collections::HashSet<[u32; 4]>> =
                        HashMap::new();
                    for r in this.values() {
                        for (s, v) in &r.own {
                            values.entry(*s).or_default().insert(*v);
                        }
                    }
                    for (k, r) in &this {
                        let prev_own: HashMap<usize, [u32; 4]> = last
                            .get(k)
                            .map(|p| p.own.iter().copied().collect())
                            .unwrap_or_default();
                        for (s, v) in &r.own {
                            if values.get(s).is_some_and(|set| set.len() == 1) {
                                w.own_frame_wide += 1;
                            } else if prev_own.get(s) == Some(v) {
                                w.own_kept += 1;
                                *w.own_kept_regs.entry(*s).or_default() += 1;
                            } else {
                                w.own_changing += 1;
                                if *s < 4 {
                                    if let Some(p) = prev_own.get(s) {
                                        let u = (0..4)
                                            .map(|j| {
                                                (p[j] as i32)
                                                    .wrapping_sub(v[j] as i32)
                                                    .unsigned_abs()
                                            })
                                            .max()
                                            .unwrap_or(0);
                                        let b = if u <= 4 {
                                            0
                                        } else if u <= 256 {
                                            1
                                        } else if u <= 65536 {
                                            2
                                        } else {
                                            3
                                        };
                                        w.mvp_ulps[b] += 1;
                                    }
                                }
                                *w.own_change_regs.entry(*s).or_default() += 1;
                            }
                        }
                    }
                    let big: std::collections::HashSet<u64> = pass_keys
                        .values()
                        .max_by_key(|v| v.len())
                        .map(|v| v.iter().copied().collect())
                        .unwrap_or_default();
                    let big_pass = pass_keys
                        .iter()
                        .max_by_key(|(_, v)| v.len())
                        .map_or(0, |(p, _)| *p);
                    for (p, n) in &pass_cmds {
                        let keys = pass_keys.get(p).map_or(&[][..], |v| v.as_slice());
                        let again = if *p == big_pass {
                            0
                        } else {
                            keys.iter().filter(|k| big.contains(k)).count() as u64
                        };
                        let at = pass_at.get(p).copied().unwrap_or(([0, 0], 0, 0));
                        let e = w.passes.entry(*p).or_insert((0, 0, 0, 0, [0, 0], 0, 0));
                        e.0 += keys.len() as u64;
                        e.1 += n;
                        e.2 += again;
                        e.3 += 1;
                        e.4 = at.0;
                        e.5 = at.1;
                        e.6 = at.2;
                    }
                    w.frames += 1;
                    w.distinct_state += frame_states.len() as u64;
                    w.distinct_pipe += frame_pipes.len() as u64;
                    w.distinct_tex += frame_tex.len() as u64;
                }
                pass = 0;
                pass_keys.clear();
                pass_cmds.clear();
                pass_at.clear();
                frame_states.clear();
                frame_pipes.clear();
                frame_tex.clear();
                pending.clear();
                last = std::mem::take(&mut this);
                seen.clear();
                present += 1;
            }
            _ => {}
        }
    }
    let kinds = [
        "vs float", "vs int", "vs bool", "ps float", "ps int", "ps bool",
    ];
    for (i, w) in wins.iter().enumerate() {
        let f = w.frames.max(1) as f64;
        let d = w.draws.max(1) as f64;
        println!(
            "window {}..{}: {} frames, {:.0} draws a frame",
            ranges[i].0,
            ranges[i].1,
            w.frames,
            w.draws as f64 / f
        );
        println!(
            "  (a) whole state the same as last frame's draw: {:.1}%",
            100.0 * w.same as f64 / d
        );
        println!(
            "  (b) same but constants/transforms: {:.1}% (of which transforms only {:.1}%)",
            100.0 * w.consts_only as f64 / d,
            100.0 * w.transforms_only as f64 / d
        );
        println!(
            "  (c) other state differs: {:.1}%",
            100.0 * w.state as f64 / d
        );
        println!(
            "  (n) no match last frame: {:.1}%",
            100.0 * w.new as f64 / d
        );
        println!("  the draw's own constant sets the same as last frame's: {:.1}% of draws ({:.1} registers a draw)", 100.0 * w.own_same as f64 / d, w.own_regs as f64 / d);
        println!(
            "  per frame: distinct states {:.0}, pipeline-ish {:.0}, texture sets {:.0}, constant bytes {:.0}; changes from the draw before: pipeline-ish {:.0}, texture sets {:.0}, streams {:.0}",
            w.distinct_state as f64 / f,
            w.distinct_pipe as f64 / f,
            w.distinct_tex as f64 / f,
            w.const_bytes as f64 / f,
            w.pipe_changes as f64 / f,
            w.tex_changes as f64 / f,
            w.stream_changes as f64 / f
        );
        let mut regs: Vec<_> = w.regs.iter().collect();
        regs.sort_by(|a, b| b.1.cmp(a.1));
        let shown: Vec<String> = regs
            .iter()
            .take(24)
            .map(|((k, r), n)| {
                format!(
                    "{} c{}: {:.0}%",
                    kinds[*k as usize],
                    r,
                    100.0 * **n as f64 / d
                )
            })
            .collect();
        println!(
            "  registers that differ in (b), share of all draws: {}",
            shown.join(", ")
        );
        let own = (w.own_frame_wide + w.own_kept + w.own_changing).max(1) as f64;
        println!("  the draws' own constant registers: frame-wide (one value for every draw that sets it) {:.1}%, the same as the draw's last frame {:.1}%, changing {:.1}% ({:.0} registers a frame)", 100.0 * w.own_frame_wide as f64 / own, 100.0 * w.own_kept as f64 / own, 100.0 * w.own_changing as f64 / own, own / f);
        println!("  ModelViewProj rows changed, by their largest float difference: <=4 ulps {:.0}, <=256 {:.0}, <=65536 {:.0}, more {:.0} a frame", w.mvp_ulps[0] as f64 / f, w.mvp_ulps[1] as f64 / f, w.mvp_ulps[2] as f64 / f, w.mvp_ulps[3] as f64 / f);
        for (name, m) in [("changing", &w.own_change_regs), ("kept", &w.own_kept_regs)] {
            let mut v: Vec<_> = m.iter().collect();
            v.sort_by(|a, b| b.1.cmp(a.1));
            let s: Vec<String> = v
                .iter()
                .take(20)
                .map(|(r, n)| format!("{} c{} {:.0}", kinds[**r / 256], **r % 256, **n as f64 / f))
                .collect();
            println!("  own registers {name}, a frame: {}", s.join(", "));
        }
        println!("  passes (0 = before the first BeginScene): draws, commands, draws of objects the biggest pass also draws, viewport, target, depth, in how many frames");
        for (p, e) in &w.passes {
            let n = e.3.max(1) as f64;
            println!("    pass {p:2}: {:6.0} draws {:7.0} commands {:6.0} again  viewport {}x{} target {:#x} depth {:#x} ({} frames)", e.0 as f64 / n, e.1 as f64 / n, e.2 as f64 / n, e.4[0], e.4[1], e.5, e.6, e.3);
        }
    }
}
