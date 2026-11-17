//! Sun shadows (port-added, not in the original; off by default): cascaded shadow maps drawn
//! from the sun out of the game's own world draws, looked up inside the game's lit shaders. Turned
//! on by `[nv-rs Display] bSunShadows=1` in the run's `Fallout.ini` or `NV_SUN_SHADOWS=1`
//! (`host::fixes::Choice::sun_shadows`); wgpu only, the real Direct3D 9 ignores it. Design:
//! `measure/newlight/SUN-SHADOWS-DESIGN.md`.
//!
//! **What casts.** At the scene's colour and depth clear ([`BEGIN_STATE`], after the camera's
//! words [`VIEW_STATE`]) the device starts a light list of the frame's world draws ([`Caster`]): a
//! vertex shader whose constant table names `ModelViewProj` (and no skinning matrices), depth test
//! and depth writes on, no blending, no vertex textures, not a face. Alpha-tested draws are
//! cut-outs ([`Cutout`]: drawn through their own pixel shader with the game's alpha test), only in
//! the first [`CUTOUT_CASCADES`] maps. The list ends at the game's depth-only clear ([`STATE`]): the
//! first-person arms and weapon after it cast nothing. At the next frame's scene clear each
//! caster's place in the world comes back from its `ModelViewProj`, that frame's frustum (rows x,
//! y and w) and the camera's rotation and place from the game's memory ([`world_of`], [`Frame`]).
//! The casters are kept across frames ([`Cache`]): what the game drew, while it is around the
//! camera (so objects behind the camera still cast), let go when it is gone (its buffers written or
//! released, or wholly in view and not drawn) or too far; the sky, clouds and far land never cast
//! ([`casts`]). Motion is decided place by place ([`Cache::mark_moves`], [`mover`]): a place that
//! came near a place of the same geometry drawn the frame before and not now moves, and is drawn
//! each frame over a copy of the still maps until it has stood still for [`SETTLE`] frames; copies
//! of one geometry (fence posts, poles, rocks) leaving and entering the view are no move. A still
//! map is drawn again only when it moved, the sun turned, or a still caster in it came, went or
//! moved; then each caster that touches it
//! ([`Cascade::touches`]) is drawn again, depth only, through its own converted vertex shader, with
//! `ModelViewProj`'s four registers replaced by the map's view of its place
//! ([`Cascade::world_to_light`]).
//!
//! **Cables** cast at their real weight. A geometry all of whose pieces ([`strands`]) are round
//! cables thinner than [`THIN_WIDTH`] or flat ribbons ([`Strand::ribbon`]: the game draws its
//! telegraph lines as alpha-tested strips 20 units wide with the cable in the texture) is drawn
//! not into the depth maps but into a coverage map a cascade ([`THIN_SIZE`], [`THIN_FORMAT`]): a
//! round cable as quads along its centreline at least [`THIN_DRAWN`] texels wide, carrying its
//! share of each texel ([`thin_quads`]); a ribbon through its own pixel shader with its texture's
//! alpha read at the level the map's texel calls for (the cable's share of the texel), added up,
//! over quads that carry its depth. The lookup takes away the sun by that share, spread over the
//! band the sun's own half-degree width makes at the cable's height above the receiver
//! ([`SUN_WIDTH`]): a faint soft line that fades the higher the cable hangs, never a solid
//! one-texel line or a broken one. A geometry that is cables and more (Goodsprings' telegraph
//! line: poles, crossbars and wires in one) has its round cables taken out ([`Body::solid`]): the
//! rest is drawn into the depth maps from an index list of its own. `NV_SUN_SHADOWS_THIN=solid`
//! (captures only) casts cables as any other caster.
//!
//! **The maps.** [`CASCADES`] cascades of [`SIZE`] texels a side (one texture array, a layer each)
//! split the camera's view from its near plane to [`FAR`] units by the practical scheme ([`splits`]:
//! Zhang et al., "Parallel-Split Shadow Maps", 2006, log/linear blend [`LAMBDA`]). Each covers the
//! bounding sphere of its slice (its size does not change as the camera turns) plus a margin, and
//! its centre moves in whole steps of [`GRID_TEXELS`] texels in the sun's space ([`place`]): edges
//! do not crawl as the camera moves (Valient, "Stable Rendering of Cascaded Shadow Maps",
//! ShaderX6), and the map stays valid until the centre crosses a step. Each map keeps the sun it
//! was drawn for ([`SunKeep`]) until the sun has turned far enough to move a shadow edge in it by
//! half its texel ([`turn_limit`]); one map a frame takes the new sun ([`sun_turns`]), so the
//! shadows glide with the sun instead of jumping.
//!
//! **How it shows.** In the game's own lighting: each lit shader gets a variant (`sm::wgsl`'s
//! `convert_with_sun`) in which every read of the sun's light colour (the pixel shader's
//! `PSLightColor`, or a grass vertex shader's `DiffuseColor`; one stage only) is multiplied by the
//! sun's visibility there ([`LOOKUP`], [`LOOKUP_VERTEX`]): soft-edged comparisons in the map of the
//! point's cascade, blended at the seams, faded at the far end. Shade keeps the ambient, lamps and
//! glow; the sky and what is drawn after the depth-only clear (the first-person arms and weapon,
//! drawn with their own camera; the image-space step, HUD and menus) are drawn as without the
//! shadows. A variant is made on a thread of its own; until it is there the plain pipeline draws.
//! `NV_SUN_SHADOWS=debug` instead lays a full-screen overlay over the scene at the depth-only
//! clear (an engineers' tool, never the shadows players see).
//!
//! **Off.** Nothing is recorded, read or made; the depth surfaces and pipelines are made as before
//! ([`enabled`] is read when the device is made), so the frame is what it was (GPU-tested).
//!
//! **Limits.** Characters (skinned) cast no shadow of ours yet (the game's own character shadows
//! stay); leaves, fronds and distant trees are not shadowed yet; objects the game has not drawn
//! since they came near (out of sight from the start) cast none until it does.

use std::sync::atomic::{AtomicU8, Ordering};

use crate::scene_light;

/// The port-private `SetTransform` state (FourCC `NVSS`) that carries [`Frame`] in its 16 words,
/// recorded just after [`scene_light::STATE`] (whose words carry the frustum) before the game's
/// depth-only clear. No Direct3D 9 transform state has this number.
pub const STATE: u32 = u32::from_le_bytes(*b"NVSS");

/// The port-private `SetTransform` states recorded at a colour and depth clear (the scene's, before
/// the world is drawn) while the sun shadows are on: the camera's words ([`scene_light::Params`],
/// `NVSV`), then the frame's ([`Frame`], `NVSB`), from which the device writes the lookup the
/// lighting variants read this frame. No Direct3D 9 transform state has these numbers.
pub const VIEW_STATE: u32 = u32::from_le_bytes(*b"NVSV");
pub const BEGIN_STATE: u32 = u32::from_le_bytes(*b"NVSB");

/// Cascades and their sizes (texels a side): nearest first.
pub const CASCADES: usize = 4;
pub const SIZE: u32 = 2048;
/// How far the shadows reach (game units from the camera; about 143 m).
pub const FAR: f32 = 10000.0;
/// The practical split scheme's blend of logarithmic (1) and even (0) splits.
pub const LAMBDA: f32 = 0.75;
/// A cascade's centre moves in steps of this many texels.
pub const GRID_TEXELS: f64 = 64.0;
/// Half the depth range of every cascade (game units toward and away from the sun).
pub const DEPTH_RANGE: f64 = 16000.0;
/// The sun this low over the horizon (the up part of the unit vector toward it) casts no shadow
/// (shadows longer than about 33 times the caster's height).
pub const MIN_SUN_HEIGHT: f32 = 0.03;
/// How much of a sunlit, sun-facing pixel's colour a shadow takes away under a full sun.
pub const DARKNESS: f32 = 0.6;
/// The fade at the far end: the last part of the last cascade.
pub const FADE: f32 = 0.15;
/// The receiver is moved this many of its cascade's texels along its normal before the lookup
/// (normal offset), and the comparison allows this many texels of depth (constant bias).
pub const NORMAL_OFFSET: f32 = 1.5;
pub const DEPTH_BIAS: f32 = 1.0;
/// The depth pipeline's slope-scaled bias.
pub const SLOPE_BIAS: f32 = 2.0;
/// The most the depth allowance grows with a face's slope to the sun, in texels.
pub const SLOPE_MAX: f32 = 2.0;
/// The band where a cascade blends into the next, as a part of its depth range.
pub const BLEND: f32 = 0.1;
/// A caster wholly in view that the game has not drawn for this many frames is let go (it is gone);
/// one that misses a frame or two (the game's own fades, its culling's edges) stays in the maps.
pub const GRACE: u64 = 8;

static MODE: AtomicU8 = AtomicU8::new(0);

/// Turns the sun shadows on or off for devices made from now on (the host, from the run's choice).
pub fn enable(on: bool) {
    MODE.store(u8::from(on), Ordering::Relaxed);
}

/// Turns them on with the debug step for devices made from now on (tests; a run asks by
/// `NV_SUN_SHADOWS=debug`, [`debug_step`]).
pub fn enable_debug() {
    MODE.store(2, Ordering::Relaxed);
}

/// Whether devices made now draw sun shadows.
pub fn enabled() -> bool {
    MODE.load(Ordering::Relaxed) != 0
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(a: [f64; 3]) -> [f64; 3] {
    let l = dot3(a, a).sqrt();
    a.map(|v| v / l)
}

fn wide(a: [f32; 3]) -> [f64; 3] {
    a.map(f64::from)
}

/// A 4x4 matrix, row by row (a point is a column: `out[i] = row i . (x, y, z, 1)`).
pub type M4 = [[f64; 4]; 4];

pub fn mul(a: &M4, b: &M4) -> M4 {
    let mut o = [[0.0; 4]; 4];
    for (i, row) in o.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    o
}

pub fn apply(m: &M4, p: [f64; 3]) -> [f64; 4] {
    std::array::from_fn(|i| m[i][0] * p[0] + m[i][1] * p[1] + m[i][2] * p[2] + m[i][3])
}

/// One frame's camera and sun beside [`scene_light::Params`], as the host reads them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// The camera's world rotation, row by row (Gamebryo: column 0 the view direction, column 1
    /// up, column 2 right).
    pub rotate: [f32; 9],
    /// The camera's place in the world.
    pub position: [f32; 3],
    /// Toward the sun in the world (z up), unit length.
    pub to_sun: [f32; 3],
    /// The sun's strength ([`scene_light::Params::shadow`]), above 0.
    pub strength: f32,
}

impl Frame {
    /// The frame's camera and sun; none when there is no sun to cast shadows (indoors, night, a
    /// dark sun, the sun too low) or the camera is not usable.
    pub fn from_game(g: &scene_light::GameView) -> Option<Frame> {
        let view = scene_light::Params::from_game(g)?;
        let to_sun = g.to_sun?;
        let usable = g.rotate.iter().chain(&g.position).all(|v| v.is_finite());
        let len = (to_sun[0] * to_sun[0] + to_sun[1] * to_sun[1] + to_sun[2] * to_sun[2]).sqrt();
        if !usable || view.shadow <= 0.0 || !len.is_finite() || len <= 1e-6 {
            return None;
        }
        let to_sun = to_sun.map(|v| v / len);
        if to_sun[2] < MIN_SUN_HEIGHT {
            return None;
        }
        Some(Frame {
            rotate: g.rotate,
            position: g.position,
            to_sun,
            strength: view.shadow,
        })
    }

    /// The 16 words of the `SetTransform` matrix ([`STATE`]).
    pub fn to_words(&self) -> [u32; 16] {
        let mut f = [0.0f32; 16];
        f[..9].copy_from_slice(&self.rotate);
        f[9..12].copy_from_slice(&self.position);
        f[12..15].copy_from_slice(&self.to_sun);
        f[15] = self.strength;
        f.map(f32::to_bits)
    }

    /// [`Frame::to_words`] read back; none when they are not usable.
    pub fn from_words(w: &[u32]) -> Option<Frame> {
        if w.len() != 16 {
            return None;
        }
        let f: Vec<f32> = w.iter().map(|&x| f32::from_bits(x)).collect();
        let frame = Frame {
            rotate: std::array::from_fn(|i| f[i]),
            position: [f[9], f[10], f[11]],
            to_sun: [f[12], f[13], f[14]],
            strength: f[15],
        };
        let len = frame.to_sun.iter().map(|v| v * v).sum::<f32>().sqrt();
        let usable = f.iter().all(|v| v.is_finite())
            && (len - 1.0).abs() < 1e-3
            && frame.to_sun[2] >= MIN_SUN_HEIGHT
            && frame.strength > 0.0
            && frame.strength <= 1.0;
        usable.then_some(frame)
    }

    /// The camera's axes in the world: right, up, ahead (view x, y, z).
    pub fn axes(&self) -> [[f64; 3]; 3] {
        let col = |i: usize| {
            [
                f64::from(self.rotate[i]),
                f64::from(self.rotate[3 + i]),
                f64::from(self.rotate[6 + i]),
            ]
        };
        [col(2), col(1), col(0)]
    }
}

/// The view depths that split the cascades, from `near` to `far` (the practical scheme: each split
/// a blend by [`LAMBDA`] of the logarithmic and the even split).
pub fn splits(near: f32, far: f32) -> [f32; CASCADES + 1] {
    let mut s = [near; CASCADES + 1];
    for (i, v) in s.iter_mut().enumerate().skip(1) {
        let t = i as f32 / CASCADES as f32;
        let log = near * (far / near).powf(t);
        let even = near + (far - near) * t;
        *v = LAMBDA * log + (1.0 - LAMBDA) * even;
    }
    s[CASCADES] = far;
    s
}

/// The sphere around the camera's view between depths `zn` and `zf` (`frustum`: left, right,
/// bottom, top at unit distance): its centre in view space and its radius. The centre is on the
/// slice's own axis, where the sphere through the slice's far corners and near corners is least;
/// it depends on the frustum alone, so it does not change as the camera turns.
pub fn slice_sphere(frustum: [f32; 4], zn: f32, zf: f32) -> ([f64; 3], f64) {
    let [l, r, b, t] = frustum.map(f64::from);
    let (zn, zf) = (f64::from(zn), f64::from(zf));
    // The slice's axis: through the middle of the frustum at each depth.
    let (mx, my) = ((l + r) / 2.0, (b + t) / 2.0);
    // Half diagonal at unit distance (the corner farthest from the axis).
    let hx = (r - l) / 2.0;
    let hy = (t - b) / 2.0;
    let h2 = hx * hx + hy * hy;
    // The centre at depth c on the axis is as far from the near corners as from the far ones:
    // (c - zn)^2 (1 + m2) + zn^2 h2 = (zf - c)^2 (1 + m2) + zf^2 h2, measured along the axis.
    let m2 = mx * mx + my * my;
    let k = 1.0 + m2;
    let mut c = (zf * zf * (k + h2) - zn * zn * (k + h2)) / (2.0 * k * (zf - zn));
    // Past the far end, the far corners alone bound it.
    c = c.clamp(zn, zf);
    let centre = [mx * c, my * c, c];
    let mut radius = 0.0f64;
    for z in [zn, zf] {
        for x in [l, r] {
            for y in [b, t] {
                let d = [x * z - centre[0], y * z - centre[1], z - centre[2]];
                radius = radius.max(dot3(d, d).sqrt());
            }
        }
    }
    (centre, radius)
}

/// The sun's basis: x and y across the map, z from the sun toward the ground. Its x is level with
/// the world's north (or east, for a sun nearly due north or south), so the maps do not turn as
/// the camera does. Left-handed, as the camera's view (x right, y up, z ahead: x cross y = -z), so a
/// face's winding seen from the sun is as seen from the camera and the draw's own culling holds.
pub fn basis(to_sun: [f64; 3]) -> [[f64; 3]; 3] {
    let z = normalize(to_sun.map(|v| -v));
    let reference = if z[1].abs() > 0.95 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let x = normalize(cross(reference, z));
    let y = cross(x, z);
    [x, y, z]
}

/// One cascade as drawn: its slice of the view, its texel and step in game units, its centre in
/// whole steps of the sun's space (what decides whether the map drawn last is still good) and
/// the matrix from the camera's view space to the map's clip space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cascade {
    pub near: f32,
    pub far: f32,
    pub size: u32,
    pub radius: f64,
    pub texel: f64,
    pub step: f64,
    pub origin: [i64; 3],
    pub view_to_light: M4,
    /// The sun's basis it was placed in ([`basis`]) and its half width in game units.
    pub basis: [[f64; 3]; 3],
    pub half: f64,
}

/// An object's place in the world: its object space to the world, rows x, y, z (the point a
/// column, the last column the translation).
pub type World = [[f64; 4]; 3];

impl Cascade {
    /// The matrix from the view space of a camera with `axes` (right, up, ahead) at `position` to
    /// this map's clip space: the camera's axes into the world (relative to the camera), the
    /// sun's basis, less the snapped centre (relative to the camera), scaled into clip space. A map
    /// kept from an earlier frame is looked up through this frame's camera so.
    #[allow(clippy::needless_range_loop)]
    pub fn view_to_light_for(&self, axes: [[f64; 3]; 3], position: [f64; 3]) -> M4 {
        let mut m = [[0.0; 4]; 4];
        for k in 0..3 {
            let offset = dot3(self.basis[k], position) - self.origin[k] as f64 * self.step;
            let (scale, add) = if k < 2 {
                (1.0 / self.half, 0.0)
            } else {
                (0.5 / DEPTH_RANGE, 0.5)
            };
            for (v, axis) in axes.iter().enumerate() {
                m[k][v] = dot3(self.basis[k], *axis) * scale;
            }
            m[k][3] = offset * scale + add;
        }
        m[3] = [0.0, 0.0, 0.0, 1.0];
        m
    }

    /// Whether a sphere in the world (`centre`, `radius`) can cast into this map: it overlaps the
    /// map's square across the sun's rays (at any depth along them).
    pub fn touches(&self, centre: [f64; 3], radius: f64) -> bool {
        (0..2).all(|k| {
            (dot3(self.basis[k], centre) - self.origin[k] as f64 * self.step).abs()
                <= self.half + radius
        })
    }

    /// The matrix that takes an object placed at `w` in the world into this map's clip space (rows
    /// x, y, depth, w), for its `ModelViewProj` registers.
    #[allow(clippy::needless_range_loop)]
    pub fn world_to_light(&self, w: &World) -> [[f32; 4]; 4] {
        let mut m = [[0.0f32; 4]; 4];
        for k in 0..3 {
            let (scale, add) = if k < 2 {
                (1.0 / self.half, 0.0)
            } else {
                (0.5 / DEPTH_RANGE, 0.5)
            };
            let b = self.basis[k];
            for j in 0..4 {
                let v = b[0] * w[0][j] + b[1] * w[1][j] + b[2] * w[2][j];
                let v = if j == 3 {
                    (v - self.origin[k] as f64 * self.step) * scale + add
                } else {
                    v * scale
                };
                m[k][j] = v as f32;
            }
        }
        m[3] = [0.0, 0.0, 0.0, 1.0];
        m
    }
}

/// A radius rounded up to the next power of 1.05, so a frustum that wobbles in the last digits
/// keeps its maps.
fn round_radius(r: f64) -> f64 {
    let step = 1.05f64.ln();
    ((r.ln() / step).ceil() * step).exp()
}

/// Cascade `i` for the camera (`frustum` left, right, bottom, top; `axes` right, up, ahead in the
/// world; `position` in the world) and the sun's basis `b`, between view depths `zn` and `zf`.
pub fn place(
    i: usize,
    frustum: [f32; 4],
    zn: f32,
    zf: f32,
    axes: [[f64; 3]; 3],
    position: [f64; 3],
    b: [[f64; 3]; 3],
) -> Cascade {
    let _ = i;
    let size = SIZE;
    let (centre_view, radius) = slice_sphere(frustum, zn, zf);
    let radius = round_radius(radius);
    // texel = 2 * half / size, half = radius + step, step = GRID_TEXELS * texel.
    let texel = 2.0 * radius / (f64::from(size) - 2.0 * GRID_TEXELS);
    let step = GRID_TEXELS * texel;
    let half = radius + step;
    // The centre in the world, then in the sun's space, then in whole steps.
    let rel: [f64; 3] = std::array::from_fn(|k| {
        axes[0][k] * centre_view[0] + axes[1][k] * centre_view[1] + axes[2][k] * centre_view[2]
    });
    let world: [f64; 3] = std::array::from_fn(|k| position[k] + rel[k]);
    let origin: [i64; 3] = std::array::from_fn(|k| (dot3(b[k], world) / step).round() as i64);
    let mut c = Cascade {
        near: zn,
        far: zf,
        size,
        radius,
        texel,
        step,
        origin,
        view_to_light: [[0.0; 4]; 4],
        basis: b,
        half,
    };
    c.view_to_light = c.view_to_light_for(axes, position);
    c
}

