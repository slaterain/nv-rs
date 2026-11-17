//! Direct3D 9's fixed-function pipeline, for draws with no shader bound: the vertex layout an FVF
//! code describes (`SetFVF`), and shaders made as Direct3D 9 shader bytecode that do what the
//! fixed-function stages do, converted like the game's own ([`crate::sm::wgsl`]).
//!
//! What the game draws so: the intro movie (`PlayBink`, `FNVIntro.bik`), each frame as tiles of
//! pre-transformed vertices (`D3DFVF_XYZRHW | D3DFVF_TEX1`) through texture stage 0 (exe
//! `00ec0280`, `00ec0460`). Every other draw of the game has both shaders; a draw this module does
//! not cover (untransformed vertices without a vertex shader, a texture operation it lacks) is
//! skipped, and the device prints its layout and stages once.
//!
//! The rules, from Microsoft's Direct3D 9 documentation (learn.microsoft.com, windows/win32/direct3d9):
//! - **The FVF's layout** (`d3dfvf`, `mapping-fvf-codes-to-a-directx-9-declaration`): position
//!   (`XYZ` 3 floats, `XYZRHW` 4 floats as `POSITIONT`, `XYZW` 4 floats, `XYZB1`..`XYZB5` 3 floats and
//!   1 to 5 blend weights, the last one blend indices with `LASTBETA_UBYTE4` / `LASTBETA_D3DCOLOR`),
//!   then normal, point size, diffuse, specular, and the texture coordinate sets, each 2 floats
//!   unless `D3DFVF_TEXCOORDSIZEn` says 1, 3 or 4, in that order, packed.
//! - **Pre-transformed vertices** (`d3dfvf`: "the position of a transformed vertex"): x and y are
//!   the render target's pixel coordinates, z the depth and the fourth value the reciprocal of
//!   the homogeneous w; the vertex is not transformed nor lit. Pixel centres are at whole
//!   coordinates (`directly-mapping-texels-to-pixels`): the converter's half-pixel move does that
//!   for every vertex shader, so the vertex shader here only turns the pixel coordinates back into
//!   the clip space of the draw's viewport, with w = 1 / RHW for perspective-correct varyings.
//! - **The texture stages** (`d3dtexturestagestatetype` with its defaults, `d3dtextureop`,
//!   `d3dta`, `texture-blending`): stage by stage until a colour operation `D3DTOP_DISABLE`, each
//!   colour (RGB) and alpha operation on its arguments (`D3DTA_DIFFUSE`, `CURRENT` (stage 0: the
//!   diffuse), `TEXTURE`, `TFACTOR`, `SPECULAR`, `TEMP`, `CONSTANT`; with `COMPLEMENT` 1 - x and
//!   `ALPHAREPLICATE` the alpha in every channel), the result clamped to [0, 1] and written to
//!   `D3DTSS_RESULTARG` (the current colour or the temporary register, 0 at the start). Texture
//!   stage n samples sampler n at the texture coordinate set `D3DTSS_TEXCOORDINDEX` names.
//!   `D3DTOP_DOTPRODUCT3` puts its result in all four channels (`d3dtextureop`).
//! - **Specular** (`d3drenderstatetype`, `D3DRS_SPECULARENABLE`): added to the colour after the
//!   last stage.
//! - The diffuse a vertex without one has is opaque white, the specular 0 (not in Microsoft's
//!   pages; DXVK's fixed-function shaders, `d3d9_fixed_function.cpp`, which are measured against
//!   Windows' drivers).
//!
//! The alpha test, blending, depth, stencil, culling and the samplers' states are the device's,
//! as for every draw. Fog (`D3DRS_FOGENABLE`) is printed as not provided, as for every draw.

/// Texture stages (`D3DTSS_*` states 1 to 32, index 0 unused).
pub const STAGES: usize = 8;
pub const STAGE_STATES: usize = 33;

/// `D3DTEXTURESTAGESTATETYPE` values used here.
pub mod tss {
    pub const COLOROP: usize = 1;
    pub const COLORARG1: usize = 2;
    pub const COLORARG2: usize = 3;
    pub const ALPHAOP: usize = 4;
    pub const ALPHAARG1: usize = 5;
    pub const ALPHAARG2: usize = 6;
    pub const TEXCOORDINDEX: usize = 11;
    pub const TEXTURETRANSFORMFLAGS: usize = 24;
    pub const COLORARG0: usize = 26;
    pub const ALPHAARG0: usize = 27;
    pub const RESULTARG: usize = 28;
    pub const CONSTANT: usize = 32;
}

