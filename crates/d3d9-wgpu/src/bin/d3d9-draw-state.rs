//! What a recording's draws are made with, beyond their shaders (`d3d9-draw-state FILE
//! [Data/Shaders folder or -] [FROM [TO]]`): a graphics driver may compile a Direct3D 9 shader
//! again for the state it is first drawn under, not only when it is first drawn with
//! (`d3d9-first-use`). For every draw, the things a driver could build into a shader's code are
//! put together with the shader: its partner shader, the vertex layout, the alpha test, fog,
//! sRGB write, the render target's format and multisampling, and the kinds and formats of the
//! textures on the samplers the pixel shader declares. A line per present from `FROM` to `TO` at
//! which any such combination is drawn for the first time in the run, by kind of combination
//! (`counts`); and with `DETAIL=1` in the environment, each such draw with its full state.
//! Shaders are named from the game's shader packages when their folder is given (the player's
//! own files; only names are printed).

use d3d9_stream::Cmd;
use d3d9_wgpu::sm::package::Package;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

/// The render states looked at, by `D3DRENDERSTATETYPE` number.
const ALPHA_TEST: u32 = 15;
const ALPHA_REF: u32 = 24;
const ALPHA_FUNC: u32 = 25;
const FOG_ENABLE: u32 = 28;
const FOG_TABLE_MODE: u32 = 35;
const FOG_VERTEX_MODE: u32 = 140;
const SHADE_MODE: u32 = 9;
const CLIP_PLANES: u32 = 152;
const POINT_SPRITES: u32 = 156;
const SRGB_WRITE: u32 = 194;
const COLOUR_WRITE: u32 = 168;
const BLEND: u32 = 27;
const SOURCE_BLEND: u32 = 19;
const DEST_BLEND: u32 = 20;
const Z_ENABLE: u32 = 7;
const Z_WRITE: u32 = 14;
const STENCIL: u32 = 52;
/// `D3DSAMP_SRGBTEXTURE`.
const SAMPLER_SRGB: u32 = 11;

/// A format as a short name: a FOURCC's letters, or the `D3DFORMAT` number.
fn format_name(format: u32) -> String {
    let known = [
        (0, "none"),
        (20, "R8G8B8"),
        (21, "A8R8G8B8"),
        (22, "X8R8G8B8"),
        (23, "R5G6B5"),
        (28, "A8"),
        (50, "L8"),
        (51, "A8L8"),
        (60, "V8U8"),
        (75, "D24S8"),
        (77, "D24X8"),
        (80, "D16"),
        (111, "R16F"),
        (112, "G16R16F"),
        (113, "A16B16G16R16F"),
        (114, "R32F"),
        (115, "G32R32F"),
        (116, "A32B32G32R32F"),
    ];
    if let Some((_, name)) = known.iter().find(|(n, _)| *n == format) {
        return (*name).to_string();
    }
    let letters = format.to_le_bytes();
    if letters.iter().all(|b| b.is_ascii_graphic()) {
        return String::from_utf8_lossy(&letters).into_owned();
    }
    format!("fmt{format}")
}

/// A surface the game can draw into or bind: its format, multisample type and size.
#[derive(Clone, Copy, Default)]
struct Surface {
    format: u32,
    multisample: u32,
    width: u32,
    height: u32,
}

/// A texture: cube or not, its format and usage.
#[derive(Clone, Copy)]
struct Texture {
    cube: bool,
    format: u32,
    usage: u32,
}

/// The samplers a pixel shader's bytecode declares (shader model 2 and 3: its `dcl` instructions
/// with a sampler register), with the declared kind (2 a 2D texture, 3 a cube, 4 a volume); and
/// the shader's major version.
fn declared_samplers(tokens: &[u32]) -> (u8, Vec<(u32, u32)>) {
    let Some(&version) = tokens.first() else {
        return (0, vec![]);
    };
    let major = ((version >> 8) & 0xff) as u8;
    let mut found = Vec::new();
    if major < 2 {
        return (major, found);
    }
    let mut at = 1;
    while at < tokens.len() {
        let token = tokens[at];
        let opcode = token & 0xffff;
        if opcode == 0xffff {
            break;
        }
        if opcode == 0xfffe {
            at += 1 + ((token >> 16) & 0x7fff) as usize;
            continue;
        }
        let length = ((token >> 24) & 0xf) as usize;
        if opcode == 0x1f && length == 2 && at + 2 < tokens.len() {
            let (usage, register) = (tokens[at + 1], tokens[at + 2]);
            let kind = ((register >> 28) & 7) | ((register >> 8) & 0x18);
            if kind == 10 {
                found.push((register & 0x7ff, (usage >> 27) & 0xf));
            }
        }
        at += 1 + length;
    }
    found.sort_unstable();
    (major, found)
}

