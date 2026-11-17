//! New lighting (port-added, not in the original; off by default): ambient occlusion, one
//! screen-space step on the finished 3D scene. Turned on by `[nv-rs Display] bAmbientOcclusion=1`
//! in the run's `Fallout.ini` or `NV_AO=1` (`host::fixes::Choice::ambient_occlusion`); wgpu only,
//! the real Direct3D 9 ignores it.
//!
//! **Where.** The game draws the world into its multisampled scene target and depth surface,
//! clears the depth alone (`Clear` with only `D3DCLEAR_ZBUFFER`), draws the first-person arms and
//! weapon, resolves the target and runs its image-space (HDR) step, then draws the HUD and menus on
//! the back buffer (`measure/newlight/NOTES.md`, a recorded frame). The step runs at that depth-only
//! clear: the depth holds the whole world and nothing else, the colour is the world before the
//! game's image-space step, and what comes after (the arms, the image-space step, HUD, menus,
//! Pip-Boy, loading screens) is never touched. The platform records [`STATE`] just before such a
//! clear with the camera and sun of the frame ([`Params`]); the device runs the step once a frame,
//! only when the bound colour target and depth surface have one size and sample count. (The sun is
//! carried for the real sun shadows to come; this step does not draw any.)
//!
//! **What.** From the scene's depth (sample 0 of the multisampled surface), at half resolution:
//! ground-truth ambient occlusion, the horizon search and cosine-weighted slice integral of
//! Jimenez, Wu, Pesce and Jarabo, "Practical Real-Time Strategies for Accurate Indirect Occlusion"
//! (SIGGRAPH 2016 course, Activision tech report ATVI-TR-16-01), as Intel's XeGTAO (MIT) arranges
//! it: [`SLICES`] slices of [`STEPS`] steps a side, a distance falloff over the outer 60 % of
//! [`RADIUS`], applied at [`STRENGTH`]; the normal from the depth (the nearer neighbour on each
//! axis). A sample counts only from [`MIN_HEIGHT`] above the surface's own plane, so decals, rugs
//! and stains lying a hair over the floor do not darken it; and thin occluders in front of what they hide (grass
//! blades, leaves, posts) weigh little ([`THIN`], XeGTAO's thin-occluder compensation, stronger), so
//! grass is not dulled and does not shimmer as it sways. A 4x4 ordered (Bayer) pattern turns each
//! pixel's slices and offsets the steps, the same in every frame (nothing crawls); the
//! full-resolution step averages a 4x4 window of the results (the pattern's whole period) weighted
//! by depth, so edges stay sharp, and multiplies the scene's colour by it. No history between
//! frames. It fades out between [`FADE_FROM`] and [`FADE_TO`] units from the camera. The sky (depth 1)
//! is not shaded, apart from multisampled edge pixels it shares with a shaded object (the step works
//! per pixel, from the depth's first sample, and multiplies all the pixel's samples alike).
//!
//! **When.** Measured in a recording of Doc's house, third person, the Pip-Boy and the street
//! (`measure/newlight/NOTES.md`): in first person, indoors and out, the game clears the depth alone
//! once a frame, on the scene target; with the Pip-Boy open it does so for the world and again on
//! the back buffer for the Pip-Boy's own 3D (never shaded: the back buffer is refused, and the frame's
//! one run is taken by then); in third person it issues no depth-only clear, so there is no ambient
//! occlusion in third person; loading screens and menus have none either.
//!
//! **Limits.** Screen space: only what is on screen occludes; transparent things the game drew
//! before the clear (particles, glass, water) take the darkening of the opaque surface behind them.
//! Screen-space sun shadows were tried and dropped (2026-11-16): on the street they gave only thin
//! dark edges and speckle, not shadows (`measure/newlight/NOTES.md`); real sun shadows need a shadow
//! pass from the sun.
//!
//! **Off.** Nothing is recorded, no texture or pipeline is made, the depth surfaces are made as
//! before ([`enabled`] is read when the device is made), so the frame is what it was.

use std::sync::atomic::{AtomicBool, Ordering};

/// The port-private `SetTransform` state (FourCC `NVAO`, as NVIDIA's `ATOC` render state) that
/// carries [`Params`] in its 16 words. No Direct3D 9 transform state has this number.
pub const STATE: u32 = u32::from_le_bytes(*b"NVAO");

