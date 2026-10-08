//! A ragdoll: rigid bodies joined at pivots, falling under gravity and
//! coming to rest on a [`Collider`], as the game's dead do.
//!
//! The bodies, their joints and the joints' limits come from the skeleton
//! (`nif::ragdoll`); this module only moves them. The game hands them to
//! Havok; here they're stepped with extended position-based dynamics
//! (Müller et al., "Detailed Rigid Body Simulation with Extended Position
//! Based Dynamics", 2020): each step is split into substeps that move the
//! bodies, then correct their positions so the joints hold and nothing sits
//! inside the world, then work out the velocities from the moves.
//!
//! Read from the game: masses, inertia, centres of mass, damping, friction,
//! the most speed, every capsule, pivot, axis and angle limit, and the step
//! (`[HAVOK] fMaxTime` 0.016 in `Fallout.ini`, about 1/60 s). Not read
//! (guesses, marked where they're used): gravity (real gravity, as for
//! walking), how Havok measures the limits' angles (checked only in that the
//! skeleton's own pose falls inside every limit), no bouncing (the bodies'
//! restitution isn't combined with the world's). A ragdoll's own bodies
//! meet as the game's collision filter has them: one system group, so the
//! part table decides ([`parts_meet`], `00c84740`); they're pushed apart
//! without friction (this solver's).

use crate::vec::*;
use crate::{segment_triangle_closest, Collider, Vec3};

/// A rotation as a row-major matrix (column vectors), as `nif` stores it.
pub type Mat3 = [[f32; 3]; 3];
/// A rotation as a unit quaternion (x, y, z, w).
pub(crate) type Quat = [f32; 4];

/// The physics step: `[HAVOK] fMaxTime` (0.016 s).
pub const STEP: f32 = 0.016;
/// Substeps per step (a choice of this solver, not the game's).
const SUBSTEPS: usize = 10;

/// What a body is made of, in its own frame (game units).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodySetup {
    pub mass: f32,
    pub center: Vec3,
    /// The inertia tensor's diagonal about the centre.
    pub inertia: Vec3,
    /// Two ends and a radius.
    pub capsule: Option<(Vec3, Vec3, f32)>,
    pub friction: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub max_linear_speed: f32,
    pub max_angular_speed: f32,
    /// Its Havok layer and body part number (the filter word's low byte
    /// and bits 8–12): which other bodies of the same ragdoll it meets
    /// ([`crate::layers::Filter::collides`]).
    pub layer: u8,
    pub part: u8,
}

/// Whether two bodies of one ragdoll meet: one ragdoll's bodies share a
/// system group, so the collision filter (`00c84740`) takes the part
/// table for them when both are on the biped or dead-biped layer
/// (`00624070`: layer 8 or 29); the order it's asked in isn't known, so
/// either way round counts.
pub fn parts_meet(a: &BodySetup, b: &BodySetup) -> bool {
    let f = crate::layers::Filter::shared();
    // Any non-zero system group, the same for both.
    let word = |s: &BodySetup| u32::from(s.layer & 0x7f) | u32::from(s.part & 0x1f) << 8 | 1 << 16;
    let linked = |s: &BodySetup| matches!(s.layer, 8 | 29);
    let both = linked(a) && linked(b);
    f.collides(word(a), word(b), both) || f.collides(word(b), word(a), both)
}

/// How far a joint lets its bodies turn: each vector in its own body's
/// frame (the first body's, then the second's).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Limit {
    /// The first body's twist axis stays within `cone` of the second's;
    /// its angle out of the plane across the second's plane axis within
    /// `plane_range`; its turn about the twist axis (measured between the
    /// plane axes) within `twist_range`.
    Cone {
        twist: [Vec3; 2],
        plane: [Vec3; 2],
        cone: f32,
        plane_range: (f32, f32),
        twist_range: (f32, f32),
    },
    /// The axles stay lined up; the turn about them (between the
    /// perpendicular axes) stays within `range`.
    Hinge {
        axle: [Vec3; 2],
        perpendicular: [Vec3; 2],
        range: (f32, f32),
    },
    Free,
}

/// A joint: two bodies, and the point each holds (in its frame).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointSetup {
    pub bodies: [usize; 2],
    pub pivots: [Vec3; 2],
    pub limit: Limit,
}

/// A joint's angles now, against its limits (radians).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JointAngles {
    Cone { cone: f32, plane: f32, twist: f32 },
    Hinge { misaligned: f32, angle: f32 },
    Free,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct State {
    /// The centre of mass, in the world.
    x: Vec3,
    q: Quat,
    v: Vec3,
    w: Vec3,
}