/// Where a caster drawn with `mvp` (its `ModelViewProj` registers: rows of clip x, y, z, w) by a
/// camera of `frustum` (left, right, bottom, top), `axes` (right, up, ahead) and `position` is in
/// the world. The object's view space comes from rows x, y and w alone (`x_v = (r - l) / 2 x +
/// (r + l) / 2 w`, the same for y, `z_v = w`): no inverse of the projection, no near or far plane,
/// nothing from the depth row's rounding; then the camera's axes and place.
pub fn world_of(
    mvp: &[[f32; 4]; 4],
    frustum: [f32; 4],
    axes: [[f64; 3]; 3],
    position: [f64; 3],
) -> World {
    let [l, r, b, t] = frustum.map(f64::from);
    let row = |i: usize| mvp[i].map(f64::from);
    let (x, y, w) = (row(0), row(1), row(3));
    let view: [[f64; 4]; 3] = [
        std::array::from_fn(|k| (r - l) / 2.0 * x[k] + (r + l) / 2.0 * w[k]),
        std::array::from_fn(|k| (t - b) / 2.0 * y[k] + (t + b) / 2.0 * w[k]),
        w,
    ];
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let v = axes[0][i] * view[0][j] + axes[1][i] * view[1][j] + axes[2][i] * view[2][j];
            if j == 3 {
                v + position[i]
            } else {
                v
            }
        })
    })
}

/// A point of object space placed by `w`.
pub fn place_point(w: &World, p: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| w[i][0] * p[0] + w[i][1] * p[1] + w[i][2] * p[2] + w[i][3])
}

/// The largest stretch of `w` (the longest of its three axes), for a bounding sphere's radius.
pub fn stretch(w: &World) -> f64 {
    (0..3)
        .map(|j| (0..3).map(|i| w[i][j] * w[i][j]).sum::<f64>().sqrt())
        .fold(0.0, f64::max)
}

/// Two places of one object that are the same (the camera's rounding between frames aside: a
/// hundredth of a unit, a ten-thousandth of the scale).
pub fn same_place(a: &World, b: &World) -> bool {
    let s = stretch(a).max(1.0);
    (0..3).all(|i| {
        (0..3).all(|j| (a[i][j] - b[i][j]).abs() <= 1e-4 * s) && (a[i][3] - b[i][3]).abs() <= 0.05
    })
}

/// A caster larger than this (game units of radius) casts nothing: the sky's dome, the far land's
/// blocks.
pub const MAX_CASTER_RADIUS: f64 = 12000.0;
/// A caster around the camera larger than this casts nothing either (the sky's clouds).
pub const MAX_AROUND_CAMERA: f64 = 2000.0;

/// Whether an object placed by `w` with object-space bounds `local` casts: not the sky's dome or
/// clouds (spheres round the camera at `camera`) and not larger than the maps reach.
pub fn casts(w: &World, local: ([f64; 3], f64), camera: [f64; 3]) -> bool {
    let c = place_point(w, local.0);
    let r = local.1 * stretch(w);
    let d = [c[0] - camera[0], c[1] - camera[1], c[2] - camera[2]];
    let around = dot3(d, d).sqrt() < r;
    r <= MAX_CASTER_RADIUS && !(around && r > MAX_AROUND_CAMERA)
}

/// The sphere around object-space points (the middle of their box and the half diagonal); none
/// without points or with points that are not numbers.
pub fn bounds(points: impl Iterator<Item = [f32; 3]>) -> Option<([f64; 3], f64)> {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    let mut any = false;
    for p in points {
        if !p.iter().all(|v| v.is_finite()) {
            return None;
        }
        for k in 0..3 {
            lo[k] = lo[k].min(f64::from(p[k]));
            hi[k] = hi[k].max(f64::from(p[k]));
        }
        any = true;
    }
    if !any {
        return None;
    }
    let centre = std::array::from_fn(|k| (lo[k] + hi[k]) / 2.0);
    let d: [f64; 3] = std::array::from_fn(|k| (hi[k] - lo[k]) / 2.0);
    Some((centre, dot3(d, d).sqrt()))
}

/// A strand narrower than this (game units) is thin: a cable or a wire. The sun is half a degree
/// wide, so a few hundred units under a cable its shadow's soft edge is wider than the cable itself
/// (about 0.009 x the distance): the real shadow is a faint smear, while a depth map draws it as a
/// solid band (or, under a texel wide, as a broken line that crawls). Thin casters are drawn into
/// their own coverage maps instead ([`Thin`]), at their real weight.
pub const THIN_WIDTH: f64 = 7.0;
/// ... and longer than this many times its width (a peg or a bolt is no cable).
pub const THIN_LENGTH: f64 = 20.0;
/// A strand's centreline has a point about this many game units apart (at most [`STRAND_POINTS`]).
pub const STRAND_STEP: f64 = 48.0;
pub const STRAND_POINTS: usize = 33;
/// A geometry is split into pieces only when its area over its longest side is under this many
/// round cables of [`THIN_WIDTH`] side by side ([`Body::of`]).
pub const MANY_CABLES: f64 = 16.0;

/// One connected piece of a caster's surface seen as a strand: its centreline end to end (object
/// space), its width (twice the mean distance of its corners from the centreline: a round cable's
/// diameter, a flat strip's width), its length and its area.
#[derive(Clone, Debug, PartialEq)]
pub struct Strand {
    pub line: Vec<[f32; 3]>,
    pub width: f64,
    pub length: f64,
    pub area: f64,
    /// Its number among the pieces ([`pieces`]).
    pub piece: u32,
}

impl Strand {
    /// Whether it is a cable ([`THIN_WIDTH`], [`THIN_LENGTH`]).
    pub fn thin(&self) -> bool {
        self.width < THIN_WIDTH && self.length > THIN_LENGTH * self.width
    }

    /// Whether it is a ribbon: a flat strip (its area under [`FLAT`] times its length by its width;
    /// a round tube's is about pi) narrower than [`RIBBON_WIDTH`] and longer than [`RIBBON_LENGTH`]
    /// times its width: what the game draws a cable as, with the cable in its texture's alpha.
    pub fn ribbon(&self) -> bool {
        self.width < RIBBON_WIDTH
            && self.length > RIBBON_LENGTH * self.width
            && self.area < FLAT * self.length * self.width
    }
}

/// A ribbon ([`Strand::ribbon`]) is narrower than this (game units) ...
pub const RIBBON_WIDTH: f64 = 128.0;
/// ... longer than this many times its width ...
pub const RIBBON_LENGTH: f64 = 8.0;
/// ... and flat: its area under this many times its length by its width (one side 1, both 2, a
/// round tube about 3).
pub const FLAT: f64 = 2.5;
/// A geometry of cables may hold small other pieces (insulators, clamps, a pole top's bolts) of
/// at most this part of its area; they cast with it as cables do.
pub const BITS: f64 = 0.1;

/// The caster's triangles (object-space corners) split into connected pieces (corners welded by
/// place, so a seam in the vertex list does not split a cable; degenerate triangles, which join
/// a strip's runs, join nothing), each as a [`Strand`]. Its centreline follows the piece along its
/// surface: the corners' distances over the surface from one end (the corner furthest from any
/// other), in [`STRAND_STEP`] steps, each step's corners' middle a point (a sagging cable, or one
/// that turns at a junction, kept); a piece that branches has steps whose corners lie far from
/// their middle, so its width comes out wide.
pub fn strands(triangles: &[[[f32; 3]; 3]]) -> Vec<Strand> {
    pieces(triangles).0
}

/// [`strands`], with each triangle's piece ([`Strand::piece`]; `u32::MAX` for one left out: degenerate or
/// not a number).
pub fn pieces(triangles: &[[[f32; 3]; 3]]) -> (Vec<Strand>, Vec<u32>) {
    let mut ids = crate::fxhash::Map::<[i64; 3], u32>::default();
    let mut at: Vec<[f64; 3]> = Vec::new();
    let mut parent: Vec<u32> = Vec::new();
    fn root(parent: &mut [u32], mut i: u32) -> u32 {
        while parent[i as usize] != i {
            let up = parent[parent[i as usize] as usize];
            parent[i as usize] = up;
            i = up;
        }
        i
    }
    let area_of = |t: &[[f64; 3]; 3]| {
        let e1 = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
        let e2 = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
        let n = cross(e1, e2);
        0.5 * dot3(n, n).sqrt()
    };
    // Each triangle kept: its corners' places and their welded numbers, its area.
    let mut kept: Vec<([u32; 3], f64, usize)> = Vec::with_capacity(triangles.len());
    for (input, t) in triangles.iter().enumerate() {
        if !t.iter().flatten().all(|v| v.is_finite()) {
            continue;
        }
        let p = t.map(wide);
        let area = area_of(&p);
        if area <= 1e-9 {
            continue;
        }
        let corner: [u32; 3] = p.map(|c| {
            let k = c.map(|v| (v * 16.0).round() as i64);
            *ids.entry(k).or_insert_with(|| {
                parent.push(parent.len() as u32);
                at.push(c);
                (parent.len() - 1) as u32
            })
        });
        let (a, b) = (root(&mut parent, corner[0]), root(&mut parent, corner[1]));
        parent[b as usize] = a;
        let c = root(&mut parent, corner[2]);
        parent[c as usize] = a;
        kept.push((corner, area, input));
    }
    let mut pieces = crate::fxhash::Map::<u32, Vec<usize>>::default();
    for (n, k) in kept.iter().enumerate() {
        let r = root(&mut parent, k.0[0]);
        pieces.entry(r).or_default().push(n);
    }
    let dist3 = |a: [f64; 3], b: [f64; 3]| {
        let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        dot3(d, d).sqrt()
    };
    let mut of = vec![u32::MAX; triangles.len()];
    let mut out: Vec<Strand> = pieces
        .into_values()
        .enumerate()
        .map(|(piece, tris)| {
            for &n in &tris {
                of[kept[n].2] = piece as u32;
            }
            let area: f64 = tris.iter().map(|&n| kept[n].1).sum();
            // The piece's corners (numbered 0.. here) and its edges.
            let mut local = crate::fxhash::Map::<u32, usize>::default();
            let mut vert: Vec<[f64; 3]> = Vec::new();
            let mut edges: Vec<Vec<(usize, f64)>> = Vec::new();
            for &n in &tris {
                let c = kept[n].0.map(|id| {
                    *local.entry(id).or_insert_with(|| {
                        vert.push(at[id as usize]);
                        edges.push(Vec::new());
                        vert.len() - 1
                    })
                });
                for (i, j) in [(0, 1), (1, 2), (2, 0)] {
                    let l = dist3(vert[c[i]], vert[c[j]]);
                    edges[c[i]].push((c[j], l));
                    edges[c[j]].push((c[i], l));
                }
            }
            // Distances over the surface from `from` (Dijkstra).
            let over = |from: usize| -> Vec<f64> {
                let mut d = vec![f64::INFINITY; vert.len()];
                let mut heap = std::collections::BinaryHeap::new();
                d[from] = 0.0;
                heap.push((std::cmp::Reverse(0u64), from));
                while let Some((std::cmp::Reverse(k), v)) = heap.pop() {
                    let dv = f64::from_bits(k);
                    if dv > d[v] {
                        continue;
                    }
                    for &(w, l) in &edges[v] {
                        let nd = dv + l;
                        if nd < d[w] {
                            d[w] = nd;
                            // Distances are not negative: their bits order as they do.
                            heap.push((std::cmp::Reverse(nd.to_bits()), w));
                        }
                    }
                }
                d
            };
            let furthest = |d: &[f64]| {
                d.iter()
                    .enumerate()
                    .fold((0, -1.0), |b, (i, &x)| if x > b.1 { (i, x) } else { b })
            };
            let start = furthest(&over(0)).0;
            let d = over(start);
            let length = furthest(&d).1.max(0.0);
            let n = ((length / STRAND_STEP).ceil() as usize).clamp(1, STRAND_POINTS - 2);
            let step = |i: usize| ((d[i] / length.max(1e-9) * n as f64) as usize).min(n - 1);
            let mut sum = vec![([0.0f64; 3], 0.0f64); n];
            for (i, p) in vert.iter().enumerate() {
                let s = &mut sum[step(i)];
                for (acc, v) in s.0.iter_mut().zip(p) {
                    *acc += v;
                }
                s.1 += 1.0;
            }
            // The steps' middles in order, and where each step's is among them.
            let mut index = vec![0usize; n];
            let mut inner: Vec<[f64; 3]> = Vec::with_capacity(n);
            for (b, (c, w)) in sum.iter().enumerate() {
                if *w > 0.0 {
                    inner.push(c.map(|v| v / w));
                }
                index[b] = inner.len().saturating_sub(1);
            }
            // The line: the middles, carried on at each end as far as the end step's corners
            // reach past its middle.
            let last = inner.len() - 1;
            let out_from = |end: usize, q: usize| {
                let p = inner[end];
                let w = [p[0] - inner[q][0], p[1] - inner[q][1], p[2] - inner[q][2]];
                let l = dot3(w, w).sqrt();
                if l.is_nan() || l <= 1e-9 {
                    return p;
                }
                let w = w.map(|v| v / l);
                let reach = vert
                    .iter()
                    .enumerate()
                    .filter(|&(k, _)| index[step(k)] == end)
                    .map(|(_, c)| dot3([c[0] - p[0], c[1] - p[1], c[2] - p[2]], w))
                    .fold(0.0, f64::max);
                [p[0] + w[0] * reach, p[1] + w[1] * reach, p[2] + w[2] * reach]
            };
            let mut line = Vec::with_capacity(inner.len() + 2);
            line.push(if last > 0 { out_from(0, 1) } else { inner[0] });
            line.extend(inner.iter().copied());
            line.push(if last > 0 {
                out_from(last, last - 1)
            } else {
                inner[0]
            });
            // Each corner's distance from the line (the segments on either side of its own
            // step's middle, line[i + 1]).
            let to_segment = |p: [f64; 3], a: [f64; 3], b: [f64; 3]| {
                let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
                let t = (dot3(ap, ab) / dot3(ab, ab).max(1e-12)).clamp(0.0, 1.0);
                let e = [ap[0] - ab[0] * t, ap[1] - ab[1] * t, ap[2] - ab[2] * t];
                dot3(e, e).sqrt()
            };
            let mut across = 0.0;
            for (k, p) in vert.iter().enumerate() {
                let i = index[step(k)];
                across += (i..(i + 2).min(line.len() - 1))
                    .map(|j| to_segment(*p, line[j], line[j + 1]))
                    .fold(f64::INFINITY, f64::min);
            }
            let length: f64 = line.windows(2).map(|w| dist3(w[0], w[1])).sum();
            // Twice the corners' mean distance from the line; at least the area over the length
            // over pi (a round cable's diameter; a coarse piece whose few corners all lie on its
            // own line is not taken for a thin one).
            let width = (2.0 * across / vert.len() as f64)
                .max(area / length.max(1e-9) / std::f64::consts::PI);
            Strand {
                line: line.into_iter().map(|p| p.map(|v| v as f32)).collect(),
                width,
                length,
                area,
                piece: piece as u32,
            }
        })
        .collect();
    // The largest first (the order of a map's values is not stable).
    out.sort_by(|x, y| y.area.total_cmp(&x.area));
    (out, of)
}

/// What the maps keep of a geometry: its object-space bounding sphere, its cables when it is
/// cables ([`thin_shape`]: round cables, or ribbons with the cable in their alpha), and for the
/// dump tool its pieces' count and the largest few's width and length.
#[derive(Clone, Debug)]
pub(crate) struct Body {
    pub bounds: ([f64; 3], f64),
    pub thin: Option<Thin>,
    /// With cables taken out of a geometry that is not all cables: its other triangles (their
    /// numbers in its draw), drawn into the depth maps instead of the whole.
    pub solid: Option<std::sync::Arc<Vec<u32>>>,
    /// ... and those triangles' corners' numbers in the draw (its indices as read, or its vertices),
    /// three a triangle, as the device found them.
    pub solid_indices: Option<std::sync::Arc<Vec<u32>>>,
    pub pieces: usize,
    pub look: Vec<(f32, f32)>,
    /// Its pieces by kind: round cables, ribbons, others (how many, their part of its area).
    pub kinds: [(usize, f32); 3],
    /// Its triangles, kept only for the dump tool (`NV_SUN_SHADOWS_DUMP_DIR`).
    pub triangles: Option<std::sync::Arc<Vec<[[f32; 3]; 3]>>>,
}

impl Body {
    /// A geometry's body from its triangles (object-space corners); none without any.
    pub fn of(triangles: &[[[f32; 3]; 3]]) -> Option<Body> {
        let bounds = bounds(triangles.iter().flatten().copied())?;
        // Cheap first: a geometry whose area over its longest side is wider than many cables side
        // by side (a wall, a rock, a house) is not split into pieces.
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        let mut area = 0.0;
        for t in triangles {
            let p = t.map(wide);
            for c in &p {
                for k in 0..3 {
                    lo[k] = lo[k].min(c[k]);
                    hi[k] = hi[k].max(c[k]);
                }
            }
            let e1 = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
            let e2 = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
            let n = cross(e1, e2);
            area += 0.5 * dot3(n, n).sqrt();
        }
        let long = (0..3).map(|k| hi[k] - lo[k]).fold(0.0, f64::max);
        let girth = area / long.max(1e-6);
        if girth.is_nan() || girth >= MANY_CABLES * THIN_WIDTH * std::f64::consts::PI {
            return Some(Body {
                bounds,
                thin: None,
                solid: None,
                solid_indices: None,
                pieces: 0,
                look: Vec::new(),
                kinds: [(0, 0.0); 3],
                triangles: dump_dir().map(|_| std::sync::Arc::new(triangles.to_vec())),
            });
        }
        let (s, of) = pieces(triangles);
        let look = s
            .iter()
            .take(4)
            .map(|p| (p.width as f32, p.length as f32))
            .collect();
        let pieces = s.len();
        let total: f64 = s.iter().map(|p| p.area).sum::<f64>().max(1e-12);
        let mut kinds = [(0usize, 0.0f32); 3];
        for p in &s {
            let k = if p.thin() {
                0
            } else if p.ribbon() {
                1
            } else {
                2
            };
            kinds[k].0 += 1;
            kinds[k].1 += (p.area / total) as f32;
        }
        // All cables (and small bits): drawn as cables. Else its round cables are taken out of it:
        // drawn as cables, the rest as before (a telegraph line's poles and crossbars with its wires
        // in one geometry, as Goodsprings' is).
        let (thin, solid) = match thin_of(s.clone()) {
            Some(t) => (Some(t), None),
            None => {
                let cables: Vec<Strand> = s.iter().filter(|p| p.thin()).cloned().collect();
                if cables.is_empty() {
                    (None, None)
                } else {
                    let out: Vec<u32> = cables.iter().map(|c| c.piece).collect();
                    let rest: Vec<u32> = (0..triangles.len() as u32)
                        .filter(|&t| {
                            let p = of[t as usize];
                            p != u32::MAX && !out.contains(&p)
                        })
                        .collect();
                    (
                        Some(Thin {
                            strands: std::sync::Arc::new(cables),
                            ribbons: false,
                        }),
                        Some(std::sync::Arc::new(rest)),
                    )
                }
            }
        };
        Some(Body {
            bounds,
            thin,
            solid,
            solid_indices: None,
            pieces,
            look,
            kinds,
            triangles: dump_dir().map(|_| std::sync::Arc::new(triangles.to_vec())),
        })
    }
}

/// `NV_SUN_SHADOWS_DUMP_DIR` (a tool, with `NV_SUN_SHADOWS_DUMP`): where the dump writes each near
/// caster's triangles (object space, an OBJ file) for study; read once.
pub fn dump_dir() -> Option<&'static str> {
    static DIR: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    DIR.get_or_init(|| std::env::var("NV_SUN_SHADOWS_DUMP_DIR").ok())
        .as_deref()
}

/// A geometry of cables: the strands of its cables (round ones, [`Strand::thin`], or ribbons,
/// [`Strand::ribbon`]) and whether any is a ribbon.
#[derive(Clone, Debug, PartialEq)]
pub struct Thin {
    pub strands: std::sync::Arc<Vec<Strand>>,
    pub ribbons: bool,
}

/// The cables of a geometry's pieces ([`strands`]): none unless every piece is a cable or a ribbon
/// but small bits of at most [`BITS`] of its area (a pole with its wires is drawn as it always
/// was).
pub fn thin_of(pieces: Vec<Strand>) -> Option<Thin> {
    let total: f64 = pieces.iter().map(|s| s.area).sum();
    let (lines, bits): (Vec<Strand>, Vec<Strand>) =
        pieces.into_iter().partition(|s| s.thin() || s.ribbon());
    let bits: f64 = bits.iter().map(|s| s.area).sum();
    if lines.is_empty() || bits.is_nan() || bits > BITS * total {
        return None;
    }
    let ribbons = lines.iter().any(|s| !s.thin());
    Some(Thin {
        strands: std::sync::Arc::new(lines),
        ribbons,
    })
}