/// The occlusion's reach, in game units (about 0.7 m; a unit is about 1.43 cm).
pub const RADIUS: f32 = 48.0;
/// The most the occlusion's reach spans on screen, as a part of the width.
pub const MAX_RADIUS_SCREEN: f32 = 0.04;
pub const SLICES: u32 = 2;
pub const STEPS: u32 = 4;
/// How much of the occlusion is applied, and its curve.
pub const STRENGTH: f32 = 0.5;
/// How far above the surface's own plane (game units, plus a little with distance) a sample must be
/// to occlude: decals, rugs and stains lie a hair above the floor and must not darken it.
pub const MIN_HEIGHT: f32 = 2.0;
/// Thin-occluder compensation (XeGTAO's, which goes to about 0.7; stronger here, chosen on a comb of
/// grass blades in the GPU tests): how much longer the step toward the camera counts in an
/// occluder's falloff distance, so a blade standing in front of the ground behind it weighs
/// little, while a wall meeting the floor (no step toward the camera at its foot) weighs fully.
pub const THIN: f32 = 3.0;
pub const POWER: f32 = 1.5;
/// How strong the sun is taken to be under a full, bright sun ([`Params::shadow`]; carried for the
/// sun shadows to come, not drawn here).
pub const SHADOW_STRENGTH: f32 = 0.8;
/// The distance band (game units) over which it fades out.
pub const FADE_FROM: f32 = 3000.0;
pub const FADE_TO: f32 = 6000.0;

static ON: AtomicBool = AtomicBool::new(false);

/// Turns the step on or off for devices made from now on (the host, from the run's choice).
pub fn enable(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// Whether devices made now run the step.
pub fn enabled() -> bool {
    ON.load(Ordering::Relaxed)
}

/// What the game's memory gives the step for a frame, as read by the host.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameView {
    /// The world camera's frustum at unit distance: left, right, top, bottom, near, far.
    pub frustum: [f32; 6],
    /// Whether the frustum is orthographic.
    pub ortho: bool,
    /// The camera's world rotation, row by row (Gamebryo: column 0 the view direction, column 1 up,
    /// column 2 right).
    pub rotate: [f32; 9],
    /// Toward the sun in the world (the sun disc's place in the sky, z up); none without a sky.
    pub to_sun: Option<[f32; 3]>,
    /// The sky's sun light colour (red, green, blue).
    pub sun_colour: [f32; 3],
    /// Whether the current cell is an interior.
    pub interior: bool,
}

/// One frame's camera and sun, as the step reads them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub near: f32,
    pub far: f32,
    /// The frustum at unit distance: left, right, bottom, top.
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    /// Toward the sun in view space (x right, y up, z forward), unit length.
    pub sun: [f32; 3],
    /// How strong the sun is: 0 for none (indoors, night, a dark sun). Carried for the sun shadows
    /// to come; this step does not use it.
    pub shadow: f32,
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

impl Params {
    /// The frame's parameters from the game's camera and sun; none for a camera the step cannot
    /// use (orthographic, a frustum out of order, values that are not numbers).
    pub fn from_game(g: &GameView) -> Option<Params> {
        let [left, right, top, bottom, near, far] = g.frustum;
        let usable = g.frustum.iter().chain(&g.rotate).all(|v| v.is_finite())
            && !g.ortho
            && near > 0.0
            && far > near
            && right > left
            && top > bottom;
        if !usable {
            return None;
        }
        let col = |i: usize| [g.rotate[i], g.rotate[3 + i], g.rotate[6 + i]];
        let (forward, up, right_axis) = (col(0), col(1), col(2));
        let mut sun = [0.0; 3];
        let mut shadow = 0.0;
        if let (false, Some(to_sun)) = (g.interior, g.to_sun) {
            let len = dot(to_sun, to_sun).sqrt();
            // Above the horizon (the world's z is up), and a light that is lit.
            let lum =
                0.2126 * g.sun_colour[0] + 0.7152 * g.sun_colour[1] + 0.0722 * g.sun_colour[2];
            if len > 1e-6 && len.is_finite() && to_sun[2] > 0.0 && lum.is_finite() {
                let to_sun = to_sun.map(|v| v / len);
                sun = [
                    dot(to_sun, right_axis),
                    dot(to_sun, up),
                    dot(to_sun, forward),
                ];
                let n = dot(sun, sun).sqrt();
                if n > 1e-6 {
                    sun = sun.map(|v| v / n);
                    shadow = SHADOW_STRENGTH * (lum / 0.5).clamp(0.0, 1.0);
                }
            }
        }
        Some(Params {
            near,
            far,
            left,
            right,
            bottom,
            top,
            sun,
            shadow,
        })
    }

