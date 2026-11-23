//! A measuring tool: `merge_census FILE A B [C D ...]`.
//! How many draws of an `NV_D3D9_RECORD` file's frames could be drawn as one instanced draw with an
//! earlier draw, for the windows of Presents [A, B). The merge key is everything a draw gives the
//! graphics card except its shader constants (which an instanced draw reads per instance): shaders,
//! declaration, render states, targets, depth surface, viewport, scissor, clip planes, textures,
//! sampler and stage states, streams (buffer, offset, stride), index buffer and the draw's own
//! arguments (its index range).
//!
//! Per frame, for the biggest pass (the world pass) and for all passes:
//! - `any order`: draws whose key an earlier draw of the same pass already had (an upper bound: the
//!   order of work is ignored);
//! - `kept order`: only "movable" draws (depth test LESS or LESSEQUAL with depth writes on, no
//!   blending, no stencil) may join, and only a group opened in the same run of movable draws: a run
//!   ends at a draw that is not movable, a pass, target or depth surface change, a clear, a copy, or
//!   a texture upload; a buffer upload closes only the groups that read that buffer;
//! - switches between consecutive draws as drawn, after merging in place, and after merging plus
//!   sorting each run by (pipeline, textures, streams);
//! - the world pass's draws by blend mode, alpha test, skinned (declaration with blend indices) and
//!   skinned pieces per body part (consecutive skinned draws with the same textures and vertex
//!   buffer).
use d3d9_stream::Cmd;
use std::collections::{BTreeMap, HashMap, HashSet};

fn h64(mut h: u64, words: &[u32]) -> u64 {
    for &w in words {
        h ^= u64::from(w);
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= h >> 29;
    }
    h
}

#[derive(Clone)]
struct D {
    pass: u32,
    pipe: u64,
    tex: u64,
    streams: u64,
    key: u64,
    buffers: Vec<u32>,
    movable: bool,
    /// Blended, tested against depth (LESS or LESSEQUAL), writing none, no stencil: what
    /// `NV_WGPU_SORT_DRAWS=pass` holds after the opaque draws.
    see_through: bool,
    blend: usize,
    alpha_test: bool,
    skinned: bool,
    vb: u32,
    bones: u32,
    ffp: bool,
}

enum Ev {
    Draw(D),
    /// Ends every run (target, clear, copy, texture upload, scene end).
    Wall,
    /// A buffer's contents changed.
    Upload(u32),
}

const BLENDS: [&str; 5] = [
    "no blending",
    "additive (one, one)",
    "alpha (src alpha, inv src alpha)",
    "multiply (dest colour, zero)",
    "other blending",
];

#[derive(Default)]
struct Sums {
    frames: u64,
    draws: u64,
    any_order: u64,
    movable: u64,
    kept_order: u64,
    kept_order_coarse: u64,
    in_place: u64,
    opaque_after_see_through: u64,
    runs: u64,
    ch_asis: [u64; 3],
    ch_merged: [u64; 3],
    ch_sorted: [u64; 3],
    ch_sorted_tex: [u64; 3],
    calls_after: u64,
    blend: [u64; 5],
    blend_movable_not: [u64; 5],
    alpha_test: u64,
    skinned: u64,
    skinned_parts: u64,
    skinned_parts_tex: u64,
    skinned_bone_regs: u64,
    skinned_vbs: u64,
    skinned_all_passes: u64,
    ffp: u64,
    group_sizes: BTreeMap<usize, u64>,
    passes: BTreeMap<u32, (u64, u64, u64)>,
    not_movable_why: BTreeMap<&'static str, u64>,
}

fn changes(list: &[&D], out: &mut [u64; 3]) {
    let mut prev: Option<&D> = None;
    for d in list {
        if let Some(p) = prev {
            out[0] += u64::from(p.pipe != d.pipe);
            out[1] += u64::from(p.tex != d.tex);
            out[2] += u64::from(p.streams != d.streams);
        } else {
            out[0] += 1;
            out[1] += 1;
            out[2] += 1;
        }
        prev = Some(d);
    }
}