/// A contact found while correcting positions, for the velocity pass.
struct Contact {
    body: usize,
    /// The touching point, from the centre, in the body's frame.
    local: Vec3,
    normal: Vec3,
    /// How much correction it took (mass × distance).
    lambda: f32,
}

/// A ragdoll in motion.
#[derive(Debug, Clone)]
pub struct Ragdoll {
    pub bodies: Vec<BodySetup>,
    pub joints: Vec<JointSetup>,
    /// Pairs of its own bodies that meet ([`parts_meet`]).
    pairs: Vec<(usize, usize)>,
    state: Vec<State>,
    /// The world's step clock (`crate::havok::Clock`).
    pub clock: crate::havok::Clock,
    /// How long everything has been nearly still.
    still: f32,
    /// At rest: no longer stepped.
    pub asleep: bool,
}

/// Downward acceleration, units per second squared: the Havok world's
/// (`crate::GRAVITY`); nothing in the game's death path changes a body's
/// gravity factor.
pub const GRAVITY: f32 = crate::GRAVITY;

impl Ragdoll {
    /// Bodies placed at `frames` (each body frame's rotation and origin in
    /// the world), at rest.
    pub fn new(bodies: Vec<BodySetup>, joints: Vec<JointSetup>, frames: &[(Mat3, Vec3)]) -> Self {
        let state = bodies
            .iter()
            .zip(frames)
            .map(|(b, (r, origin))| State {
                x: add(*origin, mat_vec(r, b.center)),
                q: mat_quat(r),
                v: [0.0; 3],
                w: [0.0; 3],
            })
            .collect();
        let mut pairs = Vec::new();
        for i in 0..bodies.len() {
            for j in i + 1..bodies.len() {
                let (a, b) = (&bodies[i], &bodies[j]);
                if a.capsule.is_some() && b.capsule.is_some() && parts_meet(a, b) {
                    pairs.push((i, j));
                }
            }
        }
        Ragdoll {
            bodies,
            joints,
            pairs,
            state,
            clock: crate::havok::Clock::default(),
            still: 0.0,
            asleep: false,
        }
    }

    /// A body's frame now: its rotation and origin in the world.
    pub fn frame(&self, body: usize) -> (Mat3, Vec3) {
        let s = &self.state[body];
        let r = quat_mat(s.q);
        (r, sub(s.x, mat_vec(&r, self.bodies[body].center)))
    }

    /// Each body's capsule where it is now, in the world: body index, ends
    /// and radius (bodies of other shapes are left out).
    pub fn world_capsules(&self) -> Vec<(usize, Vec3, Vec3, f32)> {
        (0..self.bodies.len())
            .filter_map(|i| {
                let (a, b, r) = self.bodies[i].capsule?;
                let (m, origin) = self.frame(i);
                Some((
                    i,
                    add(origin, mat_vec(&m, a)),
                    add(origin, mat_vec(&m, b)),
                    r,
                ))
            })
            .collect()
    }

    /// Sets every body moving at `velocity`.
    pub fn set_velocity(&mut self, velocity: Vec3) {
        for s in &mut self.state {
            s.v = velocity;
        }
        self.asleep = false;
    }

    /// Every body's speed changes by `velocity`, whatever its mass (the
    /// game adds a velocity to each moving body of a tree: `0062b930`).
    pub fn add_velocity(&mut self, velocity: Vec3) {
        for s in &mut self.state {
            s.v = add(s.v, velocity);
        }
        self.asleep = false;
    }

    /// The game's death push (`0062b660`): every body's speed changes by
    /// `speed` × its multiplier, away from `origin` (each body along the
    /// line from `origin` to its centre), whatever its mass.
    pub fn throw_from(&mut self, origin: Vec3, speed: f32, multipliers: &[f32]) {
        for (i, s) in self.state.iter_mut().enumerate() {
            let m = multipliers.get(i).copied().unwrap_or(1.0);
            let away = sub(s.x, origin);
            let d = length(away);
            if d > 1e-6 && m != 0.0 {
                s.v = add(s.v, scale(away, speed * m / d));
            }
        }
        self.asleep = false;
    }

    /// A blow (mass × units a second) to one body, through its centre.
    pub fn push(&mut self, body: usize, impulse: Vec3) {
        let m = self.bodies[body].mass;
        if m > 0.0 {
            let s = &mut self.state[body];
            s.v = add(s.v, scale(impulse, 1.0 / m));
            self.asleep = false;
        }
    }