    /// The 16 words of the `SetTransform` matrix ([`STATE`]).
    pub fn to_words(&self) -> [u32; 16] {
        let f = [
            self.near,
            self.far,
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.sun[0],
            self.sun[1],
            self.sun[2],
            self.shadow,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        f.map(f32::to_bits)
    }

    /// [`Params::to_words`] read back; none when they are not usable.
    pub fn from_words(w: &[u32]) -> Option<Params> {
        if w.len() != 16 {
            return None;
        }
        let f: Vec<f32> = w.iter().map(|&x| f32::from_bits(x)).collect();
        let p = Params {
            near: f[0],
            far: f[1],
            left: f[2],
            right: f[3],
            bottom: f[4],
            top: f[5],
            sun: [f[6], f[7], f[8]],
            shadow: f[9],
        };
        let usable = f[..10].iter().all(|v| v.is_finite())
            && p.near > 0.0
            && p.far > p.near
            && p.right > p.left
            && p.top > p.bottom
            && (0.0..=1.0).contains(&p.shadow);
        usable.then_some(p)
    }

    /// The uniform block the shaders read: frustum (left, right, bottom, top), depth (near, far,
    /// width, height of the full-resolution target), sun (toward it, then the shadow's strength).
    fn uniform_bytes(&self, width: u32, height: u32) -> Vec<u8> {
        let f = [
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
            width as f32,
            height as f32,
            self.sun[0],
            self.sun[1],
            self.sun[2],
            self.shadow,
        ];
        f.iter().flat_map(|v| v.to_le_bytes()).collect()
    }
}

// The shaders' arithmetic, kept here for the tests (the WGSL below says the same).

/// A depth value (0 near, 1 far: Direct3D's projection) as distance along the view axis.
pub fn linear_depth(d: f32, near: f32, far: f32) -> f32 {
    near * far / (far - d * (far - near))
}

/// The view-space point at the full-resolution position `xy` (pixel centres at .5) and distance
/// `z`, for a `width` by `height` target.
pub fn view_at(p: &Params, xy: [f32; 2], z: f32, width: f32, height: f32) -> [f32; 3] {
    let (u, v) = (xy[0] / width, xy[1] / height);
    let x = p.left + u * (p.right - p.left);
    let y = p.top + v * (p.bottom - p.top);
    [x * z, y * z, z]
}

/// The full-resolution position of the view-space point `q`.
pub fn pixel_of(p: &Params, q: [f32; 3], width: f32, height: f32) -> [f32; 2] {
    let u = (q[0] / q[2] - p.left) / (p.right - p.left);
    let v = (p.top - q[1] / q[2]) / (p.top - p.bottom);
    [u * width, v * height]
}

/// The 4x4 ordered (Bayer) pattern's value at `x`, `y`, from 0 to 15.
pub fn bayer(x: i32, y: i32) -> u32 {
    let (x, y) = ((x & 3) as u32, (y & 3) as u32);
    let a = x ^ y;
    ((a & 1) << 3) | ((y & 1) << 2) | ((a >> 1) << 1) | (y >> 1)
}

/// The fade with distance: 1 near, 0 from [`FADE_TO`] on.
pub fn fade(z: f32) -> f32 {
    let t = ((z - FADE_FROM) / (FADE_TO - FADE_FROM)).clamp(0.0, 1.0);
    1.0 - t * t * (3.0 - 2.0 * t)
}

const COMMON: &str = "
struct P {
    frustum: vec4<f32>,
    depth: vec4<f32>,
    sun: vec4<f32>,
};
@group(0) @binding(0) var<uniform> p: P;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

fn linear(d: f32) -> f32 {
    return p.depth.x * p.depth.y / (p.depth.y - d * (p.depth.y - p.depth.x));
}

fn view_at(xy: vec2<f32>, z: f32) -> vec3<f32> {
    let uv = xy / p.depth.zw;
    return vec3<f32>(mix(p.frustum.x, p.frustum.y, uv.x) * z, mix(p.frustum.w, p.frustum.z, uv.y) * z, z);
}

fn pixel_of(q: vec3<f32>) -> vec2<f32> {
    let u = (q.x / q.z - p.frustum.x) / (p.frustum.y - p.frustum.x);
    let v = (p.frustum.w - q.y / q.z) / (p.frustum.w - p.frustum.z);
    return vec2<f32>(u, v) * p.depth.zw;
}

fn bayer(q: vec2<i32>) -> f32 {
    let x = u32(q.x & 3);
    let y = u32(q.y & 3);
    let a = x ^ y;
    let b = ((a & 1u) << 3u) | ((y & 1u) << 2u) | ((a >> 1u) << 1u) | (y >> 1u);
    return (f32(b) + 0.5) / 16.0;
}

fn fade_at(z: f32) -> f32 {
    let t = clamp((z - FADE_FROM) / (FADE_TO - FADE_FROM), 0.0, 1.0);
    return 1.0 - t * t * (3.0 - 2.0 * t);
}

";

/// The full-resolution scene depth (the depth and apply steps).
const FULL: &str = "
@group(0) @binding(1) var depth_tex: DEPTH_TYPE;

fn depth_at(xy: vec2<i32>) -> f32 {
    let c = clamp(xy, vec2<i32>(0), vec2<i32>(p.depth.zw) - 1);
    return textureLoad(depth_tex, c, 0);
}
";

/// The half-resolution distances the depth step made (the occlusion step): one load each, not a
/// load of the multisampled depth surface.
const HALF: &str = "
@group(0) @binding(1) var z_tex: texture_2d<f32>;

fn z_at(q: vec2<i32>) -> f32 {
    let c = clamp(q, vec2<i32>(0), vec2<i32>(textureDimensions(z_tex)) - 1);
    return textureLoad(z_tex, c, 0).x;
}

// The view-space point of the half-resolution texel `q` (its full-resolution pixel 2q).
fn point_at(q: vec2<i32>) -> vec3<f32> {
    return view_at(vec2<f32>(q) * 2.0 + 0.5, z_at(q));
}

// The normal at the texel `c` (its point `pc`), toward the camera, from the nearer neighbour on
// each axis.
fn normal_at(c: vec2<i32>, pc: vec3<f32>) -> vec3<f32> {
    let pl = point_at(c - vec2<i32>(1, 0));
    let pr = point_at(c + vec2<i32>(1, 0));
    let pu = point_at(c - vec2<i32>(0, 1));
    let pd = point_at(c + vec2<i32>(0, 1));
    let dx = select(pr - pc, pc - pl, abs(pl.z - pc.z) < abs(pr.z - pc.z));
    let dy = select(pd - pc, pc - pu, abs(pu.z - pc.z) < abs(pd.z - pc.z));
    var n = normalize(cross(dx, dy));
    if (dot(n, pc) > 0.0) {
        n = -n;
    }
    return n;
}
";

/// The depth step: each half-resolution texel's distance (its full-resolution pixel 2q, sample 0;
/// the sky gives the far distance).
const DEPTH: &str = "
@fragment fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(linear(depth_at(vec2<i32>(at.xy) * 2)), 0.0, 0.0, 1.0);
}
";