/// `D3DTEXTUREOP`.
pub mod top {
    pub const DISABLE: u32 = 1;
    pub const SELECTARG1: u32 = 2;
    pub const SELECTARG2: u32 = 3;
    pub const MODULATE: u32 = 4;
    pub const MODULATE2X: u32 = 5;
    pub const MODULATE4X: u32 = 6;
    pub const ADD: u32 = 7;
    pub const ADDSIGNED: u32 = 8;
    pub const ADDSIGNED2X: u32 = 9;
    pub const SUBTRACT: u32 = 10;
    pub const ADDSMOOTH: u32 = 11;
    pub const BLENDDIFFUSEALPHA: u32 = 12;
    pub const BLENDTEXTUREALPHA: u32 = 13;
    pub const BLENDFACTORALPHA: u32 = 14;
    pub const BLENDTEXTUREALPHAPM: u32 = 15;
    pub const BLENDCURRENTALPHA: u32 = 16;
    pub const PREMODULATE: u32 = 17;
    pub const MODULATEALPHA_ADDCOLOR: u32 = 18;
    pub const MODULATECOLOR_ADDALPHA: u32 = 19;
    pub const MODULATEINVALPHA_ADDCOLOR: u32 = 20;
    pub const MODULATEINVCOLOR_ADDALPHA: u32 = 21;
    pub const DOTPRODUCT3: u32 = 24;
    pub const MULTIPLYADD: u32 = 25;
    pub const LERP: u32 = 26;
}

/// `D3DTA_*` arguments and modifiers.
pub mod ta {
    pub const DIFFUSE: u32 = 0;
    pub const CURRENT: u32 = 1;
    pub const TEXTURE: u32 = 2;
    pub const TFACTOR: u32 = 3;
    pub const SPECULAR: u32 = 4;
    pub const TEMP: u32 = 5;
    pub const CONSTANT: u32 = 6;
    pub const SELECTMASK: u32 = 0xF;
    pub const COMPLEMENT: u32 = 0x10;
    pub const ALPHAREPLICATE: u32 = 0x20;
}

/// The texture stage states every device starts with (`d3dtexturestagestatetype`): stage 0
/// modulates its texture by the diffuse and takes the texture's alpha, the others are off; every
/// stage reads the texture coordinate set of its own number.
pub fn default_stage_states() -> [[u32; STAGE_STATES]; STAGES] {
    std::array::from_fn(|stage| {
        let mut s = [0u32; STAGE_STATES];
        let first = stage == 0;
        s[tss::COLOROP] = if first { top::MODULATE } else { top::DISABLE };
        s[tss::COLORARG1] = ta::TEXTURE;
        s[tss::COLORARG2] = ta::CURRENT;
        s[tss::ALPHAOP] = if first { top::SELECTARG1 } else { top::DISABLE };
        s[tss::ALPHAARG1] = ta::TEXTURE;
        s[tss::ALPHAARG2] = ta::CURRENT;
        s[tss::TEXCOORDINDEX] = stage as u32;
        s[tss::COLORARG0] = ta::CURRENT;
        s[tss::ALPHAARG0] = ta::CURRENT;
        s[tss::RESULTARG] = ta::CURRENT;
        s
    })
}

/// `D3DDECLTYPE`s and usages of the elements an FVF makes.
const FLOAT1: u8 = 0;
const D3DCOLOR: u8 = 4;
const UBYTE4: u8 = 5;
const POSITION: u8 = 0;
const BLENDWEIGHT: u8 = 1;
const BLENDINDICES: u8 = 2;
const NORMAL: u8 = 3;
const PSIZE: u8 = 4;
const TEXCOORD: u8 = 5;
const POSITIONT: u8 = 9;
const COLOR: u8 = 10;