/// [`thin_of`] a geometry's triangles.
pub fn thin_shape(triangles: &[[[f32; 3]; 3]]) -> Option<Thin> {
    thin_of(strands(triangles))
}

/// Whether a sphere in the camera's view space (`centre`, `radius`) lies wholly inside the
/// camera's view (`frustum` left, right, bottom, top at unit distance) between depths `near` and
/// `far`: the game draws such an object every frame it is there (unless its own distance fade
/// hides it), so one it did not draw is gone.
pub fn wholly_in_view(
    frustum: [f32; 4],
    centre: [f64; 3],
    radius: f64,
    near: f64,
    far: f64,
) -> bool {
    let [l, r, b, t] = frustum.map(f64::from);
    let [x, y, z] = centre;
    let inside = |n: [f64; 3]| {
        let len = dot3(n, n).sqrt();
        dot3(n, [x, y, z]) / len >= radius
    };
    z - radius >= near
        && z + radius <= far
        && inside([1.0, 0.0, -l])
        && inside([-1.0, 0.0, r])
        && inside([0.0, 1.0, -b])
        && inside([0.0, -1.0, t])
}

/// The sun direction one cascade's map was drawn for: kept until the sun has turned far enough
/// from it to move a shadow edge in that map by half a texel ([`turn_limit`]); then the map is
/// drawn again with the sun as it is ([`sun_turns`] picks which maps take it in a frame).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SunKeep {
    pub kept: Option<[f64; 3]>,
}

impl SunKeep {
    /// How far (radians) the sun has turned from the direction kept; none before the first.
    pub fn turn(&self, to_sun: [f32; 3]) -> Option<f64> {
        let now = normalize(wide(to_sun));
        self.kept.map(|k| dot3(k, now).clamp(-1.0, 1.0).acos())
    }

    /// Keeps the sun as it is now.
    pub fn take(&mut self, to_sun: [f32; 3]) {
        self.kept = Some(normalize(wide(to_sun)));
    }

    /// The direction kept, or the sun as it is now before the first.
    pub fn direction(&self, to_sun: [f32; 3]) -> [f64; 3] {
        self.kept.unwrap_or_else(|| normalize(wide(to_sun)))
    }
}

/// How high (game units, measured along the sun's rays) a caster stands over the ground its
/// shadow falls on, for the turn rule: a telegraph pole, a tall house.
pub const SHADOW_REACH: f64 = 1500.0;
/// The sun this low (the sine of its height over the horizon) and lower counts as this low for the
/// turn rule.
pub const TURN_MIN_SUN_SINE: f64 = 0.17;
/// A map whose sun is this many times past its limit takes the sun in the same frame as the others
/// that are (time running fast: a wait, a sleep, a set hour); below it, one map a frame.
pub const OVERDUE: f64 = 4.0;

/// The sun's turn (radians) that moves a shadow edge by half a texel of a map whose texel is
/// `texel` (game units), for a caster [`SHADOW_REACH`] over the ground with the sun's height's sine
/// `sine`: the edge's end moves by reach x turn / sine^2 (a low sun's long shadows move fastest).
pub fn turn_limit(texel: f64, sine: f64) -> f64 {
    let s = sine.clamp(TURN_MIN_SUN_SINE, 1.0);
    0.5 * texel * s * s / SHADOW_REACH
}

/// Which maps take the sun as it is now, from each one's turn as a part of its limit (`over`:
/// infinite for a map never drawn): every map never drawn, and every map [`OVERDUE`]; else the one
/// furthest past its limit (at least 1), so a map is drawn again at most once a frame for the sun
/// and the shadows glide in steps of under half a texel.
pub fn sun_turns(over: [f64; CASCADES]) -> [bool; CASCADES] {
    let mut take = over.map(|o| o >= OVERDUE);
    if !take.iter().any(|&t| t) {
        let (k, most) = over
            .iter()
            .enumerate()
            .fold((0, 0.0f64), |b, (k, &o)| if o > b.1 { (k, o) } else { b });
        if most >= 1.0 {
            take[k] = true;
        }
    }
    take
}

/// Whether a map drawn for `drawn` (none: never drawn) is good for `now`: the same size, step and
/// snapped centre with the same sun.
pub fn still_good(drawn: Option<&Cascade>, now: &Cascade, sun_changed: bool) -> bool {
    match drawn {
        Some(d) => {
            !sun_changed
                && d.origin == now.origin
                && d.size == now.size
                && d.step.to_bits() == now.step.to_bits()
        }
        None => false,
    }
}

/// What a caster draws: its pipeline, buffers (by address, offset and upload count) and range. One
/// geometry can stand in several places ([`Cache`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct GeoKey {
    pub pipe: u32,
    pub streams: [(usize, u64, u64); 8],
    pub index: (usize, u64),
    pub start: u32,
    pub count: u32,
    pub base_vertex: i32,
}

impl GeoKey {
    /// The same geometry whatever has been written into its buffers since (their upload counts
    /// left out): what a caster whose buffers the game writes every frame keeps from one frame to
    /// the next.
    pub fn geometry(&self) -> GeoKey {
        GeoKey {
            streams: self.streams.map(|(b, o, _)| (b, o, 0)),
            index: (self.index.0, 0),
            ..self.clone()
        }
    }
}

/// What drawing a caster again takes.
pub(crate) struct Draw {
    /// Its depth-only pipeline ([`Gpu::pipelines`]).
    pub pipe: u32,
    pub streams: [Option<(std::sync::Arc<crate::device::Buf>, u64, u64)>; 8],
    /// The index buffer and its upload count when drawn ([`crate::device::Buf::uploads`]).
    pub index: Option<(std::sync::Arc<crate::device::Buf>, u64)>,
    /// The game's index buffer and its upload count when `index` is the device's own (a geometry
    /// with its cables taken out, [`Body::solid`]): watched for writes as `index` is.
    pub watch: Option<(std::sync::Arc<crate::device::Buf>, u64)>,
    /// Cables taken out of it ([`Body::solid`]): its draw is the rest, drawn into the depth maps as
    /// any caster while its cables are drawn into the coverage maps.
    pub rest: bool,
    pub start: u32,
    pub count: u32,
    pub base_vertex: i32,
    /// Its vertex shader's float constants as drawn and `ModelViewProj`'s first register.
    pub consts: Box<[u8]>,
    pub mvp: u32,
    /// Its vertex block's header (in the device's list of them).
    pub header: u32,
    /// An alpha-tested caster's cut-out ([`Cutout`]): drawn through its own pixel shader.
    pub cutout: Option<Cutout>,
    /// A cable's strands ([`thin_shape`]): drawn into the thin casters' coverage maps, not the
    /// depth maps.
    pub thin: Option<Thin>,
    /// A ribbon cable's (a cut-out of [`Thin::ribbons`]) pipeline that adds its texture's alpha
    /// into the coverage maps ([`Gpu::make_pipeline`] with a target), and its pixel block's header
    /// (no alpha test, the textures read at their own level).
    pub ribbon: Option<(u32, u32)>,
}

/// What an alpha-tested caster draws its cut-out with: its pixel shader's textures (bind group 1),
/// the vertex shader's (none: an empty group 2), its pixel block's header with the alpha test as
/// the game set it (never alpha to coverage: the map has one sample) and its float constants.
pub(crate) struct Cutout {
    pub textures: wgpu::BindGroup,
    pub empty: wgpu::BindGroup,
    pub header: u32,
    pub consts: Box<[u8]>,
}

impl Draw {
    /// Whether a buffer it draws from was written since, or let go by the game (only the light
    /// list holds it).
    pub fn stale(&self) -> bool {
        let gone = |b: &std::sync::Arc<crate::device::Buf>, n: u64| {
            std::sync::Arc::strong_count(b) == 1 || b.uploads.load(Ordering::Relaxed) != n
        };
        self.streams.iter().flatten().any(|(b, _, n)| gone(b, *n))
            || self.index.as_ref().is_some_and(|(b, n)| gone(b, *n))
            || self.watch.as_ref().is_some_and(|(b, n)| gone(b, *n))
    }

    /// Its `ModelViewProj` as drawn.
    pub fn mvp(&self) -> [[f32; 4]; 4] {
        std::array::from_fn(|r| {
            std::array::from_fn(|j| {
                let o = 16 * (self.mvp as usize + r) + 4 * j;
                f32::from_le_bytes(self.consts[o..o + 4].try_into().expect("4 bytes"))
            })
        })
    }
}

/// An opaque world draw of this frame on the scene, for the sun's maps.
pub(crate) struct Caster {
    pub key: GeoKey,
    /// Its object-space bounding sphere.
    pub bounds: ([f64; 3], f64),
    /// Its pieces ([`Body`]).
    pub body: Body,
    pub draw: Draw,
    /// The depth surface it was drawn on.
    pub depth: u64,
}

/// One place of a geometry in the world, with the frame it was last drawn in and the frame it came.
pub struct Instance<T> {
    pub world: World,
    pub centre: [f64; 3],
    pub radius: f64,
    pub seen: u64,
    pub born: u64,
    /// It moves: drawn each frame over a copy of the still maps, not into them
    /// ([`Cache::mark_moves`]); let go as soon as a frame does not draw it.
    pub moving: bool,
    /// Its geometry moved on from here (this place was paired with a new one): let go at the sweep.
    pub left: bool,
    pub payload: T,
}

impl<T> Instance<T> {
    /// Whether, not drawn in frame `now`, it is gone because it moved: a moving place, or one a
    /// new place of its geometry was paired with.
    pub fn moved_on(&self, now: u64) -> bool {
        self.seen != now && (self.moving || self.left)
    }
}

/// How far a moving caster's centre may go between two frames and still be the same object
/// moved (about 3.7 m: faster than anything the game moves that casts).
pub const MOVE_REACH: f64 = 256.0;

/// The frames a moving place stands still before it is still again ([`Cache::settle`]).
pub const SETTLE: u64 = 30;

/// Whether a sphere in the camera's view space (`centre`, `radius`) reaches into the camera's view
/// (`frustum` left, right, bottom, top at unit distance) between depths `near` and `far`.
pub fn partly_in_view(
    frustum: [f32; 4],
    centre: [f64; 3],
    radius: f64,
    near: f64,
    far: f64,
) -> bool {
    let [l, r, b, t] = frustum.map(f64::from);
    let inside = |n: [f64; 3]| dot3(n, centre) / dot3(n, n).sqrt() >= -radius;
    centre[2] + radius >= near
        && centre[2] - radius <= far
        && inside([1.0, 0.0, -l])
        && inside([-1.0, 0.0, r])
        && inside([0.0, 1.0, -b])
        && inside([0.0, -1.0, t])
}

/// The pairing rule of [`Cache::mark_moves`] for a camera at `position` with `axes` (right, up,
/// ahead) and `frustum`, `near`, `far`: a new place is an old one moved when the two centres are
/// within [`MOVE_REACH`] and the old place still reaches into the view (a copy that left the view
/// was not drawn because it is out of sight, not because it moved: a fence post leaving at one
/// edge as another enters is no move), or, out of the view (the camera turned, or the object
/// moved out past the edge), when the new centre lies inside the old place's own sphere (no two
/// copies of a geometry stand that close).
pub fn mover<T>(
    frustum: [f32; 4],
    position: [f64; 3],
    axes: [[f64; 3]; 3],
    near: f64,
    far: f64,
) -> impl Fn(&Instance<T>, &Instance<T>) -> bool {
    move |old, new| {
        let d: [f64; 3] = std::array::from_fn(|k| old.centre[k] - position[k]);
        let v = [dot3(axes[0], d), dot3(axes[1], d), dot3(axes[2], d)];
        let m: [f64; 3] = std::array::from_fn(|k| new.centre[k] - old.centre[k]);
        let apart = dot3(m, m).sqrt();
        apart <= MOVE_REACH
            && (partly_in_view(frustum, v, old.radius, near, far) || apart <= old.radius)
    }
}

/// The casters around the camera across frames: what the game drew, kept while it can still
/// cast (the maps are drawn from it, so objects out of sight behind the camera still cast, and a
/// map is drawn again only when something in it came, went or moved).
pub struct Cache<K, T> {
    /// (The fast hasher: the keys are the device's own buffers and ranges, and hashing every kept
    /// caster's key each frame with the default one cost most of a millisecond standing still.)
    map: crate::fxhash::Map<K, Vec<Instance<T>>>,
    len: usize,
}

impl<K: std::hash::Hash + Eq, T> Default for Cache<K, T> {
    fn default() -> Self {
        Cache {
            map: crate::fxhash::Map::default(),
            len: 0,
        }
    }
}

impl<K: std::hash::Hash + Eq + Clone, T> Cache<K, T> {
    /// A caster drawn in frame `now` at `world` (object-space bounds `local`): kept when the same
    /// geometry is at the same place, else added (moving from the start when `moving`: a geometry
    /// whose buffers the game writes); the world sphere of one added.
    pub fn see(
        &mut self,
        key: K,
        world: World,
        local: ([f64; 3], f64),
        now: u64,
        moving: bool,
        payload: impl FnOnce() -> T,
    ) -> Option<([f64; 3], f64)> {
        let places = self.map.entry(key).or_default();
        if let Some(i) = places.iter_mut().find(|i| same_place(&i.world, &world)) {
            i.seen = now;
            return None;
        }
        let centre = place_point(&world, local.0);
        let radius = local.1 * stretch(&world);
        places.push(Instance {
            world,
            centre,
            radius,
            seen: now,
            born: now,
            moving,
            left: false,
            payload: payload(),
        });
        self.len += 1;
        Some((centre, radius))
    }

    /// Motion, decided place by place (never for a whole geometry: fence posts, poles and rocks
    /// are many places of one geometry). For each geometry in `came` (places added in frame
    /// `now`), its new places are paired with its places drawn the frame before and not in this
    /// one, one for one, the nearest pairs first, wherever `pair(old, new)` holds ([`mover`]);
    /// other copies of the geometry coming or going in the same frame (a second can, a fence post
    /// leaving the view) change nothing for a real move. A paired new place moves; its old place
    /// has left (the sweep lets it go). At most `room` places are marked; returns how many were.
    pub fn mark_moves<'a>(
        &mut self,
        now: u64,
        came: impl IntoIterator<Item = &'a K>,
        room: usize,
        pair: impl Fn(&Instance<T>, &Instance<T>) -> bool,
    ) -> usize
    where
        K: 'a,
    {
        let mut marked = 0;
        for key in came {
            let Some(places) = self.map.get_mut(key) else {
                continue;
            };
            let new: Vec<usize> = (0..places.len())
                .filter(|&n| places[n].born == now && places[n].seen == now)
                .collect();
            let old: Vec<usize> = (0..places.len())
                .filter(|&o| places[o].seen + 1 == now && !places[o].left)
                .collect();
            let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
            for &n in &new {
                for &o in &old {
                    if pair(&places[o], &places[n]) {
                        let d: [f64; 3] =
                            std::array::from_fn(|k| places[o].centre[k] - places[n].centre[k]);
                        pairs.push((dot3(d, d), n, o));
                    }
                }
            }
            pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut taken = vec![false; places.len()];
            for (_, n, o) in pairs {
                if taken[n] || taken[o] {
                    continue;
                }
                taken[n] = true;
                taken[o] = true;
                if marked >= room {
                    return marked;
                }
                places[n].moving = true;
                places[o].left = true;
                marked += 1;
            }
        }
        marked
    }

    /// Moving places that have stood still: one drawn in frame `now` at the place it came to
    /// `frames` or more frames before stops moving (it casts from the still maps again, also from
    /// out of sight); the world spheres of those, for the still maps to be drawn again.
    pub fn settle(&mut self, now: u64, frames: u64) -> Vec<([f64; 3], f64)> {
        let mut out = Vec::new();
        for i in self.map.values_mut().flatten() {
            if i.moving && i.seen == now && now - i.born >= frames {
                i.moving = false;
                out.push((i.centre, i.radius));
            }
        }
        out
    }

    /// Lets go of the casters `gone` says are gone (it is asked of every one); the geometry, world
    /// sphere and whether it was moving (then it was in no still map) of each one let go.
    #[allow(clippy::type_complexity)]
    pub fn sweep(
        &mut self,
        mut gone: impl FnMut(&K, &Instance<T>) -> bool,
    ) -> Vec<(K, ([f64; 3], f64), bool)> {
        let mut out = Vec::new();
        self.map.retain(|k, places| {
            places.retain(|i| {
                let g = gone(k, i);
                if g {
                    out.push((k.clone(), (i.centre, i.radius), i.moving));
                }
                !g
            });
            !places.is_empty()
        });
        self.len -= out.len();
        out
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether any place of geometry `key` is kept.
    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &Instance<T>> {
        self.map.values().flatten()
    }

    /// Each place with its geometry.
    pub fn iter_keyed(&self) -> impl Iterator<Item = (&K, &Instance<T>)> {
        self.map
            .iter()
            .flat_map(|(k, places)| places.iter().map(move |i| (k, i)))
    }
}

/// The light list of a frame.
#[derive(Default)]
pub(crate) struct Lists {
    pub casters: Vec<Caster>,
}

impl Lists {
    pub fn clear(&mut self) {
        self.casters.clear();
    }
}

/// Where sample 0 of a pixel lies, from the pixel's centre, in Vulkan's and Direct3D's standard
/// sample patterns: the debug step reads a multisampled depth's sample 0, so it places the point
/// there.
pub fn sample0_offset(samples: u32) -> [f32; 2] {
    match samples {
        2 => [0.25, 0.25],
        4 => [-0.125, -0.375],
        8 => [0.0625, -0.1875],
        16 => [0.0625, 0.0625],
        _ => [0.0, 0.0],
    }
}

/// The lookup's uniform block, in bytes ([`lookup_uniforms`]).
pub const LOOKUP_SIZE: u64 = 8 * 16 + 4 * CASCADES as u64 * 16;

/// The thin casters' coverage maps: texels a side (a layer a cascade, over the same square as the
/// cascade's depth map, so a texel is `SIZE / THIN_SIZE` of its texels).
pub const THIN_SIZE: u32 = 1024;
/// Their format: coverage, coverage x depth (less the middle of the depth range, where half
/// floats are finest), coverage x the width drawn (game units), unused; added up.
pub const THIN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// The sun's width across the sky (radians: half a degree): a strand `d` units over a receiver
/// casts a soft band about `SUN_WIDTH x d` wide.
pub const SUN_WIDTH: f32 = 0.0093;
/// A strand is drawn at least this many of its map's texels wide (so it covers a texel centre all
/// along its length, whatever its slant: no broken line), with its coverage spread over that
/// width.
pub const THIN_DRAWN: f64 = 2.0;
/// A receiver nearer than this (game units, along the sun's rays) to the strand over it takes none
/// of its shadow (the cable itself, the pole top it hangs from), and twice as far takes it whole.
pub const THIN_NEAR: f32 = 12.0;

/// How the thin casters ([`Body::thin`]) are drawn: into their coverage maps (1, the default), or
/// as `NV_SUN_SHADOWS_THIN=solid` asks (captures only, to compare): into the depth maps as any
/// other caster (0); read once.
pub fn thin_maps() -> bool {
    static THIN: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *THIN.get_or_init(|| std::env::var("NV_SUN_SHADOWS_THIN").map_or(true, |v| v.trim() != "solid"))
}

