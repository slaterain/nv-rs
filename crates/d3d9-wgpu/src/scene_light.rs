//! New lighting (port-added, not in the original; off by default): ambient occlusion and sun
//! contact shadows, one screen-space step on the finished 3D scene. Turned on by
//! `[nv-rs Display] bAmbientOcclusion=1` in the run's `Fallout.ini` or `NV_AO=1`
//! (`host::fixes::Choice::ambient_occlusion`); wgpu only, the real Direct3D 9 ignores it.
//!
//! **Where.** The game draws the world into its multisampled scene target and depth surface,
//! clears the depth alone (`Clear` with only `D3DCLEAR_ZBUFFER`), draws the first-person arms and
//! weapon, resolves the target and runs its image-space (HDR) step, then draws the HUD and menus on
//! the back buffer (`measure/newlight/NOTES.md`, a recorded frame). The step runs at that depth-only
//! clear: the depth holds the whole world and nothing else, the colour is the world before the
//! game's image-space step, and what comes after (the arms, the image-space step, HUD, menus,
//! Pip-Boy, loading screens) is never touched. The platform records [`STATE`] just before such a
//! clear with the camera and sun of the frame ([`Params`]); the device runs the step once a frame,
//! only when the bound colour target and depth surface have one size and sample count.
//!
//! **What.** From the scene's depth (sample 0 of the multisampled surface):
//! - ambient occlusion, at half resolution: ground-truth ambient occlusion, the horizon search and
//!   cosine-weighted slice integral of Jimenez, Wu, Pesce and Jarabo, "Practical Real-Time
//!   Strategies for Accurate Indirect Occlusion" (SIGGRAPH 2016 course, Activision tech report
//!   ATVI-TR-16-01), as Intel's XeGTAO (MIT) arranges it: [`SLICES`] slices of [`STEPS`] steps a
//!   side, a distance falloff over the outer 60 % of [`RADIUS`], never darker than [`FLOOR`]; the
//!   normal from the depth (the nearer neighbour on each axis). A 4x4 ordered (Bayer) pattern turns
//!   each pixel's slices and offsets the steps, the same in every frame (nothing crawls); the
//!   full-resolution step averages a 4x4 window of the results (the pattern's whole period)
//!   weighted by depth, so edges stay sharp;
//! - sun contact shadows (outdoors, the sun up), at full resolution and not blurred, so a shadow's
//!   edge is a clean line: [`SHADOW_STEPS`] fixed steps toward the sun in view space, closer
//!   together near the pixel, [`SHADOW_LENGTH`] game units (about 1.7 m) long, through the depth: three
//!   steps in a row behind something nearer the camera by less than [`SHADOW_THICKNESS`] (so a
//!   blade of grass a step thin does not cast, and a post casts a post-wide shadow, not a smear)
//!   darken the pixel by [`SHADOW_STRENGTH`]. Only faces turned toward the sun take it: the game's
//!   own lighting already darkens the others.
//!
//! Thin things (a pixel apart in depth from two or more of its four neighbours: grass blades, leaves,
//! wires, far posts) take neither: alpha-tested foliage gives no surface to shade and would only
//! speckle. The product multiplies the scene's colour. No history between frames. Both fade out
//! between [`FADE_FROM`] and [`FADE_TO`] units from the camera; the sky (depth 1) is never touched.
//!
//! **Limits.** Screen space: only what is on screen casts or receives (a caster off screen or hidden
//! behind something casts nothing); the reach is [`SHADOW_LENGTH`], so low evening sun gives short
//! shadows, not long ones; a thin object seen edge-on casts little; transparent things the game drew
//! before the clear (particles, glass, water) take the darkening of the opaque surface behind them;
//! no character-accurate shadows. A real shadow pass from the sun would remove these.
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
/// The darkest the occlusion makes a pixel (a deep crease reads as shade, not a black patch).
pub const FLOOR: f32 = 0.0;
/// How far above the surface's own plane (game units, plus a little with distance) a sample must be
/// to occlude: decals, rugs and stains lie a hair above the floor and must not darken it.
pub const MIN_HEIGHT: f32 = 2.0;
pub const POWER: f32 = 1.5;
/// The sun march: its length and steps, and how thick a surface is taken to be.
pub const SHADOW_LENGTH: f32 = 120.0;
pub const SHADOW_STEPS: u32 = 28;
pub const SHADOW_THICKNESS: f32 = 32.0;
/// How dark a full contact shadow is, under a full sun.
pub const SHADOW_STRENGTH: f32 = 0.8;
/// The distance band (game units) over which both fade out.
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
    /// How dark a contact shadow is: 0 for none (indoors, night, no sun).
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

