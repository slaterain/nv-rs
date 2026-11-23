//! One draw for an object's base pass and its extra light passes (`NV_WGPU_FUSE_LIGHTS`, off by
//! default): the game draws an object lit by more lamps than its first pass takes again for each
//! group of lamps (additive passes, a texture pass that multiplies), each with its own vertex and
//! pixel shader, on exactly the base's pixels (depth EQUAL, no depth writes, colour only). The
//! fused pipeline runs every pass's converted shaders in one draw and does the blending the blend
//! unit did between them: the light of an additive pass added, a multiply pass's colour multiplied,
//! the base's alpha kept (the extra passes write colour only).
//!
//! How the modules are made: each pass is converted as usual ([`convert`], registers from storage),
//! its own names prefixed (`p0_`, `p1_` ...) so the passes live in one module, its textures moved to
//! bindings of their own, and its entry point turned into a plain function. A new entry point calls
//! them in order: the vertex stage gives every pass's varyings (each at a location of its own), the
//! pixel stage combines the passes' colours. Each pass reads its own registers through its own entry
//! in the draw table: the base's at the draw's entry, extra pass `k` at the entry `k` after it.
//!
//! What is refused (the passes are then drawn one by one, as the game draws them): the sun shadows'
//! variant, the discard form of the clip planes, a pass writing depth or more than one colour, and
//! more varyings than the stages may pass.

use super::wgsl::{self, Link, DRAW_SLOT};
use super::{Error, Program, Stage};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// How an extra pass's colour went into the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Blend {
    /// One, one: its colour added.
    Add,
    /// Destination colour, zero (or zero, source colour): the target multiplied by its colour.
    Multiply,
}

/// A texture binding of a fused module: pass `pass`'s binding `from` (its texture; the sampler at
/// `from + 1`) moved to `to` (the sampler at `to + 1`), in the stage's group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moved {
    pub pass: usize,
    pub from: u32,
    pub to: u32,
}

/// A fused stage's module and where each pass's textures went.
#[derive(Debug, Clone)]
pub struct Fused {
    pub source: String,
    pub bindings: Vec<Moved>,
}

/// The most locations the varyings may take: below the draw table's slot.
const LOCATIONS: u32 = DRAW_SLOT;

/// A field of a converted module's `In` or `Out` struct: its attributes and its name.
struct Field {
    attrs: String,
    name: String,
    ty: String,
}

/// The fields of the struct `name` in a converted module's text.
fn fields(text: &str, name: &str) -> Vec<Field> {
    let head = format!("struct {name} {{");
    let Some(start) = text.find(&head) else {
        return Vec::new();
    };
    let body = &text[start + head.len()..];
    let end = body.find('}').unwrap_or(body.len());
    body[..end]
        .lines()
        .filter_map(|l| {
            let l = l.trim().trim_end_matches(',');
            let (left, ty) = l.split_once(':')?;
            let left = left.trim();
            let (attrs, name) = match left.rfind(' ') {
                Some(i) => (&left[..i], &left[i + 1..]),
                None => ("", left),
            };
            Some(Field {
                attrs: attrs.to_string(),
                name: name.to_string(),
                ty: ty.trim().to_string(),
            })
        })
        .collect()
}

/// The `@location(n)` of a field's attributes.
fn location(attrs: &str) -> Option<u32> {
    let i = attrs.find("@location(")? + "@location(".len();
    attrs[i..].split(')').next()?.parse().ok()
}

/// `attrs` with its `@location(..)` set to `at`.
fn relocated(attrs: &str, at: u32) -> String {
    match attrs.find("@location(") {
        Some(i) => {
            let rest = &attrs[i..];
            let close = rest.find(')').map_or(rest.len(), |c| c + 1);
            format!("{}@location({at}){}", &attrs[..i], &rest[close..])
        }
        None => attrs.to_string(),
    }
}