/// A shader the recording made.
struct Shader {
    name: String,
    made_at: u64,
    major: u8,
    samplers: Vec<(u32, u32)>,
}

/// The state the recording's commands have set so far.
#[derive(Default)]
struct State {
    surfaces: HashMap<u32, Surface>,
    textures: HashMap<u32, Texture>,
    declarations: HashMap<u32, Vec<u8>>,
    shaders: HashMap<u32, Shader>,
    render: HashMap<u32, u32>,
    sampler_srgb: HashMap<u32, u32>,
    bound: HashMap<u32, u32>,
    targets: [u32; 4],
    depth: u32,
    vs: u32,
    ps: u32,
    /// The declaration bound, or the FVF as `(true, fvf)`.
    layout: (bool, u32),
}

impl State {
    fn render(&self, state: u32) -> String {
        self.render
            .get(&state)
            .map_or("-".to_string(), |v| format!("{v:x}"))
    }

    fn shader_name(&self, shader: u32) -> String {
        match self.shaders.get(&shader) {
            _ if shader == 0 => "none".to_string(),
            Some(s) => s.name.clone(),
            None => format!("?{shader:#x}"),
        }
    }

    fn layout_text(&self) -> String {
        let (fvf, value) = self.layout;
        if fvf {
            return format!("fvf {value:#x}");
        }
        let Some(elements) = self.declarations.get(&value) else {
            return format!("decl ?{value:#x}");
        };
        let mut text = String::new();
        for e in elements.chunks_exact(8) {
            let stream = u16::from_le_bytes([e[0], e[1]]);
            if stream == 0xff {
                break;
            }
            let offset = u16::from_le_bytes([e[2], e[3]]);
            // Stream, offset, type, method, usage, usage index.
            let _ = write!(
                text,
                "{stream}@{offset}:t{}m{}u{}.{} ",
                e[4], e[5], e[6], e[7]
            );
        }
        text.trim_end().to_string()
    }

    fn target_text(&self) -> String {
        let mut text = String::new();
        for (i, &t) in self.targets.iter().enumerate() {
            if t == 0 {
                continue;
            }
            let s = self.surfaces.get(&t).copied().unwrap_or_default();
            let _ = write!(
                text,
                "rt{i} {} ms{} {}x{} ",
                format_name(s.format),
                s.multisample,
                s.width,
                s.height
            );
        }
        let d = self.surfaces.get(&self.depth).copied().unwrap_or_default();
        let _ = write!(text, "depth {} ms{}", format_name(d.format), d.multisample);
        text
    }