/// The half-resolution step: occlusion (r), distance (b), weight (a: 0 sky).
const AO: &str = "
const PI: f32 = 3.14159265;
const HALF_PI: f32 = 1.57079633;

@fragment fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let h = vec2<i32>(at.xy);
    let c = h;
    let z = z_at(c);
    if (z >= 0.999 * p.depth.y) {
        return vec4<f32>(1.0, 1.0, 0.0, 0.0);
    }
    let pc = point_at(c);
    let fade = fade_at(pc.z);
    if (fade <= 0.0) {
        return vec4<f32>(1.0, 1.0, pc.z, 1.0);
    }
    let n = normal_at(c, pc);
    let min_h = MIN_HEIGHT + 0.002 * pc.z;
    let v = normalize(-pc);
    let noise_slice = bayer(h);
    let noise_step = bayer(h + vec2<i32>(2, 1));

    let px_per_unit = p.depth.z / ((p.frustum.y - p.frustum.x) * pc.z);
    let radius_px = min(RADIUS * px_per_unit, MAX_RADIUS_SCREEN * p.depth.z);
    var visibility = 1.0;
    if (radius_px >= 2.0) {
        visibility = 0.0;
        let falloff_range = 0.6 * RADIUS;
        let falloff_mul = -1.0 / falloff_range;
        let falloff_add = (RADIUS - falloff_range) / falloff_range + 1.0;
        for (var s = 0; s < SLICES; s++) {
            let phi = (f32(s) + noise_slice) * PI / f32(SLICES);
            let dir = vec3<f32>(cos(phi), sin(phi), 0.0);
            let omega = vec2<f32>(cos(phi), -sin(phi));
            let ortho = dir - dot(dir, v) * v;
            let axis = normalize(cross(ortho, v));
            let pn = n - axis * dot(n, axis);
            let pn_len = length(pn);
            let sgn = select(-1.0, 1.0, dot(ortho, pn) >= 0.0);
            let cos_n = clamp(dot(pn, v) / max(pn_len, 1e-4), 0.0, 1.0);
            let ang = sgn * acos(cos_n);
            let low_plus = cos(ang + HALF_PI);
            let low_minus = cos(ang - HALF_PI);
            var hc_plus = low_plus;
            var hc_minus = low_minus;
            for (var j = 0; j < STEPS; j++) {
                var st = (f32(j) + noise_step) / f32(STEPS);
                st = st * st;
                // In half-resolution texels: half the full-resolution reach.
                let off = omega * max(st * radius_px * 0.5, f32(j + 1));
                let o = vec2<i32>(round(off));
                let s0 = point_at(c + o) - pc;
                let s1 = point_at(c - o) - pc;
                let l0 = max(length(s0), 1e-4);
                let l1 = max(length(s1), 1e-4);
                // A sample barely above the surface's own plane (a decal, a rug, a stain a hair over
                // the floor) is no occluder: its weight ramps in from MIN_HEIGHT above the plane.
                let lift0 = saturate((dot(s0, n) - min_h) / min_h);
                let lift1 = saturate((dot(s1, n) - min_h) / min_h);
                // Thin-occluder compensation (XeGTAO): the falloff distance counts the step toward
                // the camera as longer, so a thin thing in front (a blade of grass, a post) weighs
                // less than a solid wall at the same place.
                let f0 = length(vec3<f32>(s0.xy, s0.z * (1.0 + THIN)));
                let f1 = length(vec3<f32>(s1.xy, s1.z * (1.0 + THIN)));
                hc_plus = max(hc_plus, mix(low_plus, dot(s0 / l0, v), saturate(f0 * falloff_mul + falloff_add) * lift0));
                hc_minus = max(hc_minus, mix(low_minus, dot(s1 / l1, v), saturate(f1 * falloff_mul + falloff_add) * lift1));
            }
            let h_plus = ang + clamp(acos(clamp(hc_plus, -1.0, 1.0)) - ang, -HALF_PI, HALF_PI);
            let h_minus = ang + clamp(-acos(clamp(hc_minus, -1.0, 1.0)) - ang, -HALF_PI, HALF_PI);
            let sin_n = sin(ang);
            let a_plus = (cos_n + 2.0 * h_plus * sin_n - cos(2.0 * h_plus - ang)) * 0.25;
            let a_minus = (cos_n + 2.0 * h_minus * sin_n - cos(2.0 * h_minus - ang)) * 0.25;
            visibility += pn_len * (a_plus + a_minus);
        }
        visibility = pow(saturate(visibility / f32(SLICES)), POWER);
    }
    let ao = mix(1.0, visibility, fade * STRENGTH);
    return vec4<f32>(ao, 1.0, pc.z, 1.0);
}
";