    /// The angles of a joint now.
    pub fn joint_angles(&self, joint: usize) -> JointAngles {
        let j = &self.joints[joint];
        let r1 = quat_mat(self.state[j.bodies[0]].q);
        let r2 = quat_mat(self.state[j.bodies[1]].q);
        joint_angles(&j.limit, &r1, &r2)
    }

    /// The fastest any body moves (units a second) and turns (radians a
    /// second) now, and which.
    pub fn fastest(&self) -> ((f32, usize), (f32, usize)) {
        let mut out = ((0.0, 0), (0.0, 0));
        for (i, s) in self.state.iter().enumerate() {
            if length(s.v) > out.0 .0 {
                out.0 = (length(s.v), i);
            }
            if length(s.w) > out.1 .0 {
                out.1 = (length(s.w), i);
            }
        }
        out
    }

    /// How far apart a joint's two pivots are now (0 when it holds).
    pub fn joint_gap(&self, joint: usize) -> f32 {
        let j = &self.joints[joint];
        let p = |k: usize| {
            let (r, origin) = self.frame(j.bodies[k]);
            add(origin, mat_vec(&r, j.pivots[k]))
        };
        length(sub(p(0), p(1)))
    }

    /// Moves on by a frame of `dt` seconds, in the world's whole steps
    /// ([`crate::havok::Clock`], `00c66760`/`00c6ae70`).
    pub fn update(&mut self, collider: &Collider, dt: f32) {
        let (n, _) = self.clock.advance(dt);
        for _ in 0..n {
            if self.asleep {
                return;
            }
            self.step(collider);
        }
    }

    /// One physics step.
    pub fn step(&mut self, collider: &Collider) {
        let h = STEP / SUBSTEPS as f32;
        // Triangles each body might touch this step.
        let nearby: Vec<Vec<u32>> = (0..self.bodies.len())
            .map(|i| {
                let Some((a, b, radius)) = self.bodies[i].capsule else {
                    return Vec::new();
                };
                let (r, origin) = self.frame(i);
                let (a, b) = (add(origin, mat_vec(&r, a)), add(origin, mat_vec(&r, b)));
                let reach = radius + length(self.state[i].v) * STEP + 4.0;
                let lo = [
                    a[0].min(b[0]) - reach,
                    a[1].min(b[1]) - reach,
                    a[2].min(b[2]) - reach,
                ];
                let hi = [
                    a[0].max(b[0]) + reach,
                    a[1].max(b[1]) + reach,
                    a[2].max(b[2]) + reach,
                ];
                collider
                    .near(lo, hi)
                    .into_iter()
                    .filter(|&t| {
                        let [p, q, s] = collider.triangle(t);
                        p[2].max(q[2]).max(s[2]) >= lo[2] && p[2].min(q[2]).min(s[2]) <= hi[2]
                    })
                    .collect()
            })
            .collect();
        for _ in 0..SUBSTEPS {
            self.substep(collider, &nearby, h);
        }
        // Asleep once everything has been nearly still for a second (this
        // solver's thresholds; Havok's deactivation isn't traced).
        let moving = self
            .state
            .iter()
            .any(|s| length(s.v) > 4.0 || length(s.w) > 0.6);
        self.still = if moving { 0.0 } else { self.still + STEP };
        if self.still > 1.0 {
            self.asleep = true;
            for s in &mut self.state {
                s.v = [0.0; 3];
                s.w = [0.0; 3];
            }
        }
    }