/// The vertex elements (offset, `D3DDECLTYPE`, usage, usage index; all in stream 0) of an FVF
/// code, as Direct3D 9 lays them out (module documentation); None for a code without a valid
/// position (0 included: no layout).
pub fn fvf_elements(fvf: u32) -> Option<Vec<(u16, u8, u8, u8)>> {
    let mut out = Vec::new();
    let mut offset = 0u16;
    let mut push = |ty: u8, usage: u8, index: u8| {
        out.push((offset, ty, usage, index));
        offset += match ty {
            0..=3 => 4 * (u16::from(ty) + 1),
            _ => 4,
        };
    };
    // D3DFVF_POSITION_MASK.
    match fvf & 0x400E {
        0x2 => push(FLOAT1 + 2, POSITION, 0),
        0x4 => push(FLOAT1 + 3, POSITIONT, 0),
        0x4002 => push(FLOAT1 + 3, POSITION, 0),
        p @ (0x6 | 0x8 | 0xA | 0xC | 0xE) => {
            push(FLOAT1 + 2, POSITION, 0);
            let betas = (p - 0x4) / 2;
            let last = fvf & (0x1000 | 0x8000);
            let weights = betas - u32::from(last != 0);
            if weights > 0 {
                push(FLOAT1 + weights as u8 - 1, BLENDWEIGHT, 0);
            }
            match last {
                0x1000 => push(UBYTE4, BLENDINDICES, 0),
                0x8000 => push(D3DCOLOR, BLENDINDICES, 0),
                _ => {}
            }
        }
        _ => return None,
    }
    if fvf & 0x10 != 0 {
        push(FLOAT1 + 2, NORMAL, 0);
    }
    if fvf & 0x20 != 0 {
        push(FLOAT1, PSIZE, 0);
    }
    if fvf & 0x40 != 0 {
        push(D3DCOLOR, COLOR, 0);
    }
    if fvf & 0x80 != 0 {
        push(D3DCOLOR, COLOR, 1);
    }
    let sets = ((fvf >> 8) & 0xF).min(8);
    for n in 0..sets {
        // D3DFVF_TEXCOORDSIZEn: 0 two floats, 1 three, 2 four, 3 one.
        let size = match (fvf >> (16 + 2 * n)) & 3 {
            0 => 2,
            1 => 3,
            2 => 4,
            _ => 1,
        };
        push(FLOAT1 + size - 1, TEXCOORD, n as u8);
    }
    Some(out)
}

// Shader bytecode (the `D3DSHADER_*` token formats).

const END: u32 = 0x0000_FFFF;
const MOV: u32 = 1;
const ADD: u32 = 2;
const MAD: u32 = 4;
const MUL: u32 = 5;
const RCP: u32 = 6;
const DP3: u32 = 8;
const LRP: u32 = 18;
const DCL: u32 = 31;
const TEX: u32 = 66;
const DEF: u32 = 81;

const TEMP: u32 = 0;
const INPUT: u32 = 1;
const CONST: u32 = 2;
const OUTPUT: u32 = 6;
const COLOROUT: u32 = 8;
const SAMPLER: u32 = 10;

const XYZW: u32 = 0xE4;
const XXXX: u32 = 0x00;
const YYYY: u32 = 0x55;
const ZZZZ: u32 = 0xAA;
const WWWW: u32 = 0xFF;
const ZWZW: u32 = 0xEE;

/// Source modifiers (`D3DSHADER_PARAM_SRCMOD_TYPE`).
const NEG: u32 = 1;
const BX2: u32 = 4;
const COMP: u32 = 6;

fn reg(ty: u32, n: u32) -> u32 {
    ((ty & 7) << 28) | ((ty & 0x18) << 8) | n
}

fn dst(ty: u32, n: u32, mask: u32) -> u32 {
    0x8000_0000 | reg(ty, n) | (mask << 16)
}

/// `dst` with the saturate modifier.
fn sat(ty: u32, n: u32, mask: u32) -> u32 {
    dst(ty, n, mask) | 0x0010_0000
}

fn src(ty: u32, n: u32, swizzle: u32, modifier: u32) -> u32 {
    0x8000_0000 | reg(ty, n) | (swizzle << 16) | (modifier << 24)
}

struct Code(Vec<u32>);

impl Code {
    fn new(version: u32) -> Code {
        Code(vec![version])
    }

    fn ins(&mut self, op: u32, params: &[u32]) {
        self.0.push(op | ((params.len() as u32) << 24));
        self.0.extend_from_slice(params);
    }

    fn dcl(&mut self, usage: u8, index: u8, ty: u32, n: u32) {
        let token = 0x8000_0000 | u32::from(usage) | (u32::from(index) << 16);
        self.ins(DCL, &[token, dst(ty, n, 0xF)]);
    }

    fn def(&mut self, n: u32, v: [f32; 4]) {
        self.ins(
            DEF,
            &[
                dst(CONST, n, 0xF),
                v[0].to_bits(),
                v[1].to_bits(),
                v[2].to_bits(),
                v[3].to_bits(),
            ],
        );
    }

    fn end(mut self) -> Vec<u32> {
        self.0.push(END);
        self.0
    }
}

/// What the fixed-function vertex shader of a pre-transformed layout reads: the diffuse, the
/// specular and the texture coordinate sets (bit n: set n) the declaration has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VertexKey {
    pub diffuse: bool,
    pub specular: bool,
    pub texcoords: u8,
}