/// The full-resolution step: the blurred occlusion multiplies the scene's colour (the blend: colour times source).
const APPLY: &str = "
@group(0) @binding(2) var ao_tex: texture_2d<f32>;

@fragment fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let xy = vec2<i32>(at.xy);
    let d = depth_at(xy);
    if (d >= 1.0) {
        if (VIEW != 0u) {
            return vec4<f32>(1.0, 1.0, 1.0, 1.0);
        }
        discard;
    }
    let z = linear(d);
    // The occlusion: the half-resolution results around the pixel (the 4x4 pattern's whole
    // period), weighted by how near their distance is to the pixel's.
    let hsize = vec2<i32>(textureDimensions(ao_tex));
    let hc = xy / 2;
    var acc = 0.0;
    var wsum = 0.0;
    for (var j = -2; j < 2; j++) {
        for (var i = -2; i < 2; i++) {
            let q = clamp(hc + vec2<i32>(i, j), vec2<i32>(0), hsize - 1);
            let s = textureLoad(ao_tex, q, 0);
            let w = s.w * saturate(1.0 - abs(s.z - z) / (0.04 * z + 2.0));
            acc += s.x * w;
            wsum += w;
        }
    }
    var ao = 1.0;
    if (wsum >= 1e-3) {
        ao = acc / wsum;
    }
    // `NV_AO_VIEW` (captures only): one term alone, as grey, over the whole picture.
    if (VIEW == 1u) {
        return vec4<f32>(ao, ao, ao, 1.0);
    }
    let f = ao;
    // Less than about 1.5 % darker: left alone (not visible), so most pixels write nothing into
    // the multisampled target.
    if (f >= 0.985) {
        discard;
    }
    return vec4<f32>(f, f, f, 1.0);
}
";

/// The three steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// The half-resolution distances from the scene's depth.
    Depth,
    /// The half-resolution occlusion.
    Occlusion,
    /// The full-resolution blur and multiply.
    Apply,
}

/// The WGSL of `stage`, for a multisampled depth surface or not.
pub fn shader(stage: Stage, multisampled_depth: bool) -> String {
    let depth_type = if multisampled_depth {
        "texture_depth_multisampled_2d"
    } else {
        "texture_depth_2d"
    };
    let constants = format!(
        "const RADIUS: f32 = {RADIUS:?};\nconst MAX_RADIUS_SCREEN: f32 = {MAX_RADIUS_SCREEN:?};\n\
         const SLICES: i32 = {SLICES};\nconst STEPS: i32 = {STEPS};\nconst STRENGTH: f32 = {STRENGTH:?};\n\
         const POWER: f32 = {POWER:?};\nconst MIN_HEIGHT: f32 = {MIN_HEIGHT:?};\n\
         const THIN: f32 = {THIN:?};\n\
         const FADE_FROM: f32 = {FADE_FROM:?};\nconst FADE_TO: f32 = {FADE_TO:?};\n\
         const VIEW: u32 = {}u;\n",
        view()
    );
    let (textures, body) = match stage {
        Stage::Depth => (FULL, DEPTH),
        Stage::Occlusion => (HALF, AO),
        Stage::Apply => (FULL, APPLY),
    };
    format!(
        "{constants}{COMMON}{}{body}",
        textures.replace("DEPTH_TYPE", depth_type)
    )
}