    fn substep(&mut self, collider: &Collider, nearby: &[Vec<u32>], h: f32) {
        let previous: Vec<State> = self.state.clone();
        for (s, b) in self.state.iter_mut().zip(&self.bodies) {
            if b.mass <= 0.0 {
                continue;
            }
            s.v[2] -= GRAVITY * h;
            s.x = add(s.x, scale(s.v, h));
            s.q = rotated(s.q, scale(s.w, h));
        }
        for j in 0..self.joints.len() {
            self.solve_joint(j);
        }
        for k in 0..self.pairs.len() {
            let (a, b) = self.pairs[k];
            self.solve_pair(a, b);
        }
        let mut contacts = Vec::new();
        for (i, tris) in nearby.iter().enumerate() {
            self.solve_contacts(collider, i, tris, &mut contacts);
        }
        // Velocities from the moves.
        for ((s, p), b) in self.state.iter_mut().zip(&previous).zip(&self.bodies) {
            if b.mass <= 0.0 {
                continue;
            }
            s.v = scale(sub(s.x, p.x), 1.0 / h);
            let d = quat_mul(s.q, conjugate(p.q));
            let w = scale([d[0], d[1], d[2]], 2.0 / h);
            s.w = if d[3] < 0.0 { scale(w, -1.0) } else { w };
        }
        // Friction (stopping a sliding point outright when the push it got
        // allows: static friction) and no bounce.
        for c in &contacts {
            let b = self.bodies[c.body];
            let s = self.state[c.body];
            let r = mat_vec(&quat_mat(s.q), c.local);
            let at = add(s.v, cross(s.w, r));
            let vn = dot(at, c.normal);
            let vt = sub(at, scale(c.normal, vn));
            let mut dv = [0.0; 3];
            let slide = length(vt);
            if slide > 1e-6 {
                let stop = (b.friction * c.lambda / h).min(slide);
                dv = add(dv, scale(vt, -stop / slide));
            }
            if vn < 0.0 {
                dv = add(dv, scale(c.normal, -vn));
            }
            let size = length(dv);
            if size > 1e-6 {
                let n = scale(dv, 1.0 / size);
                let w = self.inverse_mass_at(c.body, r, n);
                if w > 0.0 {
                    self.impulse(c.body, scale(n, size / w), r);
                }
            }
        }
        for (s, b) in self.state.iter_mut().zip(&self.bodies) {
            s.v = scale(s.v, (1.0 - b.linear_damping * h).max(0.0));
            s.w = scale(s.w, (1.0 - b.angular_damping * h).max(0.0));
            let speed = length(s.v);
            if speed > b.max_linear_speed && b.max_linear_speed > 0.0 {
                s.v = scale(s.v, b.max_linear_speed / speed);
            }
            let spin = length(s.w);
            if spin > b.max_angular_speed && b.max_angular_speed > 0.0 {
                s.w = scale(s.w, b.max_angular_speed / spin);
            }
        }
    }

    /// The inverse of a body's inertia, in the world, applied to `a`.
    fn inverse_inertia(&self, body: usize, a: Vec3) -> Vec3 {
        let b = &self.bodies[body];
        if b.mass <= 0.0 {
            return [0.0; 3];
        }
        let r = quat_mat(self.state[body].q);
        let local = mat_t_vec(&r, a);
        let scaled = [
            local[0] / b.inertia[0].max(1e-6),
            local[1] / b.inertia[1].max(1e-6),
            local[2] / b.inertia[2].max(1e-6),
        ];
        mat_vec(&r, scaled)
    }

    /// How readily a body gives at `r` (from its centre) along `n`.
    fn inverse_mass_at(&self, body: usize, r: Vec3, n: Vec3) -> f32 {
        let b = &self.bodies[body];
        if b.mass <= 0.0 {
            return 0.0;
        }
        let rn = cross(r, n);
        1.0 / b.mass + dot(rn, self.inverse_inertia(body, rn))
    }

    /// Moves a body as a positional push `p` at `r` would.
    fn shift(&mut self, body: usize, p: Vec3, r: Vec3) {
        let m = self.bodies[body].mass;
        if m <= 0.0 {
            return;
        }
        let turn = self.inverse_inertia(body, cross(r, p));
        let s = &mut self.state[body];
        s.x = add(s.x, scale(p, 1.0 / m));
        s.q = rotated(s.q, turn);
    }

    /// Changes a body's velocities as an impulse `p` at `r` would.
    fn impulse(&mut self, body: usize, p: Vec3, r: Vec3) {
        let m = self.bodies[body].mass;
        if m <= 0.0 {
            return;
        }
        let turn = self.inverse_inertia(body, cross(r, p));
        let s = &mut self.state[body];
        s.v = add(s.v, scale(p, 1.0 / m));
        s.w = add(s.w, turn);
    }

    /// Turns the first body by `angle` about `axis` (unit) relative to the
    /// second, shared by how readily each turns.
    fn turn_apart(&mut self, a: usize, b: usize, axis: Vec3, angle: f32) {
        let wa = dot(axis, self.inverse_inertia(a, axis));
        let wb = dot(axis, self.inverse_inertia(b, axis));
        if wa + wb <= 0.0 {
            return;
        }
        let p = scale(axis, angle / (wa + wb));
        let ta = self.inverse_inertia(a, p);
        let tb = self.inverse_inertia(b, p);
        self.state[a].q = rotated(self.state[a].q, ta);
        self.state[b].q = rotated(self.state[b].q, scale(tb, -1.0));
    }