/// One frame's events of one pass (or all passes when `pass` is None).
fn frame(evs: &[Ev], world: u32, s: &mut Sums) {
    let draws: Vec<&D> = evs
        .iter()
        .filter_map(|e| match e {
            Ev::Draw(d) if d.pass == world => Some(d),
            _ => None,
        })
        .collect();
    s.frames += 1;
    s.draws += draws.len() as u64;
    // Any order.
    let mut seen = HashSet::new();
    for d in &draws {
        if !seen.insert(d.key) {
            s.any_order += 1;
        }
    }
    // Kept order: runs of movable draws.
    let mut runs: Vec<Vec<Vec<&D>>> = Vec::new(); // run -> groups in first-member order
    let mut open: HashMap<u64, usize> = HashMap::new(); // key -> group index in the current run
    let mut coarse_seen: HashSet<u64> = HashSet::new();
    let mut asis: Vec<&D> = Vec::new();
    let mut merged_order: Vec<&D> = Vec::new();
    let mut in_run = false;
    fn close<'a>(
        runs: &mut [Vec<Vec<&'a D>>],
        open: &mut HashMap<u64, usize>,
        coarse: &mut HashSet<u64>,
        merged_order: &mut Vec<&'a D>,
        in_run: &mut bool,
    ) {
        if *in_run {
            if let Some(r) = runs.last() {
                for g in r {
                    merged_order.push(g[0]);
                }
            }
        }
        open.clear();
        coarse.clear();
        *in_run = false;
    }
    let mut pass_now = 0u32;
    for e in evs {
        match e {
            Ev::Draw(d) => {
                if d.pass != pass_now {
                    close(
                        &mut runs,
                        &mut open,
                        &mut coarse_seen,
                        &mut merged_order,
                        &mut in_run,
                    );
                    pass_now = d.pass;
                }
                if d.pass != world {
                    continue;
                }
                asis.push(d);
                if !d.movable {
                    close(
                        &mut runs,
                        &mut open,
                        &mut coarse_seen,
                        &mut merged_order,
                        &mut in_run,
                    );
                    merged_order.push(d);
                    continue;
                }
                s.movable += 1;
                if !in_run {
                    runs.push(Vec::new());
                    in_run = true;
                    s.runs += 1;
                }
                if !coarse_seen.insert(d.key) {
                    s.kept_order_coarse += 1;
                }
                let run = runs.last_mut().expect("a run");
                match open.get(&d.key) {
                    Some(&g) => {
                        run[g].push(d);
                        s.kept_order += 1;
                    }
                    None => {
                        open.insert(d.key, run.len());
                        run.push(vec![d]);
                    }
                }
            }
            Ev::Wall => close(
                &mut runs,
                &mut open,
                &mut coarse_seen,
                &mut merged_order,
                &mut in_run,
            ),
            Ev::Upload(h) => {
                if let Some(run) = runs.last() {
                    open.retain(|_, g| !run[*g][0].buffers.contains(h));
                }
                // The coarse count ends the run at any upload.
                coarse_seen.clear();
            }
        }
    }
    close(
        &mut runs,
        &mut open,
        &mut coarse_seen,
        &mut merged_order,
        &mut in_run,
    );
    // In place: a draw joins only the draw just before it (the same key, nothing between). And the
    // opaque draws given after a see-through draw with only state sets between (pass mode would move
    // them ahead of it).
    let mut prev_key: Option<u64> = None;
    let mut see_through = false;
    let mut pass_now = 0u32;
    for e in evs {
        match e {
            Ev::Draw(d) => {
                if d.pass != pass_now {
                    prev_key = None;
                    see_through = false;
                    pass_now = d.pass;
                }
                if d.pass != world {
                    continue;
                }
                if d.movable {
                    if prev_key == Some(d.key) {
                        s.in_place += 1;
                    }
                    prev_key = Some(d.key);
                    if see_through {
                        s.opaque_after_see_through += 1;
                    }
                } else {
                    prev_key = None;
                    see_through = d.see_through;
                }
            }
            Ev::Wall | Ev::Upload(_) => {
                prev_key = None;
                see_through = false;
            }
        }
    }
    changes(&asis, &mut s.ch_asis);
    changes(&merged_order, &mut s.ch_merged);
    s.calls_after += merged_order.len() as u64;
    for r in &runs {
        for g in r {
            *s.group_sizes.entry(g.len()).or_default() += 1;
        }
    }
    // Sorted: rebuild the order with each run's groups sorted.
    let mut sorted: Vec<&D> = Vec::new();
    let mut sorted_tex: Vec<&D> = Vec::new();
    let mut ri = 0usize;
    let mut in_run = false;
    fn flush<'a>(
        runs: &[Vec<Vec<&'a D>>],
        ri: &mut usize,
        in_run: &mut bool,
        sorted: &mut Vec<&'a D>,
        st: &mut Vec<&'a D>,
    ) {
        if *in_run {
            let mut firsts: Vec<&D> = runs[*ri].iter().map(|g| g[0]).collect();
            firsts.sort_by_key(|d| (d.pipe, d.tex, d.streams));
            sorted.extend(firsts.iter().copied());
            firsts.sort_by_key(|d| (d.tex, d.pipe, d.streams));
            st.extend(firsts);
            *ri += 1;
        }
        *in_run = false;
    }
    // Walk again with the same run boundaries (runs were opened in order).
    let mut pass_now = 0u32;
    for e in evs {
        match e {
            Ev::Draw(d) => {
                if d.pass != pass_now {
                    flush(&runs, &mut ri, &mut in_run, &mut sorted, &mut sorted_tex);
                    pass_now = d.pass;
                }
                if d.pass != world {
                    continue;
                }
                if !d.movable {
                    flush(&runs, &mut ri, &mut in_run, &mut sorted, &mut sorted_tex);
                    sorted.push(d);
                    sorted_tex.push(d);
                } else {
                    in_run = true;
                }
            }
            Ev::Wall => flush(&runs, &mut ri, &mut in_run, &mut sorted, &mut sorted_tex),
            Ev::Upload(_) => {}
        }
    }
    flush(&runs, &mut ri, &mut in_run, &mut sorted, &mut sorted_tex);
    changes(&sorted, &mut s.ch_sorted);
    changes(&sorted_tex, &mut s.ch_sorted_tex);
    // Blend, skinned.
    let mut prev_part: Option<(u64, u32)> = None;
    let mut prev_tex: Option<u64> = None;
    let mut vbs = HashSet::new();
    for d in &draws {
        s.blend[d.blend] += 1;
        if !d.movable {
            s.blend_movable_not[d.blend] += 1;
        }
        s.alpha_test += u64::from(d.alpha_test);
        s.ffp += u64::from(d.ffp);
        if d.skinned {
            s.skinned += 1;
            if prev_part != Some((d.tex, d.vb)) {
                s.skinned_parts += 1;
            }
            prev_part = Some((d.tex, d.vb));
            if prev_tex != Some(d.tex) {
                s.skinned_parts_tex += 1;
            }
            prev_tex = Some(d.tex);
            s.skinned_bone_regs += u64::from(d.bones);
            vbs.insert(d.vb);
        } else {
            prev_part = None;
            prev_tex = None;
        }
    }
    s.skinned_vbs += vbs.len() as u64;
    for e in evs {
        if let Ev::Draw(d) = e {
            s.skinned_all_passes += u64::from(d.skinned);
            let p = s.passes.entry(d.pass).or_default();
            p.0 += 1;
            p.1 += u64::from(d.blend != 0);
            p.2 += u64::from(d.skinned);
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(&a[0]).expect("read");
    let ranges: Vec<(u64, u64)> = a[1..]
        .chunks(2)
        .map(|c| (c[0].parse().unwrap(), c[1].parse().unwrap()))
        .collect();
    let mut sums: Vec<Sums> = ranges.iter().map(|_| Sums::default()).collect();
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
    let mut scissor = [0i32; 4];
    let mut clips: BTreeMap<u32, [u32; 4]> = BTreeMap::new();
    let mut skinned_decls: HashSet<u32> = HashSet::new();
    let mut present = 0u64;
    let mut pass = 0u32;
    let mut evs: Vec<Ev> = Vec::new();
    let win_of = |p: u64| ranges.iter().position(|&(lo, hi)| p >= lo && p < hi);
    let mut why: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut bone_regs = 0u32;
    for cmd in d3d9_stream::wire::commands(&bytes).expect("stream") {
        let cmd = cmd.expect("command");
        let inwin = win_of(present).is_some();
        match &cmd {
            Cmd::BeginScene { .. } => {
                pass += 1;
                if inwin {
                    evs.push(Ev::Wall);
                }
            }
            Cmd::EndScene { .. }
            | Cmd::Clear { .. }
            | Cmd::StretchRect { .. }
            | Cmd::ColorFill { .. }
            | Cmd::UpdateTexture { .. }
            | Cmd::UploadSurface { .. }
            | Cmd::GetRenderTargetData { .. } => {
                if inwin {
                    evs.push(Ev::Wall);
                }
            }
            Cmd::UploadBuffer { buffer, .. } => {
                if inwin {
                    evs.push(Ev::Upload(*buffer));
                }
            }
            Cmd::CreateDeclaration {
                declaration,
                elements,
                ..
            } => {
                let b = elements.bytes();
                // D3DVERTEXELEMENT9: stream u16, offset u16, type u8, method u8, usage u8, index u8.
                let skinned = b.chunks_exact(8).any(|e| e[0] != 0xff && e[6] == 2);
                if skinned {
                    skinned_decls.insert(*declaration);
                } else {
                    skinned_decls.remove(declaration);
                }
            }
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
            Cmd::SetConstants {
                kind, start, count, ..
            } => {
                if *kind == 0 {
                    bone_regs += (start + count).saturating_sub((*start).max(44));
                }
            }
            Cmd::SetIndices { buffer, .. } => indices = *buffer,
            Cmd::SetShader { pixel, shader, .. } => shaders[*pixel as usize] = *shader,
            Cmd::SetVertexDeclaration { declaration, .. } => decl = *declaration,
            Cmd::SetFvf { fvf, .. } => decl = 0x8000_0000 | *fvf,
            Cmd::SetRenderTarget { index, surface, .. } => {
                targets.insert(*index, *surface);
                if inwin {
                    evs.push(Ev::Wall);
                }
            }
            Cmd::SetDepthStencilSurface { surface, .. } => {
                depth = *surface;
                if inwin {
                    evs.push(Ev::Wall);
                }
            }
            Cmd::SetViewport { viewport: v, .. } => viewport = *v,
            Cmd::SetScissorRect { rect, .. } => scissor = *rect,
            Cmd::SetClipPlane { index, plane, .. } => {
                clips.insert(*index, *plane);
            }
            Cmd::DrawPrimitiveUp { .. } => {
                if inwin {
                    evs.push(Ev::Wall);
                }
            }
            Cmd::DrawIndexedPrimitive { .. } | Cmd::DrawPrimitive { .. } if inwin => {
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
                        1,
                    ],
                    Cmd::DrawPrimitive {
                        kind, start, count, ..
                    } => [*kind, 0, 0, 0, *start, *count, 0],
                    _ => unreachable!(),
                };
                let r = |k: u32, d: u32| rs.get(&k).copied().unwrap_or(d);
                let mut sw: Vec<u32> = streams
                    .iter()
                    .flat_map(|(k, v)| [*k, v.0, v.1, v.2])
                    .collect();
                sw.push(indices);
                let streams_h = h64(7, &sw);
                let mut pw: Vec<u32> = vec![shaders[0], shaders[1], decl, depth];
                pw.extend(rs.iter().flat_map(|(k, v)| [*k, *v]));
                pw.extend(targets.iter().flat_map(|(k, v)| [*k, *v]));
                pw.extend(viewport);
                if r(174, 0) != 0 {
                    pw.extend(scissor.map(|x| x as u32));
                }
                if r(152, 0) != 0 {
                    pw.extend(clips.iter().flat_map(|(k, v)| [*k, v[0], v[1], v[2], v[3]]));
                }
                let pipe = h64(3, &pw);
                let mut tw: Vec<u32> = tex.iter().flat_map(|(k, v)| [*k, *v]).collect();
                tw.extend(ss.iter().flat_map(|((a, b), v)| [*a, *b, *v]));
                tw.extend(tss.iter().flat_map(|((a, b), v)| [*a, *b, *v]));
                let textures = h64(5, &tw);
                let key = h64(
                    h64(pipe, &[textures as u32, (textures >> 32) as u32]),
                    &[streams_h as u32, (streams_h >> 32) as u32],
                );
                let key = h64(key, &args);
                let zon = r(7, 1) != 0;
                let zwrite = r(14, 1) != 0;
                let zfunc = r(23, 4);
                let blend_on = r(27, 0) != 0;
                let stencil = r(52, 0) != 0;
                let (sb, db) = (r(19, 2), r(20, 1));
                let blend = if !blend_on {
                    0
                } else if sb == 2 && db == 2 {
                    1
                } else if sb == 5 && db == 6 {
                    2
                } else if (sb == 9 && db == 1) || (sb == 1 && db == 3) {
                    3
                } else {
                    4
                };
                let movable = zon && zwrite && (zfunc == 2 || zfunc == 4) && !blend_on && !stencil;
                if !movable {
                    let w: &'static str = if blend_on {
                        "blending"
                    } else if !zon {
                        "no depth test"
                    } else if !zwrite {
                        "no depth writes"
                    } else if stencil {
                        "stencil"
                    } else {
                        "depth function"
                    };
                    *why.entry(w).or_default() += 1;
                }
                let mut buffers: Vec<u32> = streams.values().map(|v| v.0).collect();
                buffers.push(indices);
                evs.push(Ev::Draw(D {
                    pass,
                    pipe,
                    tex: textures,
                    streams: streams_h,
                    key,
                    buffers,
                    movable,
                    see_through: blend_on
                        && zon
                        && (zfunc == 2 || zfunc == 4)
                        && !zwrite
                        && !stencil,
                    blend,
                    alpha_test: r(15, 0) != 0,
                    skinned: skinned_decls.contains(&decl),
                    vb: streams.get(&0).map_or(0, |v| v.0),
                    bones: std::mem::take(&mut bone_regs),
                    ffp: shaders[0] == 0,
                }));
            }
            Cmd::Present { .. } => {
                if let Some(i) = win_of(present) {
                    let mut counts: BTreeMap<u32, u64> = BTreeMap::new();
                    for e in &evs {
                        if let Ev::Draw(d) = e {
                            *counts.entry(d.pass).or_default() += 1;
                        }
                    }
                    let world = counts
                        .iter()
                        .max_by_key(|(_, n)| **n)
                        .map_or(0, |(p, _)| *p);
                    frame(&evs, world, &mut sums[i]);
                    for (k, v) in std::mem::take(&mut why) {
                        *sums[i].not_movable_why.entry(k).or_default() += v;
                    }
                }
                why.clear();
                evs.clear();
                pass = 0;
                present += 1;
            }
            _ => {}
        }
    }
    for (i, s) in sums.iter().enumerate() {
        let f = s.frames.max(1) as f64;
        let pf = |x: u64| x as f64 / f;
        println!(
            "window {}..{}: {} frames; world pass {:.0} draws a frame",
            ranges[i].0,
            ranges[i].1,
            s.frames,
            pf(s.draws)
        );
        println!(
            "  join an earlier draw with the same key, any order (upper bound): {:.0}",
            pf(s.any_order)
        );
        println!(
            "  movable (opaque: depth test less/lessequal + depth writes, no blending, no stencil): {:.0}; in {:.0} runs a frame",
            pf(s.movable),
            pf(s.runs)
        );
        println!(
            "  join a group of their run, kept order (buffer uploads close only groups reading the buffer): {:.0}; with any upload ending the run: {:.0}",
            pf(s.kept_order),
            pf(s.kept_order_coarse)
        );
        println!("  draw calls after merging: {:.0}", pf(s.calls_after));
        println!(
            "  join the draw just before them only (in place): {:.0}; opaque draws after a held see-through draw: {:.0}",
            pf(s.in_place),
            pf(s.opaque_after_see_through)
        );
        let gs: Vec<String> = s
            .group_sizes
            .iter()
            .filter(|(k, _)| **k > 1)
            .map(|(k, n)| format!("{k}x{:.1}", pf(*n)))
            .collect();
        println!(
            "  merged groups by size (size x groups a frame): {}",
            gs.join(" ")
        );
        let c = |x: &[u64; 3]| {
            format!(
                "pipeline {:.0}, textures {:.0}, streams {:.0}",
                pf(x[0]),
                pf(x[1]),
                pf(x[2])
            )
        };
        println!("  switches as drawn: {}", c(&s.ch_asis));
        println!("  switches merged in place: {}", c(&s.ch_merged));
        println!(
            "  switches merged + runs sorted by pipeline, textures, streams: {}",
            c(&s.ch_sorted)
        );
        println!(
            "  switches merged + runs sorted by textures, pipeline, streams: {}",
            c(&s.ch_sorted_tex)
        );
        let b: Vec<String> = (0..5)
            .map(|k| format!("{} {:.0}", BLENDS[k], pf(s.blend[k])))
            .collect();
        println!("  world draws by blend mode: {}", b.join(", "));
        let w: Vec<String> = s
            .not_movable_why
            .iter()
            .map(|(k, n)| format!("{k} {:.0}", pf(*n)))
            .collect();
        println!("  not movable, all passes, why: {}", w.join(", "));
        println!(
            "  alpha-tested {:.0}; no vertex shader (fixed function) {:.0}",
            pf(s.alpha_test),
            pf(s.ffp)
        );
        println!(
            "  skinned: world pass {:.1} draws in {:.1} body parts ({:.2} pieces a part); all passes {:.1}",
            pf(s.skinned),
            pf(s.skinned_parts),
            s.skinned as f64 / s.skinned_parts.max(1) as f64,
            pf(s.skinned_all_passes)
        );
        println!(
            "  skinned pieces: {:.1} parts by textures alone ({:.2} pieces a part), {:.1} distinct vertex buffers, {:.1} vs registers c44 up set a skinned draw",
            pf(s.skinned_parts_tex),
            s.skinned as f64 / s.skinned_parts_tex.max(1) as f64,
            pf(s.skinned_vbs),
            s.skinned_bone_regs as f64 / s.skinned.max(1) as f64
        );
        println!("  passes: draws, blended, skinned a frame");
        for (p, e) in &s.passes {
            println!(
                "    pass {p:2}: {:7.1} {:7.1} {:7.1}",
                pf(e.0),
                pf(e.1),
                pf(e.2)
            );
        }
    }
}