impl VertexKey {
    /// The key of a declaration's elements (usage, index), when it is pre-transformed (has a
    /// `POSITIONT`); None otherwise.
    pub fn of(elements: impl IntoIterator<Item = (u8, u8)>) -> Option<VertexKey> {
        let mut k = VertexKey {
            diffuse: false,
            specular: false,
            texcoords: 0,
        };
        let mut pretransformed = false;
        for (usage, index) in elements {
            match (usage, index) {
                (POSITIONT, 0) => pretransformed = true,
                (COLOR, 0) => k.diffuse = true,
                (COLOR, 1) => k.specular = true,
                (TEXCOORD, n @ 0..=7) => k.texcoords |= 1 << n,
                _ => {}
            }
        }
        pretransformed.then_some(k)
    }
}

/// The vertex shader's constants: c0 = (2 / width, -2 / height, -1 - 2 x / width,
/// 1 + 2 y / height) turns pixel coordinates into the viewport's clip space; c1 = (1 / (max z -
/// min z), -min z / (max z - min z)) turns the depth into the viewport's depth range, which the
/// device maps back (a range of no depth: 0, the depth is then the range's). `viewport`: x, y,
/// width, height as the draw sets it, min and max z as the pass sets them.
pub fn vertex_constants(viewport: [f32; 6]) -> [[f32; 4]; 2] {
    let [x, y, w, h, min_z, max_z] = viewport;
    let range = max_z - min_z;
    let (zs, zo) = if range != 0.0 {
        (1.0 / range, -min_z / range)
    } else {
        (0.0, 0.0)
    };
    [
        [2.0 / w, -2.0 / h, -1.0 - 2.0 * x / w, 1.0 + 2.0 * y / h],
        [zs, zo, 0.0, 0.0],
    ]
}

/// The vertex shader (vs_3_0) of a pre-transformed layout: inputs v0 POSITIONT, v1 the diffuse,
/// v2 the specular, v3 + n texture coordinate set n; outputs o0 the position, o1 COLOR0, o2
/// COLOR1, o3 + n TEXCOORDn. Constants c0, c1: [`vertex_constants`].
pub fn vertex_tokens(k: VertexKey) -> Vec<u32> {
    let mut c = Code::new(0xFFFE_0300);
    c.dcl(POSITIONT, 0, INPUT, 0);
    if k.diffuse {
        c.dcl(COLOR, 0, INPUT, 1);
    }
    if k.specular {
        c.dcl(COLOR, 1, INPUT, 2);
    }
    for n in 0..8u32 {
        if k.texcoords & (1 << n) != 0 {
            c.dcl(TEXCOORD, n as u8, INPUT, 3 + n);
        }
    }
    c.dcl(POSITION, 0, OUTPUT, 0);
    c.dcl(COLOR, 0, OUTPUT, 1);
    c.dcl(COLOR, 1, OUTPUT, 2);
    for n in 0..8u32 {
        if k.texcoords & (1 << n) != 0 {
            c.dcl(TEXCOORD, n as u8, OUTPUT, 3 + n);
        }
    }
    c.def(4, [1.0, 0.0, 0.0, 0.0]);
    // w = 1 / RHW; the pixel coordinates into clip space at w = 1; then times w.
    c.ins(RCP, &[dst(TEMP, 0, 0x8), src(INPUT, 0, WWWW, 0)]);
    c.ins(
        MAD,
        &[
            dst(TEMP, 1, 0x3),
            src(INPUT, 0, XYZW, 0),
            src(CONST, 0, XYZW, 0),
            src(CONST, 0, ZWZW, 0),
        ],
    );
    c.ins(
        MAD,
        &[
            dst(TEMP, 1, 0x4),
            src(INPUT, 0, ZZZZ, 0),
            src(CONST, 1, XXXX, 0),
            src(CONST, 1, YYYY, 0),
        ],
    );
    c.ins(MOV, &[dst(TEMP, 1, 0x8), src(CONST, 4, XXXX, 0)]);
    c.ins(
        MUL,
        &[
            dst(OUTPUT, 0, 0xF),
            src(TEMP, 1, XYZW, 0),
            src(TEMP, 0, WWWW, 0),
        ],
    );
    // The diffuse (opaque white without one) and the specular (0 without one).
    let diffuse = if k.diffuse {
        src(INPUT, 1, XYZW, 0)
    } else {
        src(CONST, 4, XXXX, 0)
    };
    c.ins(MOV, &[dst(OUTPUT, 1, 0xF), diffuse]);
    let specular = if k.specular {
        src(INPUT, 2, XYZW, 0)
    } else {
        src(CONST, 4, YYYY, 0)
    };
    c.ins(MOV, &[dst(OUTPUT, 2, 0xF), specular]);
    for n in 0..8u32 {
        if k.texcoords & (1 << n) != 0 {
            c.ins(MOV, &[dst(OUTPUT, 3 + n, 0xF), src(INPUT, 3 + n, XYZW, 0)]);
        }
    }
    c.end()
}