    fn solve_joint(&mut self, joint: usize) {
        let JointSetup {
            bodies: [a, b],
            pivots,
            limit,
        } = self.joints[joint];
        // The pivots meet.
        let arm = |me: &Self, body: usize, pivot: Vec3| {
            let r = quat_mat(me.state[body].q);
            mat_vec(&r, sub(pivot, me.bodies[body].center))
        };
        let (ra, rb) = (arm(self, a, pivots[0]), arm(self, b, pivots[1]));
        let gap = sub(add(self.state[b].x, rb), add(self.state[a].x, ra));
        let c = length(gap);
        if c > 1e-6 {
            let n = scale(gap, 1.0 / c);
            let w = self.inverse_mass_at(a, ra, n) + self.inverse_mass_at(b, rb, n);
            if w > 0.0 {
                let p = scale(n, c / w);
                self.shift(a, p, ra);
                self.shift(b, scale(p, -1.0), rb);
            }
        }
        // The angles stay within the limits.
        let rot = |me: &Self, body: usize| quat_mat(me.state[body].q);
        match limit {
            Limit::Cone {
                twist,
                plane,
                cone,
                plane_range,
                twist_range,
            } => {
                let (r1, r2) = (rot(self, a), rot(self, b));
                let (a1, a2) = (mat_vec(&r1, twist[0]), mat_vec(&r2, twist[1]));
                let between = angle_between(a1, a2);
                if between > cone {
                    let axis = normalize(cross(a1, a2));
                    if length(axis) > 0.5 {
                        self.turn_apart(a, b, axis, between - cone);
                    }
                }
                let (r1, r2) = (rot(self, a), rot(self, b));
                let a1 = mat_vec(&r1, twist[0]);
                let c2 = mat_vec(&r2, plane[1]);
                let elevation = dot(a1, c2).clamp(-1.0, 1.0).asin();
                let kept = elevation.clamp(plane_range.0, plane_range.1);
                if kept != elevation {
                    let axis = normalize(cross(c2, a1));
                    if length(axis) > 0.5 {
                        self.turn_apart(a, b, axis, elevation - kept);
                    }
                }
                let (r1, r2) = (rot(self, a), rot(self, b));
                if let Some((t, turned)) = twist_angle(
                    mat_vec(&r1, twist[0]),
                    mat_vec(&r2, twist[1]),
                    mat_vec(&r1, plane[0]),
                    mat_vec(&r2, plane[1]),
                ) {
                    let kept = turned.clamp(twist_range.0, twist_range.1);
                    if kept != turned {
                        self.turn_apart(a, b, t, kept - turned);
                    }
                }
            }
            Limit::Hinge {
                axle,
                perpendicular,
                range,
            } => {
                let (r1, r2) = (rot(self, a), rot(self, b));
                let (a1, a2) = (mat_vec(&r1, axle[0]), mat_vec(&r2, axle[1]));
                let off = angle_between(a1, a2);
                if off > 1e-4 {
                    let axis = normalize(cross(a1, a2));
                    if length(axis) > 0.5 {
                        self.turn_apart(a, b, axis, off);
                    }
                }
                let (r1, r2) = (rot(self, a), rot(self, b));
                let a1 = normalize(mat_vec(&r1, axle[0]));
                let turned = turn_about(
                    a1,
                    mat_vec(&r2, perpendicular[1]),
                    mat_vec(&r1, perpendicular[0]),
                );
                if let Some(turned) = turned {
                    let kept = turned.clamp(range.0, range.1);
                    if kept != turned {
                        self.turn_apart(a, b, a1, kept - turned);
                    }
                }
            }
            Limit::Free => {}
        }
    }

    /// Pushes two of the ragdoll's own capsules apart where they overlap
    /// (bodies whose parts meet, [`parts_meet`]); shared by how readily
    /// each gives there (this solver's, as for the joints).
    fn solve_pair(&mut self, a: usize, b: usize) {
        let (Some((a0, a1, ra)), Some((b0, b1, rb))) =
            (self.bodies[a].capsule, self.bodies[b].capsule)
        else {
            return;
        };
        let ends = |me: &Self, body: usize, p: Vec3, q: Vec3| {
            let (r, origin) = me.frame(body);
            (add(origin, mat_vec(&r, p)), add(origin, mat_vec(&r, q)))
        };
        let (pa, qa) = ends(self, a, a0, a1);
        let (pb, qb) = ends(self, b, b0, b1);
        let (on_a, on_b) = segments_closest(pa, qa, pb, qb);
        let gap = sub(on_b, on_a);
        let d = length(gap);
        let depth = ra + rb - d;
        if depth <= 0.0 || d < 1e-6 {
            return;
        }
        let n = scale(gap, 1.0 / d);
        // The touching points, from each centre.
        let arm_a = sub(add(on_a, scale(n, ra)), self.state[a].x);
        let arm_b = sub(sub(on_b, scale(n, rb)), self.state[b].x);
        let w = self.inverse_mass_at(a, arm_a, n) + self.inverse_mass_at(b, arm_b, n);
        if w <= 0.0 {
            return;
        }
        let p = scale(n, depth / w);
        self.shift(a, scale(p, -1.0), arm_a);
        self.shift(b, p, arm_b);
    }