/// The quads that draw a placed cable's strands into map `c`'s coverage layer (six corners a
/// segment: clip x, y; depth less the middle; coverage; the band the coverage is spread over):
/// each segment `max(width, THIN_DRAWN texels)` wide with its coverage `width / drawn`, so a strand
/// thinner than a texel covers its real share and never draws a broken line. A `ribbon` geometry's
/// (a cut-out: its cable is in its texture's alpha, which its own pipeline adds) quads carry its
/// depth alone, over the ribbon's width, its coverage spread over [`THIN_DRAWN`] texels.
pub fn thin_quads(
    c: &Cascade,
    w: &World,
    strands: &[Strand],
    ribbon: bool,
    out: &mut Vec<[f32; 5]>,
) {
    let m = c.world_to_light(w);
    let scale = stretch(w);
    let texel = c.half * 2.0 / f64::from(THIN_SIZE);
    for s in strands {
        let width = s.width * scale;
        let drawn = width.max(THIN_DRAWN * texel);
        let (cover, band) = if ribbon {
            (0.0, THIN_DRAWN * texel)
        } else {
            ((width / drawn) as f32, drawn)
        };
        let at = |p: [f32; 3]| -> [f64; 3] {
            std::array::from_fn(|r| {
                f64::from(m[r][0]) * f64::from(p[0])
                    + f64::from(m[r][1]) * f64::from(p[1])
                    + f64::from(m[r][2]) * f64::from(p[2])
                    + f64::from(m[r][3])
            })
        };
        // Half the width drawn, in clip units (the map's square is 2 across).
        let half = drawn / c.half / 2.0;
        for seg in s.line.windows(2) {
            let (a, b) = (at(seg[0]), at(seg[1]));
            let d = [b[0] - a[0], b[1] - a[1]];
            let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
            // A segment along the rays (seen end on) covers a spot: drawn square.
            let (px, py) = if len > 1e-9 {
                (-d[1] / len * half, d[0] / len * half)
            } else {
                (half, 0.0)
            };
            let (ex, ey) = if len > 1e-9 { (0.0, 0.0) } else { (0.0, half) };
            let corner = |p: [f64; 3], s: f64, e: f64| {
                [
                    (p[0] + px * s + ex * e) as f32,
                    (p[1] + py * s + ey * e) as f32,
                    (p[2] - 0.5) as f32,
                    cover,
                    band as f32,
                ]
            };
            let (a0, a1) = (corner(a, -1.0, -1.0), corner(a, 1.0, -1.0));
            let (b0, b1) = (corner(b, -1.0, 1.0), corner(b, 1.0, 1.0));
            out.extend([a0, a1, b0, a1, b1, b0]);
        }
    }
}

/// The lookup's uniform block (`SL` in [`LOOKUP_FNS`]), the same for the lighting's variants and
/// the debug step: frustum (left, right, bottom, top), depth (near, far, width, height of the
/// scene), sun (toward it in view space; w 1 when there is a sun this frame), splits (each
/// cascade's far view depth), texels (each cascade's texel in game units), extra (sample 0's place
/// in the pixel ([`sample0_offset`]), where the fade to lit starts, the blend band as a part of a
/// cascade's depth), misc (the debug step's darkness), thin (each cascade's coverage-map texel in
/// game units, 0 when no thin caster is in it: `thin`), then each cascade's matrix from this
/// frame's view space to the map it was drawn as (`drawn`: a map kept from an earlier frame keeps
/// its own place), row by row.
#[allow(clippy::too_many_arguments)]
pub fn lookup_uniforms(
    view: &scene_light::Params,
    strength: f32,
    cascades: &[Cascade; CASCADES],
    drawn: &[Cascade; CASCADES],
    thin: [bool; CASCADES],
    axes: [[f64; 3]; 3],
    position: [f64; 3],
    samples: u32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let last = cascades[CASCADES - 1].far;
    let sample0 = sample0_offset(samples);
    let on = if strength > 0.0 { 1.0 } else { 0.0 };
    let mut f = vec![
        view.left,
        view.right,
        view.bottom,
        view.top,
        view.near,
        view.far,
        width as f32,
        height as f32,
        view.sun[0],
        view.sun[1],
        view.sun[2],
        on,
    ];
    f.extend(cascades.iter().map(|c| c.far));
    f.extend(drawn.iter().map(|c| c.texel as f32));
    f.extend([sample0[0], sample0[1], last * (1.0 - FADE), BLEND]);
    f.extend([
        DARKNESS * (strength / scene_light::SHADOW_STRENGTH).clamp(0.0, 1.0),
        if grey_view() { 1.0 } else { 0.0 },
        0.0,
        0.0,
    ]);
    f.extend((0..CASCADES).map(|k| {
        if thin[k] {
            (drawn[k].half * 2.0 / f64::from(THIN_SIZE)) as f32
        } else {
            0.0
        }
    }));
    for c in drawn {
        for row in &c.view_to_light_for(axes, position) {
            f.extend(row.iter().map(|&v| v as f32));
        }
    }
    debug_assert_eq!(f.len() * 4, LOOKUP_SIZE as usize);
    f.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// The lookup's functions, over `SL` (the uniform block), the maps `sl_maps` (one layer a
/// cascade) and the comparison sampler `sl_cmp`, which the user declares. `sl_visibility(p, n)`:
/// how much of the sun reaches the view-space point `p` whose face's normal (toward the camera)
/// is `n`, 1 lit, 0 shadowed; lit past the last cascade (faded to it) and outside every map.
pub const LOOKUP_FNS: &str = "
struct SunLookup {
    frustum: vec4<f32>,
    depth: vec4<f32>,
    sun: vec4<f32>,
    splits: vec4<f32>,
    texel: vec4<f32>,
    extra: vec4<f32>,
    misc: vec4<f32>,
    thin: vec4<f32>,
    m: array<vec4<f32>, 16>,
};

fn sl_linear(d: f32) -> f32 {
    return SL.depth.x * SL.depth.y / (SL.depth.y - d * (SL.depth.y - SL.depth.x));
}

// The view-space point at Direct3D pixel position `xy` (whole numbers at the pixel centres) and
// depth value `d`.
fn sl_point(xy: vec2<f32>, d: f32) -> vec3<f32> {
    let z = sl_linear(d);
    let uv = xy / SL.depth.zw;
    return vec3<f32>(mix(SL.frustum.x, SL.frustum.y, uv.x) * z, mix(SL.frustum.w, SL.frustum.z, uv.y) * z, z);
}

fn sl_to_map(k: i32, p: vec3<f32>) -> vec3<f32> {
    let q = vec4<f32>(p, 1.0);
    return vec3<f32>(dot(SL.m[k * 4], q), dot(SL.m[k * 4 + 1], q), dot(SL.m[k * 4 + 2], q));
}

fn sl_tap(k: i32, uv: vec2<f32>, r: f32) -> f32 {
    return textureSampleCompareLevel(sl_maps, sl_cmp, uv, k, r);
}

fn sl_size(_k: i32) -> vec2<f32> {
    return vec2<f32>(textureDimensions(sl_maps));
}

// Map `k`'s soft edge at `p`: nine of the hardware's filtered comparisons a texel apart, weighted
// 1 2 1 by 1 2 1 (a fixed kernel about four texels wide; no noise, nothing that moves).
fn sl_filter(k: i32, p: vec3<f32>, bias: f32) -> f32 {
    let l = sl_to_map(k, p);
    let uv = vec2<f32>(l.x * 0.5 + 0.5, 0.5 - l.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || l.z >= 1.0) {
        return 1.0;
    }
    let t = 1.0 / sl_size(k);
    let r = l.z - bias;
    var sum = 0.0;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let w = f32((2 - abs(i)) * (2 - abs(j)));
            sum += w * sl_tap(k, uv + vec2<f32>(f32(i), f32(j)) * t, r);
        }
    }
    return sum / 16.0;
}

// How much of the sun the thin casters (cables) over map `k`'s point `l` take away: their
// coverage in a tent of three of its texels, their depth and the width drawn from the sums. A
// strand `d` units over the receiver casts a band `SUN_WIDTH d` wide (the sun's own width): where
// that is wider than the band the map shows, the coverage is spread over it (a high cable's shadow
// is fainter); none on the strand itself.
fn sl_thin(k: i32, l: vec3<f32>) -> f32 {
    let texel = SL.thin[k];
    let uv = vec2<f32>(l.x * 0.5 + 0.5, 0.5 - l.y * 0.5);
    if (texel <= 0.0 || any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || l.z >= 1.0) {
        return 0.0;
    }
    let t = 0.5 / vec2<f32>(textureDimensions(sl_thin_maps));
    var s = textureSampleLevel(sl_thin_maps, sl_lin, uv + vec2<f32>(-t.x, -t.y), k, 0.0);
    s += textureSampleLevel(sl_thin_maps, sl_lin, uv + vec2<f32>(t.x, -t.y), k, 0.0);
    s += textureSampleLevel(sl_thin_maps, sl_lin, uv + vec2<f32>(-t.x, t.y), k, 0.0);
    s += textureSampleLevel(sl_thin_maps, sl_lin, uv + vec2<f32>(t.x, t.y), k, 0.0);
    s = s * 0.25;
    if (s.x < 1e-3 || s.w < 1e-3) {
        return 0.0;
    }
    let d = (l.z - 0.5 - s.y / s.x) * 2.0 * DEPTH_RANGE;
    let band = s.z / s.x + 0.5 * texel;
    let near = saturate((d - THIN_NEAR) / THIN_NEAR);
    return min(s.w, 1.0) * min(1.0, band / max(d * SUN_WIDTH, 1e-3)) * near;
}

fn sl_in(k: i32, p: vec3<f32>, n: vec3<f32>) -> f32 {
    let texel = SL.texel[k];
    // The receiver moved along its face's normal by a part of a texel that grows as the face
    // turns from the sun, and a depth allowance of a texel plus its slope (at most two).
    let c = clamp(dot(n, SL.sun.xyz), 0.0, 1.0);
    let s = sqrt(1.0 - c * c);
    let offset = NORMAL_OFFSET * texel * s;
    let slope = min(s / max(c, 0.05), SLOPE_MAX);
    let bias = (DEPTH_BIAS + slope) * texel / (2.0 * DEPTH_RANGE);
    let q = p + n * offset;
    return sl_filter(k, q, bias) * (1.0 - sl_thin(k, sl_to_map(k, q)));
}

fn sl_visibility(p: vec3<f32>, n: vec3<f32>) -> f32 {
    let z = p.z;
    if (SL.sun.w <= 0.0 || z >= SL.splits.w || z <= 0.0) {
        return 1.0;
    }
    var k = 3;
    if (z < SL.splits.x) {
        k = 0;
    } else if (z < SL.splits.y) {
        k = 1;
    } else if (z < SL.splits.z) {
        k = 2;
    }
    var v = sl_in(k, p, n);
    // Near a cascade's far end, blended into the next one, so no line shows where they meet.
    if (k < 3) {
        let far = SL.splits[k];
        var near = SL.depth.x;
        if (k > 0) {
            near = SL.splits[k - 1];
        }
        let band = SL.extra.w * (far - near);
        let t = saturate((z - (far - band)) / band);
        if (t > 0.0) {
            v = mix(v, sl_in(k + 1, p, n), t);
        }
    }
    return mix(v, 1.0, saturate((z - SL.extra.z) / (SL.splits.w - SL.extra.z)));
}
";

/// The lighting variants' declarations and entry (`crate::sm::wgsl::convert_with_sun`): bind
/// group 3 (the lookup's block at a dynamic offset, the four maps, the comparison sampler),
/// `sun_lit`, and `sun_visibility(frag)` from the pixel's own position and depth, its face's
/// normal from the position's derivatives (called in uniform control flow).
pub const LOOKUP: &str = "
@group(3) @binding(0) var<uniform> SL: SunLookup;
@group(3) @binding(1) var sl_maps: texture_depth_2d_array;
@group(3) @binding(2) var sl_cmp: sampler_comparison;
@group(3) @binding(3) var sl_thin_maps: texture_2d_array<f32>;
@group(3) @binding(4) var sl_lin: sampler;
var<private> sun_lit: f32 = 1.0;

fn sun_visibility(frag: vec4<f32>) -> f32 {
    // Direct3D 9's pixel centres are on whole coordinates (the vertex shaders move half a pixel).
    let p = sl_point(frag.xy - vec2<f32>(0.5), frag.z);
    var n = cross(dpdx(p), dpdy(p));
    let l = length(n);
    if (l < 1e-12) {
        n = -normalize(p);
    } else {
        n = n / l;
    }
    if (dot(n, p) > 0.0) {
        n = -n;
    }
    return sl_visibility(p, n);
}
";

/// The vertex-lit variants' declarations and entry: as [`LOOKUP`], and `sun_visibility_clip(o)` at
/// the vertex's clip-space place `o` (Direct3D 9's projection: w is the view depth), with no
/// normal (the depth allowance alone; the grass the variants light has no face to speak of).
pub const LOOKUP_VERTEX: &str = "
@group(3) @binding(0) var<uniform> SL: SunLookup;
@group(3) @binding(1) var sl_maps: texture_depth_2d_array;
@group(3) @binding(2) var sl_cmp: sampler_comparison;
@group(3) @binding(3) var sl_thin_maps: texture_2d_array<f32>;
@group(3) @binding(4) var sl_lin: sampler;
var<private> sun_lit: f32 = 1.0;

fn sun_visibility_clip(o: vec4<f32>) -> f32 {
    if (o.w <= 0.0) {
        return 1.0;
    }
    let ndc = o.xy / o.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let p = vec3<f32>(mix(SL.frustum.x, SL.frustum.y, uv.x) * o.w, mix(SL.frustum.w, SL.frustum.z, uv.y) * o.w, o.w);
    return sl_visibility(p, SL.sun.xyz);
}
";

/// The vertex-lit variants' WGSL addition: the constants, [`LOOKUP_VERTEX`] and [`LOOKUP_FNS`].
pub fn lookup_wgsl_vertex() -> String {
    format!("{}{LOOKUP_VERTEX}{LOOKUP_FNS}", lookup_consts())
}

/// The lighting variants' WGSL addition: the constants, [`LOOKUP`] and [`LOOKUP_FNS`].
pub fn lookup_wgsl() -> String {
    format!("{}{LOOKUP}{LOOKUP_FNS}", lookup_consts())
}

/// The constants [`LOOKUP_FNS`] reads.
fn lookup_consts() -> String {
    format!(
        "const NORMAL_OFFSET: f32 = {NORMAL_OFFSET:?};\nconst DEPTH_BIAS: f32 = {DEPTH_BIAS:?};\n\
         const SLOPE_MAX: f32 = {SLOPE_MAX:?};\nconst DEPTH_RANGE: f32 = {:?};\n\
         const SUN_WIDTH: f32 = {SUN_WIDTH:?};\nconst THIN_NEAR: f32 = {THIN_NEAR:?};\n",
        DEPTH_RANGE as f32
    )
}

const STEP: &str = "
@group(0) @binding(0) var<uniform> SL: SunLookup;
@group(0) @binding(1) var depth_tex: DEPTH_TYPE;
@group(0) @binding(2) var sl_maps: texture_depth_2d_array;
@group(0) @binding(3) var sl_cmp: sampler_comparison;
@group(0) @binding(4) var sl_thin_maps: texture_2d_array<f32>;
@group(0) @binding(5) var sl_lin: sampler;

@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

fn depth_at(xy: vec2<i32>) -> f32 {
    let c = clamp(xy, vec2<i32>(0), vec2<i32>(SL.depth.zw) - 1);
    return textureLoad(depth_tex, c, 0);
}

// Direct3D 9's pixel centres are on whole coordinates (each vertex shader moves its position half
// a pixel, `sm::wgsl`): the depth at pixel `xy` is the scene at `xy` itself, and sample 0 lies
// `extra.xy` from it.
fn point_at(xy: vec2<i32>) -> vec3<f32> {
    return sl_point(vec2<f32>(xy) + SL.extra.xy, depth_at(xy));
}

// The normal at `c` (its point `pc`), toward the camera, from the nearer neighbour on each axis.
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

@fragment fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let c = vec2<i32>(at.xy);
    let d = depth_at(c);
    let pc = point_at(c);
    if (d >= 1.0 || SL.sun.w <= 0.0 || pc.z >= SL.splits.w) {
        if (VIEW != 0u) {
            return vec4<f32>(1.0, 1.0, 1.0, 1.0);
        }
        discard;
    }
    let n = normal_at(c, pc);
    let facing = dot(n, SL.sun.xyz);
    var k = 3;
    if (pc.z < SL.splits.x) {
        k = 0;
    } else if (pc.z < SL.splits.y) {
        k = 1;
    } else if (pc.z < SL.splits.z) {
        k = 2;
    }
    // `NV_SUN_SHADOWS_VIEW=cascades` (captures only): each cascade's own tint (red, green, blue,
    // yellow nearest first), dark where shadowed, half where the face is turned from the sun.
    var tint = array<vec3<f32>, 4>(vec3<f32>(1.0, 0.55, 0.55), vec3<f32>(0.55, 1.0, 0.55), vec3<f32>(0.55, 0.55, 1.0), vec3<f32>(1.0, 1.0, 0.45));
    // Faces away from the sun are in the game's own shade already.
    if (facing <= 0.0) {
        if (VIEW != 0u) {
            return vec4<f32>(tint[k] * 0.5, 1.0);
        }
        discard;
    }
    let lit = sl_visibility(pc, n);
    if (VIEW != 0u) {
        return vec4<f32>(tint[k] * (0.15 + 0.85 * lit), 1.0);
    }
    let shade = 1.0 - SL.misc.x * (1.0 - lit) * saturate(facing * 5.0);
    if (shade >= 0.999) {
        discard;
    }
    return vec4<f32>(shade, shade, shade, 1.0);
}
";

/// The debug step's WGSL, for a multisampled scene depth or not.
pub fn step_shader(multisampled_depth: bool) -> String {
    step_shader_with(multisampled_depth, view())
}

/// [`step_shader`] with the view `view` ([`view`]).
pub fn step_shader_with(multisampled_depth: bool, view: u32) -> String {
    let depth_type = if multisampled_depth {
        "texture_depth_multisampled_2d"
    } else {
        "texture_depth_2d"
    };
    format!(
        "{}const VIEW: u32 = {view}u;\n{}{LOOKUP_FNS}",
        lookup_consts(),
        STEP.replace("DEPTH_TYPE", depth_type)
    )
}

/// `NV_SUN_SHADOWS_VIEW=cascades` (diagnostic captures only, not a player setting): the scene replaced
/// by each cascade's tint, dark where shadowed; read once; 0 otherwise.
pub fn view() -> u32 {
    static VIEW: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *VIEW.get_or_init(|| {
        match std::env::var("NV_SUN_SHADOWS_VIEW")
            .as_deref()
            .map(str::trim)
        {
            Ok("cascades") => 1,
            _ => 0,
        }
    })
}

/// `NV_SUN_SHADOWS=debug` (diagnostic, not a player setting): the maps shown by the full-screen debug
/// step laid over the finished scene instead of inside the lighting; read once.
pub fn debug_step() -> bool {
    static DEBUG: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    MODE.load(Ordering::Relaxed) == 2
        || *DEBUG.get_or_init(|| {
            std::env::var("NV_SUN_SHADOWS").is_ok_and(|v| v.trim() == "debug")
                || std::env::var("NV_SUN_SHADOWS_VIEW").is_ok_and(|v| v.trim() == "cascades")
        })
}

/// `NV_SUN_SHADOWS_VIEW=grey` (diagnostic captures only): every lit pixel shader's output replaced by the
/// sun's visibility at the pixel as grey (white lit, black shadowed); read once.
pub fn grey_view() -> bool {
    static GREY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *GREY.get_or_init(|| std::env::var("NV_SUN_SHADOWS_VIEW").is_ok_and(|v| v.trim() == "grey"))
}

/// An IEEE half float as a float.
pub fn half_to_f32(h: u16) -> f32 {
    let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = i32::from((h >> 10) & 0x1f);
    let frac = f32::from(h & 0x3ff);
    sign * match exp {
        0 => frac * 2f32.powi(-24),
        31 if frac == 0.0 => f32::INFINITY,
        31 => f32::NAN,
        _ => (1.0 + frac / 1024.0) * 2f32.powi(exp - 15),
    }
}

/// Alpha-tested casters (cut-outs) are drawn into this many cascades, the nearest: further out a
/// fence or a bush is a few texels and would flicker.
pub const CUTOUT_CASCADES: usize = 3;
/// The level bias a cut-out caster's textures are read with in the maps: far below any level, so
/// the top level (the texture's own alpha, not a smaller level's average) decides what casts.
pub const CUTOUT_BIAS: f32 = -16.0;

/// The maps' format.
pub const MAP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// What the step is run on: the scene's colour target and depth surface.
pub(crate) struct Targets<'a> {
    pub colour: &'a wgpu::TextureView,
    pub colour_format: wgpu::TextureFormat,
    pub depth: &'a wgpu::TextureView,
    pub samples: u32,
}