/// One texture stage as the pixel shader needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StageKey {
    pub colour: [u32; 4],
    pub alpha: [u32; 4],
    pub result: u32,
    pub texcoord: u32,
    /// The texture type its sampler declares: 2 2D, 3 cube.
    pub dim: u8,
}

/// The pixel shader's key: the stages up to the first whose colour operation is off, and
/// whether the specular is added.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PixelKey {
    pub stages: Vec<StageKey>,
    pub specular: bool,
}

impl PixelKey {
    /// The key of the stage states `s`, with each stage's texture type (`cube(n)`) and
    /// `D3DRS_SPECULARENABLE`. An error names a state this module does not provide.
    pub fn of(
        s: &[[u32; STAGE_STATES]; STAGES],
        cube: impl Fn(usize) -> bool,
        specular: bool,
    ) -> Result<PixelKey, String> {
        let mut stages = Vec::new();
        for (n, st) in s.iter().enumerate() {
            if st[tss::COLOROP] == top::DISABLE {
                break;
            }
            let k = StageKey {
                colour: [
                    st[tss::COLOROP],
                    st[tss::COLORARG1],
                    st[tss::COLORARG2],
                    st[tss::COLORARG0],
                ],
                alpha: [
                    st[tss::ALPHAOP],
                    st[tss::ALPHAARG1],
                    st[tss::ALPHAARG2],
                    st[tss::ALPHAARG0],
                ],
                result: st[tss::RESULTARG],
                texcoord: st[tss::TEXCOORDINDEX],
                dim: if cube(n) { 3 } else { 2 },
            };
            // A projected or generated texture coordinate: not provided.
            if st[tss::TEXCOORDINDEX] > 7 {
                return Err(format!(
                    "stage {n}: texture coordinate index {:#x} (generated coordinates)",
                    st[tss::TEXCOORDINDEX]
                ));
            }
            if st[tss::TEXTURETRANSFORMFLAGS] & 0x100 != 0 {
                return Err(format!("stage {n}: projected texture coordinates"));
            }
            for op in [k.colour[0], k.alpha[0]] {
                if !(top::DISABLE..=top::LERP).contains(&op)
                    || matches!(op, top::PREMODULATE | 22 | 23)
                {
                    return Err(format!("stage {n}: texture operation {op}"));
                }
            }
            if matches!(
                k.alpha[0],
                top::MODULATEALPHA_ADDCOLOR
                    | top::MODULATECOLOR_ADDALPHA
                    | top::MODULATEINVALPHA_ADDCOLOR
                    | top::MODULATEINVCOLOR_ADDALPHA
            ) {
                return Err(format!(
                    "stage {n}: colour-only operation {} as alpha",
                    k.alpha[0]
                ));
            }
            for a in k.colour[1..].iter().chain(&k.alpha[1..]).chain([&k.result]) {
                if a & ta::SELECTMASK > ta::CONSTANT || a & !0x3F != 0 {
                    return Err(format!("stage {n}: texture argument {a:#x}"));
                }
            }
            if !matches!(k.result & ta::SELECTMASK, ta::CURRENT | ta::TEMP) {
                return Err(format!("stage {n}: result argument {:#x}", k.result));
            }
            stages.push(k);
        }
        Ok(PixelKey { stages, specular })
    }
}

/// The pixel shader's constants: c0 the texture factor (`D3DRS_TEXTUREFACTOR`), c8 + n stage n's
/// constant (`D3DTSS_CONSTANT`), each a `D3DCOLOR` as red, green, blue, alpha.
pub fn pixel_constants(factor: u32, s: &[[u32; STAGE_STATES]; STAGES]) -> [[f32; 4]; 16] {
    let colour = |c: u32| [16, 8, 0, 24].map(|shift| ((c >> shift) & 0xFF) as f32 / 255.0);
    let mut out = [[0.0; 4]; 16];
    out[0] = colour(factor);
    for (n, st) in s.iter().enumerate() {
        out[8 + n] = colour(st[tss::CONSTANT]);
    }
    out
}

/// Registers of the pixel shader: r0 the current colour, r1 the temporary register, r3 r4 r5 a
/// stage's arguments 1, 2 and 0, r6 its result, r10 + n stage n's texture sample; v0 the
/// diffuse, v1 the specular, v2 + n texture coordinate set n; c1 1, c2 0, c3 (4, 0.5).
const CURRENT_R: u32 = 0;
const TEMP_R: u32 = 1;
const RESULT_R: u32 = 6;