    /// Pushes a body's capsule out of the triangles it overlaps.
    fn solve_contacts(
        &mut self,
        collider: &Collider,
        body: usize,
        triangles: &[u32],
        contacts: &mut Vec<Contact>,
    ) {
        let Some((ea, eb, radius)) = self.bodies[body].capsule else {
            return;
        };
        for &t in triangles {
            let (r, origin) = self.frame(body);
            let (p, q) = (add(origin, mat_vec(&r, ea)), add(origin, mat_vec(&r, eb)));
            let [ta, tb, tc] = collider.triangle(t);
            let (on_axis, on_triangle) = segment_triangle_closest(p, q, ta, tb, tc);
            let gap = sub(on_axis, on_triangle);
            let d = length(gap);
            if d >= radius {
                continue;
            }
            let n = if d > 1e-5 {
                scale(gap, 1.0 / d)
            } else {
                let face = normalize(cross(sub(tb, ta), sub(tc, ta)));
                let centre = self.state[body].x;
                if dot(face, sub(centre, ta)) >= 0.0 {
                    face
                } else {
                    scale(face, -1.0)
                }
            };
            let depth = radius - d;
            let touch = sub(on_axis, scale(n, radius));
            let arm = sub(touch, self.state[body].x);
            let w = self.inverse_mass_at(body, arm, n);
            if w <= 0.0 {
                continue;
            }
            let lambda = depth / w;
            self.shift(body, scale(n, lambda), arm);
            let r = quat_mat(self.state[body].q);
            contacts.push(Contact {
                body,
                local: mat_t_vec(&r, arm),
                normal: n,
                lambda,
            });
        }
    }
}

/// A joint's angles for bodies turned by `r1` and `r2` (see [`Limit`]).
pub fn joint_angles(limit: &Limit, r1: &Mat3, r2: &Mat3) -> JointAngles {
    match *limit {
        Limit::Cone { twist, plane, .. } => {
            let (a1, a2) = (mat_vec(r1, twist[0]), mat_vec(r2, twist[1]));
            let c2 = mat_vec(r2, plane[1]);
            JointAngles::Cone {
                cone: angle_between(a1, a2),
                plane: dot(normalize(a1), normalize(c2)).clamp(-1.0, 1.0).asin(),
                twist: twist_angle(a1, a2, mat_vec(r1, plane[0]), c2).map_or(0.0, |(_, t)| t),
            }
        }
        Limit::Hinge {
            axle,
            perpendicular,
            ..
        } => {
            let (a1, a2) = (mat_vec(r1, axle[0]), mat_vec(r2, axle[1]));
            JointAngles::Hinge {
                misaligned: angle_between(a1, a2),
                angle: turn_about(
                    normalize(a1),
                    mat_vec(r2, perpendicular[1]),
                    mat_vec(r1, perpendicular[0]),
                )
                .unwrap_or(0.0),
            }
        }
        Limit::Free => JointAngles::Free,
    }
}