fn ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The names a converted module declares at its top level (its private registers, functions,
/// textures and structs), and the storage register bases it uses.
fn declared(text: &str) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = ["rb", "cb", "main"].iter().map(|s| s.to_string()).collect();
    for line in text.lines() {
        let l = line.trim_start();
        let rest = if let Some(r) = l.strip_prefix("var<private> ") {
            r
        } else if let Some(r) = l.strip_prefix("fn ") {
            r
        } else if let Some(r) = l.strip_prefix("struct ") {
            r
        } else if l.starts_with("@group(") {
            match l.find(" var ") {
                Some(i) => &l[i + 5..],
                None => continue,
            }
        } else {
            continue;
        };
        let name: String = rest.chars().take_while(|c| ident_char(*c)).collect();
        if !name.is_empty() {
            names.insert(name);
        }
    }
    names
}

/// `text` with every use of a name of `names` prefixed by `prefix`: not a field access (after a
/// `.`), not an attribute (after `@`), and not a struct's field names (inside a struct's braces,
/// where only the types are renamed).
fn prefixed(text: &str, names: &BTreeSet<String>, prefix: &str) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / 8);
    let mut in_struct = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if in_struct {
            if trimmed.starts_with('}') {
                in_struct = false;
                out.push_str(line);
            } else {
                // A field: its name stays; a type naming a struct of the module is renamed.
                match line.find(':') {
                    Some(i) => {
                        out.push_str(&line[..=i]);
                        out.push_str(&rename_tokens(&line[i + 1..], names, prefix));
                    }
                    None => out.push_str(line),
                }
            }
            continue;
        }
        if trimmed.starts_with("struct ") {
            in_struct = !trimmed.contains('}');
        }
        out.push_str(&rename_tokens(line, names, prefix));
    }
    out
}