/// The pixel shader (ps_3_0) of `k` (module documentation).
pub fn pixel_tokens(k: &PixelKey) -> Vec<u32> {
    let mut c = Code::new(0xFFFF_0300);
    let uses = |what: u32| {
        k.stages.iter().any(|s| {
            s.colour[1..]
                .iter()
                .chain(&s.alpha[1..])
                .any(|a| a & ta::SELECTMASK == what)
        })
    };
    let textured = |s: &StageKey| {
        // The arguments an operation reads (`d3dtextureop`).
        let args = |op: &[u32; 4]| -> Vec<u32> {
            match op[0] {
                top::SELECTARG1 => vec![op[1]],
                top::SELECTARG2 => vec![op[2]],
                top::MULTIPLYADD | top::LERP => vec![op[1], op[2], op[3]],
                _ => vec![op[1], op[2]],
            }
        };
        args(&s.colour)
            .into_iter()
            .chain(
                if s.colour[0] == top::DOTPRODUCT3 || s.alpha[0] == top::DISABLE {
                    Vec::new()
                } else {
                    args(&s.alpha)
                },
            )
            .any(|a| a & ta::SELECTMASK == ta::TEXTURE)
            || [s.colour[0], s.alpha[0]]
                .iter()
                .any(|&op| matches!(op, top::BLENDTEXTUREALPHA | top::BLENDTEXTUREALPHAPM))
    };
    c.dcl(COLOR, 0, INPUT, 0);
    let specular = k.specular || uses(ta::SPECULAR);
    if specular {
        c.dcl(COLOR, 1, INPUT, 1);
    }
    let mut sets = 0u32;
    for s in k.stages.iter().filter(|s| textured(s)) {
        sets |= 1 << s.texcoord;
    }
    for n in 0..8u32 {
        if sets & (1 << n) != 0 {
            c.dcl(TEXCOORD, n as u8, INPUT, 2 + n);
        }
    }
    for (n, s) in k.stages.iter().enumerate() {
        if textured(s) {
            c.ins(
                DCL,
                &[
                    0x8000_0000 | (u32::from(s.dim) << 27),
                    dst(SAMPLER, n as u32, 0xF),
                ],
            );
        }
    }
    c.def(1, [1.0; 4]);
    c.def(2, [0.0; 4]);
    c.def(3, [4.0, 0.5, 0.0, 0.0]);
    c.ins(MOV, &[dst(TEMP, CURRENT_R, 0xF), src(INPUT, 0, XYZW, 0)]);
    c.ins(MOV, &[dst(TEMP, TEMP_R, 0xF), src(CONST, 2, XYZW, 0)]);
    for (n, s) in k.stages.iter().enumerate() {
        let n = n as u32;
        if textured(s) {
            c.ins(
                TEX,
                &[
                    dst(TEMP, 10 + n, 0xF),
                    src(INPUT, 2 + s.texcoord, XYZW, 0),
                    src(SAMPLER, n, XYZW, 0),
                ],
            );
        }
        if s.colour[0] == top::DOTPRODUCT3 {
            stage_op(&mut c, n, &s.colour, 0xF);
        } else {
            stage_op(&mut c, n, &s.colour, 0x7);
            if s.alpha[0] == top::DISABLE {
                c.ins(
                    MOV,
                    &[dst(TEMP, RESULT_R, 0x8), src(TEMP, CURRENT_R, XYZW, 0)],
                );
            } else {
                stage_op(&mut c, n, &s.alpha, 0x8);
            }
        }
        let out = if s.result & ta::SELECTMASK == ta::TEMP {
            TEMP_R
        } else {
            CURRENT_R
        };
        c.ins(MOV, &[sat(TEMP, out, 0xF), src(TEMP, RESULT_R, XYZW, 0)]);
    }
    if k.specular {
        c.ins(
            ADD,
            &[
                sat(TEMP, CURRENT_R, 0x7),
                src(TEMP, CURRENT_R, XYZW, 0),
                src(INPUT, 1, XYZW, 0),
            ],
        );
    }
    c.ins(MOV, &[dst(COLOROUT, 0, 0xF), src(TEMP, CURRENT_R, XYZW, 0)]);
    c.end()
}