/// `NV_AO_VIEW=ao` (diagnostic captures only, not a player setting): the scene replaced by
/// that one term as grey (white: untouched), read once; 0 otherwise.
pub fn view() -> u32 {
    static VIEW: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *VIEW.get_or_init(
        || match std::env::var("NV_AO_VIEW").as_deref().map(str::trim) {
            Ok("ao") => 1,
            _ => 0,
        },
    )
}

/// The format of the half-resolution occlusion results, and of the half-resolution distances.
const HALF_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const Z_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;

/// The step's GPU objects, made at its first use.
pub(crate) struct Gpu {
    uniforms: wgpu::Buffer,
    /// The depth step's layout and pipeline by multisampled depth (0 no, 1 yes); the occlusion
    /// step's.
    depth: [Option<(wgpu::BindGroupLayout, wgpu::RenderPipeline)>; 2],
    occlusion: Option<(wgpu::BindGroupLayout, wgpu::RenderPipeline)>,
    /// The full-resolution step's layout and pipelines by (multisampled depth, colour format,
    /// samples).
    #[allow(clippy::type_complexity)]
    apply: Vec<(
        (bool, wgpu::TextureFormat, u32),
        wgpu::BindGroupLayout,
        wgpu::RenderPipeline,
    )>,
    /// The half-resolution textures (width, height, distances, occlusion).
    half: Option<(u32, u32, wgpu::TextureView, wgpu::TextureView)>,
}

/// What the step is run on: the scene's colour target and depth surface.
pub(crate) struct Targets<'a> {
    pub colour: &'a wgpu::TextureView,
    pub colour_format: wgpu::TextureFormat,
    pub depth: &'a wgpu::TextureView,
    pub samples: u32,
    pub width: u32,
    pub height: u32,
}

/// A float texture read with `textureLoad`.
const FLOAT_TEXTURE: wgpu::BindingType = wgpu::BindingType::Texture {
    sample_type: wgpu::TextureSampleType::Float { filterable: false },
    view_dimension: wgpu::TextureViewDimension::D2,
    multisampled: false,
};

/// The scene's depth surface.
fn depth_texture(multisampled: bool) -> wgpu::BindingType {
    wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Depth,
        view_dimension: wgpu::TextureViewDimension::D2,
        multisampled,
    }
}

/// The uniform block and `textures` from binding 1 on.
fn layout(device: &wgpu::Device, textures: &[wgpu::BindingType]) -> wgpu::BindGroupLayout {
    let mut entries = vec![wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }];
    for (i, ty) in textures.iter().enumerate() {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 1 + i as u32,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: *ty,
            count: None,
        });
    }
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scene light"),
        entries: &entries,
    })
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source: String,
    target: wgpu::ColorTargetState,
    samples: u32,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scene light"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scene light"),
        layout: Some(&pl),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(target)],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// A full-screen triangle into `view` with `pipeline` and `group`.
fn full_screen(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    view: &wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
    pipeline: &wgpu::RenderPipeline,
    group: &wgpu::BindGroup,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, group, &[]);
    pass.draw(0..3, 0..1);
}

fn half_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("scene light half"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

fn group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniforms: &wgpu::Buffer,
    textures: &[&wgpu::TextureView],
) -> wgpu::BindGroup {
    let mut entries = vec![wgpu::BindGroupEntry {
        binding: 0,
        resource: uniforms.as_entire_binding(),
    }];
    for (i, view) in textures.iter().enumerate() {
        entries.push(wgpu::BindGroupEntry {
            binding: 1 + i as u32,
            resource: wgpu::BindingResource::TextureView(view),
        });
    }
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &entries,
    })
}

impl Gpu {
    pub(crate) fn new(device: &wgpu::Device) -> Gpu {
        Gpu {
            uniforms: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("scene light"),
                size: 48,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            depth: [None, None],
            occlusion: None,
            apply: Vec::new(),
            half: None,
        }
    }

    /// The uniform buffer and the bytes it takes for `params` on `t` (written on the queue, in
    /// order with the batch that draws: [`crate::device::Device`]'s `on_queue`).
    pub(crate) fn uniforms(&self, params: &Params, t: &Targets<'_>) -> (wgpu::Buffer, Vec<u8>) {
        (
            self.uniforms.clone(),
            params.uniform_bytes(t.width, t.height),
        )
    }