fn rename_tokens(text: &str, names: &BTreeSet<String>, prefix: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if ident_start(c) && (i == 0 || !ident_char(chars[i - 1])) {
            let start = i;
            while i < chars.len() && ident_char(chars[i]) {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let before = if start == 0 { ' ' } else { chars[start - 1] };
            if names.contains(&word) && before != '.' && before != '@' {
                out.push_str(prefix);
            }
            out.push_str(&word);
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// A pass's converted module, renamed: the shared prelude, storage declarations and texture fix-up
/// taken out, its entry point a plain function `p{k}_main`, its textures at bindings from `next`
/// on (the moves noted in `moved`).
fn pass_text(
    p: &Program,
    link: &Link,
    k: usize,
    next: &mut u32,
    moved: &mut Vec<Moved>,
) -> Result<String, Error> {
    if !link.storage {
        return Err(Error::Unsupported(
            "a fused pass without the registers from storage".into(),
        ));
    }
    // The two stages place each pass's varyings in the same order only when both list the same
    // ones (a fixed module's vertex stage may write more than its pixel stage reads).
    if link.fixed {
        return Err(Error::Unsupported(
            "a fused pass of fixed-slot modules".into(),
        ));
    }
    if matches!(link.clip, wgsl::Clip::Discard { .. }) {
        return Err(Error::Unsupported(
            "a fused pass with the clip planes' discard".into(),
        ));
    }
    let mut text = wgsl::convert(p, link)?;
    for part in [
        "enable clip_distances;\n",
        wgsl::PRELUDE,
        wgsl::STORAGE_DECLARATIONS,
        wgsl::FIX,
    ] {
        text = text.replacen(part, "", 1);
    }
    text = text.replace("@vertex\n", "").replace("@fragment\n", "");
    // The textures: each `@group(g) @binding(b) var t..` pair moved to bindings of its own.
    let mut lines = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        if let (Some(b0), true) = (line.find("@binding("), line.starts_with("@group(")) {
            let b = &line[b0 + "@binding(".len()..];
            let close = b.find(')').unwrap_or(0);
            if let Ok(from) = b[..close].parse::<u32>() {
                // The texture's line moves to an even binding, its sampler's to the one after.
                let to = if from % 2 == 0 {
                    let to = *next;
                    *next += 2;
                    moved.push(Moved { pass: k, from, to });
                    to
                } else {
                    moved
                        .iter()
                        .rev()
                        .find(|m| m.pass == k && m.from + 1 == from)
                        .map_or(from, |m| m.to + 1)
                };
                lines.push_str(&line[..b0]);
                let _ = write!(lines, "@binding({to})");
                lines.push_str(&b[close + 1..]);
                continue;
            }
        }
        lines.push_str(line);
    }
    // The pass's own register bases (the shared declarations' `rb` and `cb`, taken out above).
    lines.insert_str(0, "var<private> rb: u32;\nvar<private> cb: u32;\n");
    let names = declared(&lines);
    Ok(prefixed(&lines, &names, &format!("p{k}_")))
}

/// The fused vertex stage of `passes` (the base first): every pass's vertex shader run for the
/// vertex, the base's position and clip distances, each pass's varyings at locations of its own (in
/// pass order), and the draw table's entry for the pixel stage. Each pass's `Link` is the one its
/// own draw would use. `inputs[k]`: where pass `k`'s input registers come from, as (register, the
/// fused stage's input location): the passes' shaders may read one vertex element through other
/// registers (each pass's own declaration of its inputs), so each register is fed from the element
/// it reads in that pass's own draw. A register without a location is refused.
pub fn fused_vertex(
    passes: &[(&Program, &Link)],
    inputs: &[Vec<(u32, u32)>],
) -> Result<Fused, Error> {
    if passes.iter().any(|(p, _)| p.stage != Stage::Vertex) {
        return Err(Error::Unsupported(
            "a fused vertex stage of a pixel shader".into(),
        ));
    }
    if inputs.len() != passes.len() {
        return Err(Error::Unsupported("the inputs of each pass".into()));
    }
    let mut next = 0u32;
    let mut moved = Vec::new();
    let mut texts = Vec::new();
    for (k, (p, link)) in passes.iter().enumerate() {
        texts.push(pass_text(p, link, k, &mut next, &mut moved)?);
    }
    let mut s = String::new();
    if passes[0].1.clip == wgsl::Clip::Distances {
        s.push_str("enable clip_distances;\n");
    }
    s.push_str(wgsl::PRELUDE);
    s.push_str(wgsl::STORAGE_DECLARATIONS);
    s.push_str(wgsl::FIX);
    for t in &texts {
        s.push_str(t);
    }
    // The inputs: each vertex element once, at its fused location, whichever register a pass reads
    // it through; the built-in inputs (the vertex and instance numbers) once each.
    let mut elements: BTreeMap<u32, Field> = BTreeMap::new();
    let mut builtins: BTreeMap<String, Field> = BTreeMap::new();
    // Each pass's input field and the fused field it is given.
    let mut feeds: Vec<(usize, String, String)> = Vec::new();
    for (k, t) in texts.iter().enumerate() {
        for f in fields(t, &format!("p{k}_In")) {
            let Some(register) = location(&f.attrs) else {
                feeds.push((k, f.name.clone(), f.name.clone()));
                builtins.entry(f.name.clone()).or_insert(f);
                continue;
            };
            let Some(&(_, at)) = inputs[k].iter().find(|(r, _)| *r == register) else {
                return Err(Error::Unsupported(
                    "a fused pass reading a register no element feeds".into(),
                ));
            };
            let name = format!("a{at}");
            if let Some(e) = elements.get(&at) {
                if e.ty != f.ty {
                    return Err(Error::Unsupported(
                        "one vertex element read as two types".into(),
                    ));
                }
            } else {
                let field = Field {
                    attrs: relocated(&f.attrs, at),
                    name: name.clone(),
                    ty: f.ty.clone(),
                };
                elements.insert(at, field);
            }
            feeds.push((k, f.name, name));
        }
    }
    let _ = writeln!(s, "struct In {{");
    for f in elements.values().chain(builtins.values()) {
        let _ = writeln!(s, "    {} {}: {},", f.attrs, f.name, f.ty);
    }
    if !builtins.contains_key("draw") {
        let _ = writeln!(s, "    @builtin(instance_index) draw: u32,");
    }
    s.push_str("}\nstruct Out {\n    @builtin(position) @invariant position: vec4<f32>,\n");
    let mut at = 0u32;
    let mut outs: Vec<(usize, String, String)> = Vec::new(); // pass, its field, the fused field
    for (k, t) in texts.iter().enumerate() {
        for f in fields(t, &format!("p{k}_Out")) {
            if location(&f.attrs).is_none() {
                if k == 0 && f.attrs.contains("clip_distances") {
                    let _ = writeln!(s, "    {} clip: {},", f.attrs, f.ty);
                    outs.push((0, f.name.clone(), "clip".into()));
                }
                continue;
            }
            if f.name == "draw" {
                continue;
            }
            if at >= LOCATIONS {
                return Err(Error::Unsupported(
                    "more varyings than a fused draw passes".into(),
                ));
            }
            let name = format!("f{k}_{}", f.name);
            let _ = writeln!(s, "    {} {name}: {},", relocated(&f.attrs, at), f.ty);
            outs.push((k, f.name.clone(), name));
            at += 1;
        }
    }
    let _ = writeln!(
        s,
        "    @location({DRAW_SLOT}) @interpolate(flat) draw: u32,"
    );
    s.push_str("}\n@vertex\nfn main(input: In) -> Out {\n    var out: Out;\n");
    for (k, t) in texts.iter().enumerate() {
        let _ = writeln!(s, "    var i{k}: p{k}_In;");
        for (pass, from, to) in &feeds {
            if *pass != k {
                continue;
            }
            if from == "draw" {
                let _ = writeln!(s, "    i{k}.draw = input.draw + {k}u;");
            } else {
                let _ = writeln!(s, "    i{k}.{from} = input.{to};");
            }
        }
        let _ = writeln!(s, "    let o{k} = p{k}_main(i{k});");
    }
    s.push_str("    out.position = o0.position;\n");
    for (k, from, to) in &outs {
        let _ = writeln!(s, "    out.{to} = o{k}.{from};");
    }
    s.push_str("    out.draw = input.draw;\n    return out;\n}\n");
    Ok(Fused {
        source: s,
        bindings: moved,
    })
}

/// The fused pixel stage of `passes` (the base first; `blends[k - 1]` how extra pass `k` went into
/// the target): every pass's pixel shader run on its own varyings (as [`fused_vertex`] placed them),
/// the base's colour then each extra's added or multiplied in, clamped to [0, 1] after each step on
/// a target whose blending clamps (`clamp`: a unorm target), the base's alpha kept. The base's alpha
/// test discards the pixel as before; an extra pass with an alpha test of its own is the caller's to
/// refuse (its test would discard the whole pixel here, not only that pass's light).
pub fn fused_pixel(
    passes: &[(&Program, &Link)],
    blends: &[Blend],
    clamp: bool,
) -> Result<Fused, Error> {
    if passes.iter().any(|(p, _)| p.stage != Stage::Pixel) {
        return Err(Error::Unsupported(
            "a fused pixel stage of a vertex shader".into(),
        ));
    }
    if blends.len() + 1 != passes.len() {
        return Err(Error::Unsupported("a blend for each extra pass".into()));
    }
    for (p, _) in passes {
        let sig = wgsl::signature(p);
        if sig.depth || sig.colours != 1 {
            return Err(Error::Unsupported(
                "a fused pass writing depth or more than one colour".into(),
            ));
        }
    }
    let mut next = 0u32;
    let mut moved = Vec::new();
    let mut texts = Vec::new();
    for (k, (p, link)) in passes.iter().enumerate() {
        texts.push(pass_text(p, link, k, &mut next, &mut moved)?);
    }
    let mut s = String::new();
    s.push_str(wgsl::PRELUDE);
    s.push_str(wgsl::STORAGE_DECLARATIONS);
    s.push_str(wgsl::FIX);
    for t in &texts {
        s.push_str(t);
    }
    s.push_str("struct In {\n    @builtin(position) frag: vec4<f32>,\n");
    let front = texts.iter().enumerate().any(|(k, t)| {
        fields(t, &format!("p{k}_In"))
            .iter()
            .any(|f| f.name == "front")
    });
    if front {
        s.push_str("    @builtin(front_facing) front: bool,\n");
    }
    let mut at = 0u32;
    let mut ins: Vec<(usize, String, String)> = Vec::new();
    for (k, t) in texts.iter().enumerate() {
        for f in fields(t, &format!("p{k}_In")) {
            if location(&f.attrs).is_none() || f.name == "draw" {
                continue;
            }
            if at >= LOCATIONS {
                return Err(Error::Unsupported(
                    "more varyings than a fused draw passes".into(),
                ));
            }
            let name = format!("f{k}_{}", f.name);
            let _ = writeln!(s, "    {} {name}: {},", relocated(&f.attrs, at), f.ty);
            ins.push((k, f.name.clone(), name));
            at += 1;
        }
    }
    let _ = writeln!(
        s,
        "    @location({DRAW_SLOT}) @interpolate(flat) draw: u32,"
    );
    s.push_str("}\nstruct Out {\n    @location(0) c0: vec4<f32>,\n}\n");
    s.push_str("@fragment\nfn main(input: In) -> Out {\n");
    for (k, t) in texts.iter().enumerate() {
        let _ = writeln!(s, "    var i{k}: p{k}_In;");
        for f in fields(t, &format!("p{k}_In")) {
            match f.name.as_str() {
                "frag" | "front" => {
                    let _ = writeln!(s, "    i{k}.{0} = input.{0};", f.name);
                }
                "draw" => {
                    let _ = writeln!(s, "    i{k}.draw = input.draw + {k}u;");
                }
                _ => {}
            }
        }
        for (pass, from, to) in &ins {
            if *pass == k {
                let _ = writeln!(s, "    i{k}.{from} = input.{to};");
            }
        }
        let _ = writeln!(s, "    let o{k} = p{k}_main(i{k});");
    }
    s.push_str("    var c = o0.c0;\n");
    for (k, b) in blends.iter().enumerate() {
        let e = format!("o{}.c0.rgb", k + 1);
        let step = match b {
            Blend::Add => format!("c.rgb + {e}"),
            Blend::Multiply => format!("c.rgb * {e}"),
        };
        if clamp {
            let _ = writeln!(s, "    c = vec4<f32>(saturate({step}), c.a);");
        } else {
            let _ = writeln!(s, "    c = vec4<f32>({step}, c.a);");
        }
    }
    s.push_str("    var out: Out;\n    out.c0 = c;\n    return out;\n}\n");
    Ok(Fused {
        source: s,
        bindings: moved,
    })
}

#[cfg(test)]
mod tests {
    use super::super::{parse, tests::small_vertex_shader, usage, validate};
    use super::*;

    /// ps_2_0: `dcl t0.xy; dcl_2d s0; texld r0, t0, s0; mul r0, r0, c0; mov oC0, r0`.
    fn small_pixel_shader() -> Vec<u32> {
        vec![
            0xFFFF_0200,
            0x0200_001F,
            0x8000_0000,
            0xB003_0000,
            0x0200_001F,
            0x9000_0000,
            0xA00F_0800,
            0x0300_0042,
            0x800F_0000,
            0xB0E4_0000,
            0xA0E4_0800,
            0x0300_0005,
            0x800F_0000,
            0x80E4_0000,
            0xA0E4_0000,
            0x0200_0001,
            0x800F_0800,
            0x80E4_0000,
            0x0000_FFFF,
        ]
    }

    fn links() -> (Link, Link) {
        let varyings = vec![wgsl::Semantic {
            usage: usage::TEXCOORD,
            index: 0,
        }];
        let vs = Link {
            inputs: vec![wgsl::Input::Float(4)],
            centroid: wgsl::centroid_everywhere(1),
            varyings: varyings.clone(),
            storage: true,
            ..Link::default()
        };
        let ps = Link {
            centroid: wgsl::centroid_everywhere(1),
            varyings,
            storage: true,
            ..Link::default()
        };
        (vs, ps)
    }

    /// A base and two extra passes (one added, one multiplied) make modules that validate: each pass's
    /// varying at a location of its own, each pass's texture at bindings of its own, the extras' draw
    /// table entries after the base's.
    #[test]
    fn a_base_and_two_extra_passes_make_valid_modules() {
        let vp = parse(&small_vertex_shader()).unwrap();
        let pp = parse(&small_pixel_shader()).unwrap();
        let (vl, pl) = links();
        let fed = vec![vec![(0, 0)]; 3];
        let v = fused_vertex(&[(&vp, &vl), (&vp, &vl), (&vp, &vl)], &fed).unwrap();
        validate(&v.source).unwrap_or_else(|e| panic!("{e}\n{}", v.source));
        for k in 0..3 {
            assert!(
                v.source.contains(&format!("@location({k})")),
                "{}",
                v.source
            );
            assert!(v
                .source
                .contains(&format!("i{k}.draw = input.draw + {k}u;")));
        }
        let p = fused_pixel(
            &[(&pp, &pl), (&pp, &pl), (&pp, &pl)],
            &[Blend::Add, Blend::Multiply],
            true,
        )
        .unwrap();
        validate(&p.source).unwrap_or_else(|e| panic!("{e}\n{}", p.source));
        assert_eq!(
            p.bindings,
            (0..3)
                .map(|k| Moved {
                    pass: k,
                    from: 0,
                    to: 2 * k as u32
                })
                .collect::<Vec<_>>()
        );
        assert!(p
            .source
            .contains("c = vec4<f32>(saturate(c.rgb + o1.c0.rgb), c.a);"));
        assert!(p
            .source
            .contains("c = vec4<f32>(saturate(c.rgb * o2.c0.rgb), c.a);"));
        // On a float target the steps are not clamped.
        let f = fused_pixel(&[(&pp, &pl), (&pp, &pl)], &[Blend::Add], false).unwrap();
        assert!(f.source.contains("c = vec4<f32>(c.rgb + o1.c0.rgb, c.a);"));
        validate(&f.source).unwrap_or_else(|e| panic!("{e}\n{}", f.source));
    }

    /// Passes that read one vertex element through other registers (their own declarations): each
    /// register is fed from its element's fused location, and an element read by two passes is one
    /// input.
    #[test]
    fn each_pass_reads_its_registers_from_its_own_elements() {
        let vp = parse(&small_vertex_shader()).unwrap();
        let (vl, _) = links();
        let fed = [vec![(0, 0)], vec![(0, 1)], vec![(0, 0)]];
        let v = fused_vertex(&[(&vp, &vl), (&vp, &vl), (&vp, &vl)], &fed).unwrap();
        validate(&v.source).unwrap_or_else(|e| panic!("{e}\n{}", v.source));
        for line in ["i0.v0 = input.a0;", "i1.v0 = input.a1;", "i2.v0 = input.a0;"] {
            assert!(v.source.contains(line), "{line}\n{}", v.source);
        }
        assert_eq!(
            v.source.matches("@location(0) a0:").count(),
            1,
            "{}",
            v.source
        );
    }

    /// More varyings than the stages pass, a pass without the registers from storage, or a missing
    /// blend: refused, so the passes are drawn one by one.
    #[test]
    fn what_cannot_be_fused_is_refused() {
        let vp = parse(&small_vertex_shader()).unwrap();
        let pp = parse(&small_pixel_shader()).unwrap();
        let (vl, pl) = links();
        // One varying a pass: one pass more than the locations below the draw table's slot.
        let many: Vec<(&Program, &Link)> = (0..=DRAW_SLOT).map(|_| (&vp, &vl)).collect();
        assert!(fused_vertex(&many, &vec![vec![(0, 0)]; many.len()]).is_err());
        // A register no vertex element feeds.
        assert!(fused_vertex(&[(&vp, &vl), (&vp, &vl)], &[vec![(0, 0)], Vec::new()]).is_err());
        let uniform = Link {
            storage: false,
            ..pl.clone()
        };
        assert!(fused_pixel(&[(&pp, &uniform), (&pp, &pl)], &[Blend::Add], true).is_err());
        assert!(fused_pixel(&[(&pp, &pl), (&pp, &pl)], &[], true).is_err());
    }
}