/// The sun shadows' GPU objects, made at first use.
pub(crate) struct Gpu {
    /// The maps the lookups read (the still casters' maps with the moving casters drawn over a
    /// copy), their textures, and the still casters' maps, drawn again only when needed.
    pub maps: Vec<wgpu::TextureView>,
    live: wgpu::Texture,
    statics: wgpu::Texture,
    static_views: Vec<wgpu::TextureView>,
    /// The maps the lookups read, as one array (a layer a cascade).
    array: wgpu::TextureView,
    /// What each map was last drawn for.
    pub drawn: [Option<Cascade>; CASCADES],
    /// Whether each map the lookups read holds moving casters drawn over its still map's copy.
    live_moving: [bool; CASCADES],
    /// The thin casters' coverage maps ([`THIN_SIZE`], [`THIN_FORMAT`]): a layer each cascade,
    /// as one array; whether each layer holds any (the lookups skip an empty one) and whether a
    /// moving one (drawn again each frame while one does); the pipeline that draws their quads
    /// ([`thin_quads`]), the sampler the lookups filter them with, and the quads' buffer.
    thin_views: Vec<wgpu::TextureView>,
    thin_array: wgpu::TextureView,
    pub thin_in: [bool; CASCADES],
    thin_moving: [bool; CASCADES],
    thin_pipe: Option<wgpu::RenderPipeline>,
    linear: wgpu::Sampler,
    thin_quads: Option<wgpu::Buffer>,
    compare: wgpu::Sampler,
    uniforms: wgpu::Buffer,
    #[allow(clippy::type_complexity)]
    apply: Vec<(
        (bool, wgpu::TextureFormat, u32),
        wgpu::BindGroupLayout,
        wgpu::RenderPipeline,
    )>,
    /// The ring of the casters' register blocks (header, then float constants, each at a dynamic
    /// offset) and its group in the device's register layout.
    ring: Option<(wgpu::Buffer, wgpu::BindGroup, u64)>,
    /// The depth-only pipelines, in the order the device made them.
    pub pipelines: Vec<Option<wgpu::RenderPipeline>>,
    /// The lighting variants' bind group 3 ([`LOOKUP`]): its layout, the ring of lookup blocks
    /// (one slot a frame, [`LIGHT_SLOTS`] in turn, at a dynamic offset) and the group.
    pub light_layout: wgpu::BindGroupLayout,
    pub light_ring: wgpu::Buffer,
    pub light_group: wgpu::BindGroup,
}

/// A lookup block's slot in the lighting variants' ring, and the slots: a frame writes its own
/// slot, so draws of a frame still on their way read theirs.
pub const LIGHT_SLOT: u64 = LOOKUP_SIZE.div_ceil(256) * 256;
pub const LIGHT_SLOTS: u64 = 4;

/// The lighting variants' bind group 3 layout: the lookup block (dynamic offset), the maps, the
/// comparison sampler; fragment stage.
pub(crate) fn light_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let mut entries = vec![wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: true,
            min_binding_size: std::num::NonZeroU64::new(LOOKUP_SIZE),
        },
        count: None,
    }];
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Depth,
            view_dimension: wgpu::TextureViewDimension::D2Array,
            multisampled: false,
        },
        count: None,
    });
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
        count: None,
    });
    entries.extend(thin_entries(3, wgpu::ShaderStages::VERTEX_FRAGMENT));
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sun shadow lookup"),
        entries: &entries,
    })
}

/// The thin casters' coverage maps' layout entries at `first` and `first + 1`: the array and its
/// filtering sampler.
fn thin_entries(first: u32, visibility: wgpu::ShaderStages) -> [wgpu::BindGroupLayoutEntry; 2] {
    [
        wgpu::BindGroupLayoutEntry {
            binding: first,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: first + 1,
            visibility,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        },
    ]
}

/// The quads' WGSL ([`thin_quads`]): placed as given; one, their depth and their band added into the
/// layer's red, green and blue (the band and depth where the cables are), their coverage into its
/// alpha (with a ribbon's, [`Gpu::make_pipeline`]'s coverage pipeline).
const THIN_DRAW: &str = "
struct Q {
    @builtin(position) at: vec4<f32>,
    @location(0) v: vec3<f32>,
};

@vertex fn vs(@location(0) p: vec2<f32>, @location(1) c: vec3<f32>) -> Q {
    var q: Q;
    q.at = vec4<f32>(p, 0.5, 1.0);
    q.v = c;
    return q;
}

@fragment fn fs(q: Q) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, q.v.x, q.v.z, q.v.y);
}
";

/// A map's casters: each one's draw, place, and register offsets (vertex, pixel).
type MapList<'a> = Vec<(&'a Draw, World, [u32; 2])>;

/// The register ring's alignment (wgpu's dynamic uniform offsets).
const ALIGN: usize = 256;

impl Gpu {
    pub(crate) fn new(device: &wgpu::Device) -> Gpu {
        // One texture of a layer a cascade (one binding, one sample instruction a tap), for the
        // maps the lookups read and for the still casters' maps.
        let texture = |label| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: SIZE,
                    height: SIZE,
                    depth_or_array_layers: CASCADES as u32,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: MAP_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        };
        let layers = |t: &wgpu::Texture| -> Vec<wgpu::TextureView> {
            (0..CASCADES as u32)
                .map(|k| {
                    t.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_array_layer: k,
                        array_layer_count: Some(1),
                        ..Default::default()
                    })
                })
                .collect()
        };
        let live = texture("sun shadow maps");
        let statics = texture("sun shadow maps (still casters)");
        let maps = layers(&live);
        let static_views = layers(&statics);
        let array = live.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let compare = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sun shadow compare"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let thin = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sun shadow thin casters"),
            size: wgpu::Extent3d {
                width: THIN_SIZE,
                height: THIN_SIZE,
                depth_or_array_layers: CASCADES as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: THIN_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let thin_views = layers(&thin);
        let thin_array = thin.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let linear = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sun shadow thin casters"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let light_layout = light_layout(device);
        let light_ring = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sun shadow lookup"),
            size: LIGHT_SLOT * LIGHT_SLOTS,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let maps: Vec<wgpu::TextureView> = maps;
        let entries = [
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &light_ring,
                    offset: 0,
                    size: std::num::NonZeroU64::new(LOOKUP_SIZE),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&array),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&compare),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&thin_array),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(&linear),
            },
        ];
        let light_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sun shadow lookup"),
            layout: &light_layout,
            entries: &entries,
        });
        Gpu {
            maps,
            live,
            statics,
            static_views,
            array,
            drawn: [None; CASCADES],
            live_moving: [false; CASCADES],
            thin_views,
            thin_array,
            thin_in: [false; CASCADES],
            thin_moving: [false; CASCADES],
            thin_pipe: None,
            linear,
            thin_quads: None,
            compare,
            light_layout,
            light_ring,
            light_group,
            uniforms: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sun shadow step"),
                size: LOOKUP_SIZE,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            apply: Vec::new(),
            ring: None,
            pipelines: Vec::new(),
        }
    }

    /// A depth-only pipeline drawing with the vertex module `module` (a converted game vertex
    /// shader) from vertex buffers `buffers` (stride and attributes by slot), culling `cull` (the
    /// draw's own: a two-sided material none), in `layout`; with the pixel module `cutout` (an
    /// alpha-tested caster's own, which discards what its alpha test would) or none. With `coverage`
    /// (a ribbon cable's, its pixel module in `cutout`): no depth, its pixel shader's alpha added
    /// into the coverage maps' alpha ([`THIN_FORMAT`]).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn make_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        cache: Option<&wgpu::PipelineCache>,
        module: &wgpu::ShaderModule,
        cutout: Option<&wgpu::ShaderModule>,
        buffers: &[Option<(u64, Vec<wgpu::VertexAttribute>)>],
        topology: wgpu::PrimitiveTopology,
        index32: bool,
        cull: Option<wgpu::Face>,
        coverage: bool,
    ) -> Result<wgpu::RenderPipeline, String> {
        let add = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        // Coverage: the most of what is drawn over a texel (a strand's quads overlap where it
        // bends; a ribbon's triangles at its seams), not their sum.
        let most = wgpu::BlendComponent {
            operation: wgpu::BlendOperation::Max,
            ..add
        };
        let alpha = [Some(wgpu::ColorTargetState {
            format: THIN_FORMAT,
            blend: Some(wgpu::BlendState {
                color: add,
                alpha: most,
            }),
            write_mask: wgpu::ColorWrites::ALPHA,
        })];
        let buffers: Vec<Option<wgpu::VertexBufferLayout>> = buffers
            .iter()
            .map(|b| {
                b.as_ref().map(|(stride, a)| wgpu::VertexBufferLayout {
                    array_stride: *stride,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: a,
                })
            })
            .collect();
        let strip = matches!(
            topology,
            wgpu::PrimitiveTopology::TriangleStrip | wgpu::PrimitiveTopology::LineStrip
        );
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sun shadow caster"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            primitive: wgpu::PrimitiveState {
                topology,
                strip_index_format: strip.then_some(if index32 {
                    wgpu::IndexFormat::Uint32
                } else {
                    wgpu::IndexFormat::Uint16
                }),
                front_face: wgpu::FrontFace::Cw,
                cull_mode: cull,
                // Casters between the sun and the map's near plane are clamped to it, not cut away
                // (where the adapter can).
                unclipped_depth: !coverage
                    && device
                        .features()
                        .contains(wgpu::Features::DEPTH_CLIP_CONTROL),
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: (!coverage).then(|| wgpu::DepthStencilState {
                format: MAP_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState {
                    constant: 0,
                    slope_scale: SLOPE_BIAS,
                    clamp: 0.0,
                },
            }),
            multisample: Default::default(),
            fragment: cutout.map(|m| wgpu::FragmentState {
                module: m,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: if coverage { &alpha } else { &[] },
            }),
            multiview_mask: None,
            cache,
        });
        match crate::device::block_on(scope.pop()) {
            None => Ok(pipeline),
            Some(e) => Err(e.to_string()),
        }
    }

    /// Draws the maps: the still casters' map of each cascade `redraw` names, from the casters in
    /// `cache` that touch it ([`Cascade::touches`]) and do not move ([`Instance::moving`]); then,
    /// for each cascade whose still map was drawn, that a moving caster touches, or whose map held
    /// moving casters the frame before (so a moving shadow that is gone leaves no ghost), the map
    /// the lookups read: a copy of the still map with the moving casters drawn over it.
    /// Each caster through its own pipeline with its `ModelViewProj` replaced by the map's view of
    /// its place; alpha-tested ones (cut-outs) only in the first [`CUTOUT_CASCADES`]. `headers`:
    /// the register blocks' headers. Returns the register ring's bytes, to be written before the
    /// encoder runs (with the thin casters' quads, [`thin_quads`]), and how many caster draws were recorded.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_maps(
        &mut self,
        device: &wgpu::Device,
        registers: &wgpu::BindGroupLayout,
        encoder: &mut wgpu::CommandEncoder,
        cache: &Cache<GeoKey, Draw>,
        headers: &[Vec<u8>],
        cascades: &[Cascade; CASCADES],
        redraw: [bool; CASCADES],
    ) -> (Vec<(wgpu::Buffer, Vec<u8>)>, u32) {
        // Each cascade's still and moving casters touching it, and its ribbon cables.
        let mut lists: Vec<[MapList<'_>; 3]> = (0..CASCADES)
            .map(|_| [Vec::new(), Vec::new(), Vec::new()])
            .collect();
        let thin_maps = thin_maps();
        let kept: Vec<(&Instance<Draw>, usize)> =
            cache.iter().map(|i| (i, usize::from(i.moving))).collect();
        // The cables of each cascade, and whether one moves: a cascade's coverage layer is drawn
        // again with its still map, and each frame while a cable in it moves (or moved the frame
        // before).
        let mut thin: Vec<Vec<&Instance<Draw>>> = (0..CASCADES).map(|_| Vec::new()).collect();
        let mut thin_moving = [false; CASCADES];
        for (k, c) in cascades.iter().enumerate() {
            for &(i, m) in &kept {
                if i.payload.thin.is_some() && thin_maps && c.touches(i.centre, i.radius) {
                    thin[k].push(i);
                    thin_moving[k] |= m == 1;
                }
            }
        }
        let thin_redraw: [bool; CASCADES] =
            std::array::from_fn(|k| redraw[k] || thin_moving[k] || self.thin_moving[k]);
        self.thin_moving = thin_moving;
        for (k, c) in cascades.iter().enumerate() {
            for &(i, m) in &kept {
                let d = &i.payload;
                if d.thin.is_some() && thin_maps && !d.rest {
                    // A ribbon's alpha, in the cascades cut-outs are drawn into.
                    if let (true, true, Some((p, _))) =
                        (thin_redraw[k], k < CUTOUT_CASCADES, d.ribbon)
                    {
                        if c.touches(i.centre, i.radius) && self.pipelines[p as usize].is_some() {
                            lists[k][2].push((d, i.world, [0, 0]));
                        }
                    }
                    continue;
                }
                if (m == 0 && !redraw[k])
                    || (d.cutout.is_some() && k >= CUTOUT_CASCADES)
                    || !c.touches(i.centre, i.radius)
                    || self.pipelines[d.pipe as usize].is_none()
                {
                    continue;
                }
                lists[k][m].push((d, i.world, [0, 0]));
            }
        }
        let live: [bool; CASCADES] = std::array::from_fn(|k| {
            redraw[k] || !lists[k][1].is_empty() || self.live_moving[k] || self.drawn[k].is_none()
        });
        self.live_moving = std::array::from_fn(|k| !lists[k][1].is_empty());
        // The cables' quads (their depth, width and a round cable's coverage).
        let mut writes = Vec::new();
        let mut quads: Vec<[f32; 5]> = Vec::new();
        let mut thin_ranges: [std::ops::Range<u32>; CASCADES] = std::array::from_fn(|_| 0..0);
        for k in 0..CASCADES {
            if thin_redraw[k] {
                let from = quads.len();
                for i in &thin[k] {
                    let t = i.payload.thin.as_ref().expect("listed as thin");
                    thin_quads(
                        &cascades[k],
                        &i.world,
                        &t.strands,
                        t.ribbons,
                        &mut quads,
                    );
                }
                thin_ranges[k] = from as u32..quads.len() as u32;
                self.thin_in[k] = !thin_ranges[k].is_empty();
            }
        }
        let any_lists = lists.iter().flatten().any(|l| !l.is_empty());
        if quads.is_empty() && !any_lists && !live.iter().any(|&l| l) {
            return (writes, 0);
        }
        // The ring: every header once, then each caster's float constants for each map it is
        // drawn into (its matrix in place), and a cut-out's pixel constants.
        let mut bytes = Vec::new();
        let mut header_at = Vec::with_capacity(headers.len());
        for h in headers {
            debug_assert_eq!(h.len(), wgsl_header_size());
            header_at.push(bytes.len() as u32);
            bytes.extend_from_slice(h);
            bytes.resize(bytes.len().div_ceil(ALIGN) * ALIGN, 0);
        }
        for (c, all) in cascades.iter().zip(lists.iter_mut()) {
            for (n, list) in all.iter_mut().enumerate() {
                for (d, world, at) in list.iter_mut() {
                    let vs = bytes.len();
                    bytes.extend_from_slice(&d.consts);
                    let m0 = vs + 16 * d.mvp as usize;
                    for (r, row) in c.world_to_light(world).iter().enumerate() {
                        for (j, v) in row.iter().enumerate() {
                            let o = m0 + 16 * r + 4 * j;
                            bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
                        }
                    }
                    bytes.resize(bytes.len().div_ceil(ALIGN) * ALIGN, 0);
                    let mut ps = vs;
                    if let Some(cut) = &d.cutout {
                        ps = bytes.len();
                        bytes.extend_from_slice(&cut.consts);
                        bytes.resize(bytes.len().div_ceil(ALIGN) * ALIGN, 0);
                    }
                    *at = [vs as u32, ps as u32];
                }
                // Fewer pipeline changes.
                if n == 2 {
                    list.sort_by_key(|(d, _, _)| d.ribbon.map_or(0, |r| r.0));
                } else {
                    list.sort_by_key(|(d, _, _)| d.pipe);
                }
            }
        }
        // The floats' binding reads a whole register file past its offset.
        let need = (bytes.len() + wgsl::CONSTS_SIZE as usize) as u64;
        if !matches!(&self.ring, Some((_, _, cap)) if *cap >= need) {
            let cap = need.next_power_of_two().max(1 << 20);
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sun shadow registers"),
                size: cap,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sun shadow registers"),
                layout: registers,
                entries: &[0u32, 1, 2, 3].map(|binding| wgpu::BindGroupEntry {
                    binding,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(if binding < 2 {
                            wgsl::REGS_C_OFFSET
                        } else {
                            wgsl::CONSTS_SIZE
                        }),
                    }),
                }),
            });
            self.ring = Some((buffer, group, cap));
        }
        let (buffer, group, _) = self.ring.as_ref().expect("made above");
        let mut drawn = 0;
        let pipelines = &self.pipelines;
        // A list's draws: through each one's depth pipeline, or a ribbon's coverage pipeline
        // (`ribbons`) with its own pixel block header.
        let mut encode =
            |pass: &mut wgpu::RenderPass<'_>, list: &[(&Draw, World, [u32; 2])], ribbons: bool| {
                let mut last_pipe = u32::MAX;
                for &(d, _, [vs, ps]) in list {
                    let (pipe, ribbon_header) = match (ribbons, d.ribbon) {
                        (true, Some((p, h))) => (p, Some(h)),
                        _ => (d.pipe, None),
                    };
                    if pipe != last_pipe {
                        let p = pipelines[pipe as usize].as_ref().expect("listed with one");
                        pass.set_pipeline(p);
                        last_pipe = pipe;
                    }
                    let h = header_at[d.header as usize];
                    let ph = match ribbon_header {
                        Some(r) => header_at[r as usize],
                        None => d
                            .cutout
                            .as_ref()
                            .map_or(h, |c| header_at[c.header as usize]),
                    };
                    pass.set_bind_group(0, group, &[h, ph, vs, ps]);
                    if let Some(cut) = &d.cutout {
                        pass.set_bind_group(1, &cut.textures, &[]);
                        pass.set_bind_group(2, &cut.empty, &[]);
                    }
                    for (slot, s) in d.streams.iter().enumerate() {
                        if let Some((b, offset, _)) = s {
                            pass.set_vertex_buffer(slot as u32, b.buffer.slice(*offset..));
                        }
                    }
                    match &d.index {
                        Some((b, _)) => {
                            pass.set_index_buffer(
                                b.buffer.slice(..),
                                if b.index32 {
                                    wgpu::IndexFormat::Uint32
                                } else {
                                    wgpu::IndexFormat::Uint16
                                },
                            );
                            pass.draw_indexed(d.start..d.start + d.count, d.base_vertex, 0..1);
                        }
                        None => pass.draw(d.start..d.start + d.count, 0..1),
                    }
                    drawn += 1;
                }
            };
        let depth_pass = |encoder: &mut wgpu::CommandEncoder,
                          view: &wgpu::TextureView,
                          clear: bool|
         -> wgpu::RenderPass<'static> {
            encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("sun shadow map"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: if clear {
                                wgpu::LoadOp::Clear(1.0)
                            } else {
                                wgpu::LoadOp::Load
                            },
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime()
        };
        for k in 0..CASCADES {
            if redraw[k] {
                let mut pass = depth_pass(encoder, &self.static_views[k], true);
                encode(&mut pass, &lists[k][0], false);
                drop(pass);
                self.drawn[k] = Some(cascades[k]);
            }
            if live[k] {
                let layer = |t| wgpu::TexelCopyTextureInfo {
                    texture: t,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: k as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                };
                encoder.copy_texture_to_texture(
                    layer(&self.statics),
                    layer(&self.live),
                    wgpu::Extent3d {
                        width: SIZE,
                        height: SIZE,
                        depth_or_array_layers: 1,
                    },
                );
                if !lists[k][1].is_empty() {
                    let mut pass = depth_pass(encoder, &self.maps[k], false);
                    encode(&mut pass, &lists[k][1], false);
                }
            }
        }
        // The coverage layers: the cables' quads, then the ribbons' alpha added over them.
        if !quads.is_empty() {
            let need = (quads.len() * 20) as u64;
            if !matches!(&self.thin_quads, Some(b) if b.size() >= need) {
                self.thin_quads = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("sun shadow thin casters"),
                    size: need.next_power_of_two().max(1 << 16),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
            }
            let pipe = self
                .thin_pipe
                .get_or_insert_with(|| Self::make_thin_pipeline(device));
            let quad_buffer = self.thin_quads.as_ref().expect("made above");
            for (k, (view, range)) in self.thin_views.iter().zip(&thin_ranges).enumerate() {
                if range.is_empty() {
                    continue;
                }
                let mut pass = encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("sun shadow thin casters"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    })
                    .forget_lifetime();
                pass.set_pipeline(pipe);
                pass.set_vertex_buffer(0, quad_buffer.slice(..));
                pass.draw(range.clone(), 0..1);
                encode(&mut pass, &lists[k][2], true);
            }
            writes.push((
                quad_buffer.clone(),
                quads
                    .iter()
                    .flatten()
                    .flat_map(|v| v.to_le_bytes())
                    .collect(),
            ));
        }
        writes.push((buffer.clone(), bytes));
        (writes, drawn)
    }

    /// The pipeline that draws the thin casters' quads into a coverage layer, added up.
    fn make_thin_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sun shadow thin casters"),
            source: wgpu::ShaderSource::Wgsl(THIN_DRAW.into()),
        });
        let add = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        // Coverage: the most of what is drawn over a texel (a strand's quads overlap where it
        // bends; a ribbon's triangles at its seams), not their sum.
        let most = wgpu::BlendComponent {
            operation: wgpu::BlendOperation::Max,
            ..add
        };
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sun shadow thin casters"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3],
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: THIN_FORMAT,
                    blend: Some(wgpu::BlendState {
                        color: add,
                        alpha: most,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
    }

    /// The step's uniform buffer.
    pub(crate) fn uniforms(&self) -> wgpu::Buffer {
        self.uniforms.clone()
    }

    /// Records the debug step: the scene's colour darkened where the maps say the sun is blocked.
    pub(crate) fn encode_step(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        t: &Targets<'_>,
    ) {
        let ms = t.samples > 1;
        let key = (ms, t.colour_format, t.samples);
        if !self.apply.iter().any(|(k, _, _)| *k == key) {
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
                        multisampled: ms,
                    },
                    count: None,
                },
            ];
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            });
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            });
            entries.extend(thin_entries(4, wgpu::ShaderStages::FRAGMENT));
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sun shadow step"),
                entries: &entries,
            });
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("sun shadow step"),
                source: wgpu::ShaderSource::Wgsl(step_shader(ms).into()),
            });
            let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("sun shadow step"),
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
                    count: t.samples,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    // The scene's colour times the source; its alpha kept (the cascades view
                    // replaces it).
                    targets: &[Some(wgpu::ColorTargetState {
                        format: t.colour_format,
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
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
            self.apply.push((key, layout, pipeline));
        }
        let (_, layout, pipeline) = self.apply.iter().find(|(k, _, _)| *k == key).expect("made");
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
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
                    resource: wgpu::BindingResource::TextureView(&self.array),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.compare),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.thin_array),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.linear),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sun shadow step"),
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
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
}