    /// The textures on the samplers the bound pixel shader declares: declared kind, then what is
    /// bound (kind, format, usage, sRGB read).
    fn textures_text(&self) -> String {
        let Some(shader) = self.shaders.get(&self.ps) else {
            return String::new();
        };
        let mut text = String::new();
        for &(sampler, kind) in &shader.samplers {
            let bound = self.bound.get(&sampler).copied().unwrap_or(0);
            let what = match self.textures.get(&bound) {
                _ if bound == 0 => "unbound".to_string(),
                Some(t) => format!(
                    "{} {} u{:x}",
                    if t.cube { "cube" } else { "2d" },
                    format_name(t.format),
                    t.usage
                ),
                None => "?".to_string(),
            };
            let srgb = self.sampler_srgb.get(&sampler).copied().unwrap_or(0);
            let _ = write!(text, "s{sampler}(k{kind})={what} srgb{srgb}; ");
        }
        text.trim_end().to_string()
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("the recording");
    let shaders_dir = args.next().filter(|a| a != "-");
    let from: u64 = args
        .next()
        .map_or(0, |a| a.parse().expect("FROM: a present"));
    let to: u64 = args
        .next()
        .map_or(u64::MAX, |a| a.parse().expect("TO: a present"));
    let detail = std::env::var_os("DETAIL").is_some();
    let mut names: HashMap<Vec<u8>, String> = HashMap::new();
    if let Some(dir) = shaders_dir {
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .expect("the Shaders folder")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sdp")))
            .collect();
        files.sort();
        for file in files {
            let stem = file.file_stem().unwrap().to_string_lossy().into_owned();
            let bytes = std::fs::read(&file).expect("a package");
            if let Ok(package) = Package::parse(&bytes) {
                for s in package.shaders {
                    names
                        .entry(s.bytecode.clone())
                        .or_insert_with(|| format!("{stem}/{}", s.name));
                }
            }
        }
    }
    let bytes = std::fs::read(&path).expect("read");
    let mut s = State::default();
    let mut presents = 0u64;
    let mut draw = 0u64;
    // The combinations drawn so far, by kind.
    let kinds = [
        "vs", "ps", "pair", "vs+layout", "ps+alpha", "ps+fog", "ps+srgb", "ps+target",
        "ps+textures", "vs+clip", "ps+shade", "ps+points",
    ];
    let mut seen: Vec<HashSet<String>> = kinds.iter().map(|_| HashSet::new()).collect();
    let mut new_here = vec![0u32; kinds.len()];
    let mut totals = vec![0u64; kinds.len()];
    let mut lines: Vec<String> = Vec::new();
    // `SHOW=P:D,D,..` in the environment: present P's draws D (numbered from 1 as printed) are
    // printed in full whether or not anything in them is new (to name a replay's slow draws).
    let show: Option<(u64, Vec<u64>)> = std::env::var("SHOW").ok().map(|v| {
        let (p, d) = v.split_once(':').expect("SHOW=present:draw,..");
        (
            p.parse().expect("a present"),
            d.split(',').map(|x| x.parse().expect("a draw")).collect(),
        )
    });
    for c in d3d9_stream::wire::commands(&bytes).expect("a stream") {
        // A recording cut by the run's end stops at its last whole command.
        let Ok(c) = c else { break };
        match c {
            Cmd::CreateDevice {
                present,
                back_buffers,
                depth_stencil,
                ..
            }
            | Cmd::Reset {
                present,
                back_buffers,
                depth_stencil,
                ..
            } => {
                let p = present.to_words();
                let word = |i: usize| p.get(i).copied().unwrap_or(0);
                let buffers = back_buffers.to_words();
                for &b in buffers.iter() {
                    s.surfaces.insert(
                        b,
                        Surface {
                            format: word(2),
                            multisample: word(4),
                            width: word(0),
                            height: word(1),
                        },
                    );
                }
                s.surfaces.insert(
                    depth_stencil,
                    Surface {
                        format: word(10),
                        multisample: word(4),
                        width: word(0),
                        height: word(1),
                    },
                );
                s.targets = [buffers.first().copied().unwrap_or(0), 0, 0, 0];
                s.depth = depth_stencil;
                s.render.clear();
                s.sampler_srgb.clear();
                s.bound.clear();
                (s.vs, s.ps, s.layout) = (0, 0, (true, 0));
            }
            Cmd::CreateTexture {
                texture,
                levels,
                width,
                height,
                usage,
                format,
                ..
            } => {
                s.textures.insert(
                    texture,
                    Texture {
                        cube: false,
                        format,
                        usage,
                    },
                );
                for (level, &surface) in levels.to_words().iter().enumerate() {
                    s.surfaces.insert(
                        surface,
                        Surface {
                            format,
                            multisample: 0,
                            width: (width >> level).max(1),
                            height: (height >> level).max(1),
                        },
                    );
                }
            }
            Cmd::CreateCubeTexture {
                texture,
                surfaces,
                edge,
                usage,
                format,
                ..
            } => {
                s.textures.insert(
                    texture,
                    Texture {
                        cube: true,
                        format,
                        usage,
                    },
                );
                for &surface in surfaces.to_words().iter() {
                    s.surfaces.insert(
                        surface,
                        Surface {
                            format,
                            multisample: 0,
                            width: edge,
                            height: edge,
                        },
                    );
                }
            }
            Cmd::CreateSurface {
                surface,
                width,
                height,
                format,
                multisample,
                ..
            } => {
                s.surfaces.insert(
                    surface,
                    Surface {
                        format,
                        multisample,
                        width,
                        height,
                    },
                );
            }
            Cmd::CreateShader {
                shader,
                pixel,
                tokens,
                ..
            } => {
                let name = names
                    .get(tokens.bytes().as_ref())
                    .cloned()
                    .unwrap_or_else(|| format!("({} bytes)", tokens.len()));
                let (major, samplers) = declared_samplers(&tokens.to_words());
                s.shaders.insert(
                    shader,
                    Shader {
                        name,
                        made_at: presents,
                        major,
                        samplers: if pixel { samplers } else { vec![] },
                    },
                );
            }
            Cmd::CreateDeclaration {
                declaration,
                elements,
                ..
            } => {
                s.declarations
                    .insert(declaration, elements.bytes().into_owned());
            }
            Cmd::SetRenderTarget { index, surface, .. } if index < 4 => {
                s.targets[index as usize] = surface;
            }
            Cmd::SetDepthStencilSurface { surface, .. } => s.depth = surface,
            Cmd::SetRenderState { state, value, .. } => {
                s.render.insert(state, value);
            }
            Cmd::SetSamplerState {
                sampler,
                kind: SAMPLER_SRGB,
                value,
                ..
            } => {
                s.sampler_srgb.insert(sampler, value);
            }
            Cmd::SetTexture {
                sampler, texture, ..
            } => {
                s.bound.insert(sampler, texture);
            }
            Cmd::SetVertexDeclaration { declaration, .. } => s.layout = (false, declaration),
            Cmd::SetFvf { fvf, .. } => s.layout = (true, fvf),
            Cmd::SetShader { pixel, shader, .. } => {
                if pixel {
                    s.ps = shader;
                } else {
                    s.vs = shader;
                }
            }
            Cmd::DrawPrimitive { .. }
            | Cmd::DrawIndexedPrimitive { .. }
            | Cmd::DrawPrimitiveUp { .. } => {
                draw += 1;
                let (vs, ps) = (s.shader_name(s.vs), s.shader_name(s.ps));
                let rt = s.surfaces.get(&s.targets[0]).copied().unwrap_or_default();
                let targets = s.targets.iter().filter(|&&t| t != 0).count();
                let alpha = format!(
                    "test {} func {}",
                    s.render(ALPHA_TEST),
                    if s.render.get(&ALPHA_TEST).copied().unwrap_or(0) != 0 {
                        s.render(ALPHA_FUNC)
                    } else {
                        "off".to_string()
                    }
                );
                let fog = format!(
                    "fog {} table {} vertex {}",
                    s.render(FOG_ENABLE),
                    s.render(FOG_TABLE_MODE),
                    s.render(FOG_VERTEX_MODE)
                );
                let target = format!(
                    "{} ms{} x{targets}",
                    format_name(rt.format),
                    rt.multisample
                );
                let textures = s.textures_text();
                let layout = s.layout_text();
                let keys = [
                    vs.clone(),
                    ps.clone(),
                    format!("{vs} + {ps}"),
                    format!("{vs} | {layout}"),
                    format!("{ps} | {alpha}"),
                    format!("{ps} | {fog}"),
                    format!("{ps} | srgb write {}", s.render(SRGB_WRITE)),
                    format!("{ps} | {target}"),
                    format!("{ps} | {textures}"),
                    format!("{vs} | clip planes {}", s.render(CLIP_PLANES)),
                    format!("{ps} | shade {}", s.render(SHADE_MODE)),
                    format!("{ps} | point sprites {}", s.render(POINT_SPRITES)),
                ];
                let mut new = Vec::new();
                for (i, key) in keys.iter().enumerate() {
                    if seen[i].insert(key.clone()) {
                        totals[i] += 1;
                        if presents >= from && presents <= to {
                            new_here[i] += 1;
                            new.push(kinds[i]);
                        }
                    }
                }
                let shown = show
                    .as_ref()
                    .is_some_and(|(p, d)| *p == presents && d.contains(&draw));
                if (detail && !new.is_empty()) || shown {
                    let made = |h: u32| s.shaders.get(&h).map_or(0, |x| x.made_at);
                    let model = |h: u32| s.shaders.get(&h).map_or(0, |x| x.major);
                    lines.push(format!(
                        "  draw {draw} new [{}]\n    vs {vs} {:#x} (model {}, made at {}) ps {ps} {:#x} (model {}, made at {})\n    layout {layout}\n    {}\n    {alpha} ref {}; {fog}; srgb write {}; clip {}; shade {}; points {}; colour write {}; blend {} {}/{}; z {} write {}; stencil {}\n    textures {textures}",
                        new.join(", "),
                        s.vs,
                        model(s.vs),
                        made(s.vs),
                        s.ps,
                        model(s.ps),
                        made(s.ps),
                        s.target_text(),
                        s.render(ALPHA_REF),
                        s.render(SRGB_WRITE),
                        s.render(CLIP_PLANES),
                        s.render(SHADE_MODE),
                        s.render(POINT_SPRITES),
                        s.render(COLOUR_WRITE),
                        s.render(BLEND),
                        s.render(SOURCE_BLEND),
                        s.render(DEST_BLEND),
                        s.render(Z_ENABLE),
                        s.render(Z_WRITE),
                        s.render(STENCIL),
                    ));
                }
            }
            Cmd::Present { .. } => {
                if new_here.iter().any(|&n| n > 0) {
                    let counts: Vec<String> = kinds
                        .iter()
                        .zip(&new_here)
                        .filter(|(_, &n)| n > 0)
                        .map(|(k, n)| format!("{k} {n}"))
                        .collect();
                    println!("present {presents}: {}", counts.join(", "));
                    for line in lines.drain(..) {
                        println!("{line}");
                    }
                }
                new_here.iter_mut().for_each(|n| *n = 0);
                presents += 1;
                draw = 0;
            }
            _ => {}
        }
    }
    let all: Vec<String> = kinds
        .iter()
        .zip(&totals)
        .map(|(k, n)| format!("{k} {n}"))
        .collect();
    println!(
        "{presents} presents; combinations drawn: {}",
        all.join(", ")
    );
}