    /// Records the three steps into `encoder`: the half-resolution distances, the
    /// half-resolution occlusion, then the scene's colour multiplied by it.
    pub(crate) fn encode(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        t: &Targets<'_>,
    ) {
        let ms = t.samples > 1;
        let (hw, hh) = (t.width.div_ceil(2), t.height.div_ceil(2));
        if !matches!(&self.half, Some((w, h, _, _)) if *w == hw && *h == hh) {
            self.half = Some((
                hw,
                hh,
                half_texture(device, hw, hh, Z_FORMAT),
                half_texture(device, hw, hh, HALF_FORMAT),
            ));
        }
        let (z, occlusion) = self
            .half
            .as_ref()
            .map(|(_, _, z, o)| (z.clone(), o.clone()))
            .expect("made");
        let k = usize::from(ms);
        if self.depth[k].is_none() {
            let l = layout(device, &[depth_texture(ms)]);
            let p = pipeline(
                device,
                &l,
                shader(Stage::Depth, ms),
                wgpu::ColorTargetState {
                    format: Z_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                },
                1,
            );
            self.depth[k] = Some((l, p));
        }
        if self.occlusion.is_none() {
            let l = layout(device, &[FLOAT_TEXTURE]);
            let p = pipeline(
                device,
                &l,
                shader(Stage::Occlusion, ms),
                wgpu::ColorTargetState {
                    format: HALF_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                },
                1,
            );
            self.occlusion = Some((l, p));
        }
        let key = (ms, t.colour_format, t.samples);
        if !self.apply.iter().any(|(k, _, _)| *k == key) {
            let l = layout(device, &[depth_texture(ms), FLOAT_TEXTURE]);
            let p = pipeline(
                device,
                &l,
                shader(Stage::Apply, ms),
                wgpu::ColorTargetState {
                    format: t.colour_format,
                    // The scene's colour times the source; its alpha kept (the occlusion alone
                    // replaces it in `NV_AO_VIEW`).
                    blend: (view() == 0).then_some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Dst,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::COLOR,
                },
                t.samples,
            );
            self.apply.push((key, l, p));
        }
        let (depth_layout, depth_pipeline) = self.depth[k].as_ref().expect("made");
        let g = group(device, depth_layout, &self.uniforms, &[t.depth]);
        full_screen(
            encoder,
            "scene light: distances",
            &z,
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            depth_pipeline,
            &g,
        );
        let (ao_layout, ao_pipeline) = self.occlusion.as_ref().expect("made");
        let g = group(device, ao_layout, &self.uniforms, &[&z]);
        full_screen(
            encoder,
            "scene light: occlusion",
            &occlusion,
            wgpu::LoadOp::Clear(wgpu::Color::WHITE),
            ao_pipeline,
            &g,
        );
        let (_, apply_layout, apply_pipeline) =
            self.apply.iter().find(|(k, _, _)| *k == key).expect("made");
        let g = group(device, apply_layout, &self.uniforms, &[t.depth, &occlusion]);
        full_screen(
            encoder,
            "scene light: apply",
            t.colour,
            wgpu::LoadOp::Load,
            apply_pipeline,
            &g,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> Params {
        // A 75-degree horizontal view at 16:9, the game's near and far of about 10 and 350,000.
        let r = (37.5f32).to_radians().tan();
        let t = r * 9.0 / 16.0;
        Params {
            near: 10.0,
            far: 350_000.0,
            left: -r,
            right: r,
            bottom: -t,
            top: t,
            sun: [0.0, 1.0, 0.0],
            shadow: 0.5,
        }
    }

    /// Direct3D's projection of a view distance, the inverse of [`linear_depth`].
    fn project(z: f32, n: f32, f: f32) -> f32 {
        f / (f - n) * (1.0 - n / z)
    }

    #[test]
    fn depth_is_made_linear_as_the_projection_made_it() {
        for z in [10.0, 11.0, 100.0, 1234.5, 20_000.0] {
            let d = project(z, 10.0, 350_000.0);
            let back = linear_depth(d, 10.0, 350_000.0);
            assert!((back - z).abs() / z < 2e-3, "{z}: {back}");
        }
        assert!((linear_depth(0.0, 10.0, 350_000.0) - 10.0).abs() < 1e-3);
        assert!((linear_depth(1.0, 10.0, 350_000.0) - 350_000.0).abs() < 1.0);
    }

    #[test]
    fn a_view_point_goes_back_to_its_pixel() {
        let p = params();
        let (w, h) = (2560.0, 1440.0);
        for xy in [[0.5, 0.5], [1280.5, 720.5], [2559.5, 3.5], [100.25, 1439.5]] {
            let v = view_at(&p, xy, 500.0, w, h);
            let back = pixel_of(&p, v, w, h);
            assert!((back[0] - xy[0]).abs() < 1e-2 && (back[1] - xy[1]).abs() < 1e-2);
        }
        // The top-left pixel is up and left of the view axis, the centre on it.
        let tl = view_at(&p, [0.0, 0.0], 100.0, w, h);
        assert!(tl[0] < 0.0 && tl[1] > 0.0);
        let c = view_at(&p, [1280.0, 720.0], 100.0, w, h);
        assert!(c[0].abs() < 1e-3 && c[1].abs() < 1e-3 && c[2] == 100.0);
    }

    #[test]
    fn the_pattern_is_the_4x4_bayer_matrix() {
        let rows = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
        for (y, row) in rows.iter().enumerate() {
            for (x, v) in row.iter().enumerate() {
                assert_eq!(bayer(x as i32, y as i32), *v, "{x} {y}");
                assert_eq!(bayer(x as i32 + 4, y as i32 - 8), *v);
            }
        }
    }

    #[test]
    fn it_fades_out_with_distance() {
        assert_eq!(fade(0.0), 1.0);
        assert_eq!(fade(FADE_FROM), 1.0);
        assert_eq!(fade(FADE_TO), 0.0);
        assert_eq!(fade(1e6), 0.0);
        let mid = fade((FADE_FROM + FADE_TO) / 2.0);
        assert!((mid - 0.5).abs() < 1e-4);
    }

    fn game() -> GameView {
        GameView {
            frustum: [-0.77, 0.77, 0.43, -0.43, 10.0, 350_000.0],
            ortho: false,
            // Looking along the world's +y: forward (0, 1, 0), up (0, 0, 1), right (1, 0, 0) as
            // the rotation's columns.
            rotate: [0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            // The sun in front of the camera, high up.
            to_sun: Some([0.0, 1.0, 1.0]),
            sun_colour: [1.0, 0.9, 0.8],
            interior: false,
        }
    }

    #[test]
    fn the_sun_goes_into_view_space() {
        let p = Params::from_game(&game()).unwrap();
        let s = std::f32::consts::FRAC_1_SQRT_2;
        assert!((p.sun[0]).abs() < 1e-6, "{:?}", p.sun);
        assert!(
            (p.sun[1] - s).abs() < 1e-6 && (p.sun[2] - s).abs() < 1e-6,
            "{:?}",
            p.sun
        );
        assert!((p.left, p.right, p.top, p.bottom) == (-0.77, 0.77, 0.43, -0.43));
        assert!(p.shadow > 0.0 && p.shadow <= SHADOW_STRENGTH);
        // Indoors, at night (the sun below the horizon) or with a dark sun: no sun.
        for g in [
            GameView {
                interior: true,
                ..game()
            },
            GameView {
                to_sun: Some([0.0, 0.0, -1.0]),
                ..game()
            },
            GameView {
                sun_colour: [0.0; 3],
                ..game()
            },
            GameView {
                to_sun: None,
                ..game()
            },
        ] {
            assert_eq!(Params::from_game(&g).unwrap().shadow, 0.0, "{g:?}");
        }
        // A camera it cannot use.
        for g in [
            GameView {
                ortho: true,
                ..game()
            },
            GameView {
                frustum: [0.77, -0.77, 0.43, -0.43, 10.0, 350_000.0],
                ..game()
            },
            GameView {
                frustum: [-0.77, 0.77, 0.43, -0.43, 0.0, 350_000.0],
                ..game()
            },
            GameView {
                frustum: [-0.77, 0.77, 0.43, -0.43, 10.0, f32::NAN],
                ..game()
            },
        ] {
            assert_eq!(Params::from_game(&g), None, "{g:?}");
        }
    }

    #[test]
    fn the_words_carry_the_parameters() {
        let p = Params::from_game(&game()).unwrap();
        assert_eq!(Params::from_words(&p.to_words()), Some(p));
        assert_eq!(Params::from_words(&[0; 16]), None);
        assert_eq!(Params::from_words(&p.to_words()[..15]), None);
        let mut w = p.to_words();
        w[9] = 2.0f32.to_bits();
        assert_eq!(Params::from_words(&w), None);
    }

    #[test]
    fn its_state_is_no_direct3d_transform() {
        // D3DTS_VIEW 2, D3DTS_PROJECTION 3, D3DTS_TEXTURE0..7 16..23, D3DTS_WORLDMATRIX(n) 256..511.
        const { assert!(STATE > 511) };
        assert_eq!(&STATE.to_le_bytes(), b"NVAO");
    }

    /// Both steps' WGSL, for both depth kinds, read and validated as wgpu reads them.
    #[test]
    fn the_shaders_are_valid_wgsl() {
        for stage in [Stage::Depth, Stage::Occlusion, Stage::Apply] {
            for ms in [false, true] {
                let src = shader(stage, ms);
                let module = naga::front::wgsl::parse_str(&src)
                    .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&src)));
                naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                )
                .validate(&module)
                .unwrap_or_else(|e| panic!("{stage:?} ms {ms}: {e:?}"));
            }
        }
    }
}