use crate::sm::wgsl;

/// The size of a vertex block's header (`Regs` up to `c`).
pub(crate) fn wgsl_header_size() -> usize {
    wgsl::REGS_C_OFFSET as usize
}

/// The first register of `ModelViewProj` in a vertex shader's constant table, when it has one of
/// four registers and no skinning matrices (`SkinModelViewProj`, `Bones`).
pub fn model_view_proj(p: &crate::sm::Program) -> Option<u32> {
    let skinned = p
        .constants
        .iter()
        .any(|c| c.name.starts_with("Skin") || c.name.starts_with("Bones"));
    if skinned {
        return None;
    }
    p.constants
        .iter()
        .find(|c| c.name == "ModelViewProj" && c.set == 2 && c.count == 4)
        .map(|c| u32::from(c.index))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> scene_light::Params {
        let r = (37.5f32).to_radians().tan();
        let t = r * 9.0 / 16.0;
        scene_light::Params {
            near: 10.0,
            far: 350_000.0,
            left: -r,
            right: r,
            bottom: -t,
            top: t,
            sun: [0.0, 1.0, 0.0],
            shadow: 0.8,
            strength: scene_light::STRENGTH,
        }
    }

    fn game() -> scene_light::GameView {
        scene_light::GameView {
            frustum: [-0.77, 0.77, 0.43, -0.43, 10.0, 350_000.0],
            ortho: false,
            rotate: [0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            position: [-70_000.0, 2_000.0, 8_000.0],
            to_sun: Some([0.3, 0.2, 0.9]),
            sun_colour: [1.0, 0.9, 0.8],
            interior: false,
        }
    }

    /// The maps look along the light the lit shaders use (the game view's `to_sun`, the way toward
    /// the sun light), at the traced 17:30 direction: from the west, north and 11 degrees up.
    #[test]
    #[allow(clippy::needless_range_loop)]
    fn the_maps_look_along_the_lights_own_direction() {
        let light = [-0.9644f32, 0.1869, 0.1869];
        let g = scene_light::GameView {
            to_sun: Some(light),
            ..game()
        };
        let f = Frame::from_game(&g).unwrap();
        let len = light.iter().map(|v| v * v).sum::<f32>().sqrt();
        for k in 0..3 {
            assert!(
                (f.to_sun[k] - light[k] / len).abs() < 1e-6,
                "{:?}",
                f.to_sun
            );
        }
        let b = basis(wide(f.to_sun));
        for k in 0..3 {
            assert!((b[2][k] + f64::from(f.to_sun[k])).abs() < 1e-6, "{b:?}");
        }
        let mut keep = SunKeep::default();
        keep.take(f.to_sun);
        let kept = keep.direction(f.to_sun);
        for k in 0..3 {
            assert!((kept[k] - f64::from(f.to_sun[k])).abs() < 1e-6, "{kept:?}");
        }
    }

    fn frustum() -> [f32; 4] {
        let v = view();
        [v.left, v.right, v.bottom, v.top]
    }

    #[test]
    fn the_splits_blend_log_and_even() {
        let s = splits(10.0, 8000.0);
        assert_eq!(s[0], 10.0);
        assert_eq!(s[CASCADES], 8000.0);
        for i in 1..CASCADES {
            let t = i as f32 / CASCADES as f32;
            let log = 10.0 * 800f32.powf(t);
            let even = 10.0 + 7990.0 * t;
            assert!((s[i] - (0.75 * log + 0.25 * even)).abs() < 0.01, "{s:?}");
            assert!(s[i] > s[i - 1]);
        }
        // Nearer than even, farther than logarithmic.
        assert!(s[1] < 10.0 + 7990.0 / 3.0 && s[1] > 10.0 * 800f32.powf(1.0 / 3.0));
    }

    #[test]
    fn the_slice_sphere_holds_the_slice_and_is_least_on_its_axis() {
        let f = frustum();
        for (zn, zf) in [(10.0, 700.0), (700.0, 2000.0), (2000.0, 8000.0)] {
            let (c, r) = slice_sphere(f, zn, zf);
            for z in [zn, zf] {
                for (x, y) in [(f[0], f[2]), (f[1], f[2]), (f[0], f[3]), (f[1], f[3])] {
                    let p = [f64::from(x * z), f64::from(y * z), f64::from(z)];
                    let d = ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2))
                        .sqrt();
                    assert!(d <= r * (1.0 + 1e-6), "{zn} {zf}: corner {d} > {r}");
                }
            }
            // Moving the centre along the axis makes the bound larger.
            for dz in [-5.0, 5.0] {
                let c2 = [c[0], c[1], c[2] + dz];
                let worst = [zn, zf]
                    .iter()
                    .flat_map(|&z| {
                        [(f[0], f[2]), (f[1], f[3])].map(|(x, y)| {
                            let p = [f64::from(x * z), f64::from(y * z), f64::from(z)];
                            ((p[0] - c2[0]).powi(2)
                                + (p[1] - c2[1]).powi(2)
                                + (p[2] - c2[2]).powi(2))
                            .sqrt()
                        })
                    })
                    .fold(0.0, f64::max);
                assert!(
                    worst >= r - 1e-6 || c[2] + dz < f64::from(zn) || c[2] + dz > f64::from(zf)
                );
            }
        }
    }

    #[test]
    fn the_basis_is_left_handed_as_the_camera_and_looks_from_the_sun() {
        for to_sun in [
            [0.3, 0.2, 0.9],
            [0.0, 0.0, 1.0],
            [0.0, 0.99, 0.1],
            [-0.8, 0.1, 0.2],
        ] {
            let b = basis(normalize(to_sun));
            for i in 0..3 {
                assert!((dot3(b[i], b[i]) - 1.0).abs() < 1e-9);
                for j in 0..i {
                    assert!(dot3(b[i], b[j]).abs() < 1e-9);
                }
            }
            assert!((dot3(cross(b[0], b[1]), b[2]) + 1.0).abs() < 1e-9);
            // z points away from the sun.
            assert!(dot3(b[2], normalize(to_sun)) < -0.999);
        }
    }

    fn cascades_of(f: &Frame, b: [[f64; 3]; 3]) -> Vec<Cascade> {
        let s = splits(view().near, FAR);
        (0..CASCADES)
            .map(|i| place(i, frustum(), s[i], s[i + 1], f.axes(), wide(f.position), b))
            .collect()
    }

    #[test]
    fn a_world_point_lands_where_the_sun_sees_it() {
        let f = Frame::from_game(&game()).unwrap();
        let b = basis(wide(f.to_sun));
        for c in cascades_of(&f, b) {
            // Two points on one sun ray land on one texel, the one nearer the sun nearer in depth.
            let p_view = [30.0, -20.0, f64::from(c.near + 5.0)];
            let p = apply(&c.view_to_light, p_view);
            let sun = wide(f.to_sun);
            // The same point 100 units toward the sun, in view space.
            let a = f.axes();
            let toward: [f64; 3] = std::array::from_fn(|v| 100.0 * dot3(a[v], sun));
            let q_view = [
                p_view[0] + toward[0],
                p_view[1] + toward[1],
                p_view[2] + toward[2],
            ];
            let q = apply(&c.view_to_light, q_view);
            assert!(
                (p[0] - q[0]).abs() < 1e-9 && (p[1] - q[1]).abs() < 1e-9,
                "{p:?} {q:?}"
            );
            assert!(q[2] < p[2]);
            assert!((p[2] - q[2] - 100.0 / (2.0 * DEPTH_RANGE)).abs() < 1e-9);
            // In the map (the slice's points fall inside it).
            assert!(p[0].abs() < 1.0 && p[1].abs() < 1.0 && (0.0..1.0).contains(&p[2]));
            // A texel is 2 / size of clip space, `texel` units of the world.
            let along: [f64; 3] = std::array::from_fn(|v| c.texel * dot3(a[v], b[0]));
            let r = apply(
                &c.view_to_light,
                [
                    p_view[0] + along[0],
                    p_view[1] + along[1],
                    p_view[2] + along[2],
                ],
            );
            assert!(((r[0] - p[0]) - 2.0 / f64::from(c.size)).abs() < 1e-9);
        }
    }

    #[test]
    #[allow(clippy::needless_range_loop)]
    fn the_centre_moves_in_whole_texel_steps_and_holds_still_between_them() {
        let mut g = game();
        let f0 = Frame::from_game(&g).unwrap();
        let b = basis(wide(f0.to_sun));
        let c0 = cascades_of(&f0, b);
        // The snapped centre is within half a step of the slice's own centre (in the sun's space):
        // the map covers the slice, it is only rounded to whole steps.
        for (i, c) in c0.iter().enumerate() {
            let (centre_view, _) = slice_sphere(frustum(), c.near, c.far);
            let axes = f0.axes();
            let world: [f64; 3] = std::array::from_fn(|k| {
                f64::from(f0.position[k])
                    + axes[0][k] * centre_view[0]
                    + axes[1][k] * centre_view[1]
                    + axes[2][k] * centre_view[2]
            });
            for k in 0..3 {
                let off = dot3(b[k], world) - c.origin[k] as f64 * c.step;
                assert!(
                    off.abs() <= c.step / 2.0 + 1e-6,
                    "cascade {i} axis {k}: {off}"
                );
            }
        }
        // A small step of the camera: the same snapped centre, so each map is still good.
        g.position[0] += 0.3;
        let f1 = Frame::from_game(&g).unwrap();
        let c1 = cascades_of(&f1, b);
        for (a, z) in c0.iter().zip(&c1) {
            assert!(still_good(Some(a), z, false), "{a:?} {z:?}");
            // A world point lands on the same map spot as before (the map did not move).
            let p_world = [-69_000.0, 2_500.0, 7_990.0];
            let to_view = |f: &Frame, p: [f64; 3]| {
                let a = f.axes();
                let d: [f64; 3] = std::array::from_fn(|k| p[k] - f64::from(f.position[k]));
                [dot3(a[0], d), dot3(a[1], d), dot3(a[2], d)]
            };
            let pa = apply(&a.view_to_light, to_view(&f0, p_world));
            let pz = apply(&z.view_to_light, to_view(&f1, p_world));
            for k in 0..3 {
                assert!((pa[k] - pz[k]).abs() < 1e-6, "{pa:?} {pz:?}");
            }
        }
        // A long step: the centre moves by whole steps; the map's texels stay on the same grid
        // (a world point's texel position changes by a whole number of texels).
        g.position[0] += 900.0;
        let f2 = Frame::from_game(&g).unwrap();
        let c2 = cascades_of(&f2, b);
        assert!(!still_good(Some(&c0[0]), &c2[0], false));
        for (a, z) in c0.iter().zip(&c2) {
            let p_world = [-69_400.0, 2_100.0, 8_000.0];
            let to_view = |f: &Frame, p: [f64; 3]| {
                let a = f.axes();
                let d: [f64; 3] = std::array::from_fn(|k| p[k] - f64::from(f.position[k]));
                [dot3(a[0], d), dot3(a[1], d), dot3(a[2], d)]
            };
            let pa = apply(&a.view_to_light, to_view(&f0, p_world));
            let pz = apply(&z.view_to_light, to_view(&f2, p_world));
            for k in 0..2 {
                let texels = (pa[k] - pz[k]) * f64::from(a.size) / 2.0;
                assert!((texels - texels.round()).abs() < 1e-4, "{texels}");
                assert!(
                    ((texels.round() / GRID_TEXELS) - (texels.round() / GRID_TEXELS).round()).abs()
                        < 1e-9
                );
            }
        }
        // The sun turned: every map is drawn again.
        assert!(!still_good(Some(&c0[0]), &c0[0], true));
        assert!(!still_good(None, &c0[0], false));
    }

    #[test]
    fn the_cascades_size_does_not_change_as_the_camera_turns() {
        let mut g = game();
        let f0 = Frame::from_game(&g).unwrap();
        let b = basis(wide(f0.to_sun));
        let c0 = cascades_of(&f0, b);
        // Turned 90 degrees about z: looking along -x.
        g.rotate = [-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let f1 = Frame::from_game(&g).unwrap();
        let c1 = cascades_of(&f1, b);
        for (a, z) in c0.iter().zip(&c1) {
            assert_eq!(a.texel.to_bits(), z.texel.to_bits());
            assert_eq!(a.size, z.size);
        }
    }

    #[test]
    fn the_sun_is_kept_and_its_turn_measured() {
        let mut k = SunKeep::default();
        assert_eq!(k.turn([0.0, 0.0, 1.0]), None);
        assert_eq!(k.direction([0.0, 0.0, 2.0]), [0.0, 0.0, 1.0]);
        k.take([0.0, 0.0, 1.0]);
        let a = (0.2f32).to_radians();
        let t = k.turn([a.sin(), 0.0, a.cos()]).unwrap();
        assert!((t - f64::from(a)).abs() < 1e-6);
        assert_eq!(k.direction([a.sin(), 0.0, a.cos()]), [0.0, 0.0, 1.0]);
    }

    /// The turn rule: at the limit a shadow edge's end moves by half the map's texel (no visible
    /// step); the near map's limit is the smallest; a low sun shrinks it.
    #[test]
    fn a_sun_turn_at_the_limit_moves_an_edge_half_a_texel() {
        for (texel, sine) in [(0.83, 0.79), (3.78, 0.79), (12.8, 0.5), (0.83, 0.17)] {
            let limit = turn_limit(texel, sine);
            let edge = SHADOW_REACH * limit / (sine * sine);
            assert!((edge - 0.5 * texel).abs() < 1e-9, "{texel} {sine}: {edge}");
        }
        assert!(turn_limit(0.83, 0.79) < turn_limit(12.8, 0.79));
        assert!(turn_limit(0.83, 0.3) < turn_limit(0.83, 0.79));
        // Below the lowest sun counted the limit stops shrinking.
        assert_eq!(turn_limit(0.83, 0.01), turn_limit(0.83, TURN_MIN_SUN_SINE));
        // At the street's 16:00 sun the near map follows a turn of about a hundredth of a degree.
        assert!(turn_limit(0.83, 0.79).to_degrees() < 0.01);
    }

    /// One map a frame takes the sun (the furthest past its limit); none under its limit; every
    /// map never drawn or far overdue at once.
    #[test]
    fn the_maps_take_the_sun_one_a_frame_unless_overdue() {
        assert_eq!(sun_turns([0.5, 0.9, 0.2, 0.0]), [false; 4]);
        assert_eq!(sun_turns([1.5, 2.5, 1.2, 0.0]), [false, true, false, false]);
        assert_eq!(sun_turns([1.0, 0.0, 0.0, 0.0]), [true, false, false, false]);
        assert_eq!(
            sun_turns([f64::INFINITY; 4]),
            [true, true, true, true],
            "the first frame draws every map"
        );
        assert_eq!(
            sun_turns([OVERDUE, 3.9, OVERDUE * 2.0, 1.0]),
            [true, false, true, false]
        );
        // Under a steady turn every map is drawn in its turn: none waits more than CASCADES frames.
        let limits = [1.0, 2.0, 4.0, 8.0];
        let mut kept = [0.0f64; 4];
        let mut waited = [0u32; 4];
        for frame in 1..400 {
            let sun = f64::from(frame) * 0.3;
            let over: [f64; 4] = std::array::from_fn(|k| (sun - kept[k]) / limits[k]);
            let take = sun_turns(over);
            for k in 0..4 {
                if take[k] {
                    kept[k] = sun;
                    waited[k] = 0;
                } else if over[k] >= 1.0 {
                    waited[k] += 1;
                    assert!(
                        waited[k] <= CASCADES as u32,
                        "map {k} waited at frame {frame}"
                    );
                }
            }
        }
    }

    /// A caster's matrix through the camera's projection gives what the sun sees of it.
    #[test]
    fn a_caster_drawn_by_the_camera_is_redrawn_from_the_sun() {
        let v = view();
        let f = Frame::from_game(&game()).unwrap();
        let b = basis(wide(f.to_sun));
        let c = cascades_of(&f, b)[0];
        // An off-centre frustum (as a view cut for one eye would be), so each term counts.
        let off_centre = [-0.6f32, 0.9, -0.4, 0.5];
        let [l, r, bt, t] = off_centre.map(f64::from);
        let (n, far) = (f64::from(v.near), f64::from(v.far));
        // The camera's projection (Direct3D: depth 0 to 1, y up).
        let proj: M4 = [
            [2.0 / (r - l), 0.0, -(r + l) / (r - l), 0.0],
            [0.0, 2.0 / (t - bt), -(t + bt) / (t - bt), 0.0],
            [0.0, 0.0, far / (far - n), -n * far / (far - n)],
            [0.0, 0.0, 1.0, 0.0],
        ];
        // An object turned, scaled and placed in view space.
        let (sa, ca) = (0.6f64.sin(), 0.6f64.cos());
        let object_to_view: M4 = [
            [2.0 * ca, -2.0 * sa, 0.0, 40.0],
            [2.0 * sa, 2.0 * ca, 0.0, -30.0],
            [0.0, 0.0, 2.0, 300.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let mvp = mul(&proj, &object_to_view).map(|row| row.map(|v| v as f32));
        let world = world_of(&mvp, off_centre, f.axes(), wide(f.position));
        let m = c.world_to_light(&world);
        for p in [[0.0, 0.0, 0.0], [1.0, 2.0, 3.0], [-5.0, 0.5, 10.0]] {
            let want = apply(&c.view_to_light, {
                let q = apply(&object_to_view, p);
                [q[0], q[1], q[2]]
            });
            let got: [f64; 4] = std::array::from_fn(|i| {
                f64::from(m[i][0]) * p[0]
                    + f64::from(m[i][1]) * p[1]
                    + f64::from(m[i][2]) * p[2]
                    + f64::from(m[i][3])
            });
            for i in 0..4 {
                assert!(
                    (want[i] - got[i]).abs() < 1e-5,
                    "{p:?} {i}: {want:?} {got:?}"
                );
            }
            assert!((got[3] - 1.0).abs() < 1e-6);
        }
    }

    /// A map kept from an earlier frame is looked up through this frame's camera: a world point
    /// lands where the map drew it, whichever camera looks.
    #[test]
    fn a_kept_map_is_looked_up_through_this_frames_camera() {
        let mut g = game();
        let f0 = Frame::from_game(&g).unwrap();
        let b = basis(wide(f0.to_sun));
        let c0 = cascades_of(&f0, b)[1];
        g.position[1] += 0.7;
        g.rotate = [-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let f1 = Frame::from_game(&g).unwrap();
        let p_world = [-69_300.0, 2_200.0, 7_995.0];
        let to_view = |f: &Frame, p: [f64; 3]| {
            let a = f.axes();
            let d: [f64; 3] = std::array::from_fn(|k| p[k] - f64::from(f.position[k]));
            [dot3(a[0], d), dot3(a[1], d), dot3(a[2], d)]
        };
        let drawn = apply(&c0.view_to_light, to_view(&f0, p_world));
        let looked = apply(
            &c0.view_to_light_for(f1.axes(), wide(f1.position)),
            to_view(&f1, p_world),
        );
        // And an object placed there by `world_to_light`.
        let w: World = [
            [1.0, 0.0, 0.0, p_world[0]],
            [0.0, 1.0, 0.0, p_world[1]],
            [0.0, 0.0, 1.0, p_world[2]],
        ];
        let m = c0.world_to_light(&w);
        for k in 0..3 {
            assert!((drawn[k] - looked[k]).abs() < 1e-6, "{drawn:?} {looked:?}");
            assert!(
                (drawn[k] - f64::from(m[k][3])).abs() < 1e-5,
                "{drawn:?} {m:?}"
            );
        }
    }

    /// A caster touches a map when its sphere overlaps the map's square across the sun's rays,
    /// however far along them.
    #[test]
    fn a_caster_touches_the_maps_it_can_cast_into() {
        let f = Frame::from_game(&game()).unwrap();
        let b = basis(wide(f.to_sun));
        let c = cascades_of(&f, b)[0];
        let centre: [f64; 3] =
            std::array::from_fn(|k| (0..3).map(|j| b[j][k] * c.origin[j] as f64 * c.step).sum());
        assert!(c.touches(centre, 1.0));
        // Along the sun's ray, far toward the sun: still touches.
        let up: [f64; 3] = std::array::from_fn(|k| centre[k] - 5000.0 * b[2][k]);
        assert!(c.touches(up, 1.0));
        // Across: just outside the square, then reaching into it by its radius.
        let side: [f64; 3] = std::array::from_fn(|k| centre[k] + (c.half + 10.0) * b[0][k]);
        assert!(!c.touches(side, 5.0));
        assert!(c.touches(side, 15.0));
    }

    #[test]
    fn a_sphere_wholly_in_view_is_inside_all_four_sides_and_the_depths() {
        let f = frustum();
        assert!(wholly_in_view(f, [0.0, 0.0, 500.0], 50.0, 10.0, 8000.0));
        // At the edge: the right side at x = 0.767 z.
        let edge = f64::from(f[1]) * 500.0;
        assert!(!wholly_in_view(
            f,
            [edge - 10.0, 0.0, 500.0],
            50.0,
            10.0,
            8000.0
        ));
        assert!(!wholly_in_view(
            f,
            [-edge + 10.0, 0.0, 500.0],
            50.0,
            10.0,
            8000.0
        ));
        let top = f64::from(f[3]) * 500.0;
        assert!(!wholly_in_view(
            f,
            [0.0, top - 10.0, 500.0],
            50.0,
            10.0,
            8000.0
        ));
        assert!(!wholly_in_view(
            f,
            [0.0, -top + 10.0, 500.0],
            50.0,
            10.0,
            8000.0
        ));
        // Behind, too near, too far.
        assert!(!wholly_in_view(f, [0.0, 0.0, -500.0], 50.0, 10.0, 8000.0));
        assert!(!wholly_in_view(f, [0.0, 0.0, 40.0], 50.0, 10.0, 8000.0));
        assert!(!wholly_in_view(f, [0.0, 0.0, 7990.0], 50.0, 10.0, 8000.0));
    }

    #[test]
    fn the_bounds_hold_the_points() {
        let pts = [[1.0f32, 2.0, 3.0], [-1.0, 0.0, 5.0], [3.0, 2.0, 3.0]];
        let (c, r) = bounds(pts.into_iter()).unwrap();
        assert_eq!(c, [1.0, 1.0, 4.0]);
        for p in pts {
            let d = [
                f64::from(p[0]) - c[0],
                f64::from(p[1]) - c[1],
                f64::from(p[2]) - c[2],
            ];
            assert!(dot3(d, d).sqrt() <= r + 1e-9);
        }
        assert!((r - (4.0f64 + 1.0 + 1.0).sqrt()).abs() < 1e-9);
        assert_eq!(bounds(std::iter::empty()), None);
        assert_eq!(bounds([[f32::NAN, 0.0, 0.0]].into_iter()), None);
    }

    /// A tube of `sides` faces, `radius` and `length` along x, as triangles.
    fn tube(radius: f32, length: f32, sides: usize) -> Vec<[[f32; 3]; 3]> {
        let ring = |i: usize, x: f32| {
            let a = i as f32 * std::f32::consts::TAU / sides as f32;
            [x, radius * a.cos(), radius * a.sin()]
        };
        (0..sides)
            .flat_map(|i| {
                let (a0, a1) = (ring(i, 0.0), ring(i + 1, 0.0));
                let (b0, b1) = (ring(i, length), ring(i + 1, length));
                [[a0, b0, a1], [a1, b0, b1]]
            })
            .collect()
    }

    /// `t` moved by `d`.
    fn moved(t: Vec<[[f32; 3]; 3]>, d: [f32; 3]) -> Vec<[[f32; 3]; 3]> {
        t.into_iter()
            .map(|tri| tri.map(|p| [p[0] + d[0], p[1] + d[1], p[2] + d[2]]))
            .collect()
    }

    #[test]
    fn cables_are_thin_and_a_pole_a_sign_and_a_bolt_are_not() {
        // A cable: 1.5 units round, 800 long: one strand as wide as the cable, along it.
        let s = thin_shape(&tube(1.5, 800.0, 8)).expect("a cable").strands;
        assert_eq!(s.len(), 1);
        assert!((s[0].width - 3.0).abs() < 0.01, "{}", s[0].width);
        assert!((s[0].length - 800.0).abs() < 1.0);
        let (a, b) = (s[0].line[0], s[0].line[s[0].line.len() - 1]);
        assert!(a[0].min(b[0]).abs() < 0.01 && (a[0].max(b[0]) - 800.0).abs() < 0.01);
        assert!(s[0]
            .line
            .iter()
            .all(|p| p[1].abs() < 0.01 && p[2].abs() < 0.01));
        // What the old rule (area over length under 12) missed: a cable 6 wide, and three cables
        // side by side in one geometry (Goodsprings' telegraph line).
        assert!(thin_shape(&tube(3.0, 1200.0, 6)).is_some());
        let mut three = tube(2.0, 1200.0, 6);
        three.extend(moved(tube(2.0, 1200.0, 6), [0.0, 40.0, 0.0]));
        three.extend(moved(tube(2.0, 1200.0, 6), [0.0, 80.0, 0.0]));
        assert_eq!(thin_shape(&three).expect("three cables").strands.len(), 3);
        // A degenerate triangle (a strip joining its runs) joins nothing.
        let mut joined = tube(2.0, 1200.0, 6);
        let (p, q) = (joined[0][0], [0.0, 40.0 + 2.0, 0.0]);
        joined.push([p, p, q]);
        joined.extend(moved(tube(2.0, 1200.0, 6), [0.0, 40.0, 0.0]));
        assert_eq!(strands(&joined).len(), 2);
        // A telegraph pole: 15 round, 700 long.
        assert!(thin_shape(&tube(15.0, 700.0, 8)).is_none());
        // A pole with its wire in one geometry: drawn as a whole, as before.
        let mut pole = tube(15.0, 700.0, 8);
        pole.extend(moved(tube(1.5, 800.0, 8), [0.0, 0.0, 300.0]));
        assert_eq!(strands(&pole).len(), 2);
        assert!(thin_shape(&pole).is_none());
        // ... but its wire is taken out of it (as Goodsprings' telegraph line, poles and wires one
        // geometry): cast as a cable, the pole's triangles (the first 16) drawn on their own.
        let body = Body::of(&pole).unwrap();
        let t = body.thin.expect("its wire");
        assert!(!t.ribbons && t.strands.len() == 1 && t.strands[0].width < 4.0);
        assert_eq!(*body.solid.unwrap(), (0..16).collect::<Vec<u32>>());
        // A sign or a fence: a flat strip 100 wide, 800 long (two triangles).
        let quad = [
            [[0.0, 0.0, 0.0], [800.0, 0.0, 0.0], [0.0, 100.0, 0.0]],
            [[800.0, 0.0, 0.0], [800.0, 100.0, 0.0], [0.0, 100.0, 0.0]],
        ];
        assert!(thin_shape(&quad).is_none());
        // A bolt: thin but short.
        assert!(thin_shape(&tube(1.0, 10.0, 8)).is_none());
        assert!(thin_shape(&[]).is_none());
        assert!(Body::of(&[]).is_none());
        assert!(Body::of(&tube(1.5, 800.0, 8)).unwrap().thin.is_some());
        assert!(Body::of(&tube(15.0, 700.0, 8)).unwrap().thin.is_none());
    }

    #[test]
    fn a_sagging_cables_line_follows_its_sag() {
        // A cable 1200 long hanging 60 units in its middle (a parabola), in 24 rings.
        let ring = |i: usize, s: usize| {
            let x = i as f32 * 50.0;
            let sag = 60.0 * (1.0 - ((x - 600.0) / 600.0).powi(2));
            let a = s as f32 * std::f32::consts::TAU / 6.0;
            [x, 2.0 * a.cos(), 2.0 * a.sin() - sag]
        };
        let mut t = Vec::new();
        for i in 0..24 {
            for s in 0..6 {
                let (a0, a1, b0, b1) = (
                    ring(i, s),
                    ring(i, s + 1),
                    ring(i + 1, s),
                    ring(i + 1, s + 1),
                );
                t.push([a0, b0, a1]);
                t.push([a1, b0, b1]);
            }
        }
        let s = thin_shape(&t).expect("a cable").strands;
        let middle = s[0]
            .line
            .iter()
            .min_by(|a, b| (a[0] - 600.0).abs().total_cmp(&(b[0] - 600.0).abs()))
            .unwrap();
        assert!((middle[2] + 60.0).abs() < 3.0, "{middle:?}");
        assert!((s[0].width - 4.0).abs() < 0.3, "{}", s[0].width);
    }

    #[test]
    fn a_cable_is_drawn_two_texels_wide_with_its_real_share() {
        let f = Frame::from_game(&game()).unwrap();
        let c = cascades_of(&f, basis([0.0, 0.0, 1.0]))[2];
        let texel = c.half * 2.0 / f64::from(THIN_SIZE);
        let w: World = [
            [1.0, 0.0, 0.0, f64::from(f.position[0])],
            [0.0, 1.0, 0.0, f64::from(f.position[1])],
            [0.0, 0.0, 1.0, f64::from(f.position[2])],
        ];
        let strand = |width: f64| Strand {
            line: vec![[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [200.0, 0.0, 0.0]],
            width,
            length: 200.0,
            area: 0.0,
            piece: 0,
        };
        let mut q = Vec::new();
        thin_quads(&c, &w, &[strand(texel / 4.0)], false, &mut q);
        // Two segments, six corners each; drawn two texels wide (in clip units, the square 2
        // across), covering an eighth: the cable's own width.
        assert_eq!(q.len(), 12);
        let across = ((q[1][0] - q[0][0]).powi(2) + (q[1][1] - q[0][1]).powi(2)).sqrt();
        assert!((f64::from(across) - THIN_DRAWN * texel / c.half).abs() < 1e-5);
        assert!(q
            .iter()
            .all(|v| (v[3] - 0.125).abs() < 1e-6 && (f64::from(v[4]) - 2.0 * texel).abs() < 1e-3));
        // Wider than that: drawn as wide as it is, whole.
        q.clear();
        thin_quads(&c, &w, &[strand(texel * 3.0)], false, &mut q);
        assert!(q.iter().all(|v| v[3] == 1.0));
        // Its depth is the map's, less the middle.
        let mid = c.world_to_light(&w)[2][3] - 0.5;
        assert!((q[0][2] - mid).abs() < 1e-6);
        // A ribbon (its cable in its texture's alpha, added by its own pipeline): its quads carry
        // no coverage, only its depth over its width and the band its alpha is spread over.
        q.clear();
        thin_quads(&c, &w, &[strand(texel * 3.0)], true, &mut q);
        assert!(q
            .iter()
            .all(|v| v[3] == 0.0 && (f64::from(v[4]) - THIN_DRAWN * texel).abs() < 1e-3));
    }

    /// A flat strip, `width` across (y) and `length` along x, in `n` quads.
    fn strip(width: f32, length: f32, n: usize) -> Vec<[[f32; 3]; 3]> {
        (0..n)
            .flat_map(|i| {
                let (x0, x1) = (
                    length * i as f32 / n as f32,
                    length * (i + 1) as f32 / n as f32,
                );
                let (a, b, c, d) = (
                    [x0, 0.0, 0.0],
                    [x1, 0.0, 0.0],
                    [x1, width, 0.0],
                    [x0, width, 0.0],
                );
                [[a, b, c], [a, c, d]]
            })
            .collect()
    }

    #[test]
    fn a_ribbon_is_a_cable_and_a_round_pole_is_not() {
        // Goodsprings' telegraph line: cut-out ribbons 20 wide, 700 long, several in a geometry,
        // with a small bit (a clamp) beside them.
        let mut line = strip(20.0, 700.0, 14);
        line.extend(moved(strip(20.0, 700.0, 14), [0.0, 60.0, 0.0]));
        line.extend(moved(tube(3.0, 12.0, 6), [0.0, 30.0, 0.0]));
        let t = thin_shape(&line).expect("ribbons");
        assert!(t.ribbons);
        assert_eq!(t.strands.len(), 2);
        assert!(
            (t.strands[0].width - 20.0).abs() < 0.5,
            "{}",
            t.strands[0].width
        );
        // A round pole as thick is no ribbon (its area about pi times its length by its width).
        assert!(thin_shape(&tube(10.0, 700.0, 8)).is_none());
        // A wide sign is none either; nor a geometry of ribbons with a large other piece.
        assert!(thin_shape(&strip(100.0, 700.0, 4)).is_none());
        let mut with_pole = strip(20.0, 700.0, 14);
        with_pole.extend(moved(tube(15.0, 700.0, 8), [0.0, 200.0, 0.0]));
        assert!(thin_shape(&with_pole).is_none());
        // Round cables alone are no ribbons.
        assert!(!thin_shape(&tube(1.5, 800.0, 8)).unwrap().ribbons);
    }

    #[test]
    fn a_geometry_written_again_is_the_same_geometry() {
        let mut streams = [(0usize, 0u64, 0u64); 8];
        streams[0] = (0x1000, 64, 7);
        let a = GeoKey {
            pipe: 3,
            streams,
            index: (0x2000, 9),
            start: 12,
            count: 300,
            base_vertex: 0,
        };
        let mut b = a.clone();
        b.streams[0].2 = 8;
        b.index.1 = 10;
        assert_ne!(a, b);
        assert_eq!(a.geometry(), b.geometry());
        let mut other = b.clone();
        other.streams[0].1 = 128;
        assert_ne!(a.geometry(), other.geometry());
        assert_eq!(a.geometry().streams[0], (0x1000, 64, 0));
    }

    #[test]
    fn the_sky_and_the_far_land_cast_nothing_and_a_pole_does() {
        let camera = [1000.0, 2000.0, 300.0];
        let at = |p: [f64; 3], scale: f64| -> World {
            [
                [scale, 0.0, 0.0, p[0]],
                [0.0, scale, 0.0, p[1]],
                [0.0, 0.0, scale, p[2]],
            ]
        };
        // A pole (radius 360) next to the camera, and one 9000 units off: both cast.
        assert!(casts(
            &at([1100.0, 2100.0, 300.0], 1.0),
            ([0.0; 3], 360.0),
            camera
        ));
        assert!(casts(
            &at([10000.0, 2000.0, 0.0], 1.0),
            ([0.0; 3], 360.0),
            camera
        ));
        // The sky's clouds: a sphere round the camera wider than MAX_AROUND_CAMERA (here its
        // radius comes from the object's scale); just under it, around the camera, it still casts.
        assert!(!casts(&at(camera, 100.0), ([0.0; 3], 25.0), camera));
        assert!(casts(
            &at(camera, 1.0),
            ([0.0; 3], MAX_AROUND_CAMERA - 1.0),
            camera
        ));
        // A far land block larger than the maps reach, away from the camera: none; at the limit: casts.
        let far = [60000.0, 2000.0, 0.0];
        assert!(!casts(
            &at(far, 1.0),
            ([0.0; 3], MAX_CASTER_RADIUS + 1.0),
            camera
        ));
        assert!(casts(&at(far, 1.0), ([0.0; 3], MAX_CASTER_RADIUS), camera));
    }

    #[test]
    fn half_floats_read_as_ieee_says() {
        assert_eq!(half_to_f32(0x3c00), 1.0);
        assert_eq!(half_to_f32(0xc000), -2.0);
        assert_eq!(half_to_f32(0x3555), 0.333_251_95);
        assert_eq!(half_to_f32(0x0001), 2f32.powi(-24));
        assert_eq!(half_to_f32(0x7c00), f32::INFINITY);
        assert!(half_to_f32(0x7e00).is_nan());
        assert_eq!(half_to_f32(0x7bff), 65504.0);
    }

    fn at(x: f64, y: f64, z: f64) -> World {
        [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]]
    }

    /// The cache keeps a caster drawn again in its place, adds one in a new place (a second copy
    /// of the same geometry, or the same one moved) and lets go of what the sweep says is gone;
    /// each change gives the world sphere that decides which maps are drawn again.
    #[test]
    fn the_cache_keeps_adds_and_lets_go() {
        let mut c: Cache<u32, &str> = Cache::default();
        let local = ([0.0, 0.0, 1.0], 2.0);
        assert_eq!(
            c.see(7, at(10.0, 0.0, 0.0), local, 1, false, || "a"),
            Some(([10.0, 0.0, 1.0], 2.0))
        );
        // The same place a frame later, the camera's rounding aside: kept.
        assert_eq!(
            c.see(7, at(10.004, 0.0, 0.0), local, 2, false, || "b"),
            None
        );
        assert_eq!(c.len(), 1);
        assert_eq!(c.iter().next().unwrap().seen, 2);
        // The same geometry elsewhere: a second place.
        assert!(c
            .see(7, at(50.0, 0.0, 0.0), local, 2, false, || "c")
            .is_some());
        assert_eq!(c.len(), 2);
        // Scaled: the sphere grows with it.
        let big = [
            [3.0, 0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 3.0, 0.0],
        ];
        assert_eq!(
            c.see(8, big, local, 2, false, || "d"),
            Some(([0.0, 0.0, 3.0], 6.0))
        );
        // Moved: a new place; the sweep lets the old one go when told it is gone.
        assert!(c
            .see(7, at(10.0, 5.0, 0.0), local, 3, false, || "e")
            .is_some());
        let gone = c.sweep(|_, i| i.seen != 3 && i.payload != "d");
        assert_eq!(gone.len(), 2);
        assert!(gone.iter().all(|(k, _, m)| *k == 7 && !m));
        let spheres: Vec<_> = gone.iter().map(|(_, s, _)| *s).collect();
        assert!(
            spheres.contains(&([10.0, 0.0, 1.0], 2.0))
                && spheres.contains(&([50.0, 0.0, 1.0], 2.0))
        );
        assert_eq!(c.iter_keyed().filter(|(k, _)| **k == 7).count(), 1);
        assert_eq!(c.len(), 2);
        let mut left: Vec<&str> = c.iter().map(|i| i.payload).collect();
        left.sort();
        assert_eq!(left, ["d", "e"]);
        assert!(c.sweep(|_, _| false).is_empty());
        assert!(!c.is_empty());
        assert_eq!(c.sweep(|_, _| true).len(), 2);
        assert!(c.is_empty());
    }

    /// The camera at `position` looking along +z (right +x, up +y), a 90-degree view, depths 10 to
    /// 10000: the pairing rule the device uses.
    fn mover_at(
        position: [f64; 3],
    ) -> impl Fn(&Instance<&'static str>, &Instance<&'static str>) -> bool {
        let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        mover([-1.0, 1.0, -1.0, 1.0], position, axes, 10.0, 10000.0)
    }

    fn moving_of(c: &Cache<u32, &'static str>) -> Vec<&'static str> {
        let mut m: Vec<&str> = c.iter().filter(|i| i.moving).map(|i| i.payload).collect();
        m.sort();
        m
    }

    /// Motion is decided per place. Two copies of one geometry (fence posts sharing a buffer): as
    /// the camera moves, one leaves the view and a third comes in near where it was: nothing moves,
    /// and the copy out of sight is kept (it still casts into view). A copy hidden in view while
    /// another comes far from it: no move either.
    #[test]
    fn a_copy_leaving_the_view_as_another_comes_is_no_move() {
        let local = ([0.0, 0.0, 0.0], 2.0);
        let mut c: Cache<u32, &str> = Cache::default();
        c.see(7, at(-900.0, 0.0, 1000.0), local, 1, false, || "left");
        c.see(7, at(0.0, 0.0, 1000.0), local, 1, false, || "stays");
        // Frame 2, the camera 300 to the right: "left" is out of the view (its edge 140 past the
        // view's left side), "stays" is drawn again, "new" comes 250 from "left", inside the view.
        c.see(7, at(0.0, 0.0, 1000.0), local, 2, false, || "x");
        let came = [7u32];
        assert!(c
            .see(7, at(-650.0, 0.0, 1000.0), local, 2, false, || "new")
            .is_some());
        assert_eq!(c.mark_moves(2, &came, 100, mover_at([300.0, 0.0, 0.0])), 0);
        assert!(moving_of(&c).is_empty());
        assert!(c.iter().all(|i| !i.left && !i.moved_on(2)));
        assert!(c.sweep(|_, i| i.moved_on(2)).is_empty());
        assert_eq!(c.len(), 3);
        // Frame 3, the camera still: "stays" and "new" drawn, "left" not (out of sight), and a
        // fourth copy comes far from everything: no move.
        c.see(7, at(0.0, 0.0, 1000.0), local, 3, false, || "x");
        c.see(7, at(-650.0, 0.0, 1000.0), local, 3, false, || "x");
        assert!(c
            .see(7, at(600.0, 0.0, 3000.0), local, 3, false, || "far")
            .is_some());
        assert_eq!(c.mark_moves(3, &came, 100, mover_at([300.0, 0.0, 0.0])), 0);
        // Frame 4, the camera still: "far" hidden in view (not drawn) and another comes 1000
        // from it: no move (too far to be the same object moved).
        c.see(7, at(0.0, 0.0, 1000.0), local, 4, false, || "x");
        c.see(7, at(-650.0, 0.0, 1000.0), local, 4, false, || "x");
        assert!(c
            .see(7, at(600.0, 0.0, 2000.0), local, 4, false, || "far2")
            .is_some());
        assert_eq!(c.mark_moves(4, &came, 100, mover_at([300.0, 0.0, 0.0])), 0);
        assert!(moving_of(&c).is_empty());
        assert!(c.sweep(|_, i| i.moved_on(4)).is_empty());
    }

    /// A real move is found whatever other copies of the geometry do in the same frame: a can
    /// moves as a second copy leaves the view (two old places, one new), then as a third copy
    /// comes in (one old, two new); the copy out of sight is kept. When the camera turns so that
    /// the moving place's old place is out of the view, a new place inside the old place's own
    /// sphere is still that object moved.
    #[test]
    fn a_real_move_is_found_whatever_other_copies_do() {
        let local = ([0.0, 0.0, 0.0], 2.0);
        let mut c: Cache<u32, &'static str> = Cache::default();
        let came = [9u32];
        let here = mover_at([0.0; 3]);
        c.see(9, at(0.0, 0.0, 1000.0), local, 1, false, || "can");
        c.see(9, at(100.0, 0.0, 1000.0), local, 1, false, || "can2");
        // Out of the view (x 1.2 times the depth), drawn while the camera still saw it.
        c.see(9, at(-1200.0, 0.0, 1000.0), local, 1, false, || "out");
        // Frame 2: the can moves 5, "can2" stays, "out" is not drawn.
        c.see(9, at(100.0, 0.0, 1000.0), local, 2, false, || "x");
        c.see(9, at(5.0, 0.0, 1000.0), local, 2, false, || "can'");
        assert_eq!(c.mark_moves(2, &came, 100, &here), 1);
        assert_eq!(moving_of(&c), ["can'"]);
        let gone = c.sweep(|_, i| i.moved_on(2));
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].1 .0, [0.0, 0.0, 1000.0]);
        assert!(c.iter().any(|i| i.payload == "out" && !i.left));
        // Frame 3: the can moves 5 more as a third copy comes in far from it.
        c.see(9, at(100.0, 0.0, 1000.0), local, 3, false, || "x");
        c.see(9, at(10.0, 0.0, 1000.0), local, 3, false, || "can''");
        c.see(9, at(500.0, 0.0, 1000.0), local, 3, false, || "more");
        assert_eq!(c.mark_moves(3, &came, 100, &here), 1);
        assert_eq!(moving_of(&c), ["can'", "can''"]);
        assert_eq!(c.sweep(|_, i| i.moved_on(3)).len(), 1);
        // Frame 4: the camera turned (now at x 1100: the old place is out of the view); the can
        // moved 1 (inside its own sphere): still a move. Frame 5: 3 more (outside it, the old
        // place out of the view): not a move.
        c.see(9, at(11.0, 0.0, 1000.0), local, 4, false, || "can'''");
        assert_eq!(c.mark_moves(4, &came, 100, mover_at([1100.0, 0.0, 0.0])), 1);
        assert!(c.iter().any(|i| i.payload == "can''" && i.left));
        c.sweep(|_, i| i.moved_on(4));
        c.see(9, at(14.0, 0.0, 1000.0), local, 5, false, || "far");
        assert_eq!(c.mark_moves(5, &came, 100, mover_at([1100.0, 0.0, 0.0])), 0);
    }

    /// A moving place that has stood still for [`SETTLE`] frames is still again (the still maps
    /// take it, so it casts from out of sight too); one that came later, or is not drawn, is not.
    #[test]
    fn a_moving_place_standing_still_is_still_again() {
        let local = ([0.0, 0.0, 0.0], 2.0);
        let mut c: Cache<u32, &'static str> = Cache::default();
        c.see(1, at(0.0, 0.0, 1000.0), local, 10, true, || "a");
        c.see(1, at(50.0, 0.0, 1000.0), local, 20, true, || "b");
        c.see(1, at(90.0, 0.0, 1000.0), local, 10, true, || "c");
        let now = 10 + SETTLE;
        c.see(1, at(0.0, 0.0, 1000.0), local, now, true, || "x");
        c.see(1, at(50.0, 0.0, 1000.0), local, now, true, || "x");
        assert!(c.settle(now - 1, SETTLE).is_empty());
        assert_eq!(c.settle(now, SETTLE), [([0.0, 0.0, 1000.0], 2.0)]);
        assert_eq!(moving_of(&c), ["b", "c"]);
        assert!(c.settle(now, SETTLE).is_empty());
    }

    /// One object moved a little between frames, in view: its new place moves and its old place
    /// is let go; the next frame's place moves on from the moving one; two moved together are
    /// paired nearest first; a written geometry moves from the start; the room is kept to.
    #[test]
    fn a_place_moved_in_view_moves_and_its_old_place_goes() {
        let local = ([0.0, 0.0, 0.0], 2.0);
        let mut c: Cache<u32, &str> = Cache::default();
        let came = [7u32];
        c.see(7, at(0.0, 0.0, 1000.0), local, 1, false, || "a");
        c.see(7, at(10.0, 0.0, 1000.0), local, 2, false, || "b");
        assert_eq!(c.mark_moves(2, &came, 100, mover_at([0.0; 3])), 1);
        assert_eq!(moving_of(&c), ["b"]);
        let gone = c.sweep(|_, i| i.moved_on(2));
        // The old place was still (in the still maps): its going redraws them.
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].1, ([0.0, 0.0, 1000.0], 2.0));
        assert!(!gone[0].2);
        c.see(7, at(20.0, 0.0, 1000.0), local, 3, false, || "c");
        assert_eq!(c.mark_moves(3, &came, 100, mover_at([0.0; 3])), 1);
        assert_eq!(moving_of(&c), ["b", "c"]);
        let gone = c.sweep(|_, i| i.moved_on(3));
        // The moving place was in no still map.
        assert_eq!(gone.len(), 1);
        assert!(gone[0].2);
        // Gone the next frame (not drawn): let go at once.
        assert_eq!(c.sweep(|_, i| i.moved_on(4)).len(), 1);
        assert!(c.is_empty());
        // Two moved together, each from the nearest old place (q2 paired with p, the farther one,
        // would leave p2 260 from q: too far, and p2 would not move).
        c.see(8, at(0.0, 0.0, 1000.0), local, 10, false, || "p");
        c.see(8, at(200.0, 0.0, 1000.0), local, 10, false, || "q");
        c.see(8, at(190.0, 0.0, 1000.0), local, 11, false, || "q2");
        c.see(8, at(-60.0, 0.0, 1000.0), local, 11, false, || "p2");
        assert_eq!(c.mark_moves(11, &[8u32], 100, mover_at([0.0; 3])), 2);
        assert_eq!(moving_of(&c), ["p2", "q2"]);
        let mut gone: Vec<f64> = c
            .sweep(|_, i| i.moved_on(11))
            .iter()
            .map(|g| g.1 .0[0])
            .collect();
        gone.sort_by(f64::total_cmp);
        assert_eq!(gone, [0.0, 200.0]);
        // No room: nothing marked.
        c.see(9, at(0.0, 0.0, 1000.0), local, 20, false, || "r");
        c.see(9, at(5.0, 0.0, 1000.0), local, 21, false, || "r2");
        assert_eq!(c.mark_moves(21, &[9u32], 0, mover_at([0.0; 3])), 0);
        // Written: moving from the start.
        c.see(10, at(0.0, 0.0, 1000.0), local, 21, true, || "w");
        assert!(c.iter().any(|i| i.payload == "w" && i.moving));
    }

    #[test]
    fn a_sphere_reaches_into_the_view_unless_wholly_outside_a_side() {
        let f = [-1.0, 1.0, -1.0, 1.0];
        assert!(partly_in_view(f, [0.0, 0.0, 100.0], 1.0, 10.0, 1000.0));
        // Just past the left side, its edge still in.
        assert!(partly_in_view(f, [-101.0, 0.0, 100.0], 1.0, 10.0, 1000.0));
        assert!(!partly_in_view(f, [-103.0, 0.0, 100.0], 1.0, 10.0, 1000.0));
        assert!(!partly_in_view(f, [103.0, 0.0, 100.0], 1.0, 10.0, 1000.0));
        assert!(!partly_in_view(f, [0.0, -103.0, 100.0], 1.0, 10.0, 1000.0));
        assert!(!partly_in_view(f, [0.0, 103.0, 100.0], 1.0, 10.0, 1000.0));
        // Behind the near plane and past the far one.
        assert!(!partly_in_view(f, [0.0, 0.0, 8.0], 1.0, 10.0, 1000.0));
        assert!(partly_in_view(f, [0.0, 0.0, 9.5], 1.0, 10.0, 1000.0));
        assert!(!partly_in_view(f, [0.0, 0.0, 1002.0], 1.0, 10.0, 1000.0));
    }

    #[test]
    fn the_frame_words_carry_it() {
        let f = Frame::from_game(&game()).unwrap();
        assert_eq!(Frame::from_words(&f.to_words()), Some(f));
        assert_eq!(Frame::from_words(&[0; 16]), None);
        assert_eq!(Frame::from_words(&f.to_words()[..15]), None);
        let len = (0.3f32 * 0.3 + 0.2 * 0.2 + 0.9 * 0.9).sqrt();
        assert!((f.to_sun[2] - 0.9 / len).abs() < 1e-6);
        // No sun: indoors, the sun down or too low, dark.
        for g in [
            scene_light::GameView {
                interior: true,
                ..game()
            },
            scene_light::GameView {
                to_sun: Some([1.0, 0.0, 0.01]),
                ..game()
            },
            scene_light::GameView {
                to_sun: None,
                ..game()
            },
            scene_light::GameView {
                sun_colour: [0.0; 3],
                ..game()
            },
            scene_light::GameView {
                position: [f32::NAN, 0.0, 0.0],
                ..game()
            },
        ] {
            assert_eq!(Frame::from_game(&g), None, "{g:?}");
        }
    }

    #[test]
    fn its_state_is_no_direct3d_transform_nor_the_new_lightings() {
        const { assert!(STATE > 511) };
        assert_ne!(STATE, scene_light::STATE);
        assert_eq!(&STATE.to_le_bytes(), b"NVSS");
    }

    #[test]
    fn the_step_is_valid_wgsl() {
        for (ms, view) in [(false, 0), (true, 0), (false, 1), (true, 1)] {
            let src = step_shader_with(ms, view);
            let module = naga::front::wgsl::parse_str(&src)
                .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&src)));
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap_or_else(|e| panic!("ms {ms}: {e:?}"));
        }
    }

    /// A pixel shader's tokens: `ps_3_0`, a constant table of `names` (float4, register, count),
    /// `body`, the end.
    fn pixel_shader(names: &[(&str, u16, u16)], body: &[u32]) -> Vec<u32> {
        shader_tokens(0xFFFF_0300, names, body)
    }

    /// A shader's tokens of version token `version`, as [`pixel_shader`].
    fn shader_tokens(version: u32, names: &[(&str, u16, u16)], body: &[u32]) -> Vec<u32> {
        let mut b = Vec::new();
        for v in [28u32, 0, version, names.len() as u32, 28, 0, 0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        let strings = 28 + 20 * names.len();
        let mut text = Vec::new();
        for &(name, index, count) in names {
            b.extend_from_slice(&((strings + text.len()) as u32).to_le_bytes());
            for v in [2u16, index, count, 0] {
                b.extend_from_slice(&v.to_le_bytes());
            }
            b.extend_from_slice(&[0; 8]);
            text.extend_from_slice(name.as_bytes());
            text.push(0);
        }
        b.extend(text);
        b.resize(b.len().div_ceil(4) * 4, 0);
        let mut t = vec![version];
        let words: Vec<u32> = std::iter::once(u32::from_le_bytes(*b"CTAB"))
            .chain(
                b.chunks_exact(4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])),
            )
            .collect();
        t.push(0xFFFE | ((words.len() as u32) << 16));
        t.extend(words);
        t.extend_from_slice(body);
        t.push(0x0000_FFFF);
        t
    }

    /// A lit pixel shader's variant multiplies every read of the sun's light colour (and nothing
    /// else) by the visibility, takes the lookup in group 3, and is valid WGSL; a shader lit by a
    /// point light alone (no ambient) has no variant.
    #[test]
    fn a_lit_pixel_shaders_variant_shadows_the_sun_colour_alone() {
        use crate::sm::{self, wgsl};
        // mov r0, c3; add r0, r0, c1; mov oC0, r0.
        let body = [
            1 | (2 << 24),
            0x800F_0000,
            0xA0E4_0003,
            2 | (3 << 24),
            0x800F_0000,
            0x80E4_0000,
            0xA0E4_0001,
            1 | (2 << 24),
            0x800F_0800,
            0x80E4_0000,
        ];
        let lit = sm::parse(&pixel_shader(
            &[("AmbientColor", 1, 1), ("PSLightColor", 3, 2)],
            &body,
        ))
        .unwrap();
        assert_eq!(wgsl::sun_register(&lit), Some(3));
        let link = wgsl::Link {
            inputs: Vec::new(),
            varyings: Vec::new(),
            centroid: Vec::new(),
            clip: wgsl::Clip::Off,
            count: false,
            fixed: true,
        };
        let plain = wgsl::convert(&lit, &link).unwrap();
        let src = wgsl::convert_with_sun(&lit, &link).unwrap();
        assert!(!plain.contains("sun_lit"));
        assert!(
            src.contains("(C.c[3] * vec4<f32>(vec3<f32>(sun_lit), 1.0))"),
            "{src}"
        );
        assert!(src.contains("C.c[1]") && !src.contains("C.c[1] * vec4<f32>(vec3<f32>(sun_lit)"));
        assert!(src.contains("sun_lit = sun_visibility(input.frag);"));
        assert!(src.contains("@group(3) @binding(0) var<uniform> SL"));
        let module = naga::front::wgsl::parse_str(&src)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&src)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{e:?}"));
        // A point light's pass: no ambient, no sun.
        let point = sm::parse(&pixel_shader(
            &[("PSLightColor", 3, 1), ("AttMapXY", 1, 1)],
            &body,
        ))
        .unwrap();
        assert_eq!(wgsl::sun_register(&point), None);
        assert!(wgsl::convert_with_sun(&point, &link).is_err());
    }

    /// A vertex-lit shader's variant (grass): its vertex shader's `DiffuseColor` times the visibility at
    /// the vertex, the body run once for the place and again with the shadowed colour; valid WGSL.
    #[test]
    fn a_vertex_lit_shaders_variant_shadows_its_diffuse_colour() {
        use crate::sm::{self, wgsl};
        let mut body = vec![31 | (2 << 24), 0x8000_0000, 0x900F_0000];
        for k in 0..4u32 {
            body.extend([
                9 | (3 << 24),
                0xC000_0000 | (0x1_0000 << k),
                0x90E4_0000,
                0xA0E4_0009 + k,
            ]);
        }
        body.extend([1 | (2 << 24), 0xE00F_0000, 0xA0E4_0001]);
        body.extend([1 | (2 << 24), 0xE00F_0001, 0xA0E4_0006]);
        let names = [
            ("DiffuseColor", 1, 1),
            ("AmbientColor", 6, 1),
            ("ModelViewProj", 9, 4),
        ];
        let grass = sm::parse(&shader_tokens(0xFFFE_0200, &names, &body)).unwrap();
        assert_eq!(wgsl::sun_register(&grass), Some(1));
        let link = wgsl::Link {
            inputs: vec![wgsl::Input::Float(4); 16],
            varyings: Vec::new(),
            centroid: Vec::new(),
            clip: wgsl::Clip::Off,
            count: false,
            fixed: true,
        };
        let src = wgsl::convert_with_sun(&grass, &link).unwrap();
        assert!(
            src.contains("(C.c[1] * vec4<f32>(vec3<f32>(sun_lit), 1.0))"),
            "{src}"
        );
        assert!(!src.contains("(C.c[6] * vec4<f32>(vec3<f32>(sun_lit)"));
        assert!(
            src.contains("sun_lit = sun_visibility_clip(o_pos);"),
            "{src}"
        );
        assert_eq!(src.matches("    body();").count(), 2);
        let module = naga::front::wgsl::parse_str(&src)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&src)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{e:?}"));
        // A plain vertex shader has no sun.
        let plain = sm::parse(&shader_tokens(0xFFFE_0200, &names[2..], &body)).unwrap();
        assert_eq!(wgsl::sun_register(&plain), None);
    }

    #[test]
    fn the_lookup_uniforms_have_the_shaders_layout() {
        let f = Frame::from_game(&game()).unwrap();
        let b = basis(wide(f.to_sun));
        let c = cascades_of(&f, b);
        let c: [Cascade; CASCADES] = std::array::from_fn(|k| c[k]);
        let u = lookup_uniforms(
            &view(),
            f.strength,
            &c,
            &c,
            [false, true, false, false],
            f.axes(),
            wide(f.position),
            8,
            2560,
            1440,
        );
        assert_eq!(u.len() as u64, LOOKUP_SIZE);
        let at = |i: usize| f32::from_le_bytes(u[4 * i..4 * i + 4].try_into().unwrap());
        assert_eq!(at(6), 2560.0);
        assert_eq!(at(11), 1.0);
        assert_eq!(at(15), FAR);
        assert_eq!(at(16), c[0].texel as f32);
        assert_eq!((at(20), at(21)), (0.0625, -0.1875));
        assert_eq!(at(23), BLEND);
        // Each cascade's thin-caster texel, 0 where none is.
        assert_eq!((at(28), at(30)), (0.0, 0.0));
        assert_eq!(at(29), (c[1].half * 2.0 / f64::from(THIN_SIZE)) as f32);
        assert_eq!(at(32), c[0].view_to_light[0][0] as f32);
        assert_eq!(at(32 + 16 * 3 + 15), 1.0);
        // No sun: the lookup is off.
        let off = lookup_uniforms(
            &view(),
            0.0,
            &c,
            &c,
            [false; CASCADES],
            f.axes(),
            wide(f.position),
            1,
            2560,
            1440,
        );
        assert_eq!(f32::from_le_bytes(off[44..48].try_into().unwrap()), 0.0);
    }

    #[test]
    fn only_unskinned_shaders_with_a_whole_model_view_proj_cast() {
        let program = |names: &[(&str, u16, u16)]| crate::sm::Program {
            stage: crate::sm::Stage::Vertex,
            major: 3,
            minor: 0,
            ins: Vec::new(),
            constants: names
                .iter()
                .map(|&(n, index, count)| crate::sm::Constant {
                    name: n.to_string(),
                    set: 2,
                    index,
                    count,
                })
                .collect(),
            creator: None,
        };
        assert_eq!(
            model_view_proj(&program(&[("ModelViewProj", 0, 4)])),
            Some(0)
        );
        assert_eq!(
            model_view_proj(&program(&[("FogParam", 0, 1), ("ModelViewProj", 4, 4)])),
            Some(4)
        );
        assert_eq!(model_view_proj(&program(&[("ModelViewProj", 0, 3)])), None);
        assert_eq!(
            model_view_proj(&program(&[
                ("SkinModelViewProj", 0, 4),
                ("ModelViewProj", 4, 4)
            ])),
            None
        );
        assert_eq!(
            model_view_proj(&program(&[("ModelViewProj", 0, 4), ("Bones", 10, 54)])),
            None
        );
        assert_eq!(model_view_proj(&program(&[])), None);
    }
}