/// How far along the march toward the sun step `i` (1 to [`SHADOW_STEPS`]) lies: closer together
/// near the point, where the edge of a contact shadow is decided (the WGSL's `sun_visible`).
pub fn march_distance(i: u32) -> f32 {
    let s = i as f32 / SHADOW_STEPS as f32;
    s * s * SHADOW_LENGTH
}

/// Whether a march step `t` units along the ray and `q_z` from the camera is shadowed by the
/// scene's surface at `scene_z` on the same pixel: the surface nearer the camera, by more than the
/// slack (the depth's, growing along the ray: a grazing face does not shadow itself) and less
/// than [`SHADOW_THICKNESS`].
pub fn blocks(t: f32, q_z: f32, scene_z: f32) -> bool {
    let delta = q_z - scene_z;
    delta > 1.0 + 0.003 * q_z + 0.05 * t && delta < SHADOW_THICKNESS
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
@group(0) @binding(1) var depth_tex: DEPTH_TYPE;

@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

fn depth_at(xy: vec2<i32>) -> f32 {
    let c = clamp(xy, vec2<i32>(0), vec2<i32>(p.depth.zw) - 1);
    return textureLoad(depth_tex, c, 0);
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

fn point_at(xy: vec2<i32>) -> vec3<f32> {
    return view_at(vec2<f32>(xy) + 0.5, linear(depth_at(xy)));
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

// The surface at the full-resolution pixel `c` (its point `pc`): its normal, toward the camera, from
// the nearer neighbour on each axis; `thin` when the pixel is apart in depth from two or more of its
// four neighbours (a grass blade, a leaf, a wire, a silhouette edge): no surface to shade.
struct Surface {
    n: vec3<f32>,
    thin: bool,
};

fn surface_at(c: vec2<i32>, pc: vec3<f32>) -> Surface {
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
    let tz = 0.05 * pc.z + 2.0;
    let breaks = u32(abs(pl.z - pc.z) > tz) + u32(abs(pr.z - pc.z) > tz)
        + u32(abs(pu.z - pc.z) > tz) + u32(abs(pd.z - pc.z) > tz);
    let thin = breaks >= 2u;
    return Surface(n, thin);
}

// The sun march from the point `pc` (normal `n`) toward the sun, in view space, SHADOW_LENGTH game
// units long, SHADOW_STEPS fixed steps closer together near the point (no pattern: the edge stays a
// clean line): 0 when three steps in a row pass behind something nearer the camera by less than
// SHADOW_THICKNESS (a blade of grass a step thin does not cast), else 1.
fn sun_visible(pc: vec3<f32>, n: vec3<f32>) -> f32 {
    let l = p.sun.xyz;
    let start = pc + n * (0.5 + 0.002 * pc.z);
    var run = 0;
    for (var i = 1; i <= SHADOW_STEPS; i++) {
        let s = f32(i) / f32(SHADOW_STEPS);
        let t = s * s * SHADOW_LENGTH;
        let q = start + l * t;
        if (q.z <= p.depth.x) {
            break;
        }
        let sp = pixel_of(q);
        if (any(sp < vec2<f32>(0.0)) || any(sp >= p.depth.zw)) {
            break;
        }
        let delta = q.z - linear(depth_at(vec2<i32>(sp)));
        // The slack grows along the ray, so a surface lit at a grazing angle does not shadow
        // itself through its own facets.
        if (delta > 1.0 + 0.003 * q.z + 0.05 * t && delta < SHADOW_THICKNESS) {
            run += 1;
            if (run >= 3) {
                return 0.0;
            }
        } else {
            run = 0;
        }
    }
    return 1.0;
}
";

/// The half-resolution step: occlusion (r), distance (b), weight (a: 0 sky).
const AO: &str = "
const PI: f32 = 3.14159265;
const HALF_PI: f32 = 1.57079633;

@fragment fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let h = vec2<i32>(at.xy);
    let c = h * 2;
    let d = depth_at(c);
    if (d >= 1.0) {
        return vec4<f32>(1.0, 1.0, 0.0, 0.0);
    }
    let pc = view_at(vec2<f32>(c) + 0.5, linear(d));
    let fade = fade_at(pc.z);
    if (fade <= 0.0) {
        return vec4<f32>(1.0, 1.0, pc.z, 1.0);
    }
    let surface = surface_at(c, pc);
    let n = surface.n;
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
                let off = omega * max(st * radius_px, f32(j + 1));
                let o = vec2<i32>(round(off));
                let s0 = point_at(c + o) - pc;
                let s1 = point_at(c - o) - pc;
                let l0 = max(length(s0), 1e-4);
                let l1 = max(length(s1), 1e-4);
                // A sample barely above the surface's own plane (a decal, a rug, a stain a hair over
                // the floor) is no occluder: its weight ramps in from MIN_HEIGHT above the plane.
                let lift0 = saturate((dot(s0, n) - min_h) / min_h);
                let lift1 = saturate((dot(s1, n) - min_h) / min_h);
                hc_plus = max(hc_plus, mix(low_plus, dot(s0 / l0, v), saturate(l0 * falloff_mul + falloff_add) * lift0));
                hc_minus = max(hc_minus, mix(low_minus, dot(s1 / l1, v), saturate(l1 * falloff_mul + falloff_add) * lift1));
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
    let ao = mix(1.0, max(visibility, FLOOR), fade * STRENGTH);
    return vec4<f32>(ao, 1.0, pc.z, 1.0);
}
";

/// The full-resolution step: the blurred occlusion times the sun's contact shadow multiply the
/// scene's colour (the blend: colour times source).
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
    var sun = 1.0;
    // The sun's contact shadow, here at full resolution and unblurred, so its edge is a clean
    // line: only on faces turned toward the sun (the game already darkens the others).
    if (p.sun.w > 0.0) {
        let pc = view_at(vec2<f32>(xy) + 0.5, z);
        let fade = fade_at(z);
        if (fade > 0.0) {
            let surface = surface_at(xy, pc);
            // Faces toward the sun, fully from 0.5 (60 degrees from the sun) up: a face lit
            // at a grazing angle is already dim in the game's lighting and only shadows itself.
            let facing = smoothstep(0.25, 0.5, dot(surface.n, p.sun.xyz));
            if (!surface.thin && facing > 0.0) {
                sun = 1.0 - p.sun.w * fade * facing * (1.0 - sun_visible(pc, surface.n));
            }
        }
    }
    // `NV_AO_VIEW` (captures only): one term alone, as grey, over the whole picture.
    if (VIEW == 1u) {
        return vec4<f32>(ao, ao, ao, 1.0);
    }
    if (VIEW == 2u) {
        return vec4<f32>(sun, sun, sun, 1.0);
    }
    let f = ao * sun;
    if (f >= 0.999) {
        discard;
    }
    return vec4<f32>(f, f, f, 1.0);
}
";

/// The WGSL of the half-resolution step (`apply` false) or the full-resolution one, for a
/// multisampled depth surface or not.
pub fn shader(apply: bool, multisampled_depth: bool) -> String {
    let depth_type = if multisampled_depth {
        "texture_depth_multisampled_2d"
    } else {
        "texture_depth_2d"
    };
    let constants = format!(
        "const RADIUS: f32 = {RADIUS:?};\nconst MAX_RADIUS_SCREEN: f32 = {MAX_RADIUS_SCREEN:?};\n\
         const SLICES: i32 = {SLICES};\nconst STEPS: i32 = {STEPS};\nconst STRENGTH: f32 = {STRENGTH:?};\n\
         const POWER: f32 = {POWER:?};\nconst FLOOR: f32 = {FLOOR:?};\nconst MIN_HEIGHT: f32 = {MIN_HEIGHT:?};\nconst SHADOW_LENGTH: f32 = {SHADOW_LENGTH:?};\n\
         const SHADOW_STEPS: i32 = {SHADOW_STEPS};\nconst SHADOW_THICKNESS: f32 = {SHADOW_THICKNESS:?};\n\
         const FADE_FROM: f32 = {FADE_FROM:?};\nconst FADE_TO: f32 = {FADE_TO:?};\n\
         const VIEW: u32 = {}u;\n",
        view()
    );
    let body = if apply { APPLY } else { AO };
    format!(
        "{constants}{}{body}",
        COMMON.replace("DEPTH_TYPE", depth_type)
    )
}

/// `NV_AO_VIEW=ao` or `sun` (diagnostic captures only, not a player setting): the scene replaced by
/// that one term as grey (white: untouched), read once; 0 otherwise.
pub fn view() -> u32 {
    static VIEW: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *VIEW.get_or_init(
        || match std::env::var("NV_AO_VIEW").as_deref().map(str::trim) {
            Ok("ao") => 1,
            Ok("sun") => 2,
            _ => 0,
        },
    )
}

/// The format of the half-resolution results.
const HALF_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The step's GPU objects, made at its first use.
pub(crate) struct Gpu {
    uniforms: wgpu::Buffer,
    ao_layouts: [Option<(wgpu::BindGroupLayout, wgpu::RenderPipeline)>; 2],
    /// The full-resolution step's layout and pipelines by (multisampled depth, colour format,
    /// samples).
    #[allow(clippy::type_complexity)]
    apply: Vec<(
        (bool, wgpu::TextureFormat, u32),
        wgpu::BindGroupLayout,
        wgpu::RenderPipeline,
    )>,
    half: Option<(u32, u32, wgpu::TextureView)>,
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

fn layout(device: &wgpu::Device, multisampled: bool, apply: bool) -> wgpu::BindGroupLayout {
    let mut entries = vec![
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled,
            },
            count: None,
        },
    ];
    if apply {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
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

impl Gpu {
    pub(crate) fn new(device: &wgpu::Device) -> Gpu {
        Gpu {
            uniforms: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("scene light"),
                size: 48,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            ao_layouts: [None, None],
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

    /// Records the two steps into `encoder`: the half-resolution results, then the scene's
    /// colour multiplied by them.
    pub(crate) fn encode(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        t: &Targets<'_>,
    ) {
        let ms = t.samples > 1;
        let (hw, hh) = (t.width.div_ceil(2), t.height.div_ceil(2));
        if !matches!(&self.half, Some((w, h, _)) if *w == hw && *h == hh) {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("scene light half"),
                size: wgpu::Extent3d {
                    width: hw,
                    height: hh,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: HALF_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.half = Some((hw, hh, tex.create_view(&Default::default())));
        }
        let half = self.half.as_ref().map(|(_, _, v)| v.clone()).expect("made");
        let k = usize::from(ms);
        if self.ao_layouts[k].is_none() {
            let l = layout(device, ms, false);
            let p = pipeline(
                device,
                &l,
                shader(false, ms),
                wgpu::ColorTargetState {
                    format: HALF_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                },
                1,
            );
            self.ao_layouts[k] = Some((l, p));
        }
        let key = (ms, t.colour_format, t.samples);
        if !self.apply.iter().any(|(k, _, _)| *k == key) {
            let l = layout(device, ms, true);
            let p = pipeline(
                device,
                &l,
                shader(true, ms),
                wgpu::ColorTargetState {
                    format: t.colour_format,
                    // The scene's colour times the source; its alpha kept (a term alone replaces it
                    // in `NV_AO_VIEW`).
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
        let (ao_layout, ao_pipeline) = self.ao_layouts[k].as_ref().expect("made");
        let ao_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: ao_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(t.depth),
                },
            ],
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene light: occlusion"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &half,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(ao_pipeline);
            pass.set_bind_group(0, &ao_group, &[]);
            pass.draw(0..3, 0..1);
        }
        let (_, apply_layout, apply_pipeline) =
            self.apply.iter().find(|(k, _, _)| *k == key).expect("made");
        let apply_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: apply_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(t.depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&half),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene light: apply"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: t.colour,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(apply_pipeline);
        pass.set_bind_group(0, &apply_group, &[]);
        pass.draw(0..3, 0..1);
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
    fn the_sun_march_walks_its_length_finer_near_the_point() {
        let d: Vec<f32> = (1..=SHADOW_STEPS).map(march_distance).collect();
        // The first step is a small part of the length; the last reaches it exactly.
        assert!(d[0] > 0.0 && d[0] < SHADOW_LENGTH / SHADOW_STEPS as f32);
        assert!((d[d.len() - 1] - SHADOW_LENGTH).abs() < 1e-3);
        // Each step longer than the one before.
        for w in d.windows(3) {
            assert!(w[1] > w[0] && w[2] - w[1] > w[1] - w[0]);
        }
        // A surface a little nearer the camera blocks; the surface itself, one far in front, or
        // one behind does not.
        assert!(blocks(10.0, 500.0, 480.0));
        // Far along the ray, a surface only a little nearer is the receiver's own facet.
        assert!(!blocks(200.0, 500.0, 495.0));
        assert!(blocks(10.0, 500.0, 495.0));
        assert!(!blocks(10.0, 500.0, 499.5));
        assert!(!blocks(10.0, 500.0, 500.0 - SHADOW_THICKNESS - 1.0));
        assert!(!blocks(10.0, 500.0, 520.0));
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
        // Indoors, at night (the sun below the horizon) or with a dark sun: no contact shadows.
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
        for apply in [false, true] {
            for ms in [false, true] {
                let src = shader(apply, ms);
                let module = naga::front::wgsl::parse_str(&src)
                    .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&src)));
                naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                )
                .validate(&module)
                .unwrap_or_else(|e| panic!("apply {apply} ms {ms}: {e:?}"));
            }
        }
    }
}