/// The closest points of two segments (`p1`–`q1`, `p2`–`q2`): on the
/// first, then on the second (Ericson, "Real-Time Collision Detection",
/// 5.1.9).
fn segments_closest(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> (Vec3, Vec3) {
    let d1 = sub(q1, p1);
    let d2 = sub(q2, p2);
    let r = sub(p1, p2);
    let a = dot(d1, d1);
    let e = dot(d2, d2);
    let f = dot(d2, r);
    let eps = 1e-9;
    let (s, t) = if a <= eps && e <= eps {
        (0.0, 0.0)
    } else if a <= eps {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = dot(d1, r);
        if e <= eps {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = dot(d1, d2);
            let denom = a * e - b * b;
            let mut s = if denom > eps {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t = (b * s + f) / e;
            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
            (s, t)
        }
    };
    (add(p1, scale(d1, s)), add(p2, scale(d2, t)))
}

fn angle_between(a: Vec3, b: Vec3) -> f32 {
    dot(normalize(a), normalize(b)).clamp(-1.0, 1.0).acos()
}

/// The turn about `axis` (unit) that takes `from` to `to`, both flattened
/// across the axis; `None` when either lies along it.
fn turn_about(axis: Vec3, from: Vec3, to: Vec3) -> Option<f32> {
    let flat = |v: Vec3| {
        let f = sub(v, scale(axis, dot(v, axis)));
        (length(f) > 1e-4).then(|| normalize(f))
    };
    let (f, t) = (flat(from)?, flat(to)?);
    Some(dot(axis, cross(f, t)).atan2(dot(f, t)))
}

/// The first body's turn about the shared twist axis relative to the
/// second (between their plane axes), and that axis.
fn twist_angle(a1: Vec3, a2: Vec3, b1: Vec3, b2: Vec3) -> Option<(Vec3, f32)> {
    let mid = add(normalize(a1), normalize(a2));
    let t = if length(mid) > 1e-3 {
        normalize(mid)
    } else {
        normalize(a1)
    };
    Some((t, turn_about(t, b2, b1)?))
}

pub(crate) fn mat_vec(m: &Mat3, v: Vec3) -> Vec3 {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// The transpose (inverse) of a rotation applied to `v`.
pub(crate) fn mat_t_vec(m: &Mat3, v: Vec3) -> Vec3 {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2],
    ]
}

pub(crate) fn quat_mul(a: Quat, b: Quat) -> Quat {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

pub(crate) fn conjugate(q: Quat) -> Quat {
    [-q[0], -q[1], -q[2], q[3]]
}

pub(crate) fn quat_normalize(q: Quat) -> Quat {
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if l > 0.0 {
        q.map(|x| x / l)
    } else {
        [0.0, 0.0, 0.0, 1.0]
    }
}

/// `q` turned by the small rotation vector `by` (in the world).
pub(crate) fn rotated(q: Quat, by: Vec3) -> Quat {
    let d = quat_mul([by[0], by[1], by[2], 0.0], q);
    quat_normalize([
        q[0] + 0.5 * d[0],
        q[1] + 0.5 * d[1],
        q[2] + 0.5 * d[2],
        q[3] + 0.5 * d[3],
    ])
}

pub(crate) fn quat_mat(q: Quat) -> Mat3 {
    let [x, y, z, w] = q;
    [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - w * z),
            2.0 * (x * z + w * y),
        ],
        [
            2.0 * (x * y + w * z),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - w * x),
        ],
        [
            2.0 * (x * z - w * y),
            2.0 * (y * z + w * x),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ]
}

/// A rotation matrix as a quaternion (Shepperd's method).
pub(crate) fn mat_quat(m: &Mat3) -> Quat {
    let trace = m[0][0] + m[1][1] + m[2][2];
    let q = if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        [
            (m[2][1] - m[1][2]) / s,
            (m[0][2] - m[2][0]) / s,
            (m[1][0] - m[0][1]) / s,
            0.25 * s,
        ]
    } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
        let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
        [
            0.25 * s,
            (m[0][1] + m[1][0]) / s,
            (m[0][2] + m[2][0]) / s,
            (m[2][1] - m[1][2]) / s,
        ]
    } else if m[1][1] > m[2][2] {
        let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
        [
            (m[0][1] + m[1][0]) / s,
            0.25 * s,
            (m[1][2] + m[2][1]) / s,
            (m[0][2] - m[2][0]) / s,
        ]
    } else {
        let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
        [
            (m[0][2] + m[2][0]) / s,
            (m[1][2] + m[2][1]) / s,
            0.25 * s,
            (m[1][0] - m[0][1]) / s,
        ]
    };
    quat_normalize(q)
}

#[cfg(test)]
mod tests {
    use super::*;

    const I3: Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    fn floor() -> Collider {
        let mut c = Collider::new();
        c.add(
            &[
                [-1000.0, -1000.0, 0.0],
                [1000.0, -1000.0, 0.0],
                [1000.0, 1000.0, 0.0],
                [-1000.0, 1000.0, 0.0],
            ],
            &[[0, 1, 2], [0, 2, 3]],
        );
        c
    }

    fn rod(mass: f32) -> BodySetup {
        // A capsule 20 long along x, radius 3.
        BodySetup {
            mass,
            center: [10.0, 0.0, 0.0],
            inertia: [mass * 9.0, mass * 40.0, mass * 40.0],
            capsule: Some(([0.0; 3], [20.0, 0.0, 0.0], 3.0)),
            friction: 0.3,
            linear_damping: 0.1,
            angular_damping: 0.05,
            max_linear_speed: 7000.0,
            max_angular_speed: 30.0,
            layer: 8,
            part: 0,
        }
    }