/// A `D3DTA_*` argument of stage `n` as a source operand.
fn argument(a: u32, n: u32) -> u32 {
    let (ty, r) = match a & ta::SELECTMASK {
        ta::DIFFUSE => (INPUT, 0),
        ta::CURRENT => (TEMP, CURRENT_R),
        ta::TEXTURE => (TEMP, 10 + n),
        ta::TFACTOR => (CONST, 0),
        ta::SPECULAR => (INPUT, 1),
        ta::TEMP => (TEMP, TEMP_R),
        _ => (CONST, 8 + n),
    };
    let swizzle = if a & ta::ALPHAREPLICATE != 0 {
        WWWW
    } else {
        XYZW
    };
    let modifier = if a & ta::COMPLEMENT != 0 { COMP } else { 0 };
    src(ty, r, swizzle, modifier)
}

/// One colour or alpha operation (`op`: operation, arguments 1, 2, 0) of stage `n` into r6's
/// `mask` components (`d3dtextureop`'s formulas).
fn stage_op(c: &mut Code, n: u32, op: &[u32; 4], mask: u32) {
    // The arguments into r3 (1), r4 (2), r5 (0), their modifiers applied.
    for (k, r) in [(1, 3u32), (2, 4), (3, 5)] {
        c.ins(MOV, &[dst(TEMP, r, 0xF), argument(op[k], n)]);
    }
    let r = |k: u32| src(TEMP, k, XYZW, 0);
    let rs = |k: u32, swizzle: u32, modifier: u32| src(TEMP, k, swizzle, modifier);
    let d = dst(TEMP, RESULT_R, mask);
    let result = r(RESULT_R);
    match op[0] {
        top::SELECTARG1 => c.ins(MOV, &[d, r(3)]),
        top::SELECTARG2 => c.ins(MOV, &[d, r(4)]),
        top::MODULATE => c.ins(MUL, &[d, r(3), r(4)]),
        top::MODULATE2X => {
            c.ins(MUL, &[d, r(3), r(4)]);
            c.ins(ADD, &[d, result, result]);
        }
        top::MODULATE4X => {
            c.ins(MUL, &[d, r(3), r(4)]);
            c.ins(MUL, &[d, result, src(CONST, 3, XXXX, 0)]);
        }
        top::ADD => c.ins(ADD, &[d, r(3), r(4)]),
        top::ADDSIGNED | top::ADDSIGNED2X => {
            c.ins(ADD, &[d, r(3), r(4)]);
            c.ins(ADD, &[d, result, src(CONST, 3, YYYY, NEG)]);
            if op[0] == top::ADDSIGNED2X {
                c.ins(ADD, &[d, result, result]);
            }
        }
        top::SUBTRACT => c.ins(ADD, &[d, r(3), rs(4, XYZW, NEG)]),
        // Arg1 + Arg2 - Arg1 * Arg2 = Arg1 + Arg2 * (1 - Arg1).
        top::ADDSMOOTH => c.ins(MAD, &[d, r(4), rs(3, XYZW, COMP), r(3)]),
        top::BLENDDIFFUSEALPHA => c.ins(LRP, &[d, src(INPUT, 0, WWWW, 0), r(3), r(4)]),
        top::BLENDTEXTUREALPHA => c.ins(LRP, &[d, src(TEMP, 10 + n, WWWW, 0), r(3), r(4)]),
        top::BLENDFACTORALPHA => c.ins(LRP, &[d, src(CONST, 0, WWWW, 0), r(3), r(4)]),
        top::BLENDCURRENTALPHA => c.ins(LRP, &[d, src(TEMP, CURRENT_R, WWWW, 0), r(3), r(4)]),
        // Arg1 + Arg2 * (1 - the texture's alpha).
        top::BLENDTEXTUREALPHAPM => c.ins(MAD, &[d, r(4), src(TEMP, 10 + n, WWWW, COMP), r(3)]),
        top::MODULATEALPHA_ADDCOLOR => c.ins(MAD, &[d, rs(3, WWWW, 0), r(4), r(3)]),
        top::MODULATECOLOR_ADDALPHA => c.ins(MAD, &[d, r(3), r(4), rs(3, WWWW, 0)]),
        top::MODULATEINVALPHA_ADDCOLOR => c.ins(MAD, &[d, rs(3, WWWW, COMP), r(4), r(3)]),
        top::MODULATEINVCOLOR_ADDALPHA => c.ins(MAD, &[d, rs(3, XYZW, COMP), r(4), rs(3, WWWW, 0)]),
        top::DOTPRODUCT3 => c.ins(DP3, &[d, rs(3, XYZW, BX2), rs(4, XYZW, BX2)]),
        top::MULTIPLYADD => c.ins(MAD, &[d, r(3), r(4), r(5)]),
        top::LERP => c.ins(LRP, &[d, r(5), r(3), r(4)]),
        // Refused by `PixelKey::of`.
        other => unreachable!("texture operation {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_movies_fvf_is_a_pretransformed_position_and_one_coordinate_pair() {
        // D3DFVF_XYZRHW | D3DFVF_TEX1: 16 bytes of position, 8 of texture coordinates: the
        // movie's 24-byte vertices (exe 00ec0280).
        assert_eq!(
            fvf_elements(0x104),
            Some(vec![(0, 3, POSITIONT, 0), (16, 1, TEXCOORD, 0)])
        );
    }

    #[test]
    fn every_fvf_part_lands_where_direct3d_9_puts_it() {
        // XYZB3 | LASTBETA_UBYTE4 | NORMAL | PSIZE | DIFFUSE | SPECULAR | TEX3 with set 1 of
        // three floats and set 2 of one.
        let fvf = 0xA | 0x1000 | 0x10 | 0x20 | 0x40 | 0x80 | 0x300 | (1 << 18) | (3 << 20);
        assert_eq!(
            fvf_elements(fvf),
            Some(vec![
                (0, 2, POSITION, 0),
                (12, 1, BLENDWEIGHT, 0),
                (20, UBYTE4, BLENDINDICES, 0),
                (24, 2, NORMAL, 0),
                (36, 0, PSIZE, 0),
                (40, D3DCOLOR, COLOR, 0),
                (44, D3DCOLOR, COLOR, 1),
                (48, 1, TEXCOORD, 0),
                (56, 2, TEXCOORD, 1),
                (68, 0, TEXCOORD, 2),
            ])
        );
        assert_eq!(fvf_elements(0x4002), Some(vec![(0, 3, POSITION, 0)]));
        assert_eq!(
            fvf_elements(0xE | 0x8000),
            Some(vec![
                (0, 2, POSITION, 0),
                (12, 3, BLENDWEIGHT, 0),
                (28, D3DCOLOR, BLENDINDICES, 0)
            ])
        );
        assert_eq!(fvf_elements(0), None);
        assert_eq!(fvf_elements(0x40), None);
    }

    #[test]
    fn the_viewports_constants_put_its_corners_on_clip_space_corners() {
        let [c0, c1] = vertex_constants([100.0, 50.0, 400.0, 200.0, 0.25, 0.75]);
        let x = |px: f32| px * c0[0] + c0[2];
        let y = |py: f32| py * c0[1] + c0[3];
        assert_eq!((x(100.0), x(500.0)), (-1.0, 1.0));
        assert_eq!((y(50.0), y(250.0)), (1.0, -1.0));
        let z = |d: f32| d * c1[0] + c1[1];
        assert_eq!((z(0.25), z(0.75)), (0.0, 1.0));
        assert_eq!(
            vertex_constants([0.0, 0.0, 8.0, 8.0, 0.5, 0.5])[1],
            [0.0; 4]
        );
    }

    #[test]
    fn the_default_stages_modulate_the_texture_and_take_its_alpha() {
        let s = default_stage_states();
        let k = PixelKey::of(&s, |_| false, false).unwrap();
        assert_eq!(k.stages.len(), 1);
        assert_eq!(
            k.stages[0].colour[..3],
            [top::MODULATE, ta::TEXTURE, ta::CURRENT]
        );
        assert_eq!(k.stages[0].alpha[..2], [top::SELECTARG1, ta::TEXTURE]);
        crate::sm::parse(&pixel_tokens(&k)).expect("a pixel shader the converter reads");
        let v = vertex_tokens(VertexKey {
            diffuse: true,
            specular: false,
            texcoords: 1,
        });
        crate::sm::parse(&v).expect("a vertex shader the converter reads");
    }

    #[test]
    fn operations_and_arguments_it_does_not_provide_are_named() {
        let mut s = default_stage_states();
        s[0][tss::COLOROP] = top::PREMODULATE;
        assert!(PixelKey::of(&s, |_| false, false)
            .unwrap_err()
            .contains("operation 17"));
        let mut s = default_stage_states();
        s[0][tss::TEXCOORDINDEX] = 0x10000;
        assert!(PixelKey::of(&s, |_| false, false).is_err());
        let mut s = default_stage_states();
        s[0][tss::ALPHAOP] = top::MODULATEALPHA_ADDCOLOR;
        assert!(PixelKey::of(&s, |_| false, false).is_err());
        let mut s = default_stage_states();
        s[0][tss::COLOROP] = top::DISABLE;
        assert_eq!(
            PixelKey::of(&s, |_| false, false).unwrap().stages,
            Vec::new()
        );
    }
}