    #[test]
    fn rotations_survive_the_round_trip() {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let m = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let q = mat_quat(&m);
        assert!((q[2] - s).abs() < 1e-5 && (q[3] - s).abs() < 1e-5, "{q:?}");
        let back = quat_mat(q);
        for i in 0..3 {
            for j in 0..3 {
                assert!((back[i][j] - m[i][j]).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn an_added_velocity_moves_every_body_alike_and_wakes_them() {
        // A light and a heavy rod, both asleep: each gains the same
        // velocity (`0062b930` adds it whatever the mass).
        let mut r = Ragdoll::new(
            vec![rod(1.0), rod(50.0)],
            Vec::new(),
            &[(I3, [0.0, 0.0, 100.0]), (I3, [0.0, 50.0, 100.0])],
        );
        r.asleep = true;
        r.add_velocity([20.0, 0.0, 0.0]);
        assert!(!r.asleep);
        r.add_velocity([0.0, -5.0, 0.0]);
        for s in &r.state {
            assert_eq!(s.v, [20.0, -5.0, 0.0]);
        }
    }

    #[test]
    fn a_body_falls_and_rests_on_the_floor() {
        let mut r = Ragdoll::new(vec![rod(5.0)], Vec::new(), &[(I3, [0.0, 0.0, 100.0])]);
        let c = floor();
        for _ in 0..300 {
            r.update(&c, 1.0 / 60.0);
        }
        let (_, origin) = r.frame(0);
        // Lying on its side: its axis the radius above the floor.
        assert!((origin[2] - 3.0).abs() < 0.5, "{origin:?}");
        assert!(r.asleep);
    }

    #[test]
    fn own_bodies_meet_only_as_the_part_table_says() {
        // Two rods crossing at the same spot, held up (no mass for the
        // first). A head (part 1) and a left forearm (6) meet in the
        // game's table (`01268078` row 1 has bit 6); a head and the body
        // (2) don't.
        let crossing = |part_a: u8, part_b: u8| {
            let mut a = rod(0.0);
            a.part = part_a;
            let mut b = rod(4.0);
            b.part = part_b;
            let mut r = Ragdoll::new(
                vec![a, b],
                Vec::new(),
                &[(I3, [0.0, 0.0, 500.0]), (I3, [0.0, 0.0, 502.0])],
            );
            r.step(&Collider::new());
            let (_, low) = r.frame(0);
            let (_, high) = r.frame(1);
            high[2] - low[2]
        };
        assert!(parts_meet(
            &BodySetup {
                part: 1,
                ..rod(1.0)
            },
            &BodySetup {
                part: 6,
                ..rod(1.0)
            }
        ));
        assert!(!parts_meet(
            &BodySetup {
                part: 1,
                ..rod(1.0)
            },
            &BodySetup {
                part: 2,
                ..rod(1.0)
            }
        ));
        // Meeting: pushed apart to the two radii (6), less a step's fall.
        let apart = crossing(1, 6);
        assert!(apart > 5.0, "{apart}");
        // Not meeting: the second falls through the first.
        let through = crossing(1, 2);
        assert!(through < 2.0, "{through}");
    }

    #[test]
    fn joints_hold_and_hinges_keep_their_range() {
        // Two rods end to end along x, joined by a hinge about y that lets
        // the second bend from 0 to 90°, held 200 up. The first is fixed
        // (no mass); the second hangs down under gravity, stopped by the
        // hinge at 90°.
        let joint = JointSetup {
            bodies: [1, 0],
            pivots: [[0.0; 3], [20.0, 0.0, 0.0]],
            limit: Limit::Hinge {
                axle: [[0.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
                perpendicular: [[1.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                range: (0.0, std::f32::consts::FRAC_PI_2),
            },
        };
        let mut r = Ragdoll::new(
            vec![rod(0.0), rod(4.0)],
            vec![joint],
            &[(I3, [0.0, 0.0, 200.0]), (I3, [20.0, 0.0, 200.0])],
        );
        let c = floor();
        for _ in 0..240 {
            r.update(&c, 1.0 / 60.0);
        }
        assert!(r.joint_gap(0) < 0.5, "gap {}", r.joint_gap(0));
        let JointAngles::Hinge { misaligned, angle } = r.joint_angles(0) else {
            unreachable!()
        };
        assert!(misaligned < 0.02, "{misaligned}");
        assert!(
            (angle.abs() - std::f32::consts::FRAC_PI_2).abs() < 0.05,
            "{angle}"
        );
        // Hanging straight down from the first rod's end.
        let (rot, origin) = r.frame(1);
        let tip = add(origin, mat_vec(&rot, [20.0, 0.0, 0.0]));
        assert!(
            (tip[0] - 20.0).abs() < 1.0 && (tip[2] - 180.0).abs() < 1.0,
            "{tip:?}"
        );
    }
}
