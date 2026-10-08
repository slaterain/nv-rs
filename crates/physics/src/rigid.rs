//! Free rigid bodies: the clutter, weapons and props whose models carry a
//! moving Havok body (`nif::RigidBodyInfo`, motion systems other than
//! keyframed and fixed), falling, sliding, tumbling and coming to rest on a
//! [`Collider`] and on each other, pushed by shots, blasts
//! ([`crate::impulses`]) and walkers, as the game hands them to Havok.
//!
//! Read from the game: each body's mass, inertia tensor, centre of mass,
//! linear and angular damping, friction, restitution, most linear and
//! angular speed and its shapes (hulls with their convex radius, boxes,
//! spheres, capsules) from its model; gravity ([`crate::GRAVITY`], the Havok
//! world's); the step: the game steps Havok in fixed steps of
//! `[HAVOK] fMaxTime` (0.016 in `Fallout_default.ini`, exe default 1/60),
//! up to three a frame, carrying the rest over ([`Clock`], `00c66760` with
//! `iUpdateType` 0).
//!
//! This solver's own (Havok's isn't reproduced, labelled where used): each
//! step is split into substeps of extended position-based dynamics (as in
//! [`crate::ragdoll`]); contacts are found between a body's corners (and
//! sphere and capsule centres) and the other side's faces, and between the
//! other side's corners and a body's hull, not edge against edge; a
//! surface added without a body's values rubs and bounces as Havok's
//! default body does (friction 0.5, restitution 0.4, `hkpRigidBodyCinfo`'s
//! defaults); bodies fall asleep after a second nearly still and wake when
//! pushed (Havok's deactivation, by its reference distance and frame
//! counters, isn't translated); walkers push bodies lighter than
//! `fMoveLimitMass` as unstoppable capsules (the character proxy's own
//! impulse, `hkpCharacterProxy`, isn't translated; the game's proxy has
//! infinite strength).
//!
//! Traced here: friction and restitution combine as the square root of
//! the two sides' product, the restitution kept as a byte × 128
//! ([`combined_friction`], [`combined_restitution`], `00cfd800`); the
//! mouse spring ([`Spring`], `00cbb1e0`); the saved velocities
//! ([`RigidWorld::set_velocity`], `00563380`).

use std::collections::HashSet;

use crate::ragdoll::{
    conjugate, mat_quat, mat_t_vec, mat_vec, quat_mat, quat_mul, rotated, Mat3, Quat,
};
use crate::vec::*;
use crate::{
    closest_on_triangle, segment_segment_closest, segment_triangle_closest, Collider, Surface,
    Vec3, GRAVITY, HAVOK_UNIT,
};

pub use crate::havok::{Clock, MAX_STEPS};

/// `[HAVOK] fMaxTime` as `Fallout_default.ini` sets it (the executable's
/// default is 1/60).
pub const STEP: f32 = crate::havok::MAX_TIME;
/// Substeps per step, and position passes over the contacts per substep
/// (choices of this solver, not the game's).
const SUBSTEPS: usize = 8;
const ITERATIONS: usize = 4;
/// The most one correction moves a touching point out (game units): placed
/// objects can start sunk into what they stand on (the VCG02 bottles sit
/// 1.4 units into their rail's shells), and pushing them out at once
/// throws them (this solver's; Havok recovers penetration gradually too,
/// by its own rule, not traced).
const MAX_CORRECTION: f32 = 0.05;
/// What a surface added without its body's values rubs and bounces like:
/// Havok's default body (`hkpRigidBodyCinfo`: friction 0.5, restitution
/// 0.4; not traced in the executable). The game's clutter models carry
/// these same values (`clutter\junk\ssbottle02.nif`).
pub const DEFAULT_SURFACE: Surface = Surface {
    friction: 0.5,
    restitution: 0.4,
};
/// Nearly still: slower than this (game units a second, radians a second)
/// for [`SLEEP_AFTER`] seconds puts a body to sleep (this solver's
/// thresholds; Havok's deactivation isn't traced).
const STILL_SPEED: f32 = 2.0;
const STILL_SPIN: f32 = 0.3;
const SLEEP_AFTER: f32 = 1.0;
/// How far a walker reaches past its capsule to push a body (game units):
/// walkers are kept their radius and the body's shell off it by the
/// collider, so the push is felt this much further out (this solver's).
pub const PUSH_SKIN: f32 = 2.0;
/// `[HAVOK] fMoveLimitMass` (95): the character controller's contact
/// callback (`00c711d0`, the copy at `011b0128`) treats a body at least
/// this heavy apart from lighter ones (it keeps the contact's surface
/// velocity only for lighter bodies). Read as: walkers push only bodies
/// lighter than this (the proxy's handling past that branch isn't traced).
pub const MOVE_LIMIT_MASS: f32 = 95.0;

/// One solid piece of a body, in its model's space (game units).
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// A convex hull: corners, face planes `n·p + d = 0` (`n` out, unit
    /// length) and Havok's convex radius around it.
    Hull {
        vertices: Vec<Vec3>,
        planes: Vec<[f32; 4]>,
        shell: f32,
    },
    Sphere {
        center: Vec3,
        radius: f32,
    },
    Capsule {
        a: Vec3,
        b: Vec3,
        radius: f32,
    },
    /// Triangles on a moving body: only their corners touch here.
    Mesh {
        vertices: Vec<Vec3>,
        triangles: Vec<[u32; 3]>,
        shell: f32,
    },
}

impl Shape {
    /// A hull with its planes made unit length.
    pub fn hull(vertices: Vec<Vec3>, planes: &[[f32; 4]], shell: f32) -> Shape {
        let planes = planes
            .iter()
            .filter_map(|p| {
                let l = length([p[0], p[1], p[2]]);
                (l > 1e-9).then(|| [p[0] / l, p[1] / l, p[2] / l, p[3] / l])
            })
            .collect();
        Shape::Hull {
            vertices,
            planes,
            shell,
        }
    }

    /// The shape as triangles in its model's space, and how far its
    /// surface stands out from them: for the collider, which walkers and
    /// shots meet.
    pub fn triangles(&self) -> (Vec<Vec3>, Vec<[u32; 3]>, f32) {
        match self {
            Shape::Hull {
                vertices,
                planes,
                shell,
            } => (
                vertices.clone(),
                crate::shapes::hull(vertices, planes),
                *shell,
            ),
            Shape::Sphere { center, radius } => {
                let (v, t) = crate::shapes::sphere(*center, *radius);
                (v, t, 0.0)
            }
            Shape::Capsule { a, b, radius } => {
                let (v, t) = crate::shapes::capsule(*a, *b, *radius);
                (v, t, 0.0)
            }
            Shape::Mesh {
                vertices,
                triangles,
                shell,
            } => (vertices.clone(), triangles.clone(), *shell),
        }
    }

    /// The farthest the shape's surface reaches from `from`.
    fn reach(&self, from: Vec3) -> f32 {
        let far = |v: &[Vec3], extra: f32| {
            v.iter()
                .map(|&p| length(sub(p, from)))
                .fold(0.0f32, f32::max)
                + extra
        };
        match self {
            Shape::Hull {
                vertices, shell, ..
            }
            | Shape::Mesh {
                vertices, shell, ..
            } => far(vertices, *shell),
            Shape::Sphere { center, radius } => far(&[*center], *radius),
            Shape::Capsule { a, b, radius } => far(&[*a, *b], *radius),
        }
    }
}

/// A body as its model gives it, in the model's space (game units; a
/// placed reference's scale already applied to the shapes and the centre).
#[derive(Debug, Clone, PartialEq)]
pub struct RigidSetup {
    /// The placed reference it belongs to: its triangles in the collider
    /// carry this owner.
    pub reference: u32,
    /// Its Havok layer (`nif::collision::layers`).
    pub layer: u8,
    pub mass: f32,
    /// The centre of mass.
    pub center: Vec3,
    /// The inertia tensor about the centre (mass × game units²).
    pub inertia: Mat3,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub friction: f32,
    pub restitution: f32,
    /// Game units a second, radians a second (0: no limit).
    pub max_linear_speed: f32,
    pub max_angular_speed: f32,
    /// Havok's motion system (`nif::RigidBodyInfo::motion`).
    pub motion: u8,
    /// The wind pushes it (`nif::collision::BODY_WIND`, [`crate::wind`]).
    pub wind: bool,
    pub shapes: Vec<Shape>,
}

/// A rotation and a translation: model space to the world.
pub type Pose = (Mat3, Vec3);

/// A body in the world.
#[derive(Debug, Clone)]
pub struct Rigid {
    pub setup: RigidSetup,
    /// The inverse inertia in the model's space (zero: it doesn't turn).
    inverse_inertia: Mat3,
    inverse_mass: f32,
    /// The centre of mass in the world, the turn, the velocities.
    x: Vec3,
    q: Quat,
    v: Vec3,
    w: Vec3,
    /// Where it was put: the pose its triangles were added to the collider
    /// at ([`RigidWorld::delta`]).
    rest: Pose,
    /// How far its surface reaches from the centre of mass.
    reach: f32,
    still: f32,
    /// How far the centre moved this substep, kept apart from `x` (far
    /// from the origin a position's rounding is worth several units a
    /// second of speed).
    moved_by: Vec3,
    /// At rest: not stepped until something pushes it.
    pub asleep: bool,
    /// Havok's motion: its speed limits, gravity factor and deactivation
    /// state (the position and velocities are the fields above).
    pub motion: crate::havok::Motion,
    /// Stepped by Havok's integrator alone this step (nothing near it).
    free: bool,
    /// Moved since it was put (the game's "Havok moved" reference change,
    /// `CHANGE_REFR_HAVOK_MOVE`, named by `0083fef0`).
    pub moved: bool,
}

/// The hkUFloat8 index of a model's most speed (Havok units a second, or
/// radians a second): [`crate::havok::ufloat8`], taking the entry below
/// when the value is that entry give or take the rounding of nv-rs's
/// game-unit scaling (1e-5 of it).
pub(crate) fn speed_index(v: f32) -> u8 {
    let i = crate::havok::ufloat8(v);
    if i > 0 {
        let below = crate::havok::UFLOAT8[i as usize - 1];
        if (below - v).abs() <= below * 1e-5 {
            return i - 1;
        }
    }
    i
}

fn mat_mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut m = [[0.0; 3]; 3];
    for (i, row) in m.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    m
}

fn transpose(a: &Mat3) -> Mat3 {
    let mut m = [[0.0; 3]; 3];
    for (i, row) in m.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = a[j][i];
        }
    }
    m
}

/// The inverse of a 3 × 3 matrix (zero when it has none).
fn invert(m: &Mat3) -> Mat3 {
    let c =
        |r1: usize, c1: usize, r2: usize, c2: usize| m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1];
    let cof = [
        [c(1, 1, 2, 2), -c(1, 0, 2, 2), c(1, 0, 2, 1)],
        [-c(0, 1, 2, 2), c(0, 0, 2, 2), -c(0, 0, 2, 1)],
        [c(0, 1, 1, 2), -c(0, 0, 1, 2), c(0, 0, 1, 1)],
    ];
    let det = m[0][0] * cof[0][0] + m[0][1] * cof[0][1] + m[0][2] * cof[0][2];
    if det.abs() < 1e-12 {
        return [[0.0; 3]; 3];
    }
    let mut out = [[0.0; 3]; 3];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = cof[j][i] / det;
        }
    }
    out
}

impl Rigid {
    fn new(setup: RigidSetup, pose: Pose) -> Rigid {
        let dynamic = crate::impulses::moves(setup.motion) && setup.mass > 0.0;
        let inverse_mass = if dynamic { 1.0 / setup.mass } else { 0.0 };
        let inverse_inertia = if dynamic {
            invert(&setup.inertia)
        } else {
            [[0.0; 3]; 3]
        };
        let reach = setup
            .shapes
            .iter()
            .map(|s| s.reach(setup.center))
            .fold(0.0f32, f32::max);
        let (r, t) = pose;
        let mut motion = crate::havok::Motion::new(
            crate::havok::motion_type::DYNAMIC,
            add(mat_vec(&r, setup.center), t),
            mat_quat(&r),
        );
        motion.linear_damping = setup.linear_damping;
        motion.angular_damping = setup.angular_damping;
        motion.max_linear_velocity = speed_index(setup.max_linear_speed / HAVOK_UNIT);
        motion.max_angular_velocity = speed_index(setup.max_angular_speed);
        motion.object_radius = reach / HAVOK_UNIT;
        Rigid {
            x: add(mat_vec(&r, setup.center), t),
            q: mat_quat(&r),
            v: [0.0; 3],
            w: [0.0; 3],
            rest: pose,
            reach,
            still: 0.0,
            moved_by: [0.0; 3],
            asleep: true,
            moved: false,
            motion,
            free: false,
            inverse_inertia,
            inverse_mass,
            setup,
        }
    }

    /// Model space to the world now.
    pub fn pose(&self) -> Pose {
        let r = quat_mat(self.q);
        (r, sub(self.x, mat_vec(&r, self.setup.center)))
    }

    /// Where it was put (its triangles' pose in the collider).
    pub fn rest(&self) -> Pose {
        self.rest
    }

    /// The centre of mass in the world.
    pub fn center(&self) -> Vec3 {
        self.x
    }

    pub fn velocity(&self) -> Vec3 {
        self.v
    }

    pub fn spin(&self) -> Vec3 {
        self.w
    }

    pub fn dynamic(&self) -> bool {
        self.inverse_mass > 0.0
    }

    /// Moved by this solver's contact substeps this step.
    fn solved(&self) -> bool {
        !self.asleep && self.dynamic() && !self.free
    }

    /// Its Havok motion with the position and velocities as they are now
    /// (game units).
    fn motion_now(&self) -> crate::havok::Motion {
        let mut m = self.motion;
        m.center = self.x;
        m.rotation = self.q;
        m.linear_velocity = self.v;
        m.angular_velocity = self.w;
        m
    }

    fn set_motion(&mut self, m: &crate::havok::Motion) {
        self.motion = *m;
        self.x = m.center;
        self.q = m.rotation;
        self.v = m.linear_velocity;
        self.w = m.angular_velocity;
    }

    /// The inverse inertia in the world applied to `a`.
    fn inverse_inertia_world(&self, a: Vec3) -> Vec3 {
        let r = quat_mat(self.q);
        mat_vec(&r, mat_vec(&self.inverse_inertia, mat_t_vec(&r, a)))
    }

    /// How readily it gives at `arm` (from its centre) along `n`.
    fn give(&self, arm: Vec3, n: Vec3) -> f32 {
        if self.inverse_mass == 0.0 {
            return 0.0;
        }
        let rn = cross(arm, n);
        self.inverse_mass + dot(rn, self.inverse_inertia_world(rn))
    }

    /// Moves it as a positional push `p` at `arm` would.
    fn shift(&mut self, p: Vec3, arm: Vec3) {
        if self.inverse_mass == 0.0 {
            return;
        }
        let turn = self.inverse_inertia_world(cross(arm, p));
        self.x = add(self.x, scale(p, self.inverse_mass));
        self.moved_by = add(self.moved_by, scale(p, self.inverse_mass));
        self.q = rotated(self.q, turn);
    }

    /// Changes its velocities as an impulse `p` (game units) at `arm` would.
    fn kick(&mut self, p: Vec3, arm: Vec3) {
        if self.inverse_mass == 0.0 {
            return;
        }
        let turn = self.inverse_inertia_world(cross(arm, p));
        self.v = add(self.v, scale(p, self.inverse_mass));
        self.w = add(self.w, turn);
    }

    fn point_velocity(&self, arm: Vec3) -> Vec3 {
        add(self.v, cross(self.w, arm))
    }

    /// Model-space point to the world.
    fn to_world(&self, p: Vec3) -> Vec3 {
        let r = quat_mat(self.q);
        add(self.x, mat_vec(&r, sub(p, self.setup.center)))
    }

    /// World point to the model's space.
    fn to_model(&self, p: Vec3) -> Vec3 {
        let r = quat_mat(self.q);
        add(self.setup.center, mat_t_vec(&r, sub(p, self.x)))
    }
}

/// A walker, as the bodies feel it: an upright capsule from the feet up
/// `height`, `radius` wide, moving at `velocity`, which nothing stops.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mover {
    pub feet: Vec3,
    pub radius: f32,
    pub height: f32,
    pub velocity: Vec3,
}

impl Mover {
    fn segment(&self) -> (Vec3, Vec3) {
        let low = self.feet[2] + self.radius;
        let high = (self.feet[2] + self.height - self.radius).max(low);
        (
            [self.feet[0], self.feet[1], low],
            [self.feet[0], self.feet[1], high],
        )
    }
}

/// The other side of a contact.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Other {
    /// The collider's triangle (still).
    World(u32),
    Body(usize),
    /// A walker moving at this velocity.
    Mover(Vec3),
}

/// A contact found while correcting positions, for the velocity pass.
#[derive(Debug, Clone, Copy)]
struct Contact {
    a: usize,
    other: Other,
    /// From each side's centre to the touching point, in the world.
    arm_a: Vec3,
    arm_b: Vec3,
    /// Pushes `a` away from the other side.
    n: Vec3,
    /// How much correction it took (mass × distance).
    lambda: f32,
    friction: f32,
    restitution: f32,
    /// The normal speed of `a` against the other side before the substep.
    approach: f32,
}

/// A touching point before it's solved: on `a` (a model-space point of
/// it), on the other side (a model-space point of the other body, or a
/// world point), the normal pushing `a` away, and the gap they keep.
#[derive(Debug, Clone, Copy)]
struct Touch {
    on_a: Vec3,
    on_b: Vec3,
    n: Vec3,
    margin: f32,
    surface: Surface,
}

/// Havok's mouse spring (`hkpMouseSpringAction`, the player's Z-key grab,
/// `crate::grab`): pulls a point of a body toward a point in the world.
/// The fields are the action's (Havok's names; `+0x20`…`+0x4c` in the
/// object `00cbb1e0` reads).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    pub body: usize,
    /// `m_positionInRbLocal`: the held point, in the body's model space.
    pub local: Vec3,
    /// `m_mousePositionInWorld` (game units).
    pub target: Vec3,
    /// `m_springDamping`, `m_springElasticity`.
    pub damping: f32,
    pub elasticity: f32,
    /// `m_maxRelativeForce`: the most force per unit mass (Havok units a
    /// second²).
    pub max_relative_force: f32,
    /// `m_objectDamping`: the body's velocities are multiplied by this
    /// every step.
    pub object_damping: f32,
}

/// A body coming into contact with something (Havok's "contact point
/// added", which the game's collision listener `00623cb0` hears for its
/// impact sounds and physics damage).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContactEvent {
    pub body: usize,
    /// The other body, or the collider's triangle it met.
    pub other_body: Option<usize>,
    pub triangle: Option<u32>,
    /// Where (game units).
    pub point: Vec3,
    /// How fast they closed along the contact's normal (game units a
    /// second; Havok's projected velocity × 7).
    pub speed: f32,
}

/// What a body touches, for telling new contacts from old: another body,
/// or the collider's triangles of one owner (0: the place's static ones).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Toucher {
    Body(usize),
    World(u32),
}

/// Free rigid bodies and the walkers that push them.
#[derive(Debug, Clone, Default)]
pub struct RigidWorld {
    pub bodies: Vec<Rigid>,
    pub clock: Clock,
    /// The world's solver settings (`hkpSolverInfo`), rescaled each frame.
    pub solver: crate::havok::SolverInfo,
    movers: Vec<Mover>,
    /// The player's grab, if they hold something.
    pub spring: Option<Spring>,
    /// Pairs touching at the end of the last step.
    touching: HashSet<(usize, Toucher)>,
    /// This step's touching pairs with their fastest closing.
    this_step: std::collections::HashMap<(usize, Toucher), ContactEvent>,
    /// Contacts begun since [`RigidWorld::take_contacts`].
    began: Vec<ContactEvent>,
}

impl RigidWorld {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a body at `pose`, asleep, and gives its index.
    pub fn add(&mut self, setup: RigidSetup, pose: Pose) -> usize {
        self.bodies.push(Rigid::new(setup, pose));
        self.bodies.len() - 1
    }

    /// The body of a reference.
    pub fn find(&self, reference: u32) -> Option<usize> {
        self.bodies
            .iter()
            .position(|b| b.setup.reference == reference)
    }

    /// Takes a reference's body out, giving it back.
    pub fn remove(&mut self, reference: u32) -> Option<Rigid> {
        let i = self.find(reference)?;
        Some(self.remove_at(i))
    }

    /// Whether another body or a walker is within body `i`'s reach and a
    /// step's travel (then this solver's contacts may act on it).
    fn near_anything(&self, i: usize, dt: f32) -> bool {
        let b = &self.bodies[i];
        let travel = length(b.v) * dt;
        let bodies = self
            .bodies
            .iter()
            .enumerate()
            .any(|(j, o)| j != i && length(sub(b.x, o.x)) <= b.reach + o.reach + 2.0 + travel);
        bodies
            || (self.pushable(i)
                && self.movers.iter().any(|m| {
                    let (s0, s1) = m.segment();
                    length(sub(b.x, closest_on_segment(b.x, s0, s1)))
                        <= b.reach + m.radius + PUSH_SKIN + travel
                }))
    }

    /// Whether walkers push body `i`: it moves and is lighter than
    /// [`MOVE_LIMIT_MASS`].
    fn pushable(&self, i: usize) -> bool {
        let b = &self.bodies[i];
        b.dynamic() && b.setup.mass < MOVE_LIMIT_MASS
    }

    /// Sets a body's velocities (game units and radians a second) and
    /// wakes it: a moving body coming back from a save (`00563380` sets
    /// both, then activates the body).
    pub fn set_velocity(&mut self, body: usize, linear: Vec3, angular: Vec3) {
        let b = &mut self.bodies[body];
        if !b.dynamic() {
            return;
        }
        b.v = linear;
        b.w = angular;
        self.wake(body);
    }

    /// The contacts begun since the last call (each pair once when it
    /// starts touching; resting pairs don't repeat).
    pub fn take_contacts(&mut self) -> Vec<ContactEvent> {
        std::mem::take(&mut self.began)
    }

    /// The heaviest other body body `i` touches now (0: none; the world's
    /// fixed bodies have no mass here, as Havok's fixed bodies report 0).
    pub fn heaviest_contact(&self, i: usize) -> f32 {
        let mut heaviest = 0.0f32;
        for j in 0..self.bodies.len() {
            if j == i || !self.bodies[j].dynamic() {
                continue;
            }
            let gap = length(sub(self.bodies[i].x, self.bodies[j].x));
            if gap > self.bodies[i].reach + self.bodies[j].reach + 2.0 {
                continue;
            }
            if !self.body_touches(i, j).is_empty() {
                heaviest = heaviest.max(self.bodies[j].setup.mass);
            }
        }
        heaviest
    }

    /// Takes body `i` out (a grab on it ends).
    pub fn remove_at(&mut self, i: usize) -> Rigid {
        // Pairs are kept by index: start them afresh.
        self.touching.clear();
        self.this_step.clear();
        self.began.clear();
        if let Some(s) = &mut self.spring {
            if s.body == i {
                self.spring = None;
            } else if s.body > i {
                s.body -= 1;
            }
        }
        self.bodies.remove(i)
    }

    /// Where a body's model-space point is now.
    pub fn point(&self, body: usize, local: Vec3) -> Vec3 {
        self.bodies[body].to_world(local)
    }

    /// A world point as a point of a body's model space.
    pub fn local_point(&self, body: usize, world: Vec3) -> Vec3 {
        self.bodies[body].to_model(world)
    }

    /// `hkpMouseSpringAction::applyAction` (Havok's, named by the Xbox
    /// PDB): the body's velocities × the object damping; then the impulse
    /// at the held point that would take away the spring damping × the
    /// point's velocity plus the elasticity × the distance to the target ÷
    /// the step, through the body's inverse mass matrix at that point;
    /// at most the step × the mass × the most relative force; applied at
    /// the point. Nothing when that matrix has no inverse.
    // Translated from 00cbb1e0 (decompiled, FalloutNV.exe 1.4.0.525)
    fn apply_spring(&mut self, s: &Spring, dt: f32) {
        let b = &mut self.bodies[s.body];
        if !b.dynamic() || dt <= 0.0 {
            return;
        }
        let p = b.to_world(s.local);
        let err = sub(p, s.target);
        let r = sub(p, b.x);
        // K: an impulse j at the point changes its velocity by K j.
        let k_of = |b: &Rigid, j: Vec3| {
            add(
                scale(j, b.inverse_mass),
                cross(b.inverse_inertia_world(cross(r, j)), r),
            )
        };
        let cols = [
            k_of(b, [1.0, 0.0, 0.0]),
            k_of(b, [0.0, 1.0, 0.0]),
            k_of(b, [0.0, 0.0, 1.0]),
        ];
        let k: Mat3 = [
            [cols[0][0], cols[1][0], cols[2][0]],
            [cols[0][1], cols[1][1], cols[2][1]],
            [cols[0][2], cols[1][2], cols[2][2]],
        ];
        let k_inv = invert(&k);
        if k_inv == [[0.0; 3]; 3] {
            return;
        }
        b.v = scale(b.v, s.object_damping);
        b.w = scale(b.w, s.object_damping);
        let v_point = b.point_velocity(r);
        let want = add(scale(v_point, s.damping), scale(err, s.elasticity / dt));
        let mut j = scale(mat_vec(&k_inv, want), -1.0);
        // The limit in game units: Havok's force per mass is in Havok
        // units a second².
        let limit = dt * s.max_relative_force * HAVOK_UNIT / b.inverse_mass;
        let l = length(j);
        if l > limit {
            j = scale(j, limit / l);
        }
        b.kick(j, r);
        b.moved = true;
    }

    /// Puts a body at a pose, still and asleep (a moved reference coming
    /// back where it was left).
    pub fn place(&mut self, body: usize, pose: Pose, moved: bool) {
        let b = &mut self.bodies[body];
        let (r, t) = pose;
        b.x = add(mat_vec(&r, b.setup.center), t);
        b.q = mat_quat(&r);
        b.v = [0.0; 3];
        b.w = [0.0; 3];
        b.asleep = true;
        b.moved = moved;
    }

    /// How a body has moved since it was put: the rotation and translation
    /// that take its rest pose to where it is (for
    /// [`Collider::move_owner`] and the drawing).
    pub fn delta(&self, body: usize) -> Pose {
        let b = &self.bodies[body];
        let (r, t) = b.pose();
        let (r0, t0) = b.rest;
        let rd = mat_mul(&r, &transpose(&r0));
        (rd, sub(t, mat_vec(&rd, t0)))
    }

    /// The walkers this frame.
    pub fn set_movers(&mut self, movers: Vec<Mover>) {
        self.movers = movers;
    }

    /// Wakes a body (it's stepped until it comes to rest again): a placed
    /// object settling when its place loads.
    pub fn wake(&mut self, body: usize) {
        let b = &mut self.bodies[body];
        if b.dynamic() {
            b.asleep = false;
            b.still = 0.0;
        }
    }

    /// `hkpRigidBody::applyPointImpulse`: an impulse in Havok units (mass ×
    /// Havok units a second) at a world point.
    pub fn apply_point_impulse(&mut self, body: usize, impulse: Vec3, point: Vec3) {
        self.wake(body);
        let b = &mut self.bodies[body];
        let arm = sub(point, b.x);
        b.kick(scale(impulse, HAVOK_UNIT), arm);
        b.moved = true;
    }

    /// `hkpMotion::applyForce(deltaTime, force)` (Havok's, Xbox PDB slot
    /// `+0x5c`) after waking the body (`00c9c1d0`): its velocity gains
    /// force ÷ mass × `dt` (a force in Havok units).
    pub fn apply_force(&mut self, body: usize, force: Vec3, dt: f32) {
        if !self.bodies[body].dynamic() {
            return;
        }
        self.wake(body);
        let b = &mut self.bodies[body];
        b.kick(scale(force, HAVOK_UNIT * dt), [0.0; 3]);
        b.moved = true;
    }

    /// `hkpRigidBody::applyLinearImpulse`: through the centre of mass.
    pub fn apply_linear_impulse(&mut self, body: usize, impulse: Vec3) {
        self.wake(body);
        let b = &mut self.bodies[body];
        b.kick(scale(impulse, HAVOK_UNIT), [0.0; 3]);
        b.moved = true;
    }

    /// `hkpRigidBody::applyAngularImpulse`: Havok units (mass × Havok
    /// units² a second).
    pub fn apply_angular_impulse(&mut self, body: usize, impulse: Vec3) {
        self.wake(body);
        let b = &mut self.bodies[body];
        if b.inverse_mass == 0.0 {
            return;
        }
        let turn = b.inverse_inertia_world(scale(impulse, HAVOK_UNIT * HAVOK_UNIT));
        b.w = add(b.w, turn);
        b.moved = true;
    }

    /// Whether any body is awake.
    pub fn awake(&self) -> bool {
        self.bodies.iter().any(|b| !b.asleep)
    }

    /// Moves on by a frame of `frame` seconds, in the game's steps
    /// ([`Clock`]); the steps taken.
    ///
    /// As `bhkWorld::Update` (`00c6ae70`) runs a frame with a frame time
    /// over 0: the solver settings scaled to the step (`00c66a00`), then
    /// `hkpWorld::stepDeltaTime` for each whole step to the frame marker
    /// (see [`Clock`]). The wind listener (`00c66e20`) runs after, with the
    /// frame's time (the viewer's `clutter`).
    // Translated from 00c6ae70 (decompiled, FalloutNV.exe 1.4.0.525)
    pub fn update(&mut self, collider: &Collider, frame: f32) -> u32 {
        let (n, _) = self.clock.advance(frame);
        if frame <= 0.0 {
            return 0;
        }
        let step = self.clock.step();
        self.solver.scale(step, crate::havok::TAU_RATIO);
        for _ in 0..n {
            self.step(collider, step);
        }
        n
    }

    /// One step of `dt` seconds (`hkpSimulation::stepDeltaTime` `00cf8730`:
    /// integrate, collide, advance time; here the integration and this
    /// solver's contacts together).
    pub fn step(&mut self, collider: &Collider, dt: f32) {
        // Walkers wake what they push.
        for i in 0..self.bodies.len() {
            if self.bodies[i].asleep && self.pushable(i) {
                let pushed = self.movers.iter().any(|m| {
                    self.mover_touches(i, m)
                        .iter()
                        .any(|t| dot(m.velocity, t.n) > STILL_SPEED)
                });
                if pushed {
                    self.wake(i);
                }
            }
        }
        // The grab's spring acts first (an action, before Havok solves).
        if let Some(s) = self.spring {
            if s.body < self.bodies.len() && self.bodies[s.body].dynamic() {
                self.wake(s.body);
                self.apply_spring(&s, dt);
            }
        }
        // `hkpSimulation::integrateInternal` (`00cf8da0`): actions (the
        // spring, above), then the deactivation flags, then the islands.
        self.solver.increment_deactivation_flags();
        if !self.awake() || dt <= 0.0 {
            return;
        }
        // Moving bodies wake the sleeping ones they run into (from the
        // next step on, a sleeping one holds still like the world).
        for i in 0..self.bodies.len() {
            if self.bodies[i].asleep {
                continue;
            }
            for j in 0..self.bodies.len() {
                if j == i || !self.bodies[j].asleep || !self.bodies[j].dynamic() {
                    continue;
                }
                let gap = length(sub(self.bodies[i].x, self.bodies[j].x));
                if gap > self.bodies[i].reach + self.bodies[j].reach + 2.0 {
                    continue;
                }
                let speed =
                    length(self.bodies[i].v) + length(self.bodies[i].w) * self.bodies[i].reach;
                if speed > STILL_SPEED && !self.body_touches(i, j).is_empty() {
                    self.wake(j);
                }
            }
        }
        let owners: HashSet<u32> = self.bodies.iter().map(|b| b.setup.reference).collect();
        // The triangles each awake body might touch this step: near its
        // centre, not its own or another body's.
        let nearby: Vec<Vec<u32>> = (0..self.bodies.len())
            .map(|i| {
                let b = &self.bodies[i];
                if b.asleep {
                    return Vec::new();
                }
                let reach = b.reach + length(b.v) * dt + 8.0;
                let lo = sub(b.x, [reach; 3]);
                let hi = add(b.x, [reach; 3]);
                collider
                    .near(lo, hi)
                    .into_iter()
                    .filter(|&t| !owners.contains(&collider.owner(t)))
                    .filter(|&t| {
                        let [p, q, s] = collider.triangle(t);
                        p[2].max(q[2]).max(s[2]) >= lo[2] && p[2].min(q[2]).min(s[2]) <= hi[2]
                    })
                    .collect()
            })
            .collect();
        // Havok's integration (`00cf8da0`): a body with nothing near it
        // (no triangle, body or walker within its reach and a step's
        // travel: no contact constraints in its island) is stepped exactly
        // by `hkRigidMotionUtilApplyForcesAndStep` (`00d28a30`); the others
        // get the same forces (`00d29830`: damping now, gravity per
        // substep), this solver's contacts, then the velocity checks of
        // `hkRigidMotionUtilApplyAccumulators` (`00d29bf0`).
        let gravity_step = scale(crate::havok::GRAVITY, dt * HAVOK_UNIT);
        for (i, tris) in nearby.iter().enumerate() {
            let free = !self.bodies[i].asleep
                && self.bodies[i].dynamic()
                && tris.is_empty()
                && !self.near_anything(i, dt);
            let b = &mut self.bodies[i];
            b.free = free;
            if b.asleep || !b.dynamic() {
                continue;
            }
            let mut m = b.motion_now();
            if free {
                m.step(dt, gravity_step, HAVOK_UNIT);
            } else {
                m.apply_damping(dt);
            }
            b.set_motion(&m);
        }
        let h = dt / SUBSTEPS as f32;
        self.this_step.clear();
        for _ in 0..SUBSTEPS {
            self.substep(collider, &nearby, h);
        }
        for b in &mut self.bodies {
            if b.asleep || !b.dynamic() || b.free {
                continue;
            }
            let mut m = b.motion_now();
            m.reset_invalid_velocities(HAVOK_UNIT);
            m.clamp_linear_velocity(HAVOK_UNIT);
            m.clamp_angular_velocity(dt);
            b.set_motion(&m);
        }
        // Contacts begun this step (Havok adds a contact point once).
        let now: HashSet<(usize, Toucher)> = self.this_step.keys().copied().collect();
        for (key, event) in &self.this_step {
            if !self.touching.contains(key) {
                self.began.push(*event);
            }
        }
        // A pair with a sleeping side stays as it was.
        let kept: Vec<(usize, Toucher)> = self
            .touching
            .iter()
            .filter(|(a, _)| self.bodies.get(*a).is_some_and(|b| b.asleep))
            .copied()
            .collect();
        self.touching = now;
        self.touching.extend(kept);
        for b in &mut self.bodies {
            if b.asleep {
                continue;
            }
            let moving = length(b.v) > STILL_SPEED || length(b.w) > STILL_SPIN;
            b.moved |= moving;
            b.still = if moving { 0.0 } else { b.still + dt };
            if b.still > SLEEP_AFTER {
                b.asleep = true;
                b.v = [0.0; 3];
                b.w = [0.0; 3];
            }
        }
    }

    fn substep(&mut self, collider: &Collider, nearby: &[Vec<u32>], h: f32) {
        let before: Vec<(Vec3, Quat, Vec3, Vec3)> =
            self.bodies.iter().map(|b| (b.x, b.q, b.v, b.w)).collect();
        for b in &mut self.bodies {
            if !b.solved() {
                continue;
            }
            // The solver's gravity per substep, × the gravity factor.
            let g = crate::havok::from_half(b.motion.gravity_factor) * h * HAVOK_UNIT;
            b.v = add(b.v, scale(crate::havok::GRAVITY, g));
            b.x = add(b.x, scale(b.v, h));
            b.moved_by = scale(b.v, h);
            b.q = rotated(b.q, scale(b.w, h));
        }
        // What touches what, found once, then corrected a few times over
        // (each correction measured from where the last left the bodies).
        let mut touches: Vec<(usize, Other, Touch)> = Vec::new();
        for (i, tris) in nearby.iter().enumerate() {
            if !self.bodies[i].solved() {
                continue;
            }
            for (tri, t) in self.world_touches(collider, i, tris) {
                touches.push((i, Other::World(tri), t));
            }
            for j in 0..self.bodies.len() {
                if j == i || (j < i && !self.bodies[j].asleep) {
                    continue;
                }
                let gap = length(sub(self.bodies[i].x, self.bodies[j].x));
                if gap > self.bodies[i].reach + self.bodies[j].reach + 2.0 {
                    continue;
                }
                let found = self.body_touches(i, j);
                touches.extend(found.into_iter().map(|t| (i, Other::Body(j), t)));
            }
            if self.pushable(i) {
                for m in &self.movers {
                    for t in self.mover_touches(i, m) {
                        touches.push((i, Other::Mover(m.velocity), t));
                    }
                }
            }
        }
        let mut lambdas = vec![0.0f32; touches.len()];
        for _ in 0..ITERATIONS {
            for (k, &(a, other, t)) in touches.iter().enumerate() {
                lambdas[k] += self.correct(a, other, &t);
            }
        }
        let contacts: Vec<Contact> = touches
            .iter()
            .zip(&lambdas)
            .filter(|(_, &l)| l > 0.0)
            .map(|(&(a, other, t), &l)| self.contact(a, other, &t, l, &before))
            .collect();
        for c in &contacts {
            let (key, other_body, triangle) = match c.other {
                Other::World(t) => (Toucher::World(collider.owner(t)), None, Some(t)),
                Other::Body(j) => (Toucher::Body(j), Some(j), None),
                Other::Mover(_) => continue,
            };
            // A pair of bodies once, under the lower index.
            let pair = match key {
                Toucher::Body(j) if j < c.a => (j, Toucher::Body(c.a)),
                k => (c.a, k),
            };
            let event = ContactEvent {
                body: c.a,
                other_body,
                triangle,
                point: add(self.bodies[c.a].x, c.arm_a),
                speed: c.approach.abs(),
            };
            self.this_step
                .entry(pair)
                .and_modify(|old| {
                    if event.speed > old.speed {
                        *old = event;
                    }
                })
                .or_insert(event);
        }
        // Velocities from the moves.
        for (b, (_, q, _, _)) in self.bodies.iter_mut().zip(&before) {
            if !b.solved() {
                continue;
            }
            b.v = scale(b.moved_by, 1.0 / h);
            let d = quat_mul(b.q, conjugate(*q));
            let w = scale([d[0], d[1], d[2]], 2.0 / h);
            b.w = if d[3] < 0.0 { scale(w, -1.0) } else { w };
        }
        // Bounce (or stop approaching), with each contact's push summed
        // over a few passes, then friction.
        let mut pushed = vec![0.0f32; contacts.len()];
        for _ in 0..ITERATIONS {
            for (c, p) in contacts.iter().zip(pushed.iter_mut()) {
                self.normal_pass(c, h, p);
            }
        }
        for c in &contacts {
            self.friction_pass(c, h);
        }
    }

    /// Both sides' points of a touch in the world, and the arms from their
    /// centres.
    fn ends(&self, a: usize, other: Other, t: &Touch) -> (Vec3, Vec3, Vec3, Vec3) {
        let pa = self.bodies[a].to_world(t.on_a);
        let pb = match other {
            Other::Body(j) => self.bodies[j].to_world(t.on_b),
            _ => t.on_b,
        };
        let arm_b = match other {
            Other::Body(j) => sub(pb, self.bodies[j].x),
            _ => [0.0; 3],
        };
        (pa, pb, sub(pa, self.bodies[a].x), arm_b)
    }

    /// Corrects one touching point's overlap (position-based): the
    /// correction it took (mass × distance), 0 when they're apart.
    fn correct(&mut self, a: usize, other: Other, t: &Touch) -> f32 {
        let (pa, pb, arm_a, arm_b) = self.ends(a, other, t);
        let depth = t.margin - dot(sub(pa, pb), t.n);
        if depth <= 0.0 {
            return 0.0;
        }
        // A deep overlap (a body placed sunk into what holds it) comes out
        // a little at a time.
        let depth = depth.min(MAX_CORRECTION);
        let wb = match other {
            Other::Body(j) if !self.bodies[j].asleep => self.bodies[j].give(arm_b, t.n),
            _ => 0.0,
        };
        let wa = self.bodies[a].give(arm_a, t.n);
        if wa + wb <= 0.0 {
            return 0.0;
        }
        let lambda = depth / (wa + wb);
        self.bodies[a].shift(scale(t.n, lambda), arm_a);
        if let Other::Body(j) = other {
            if !self.bodies[j].asleep {
                self.bodies[j].shift(scale(t.n, -lambda), arm_b);
            }
        }
        lambda
    }

    /// A corrected touch, for the velocity pass.
    fn contact(
        &self,
        a: usize,
        other: Other,
        t: &Touch,
        lambda: f32,
        before: &[(Vec3, Quat, Vec3, Vec3)],
    ) -> Contact {
        let (_, _, arm_a, arm_b) = self.ends(a, other, t);
        // How fast they approached each other before the substep.
        let (_, qa, va, wa_) = before[a];
        let ra = mat_vec(&quat_mat(qa), mat_t_vec(&quat_mat(self.bodies[a].q), arm_a));
        let mut rel = add(va, cross(wa_, ra));
        match other {
            Other::Body(j) => {
                let (_, _, vb, wb_) = before[j];
                rel = sub(rel, add(vb, cross(wb_, arm_b)));
            }
            Other::Mover(v) => rel = sub(rel, v),
            Other::World(_) => {}
        }
        let me = &self.bodies[a].setup;
        Contact {
            a,
            other,
            arm_a,
            arm_b,
            n: t.n,
            lambda,
            friction: combined_friction(me.friction, t.surface.friction),
            restitution: combined_restitution(me.restitution, t.surface.restitution),
            approach: dot(rel, t.n),
        }
    }

    /// The velocity of a contact's first side against the other at the
    /// touching point, and the other body when it moves.
    fn relative(&self, c: &Contact) -> (Vec3, Option<usize>) {
        let mut rel = self.bodies[c.a].point_velocity(c.arm_a);
        let mut moving = None;
        match c.other {
            Other::Body(j) => {
                rel = sub(rel, self.bodies[j].point_velocity(c.arm_b));
                if !self.bodies[j].asleep {
                    moving = Some(j);
                }
            }
            Other::Mover(v) => rel = sub(rel, v),
            Other::World(_) => {}
        }
        (rel, moving)
    }

    /// Gives a contact an impulse `p` (game units) on its first side, and
    /// the opposite on a moving other body.
    fn kick_contact(&mut self, c: &Contact, p: Vec3, other: Option<usize>) {
        self.bodies[c.a].kick(p, c.arm_a);
        if let Some(j) = other {
            self.bodies[j].kick(scale(p, -1.0), c.arm_b);
        }
    }

    /// How readily a contact's sides give along `dir`, together.
    fn contact_give(&self, c: &Contact, dir: Vec3, other: Option<usize>) -> f32 {
        self.bodies[c.a].give(c.arm_a, dir)
            + other.map_or(0.0, |j| self.bodies[j].give(c.arm_b, dir))
    }

    /// One pass of a contact's normal velocity: set to a bounce off a real
    /// approach (faster than gravity gives in two substeps) by the
    /// restitution, else to zero, which also takes back speed the overlap
    /// correction gave (as Müller et al.'s restitution step does); `pushed`
    /// sums the impulse given so far.
    fn normal_pass(&mut self, c: &Contact, h: f32, pushed: &mut f32) {
        let (rel, other) = self.relative(c);
        let vn = dot(rel, c.n);
        let target = if c.approach < -2.0 * GRAVITY * h {
            -c.restitution * c.approach
        } else {
            0.0
        };
        let w = self.contact_give(c, c.n, other);
        if w <= 0.0 {
            return;
        }
        let p = (target - vn) / w;
        *pushed += p;
        if p != 0.0 {
            self.kick_contact(c, scale(c.n, p), other);
        }
    }

    /// A contact's friction: sliding slowed by friction × the push that
    /// kept the sides apart, stopped outright when that's enough.
    fn friction_pass(&mut self, c: &Contact, h: f32) {
        let (rel, other) = self.relative(c);
        let vn = dot(rel, c.n);
        let vt = sub(rel, scale(c.n, vn));
        let slide = length(vt);
        if slide < 1e-6 {
            return;
        }
        let dir = scale(vt, -1.0 / slide);
        let stop = (c.friction * c.lambda / h).min(slide);
        let w = self.contact_give(c, dir, other);
        if w > 0.0 {
            self.kick_contact(c, scale(dir, stop / w), other);
        }
    }

    /// Where body `i` touches the collider's triangles `tris`.
    fn world_touches(&self, collider: &Collider, i: usize, tris: &[u32]) -> Vec<(u32, Touch)> {
        let b = &self.bodies[i];
        let mut out = Vec::new();
        let mut tags = Vec::new();
        for &t in tris {
            let start = out.len();
            self.triangle_touches(collider, b, t, &mut out);
            tags.extend(std::iter::repeat(t).take(out.len() - start));
        }
        tags.into_iter().zip(out).collect()
    }

    /// Where body `b` touches the collider's triangle `t`.
    fn triangle_touches(&self, collider: &Collider, b: &Rigid, t: u32, out: &mut Vec<Touch>) {
        {
            let [ta, tb, tc] = collider.triangle(t);
            let shell = collider.shell(t);
            let surface = collider.surface(t).unwrap_or(DEFAULT_SURFACE);
            let mut face = normalize(cross(sub(tb, ta), sub(tc, ta)));
            if dot(face, sub(b.x, ta)) < 0.0 {
                face = scale(face, -1.0);
            }
            for shape in &b.setup.shapes {
                let mut point = |p_model: Vec3, radius: f32| {
                    let p = b.to_world(p_model);
                    if let Some((on_b, n, margin)) =
                        point_triangle(p, radius + shell, [ta, tb, tc], face, b.reach)
                    {
                        out.push(Touch {
                            on_a: p_model,
                            on_b,
                            n,
                            margin,
                            surface,
                        });
                    }
                };
                match shape {
                    Shape::Hull {
                        vertices, shell: s, ..
                    }
                    | Shape::Mesh {
                        vertices, shell: s, ..
                    } => {
                        for &v in vertices {
                            point(v, *s);
                        }
                    }
                    Shape::Sphere { center, radius } => point(*center, *radius),
                    Shape::Capsule { a, b: e, radius } => {
                        point(*a, *radius);
                        point(*e, *radius);
                        let (pa, pe) = (b.to_world(*a), b.to_world(*e));
                        let (on_axis, on_tri) = segment_triangle_closest(pa, pe, ta, tb, tc);
                        let mid_model = b.to_model(on_axis);
                        if dist2(on_axis, pa) > 1e-4 && dist2(on_axis, pe) > 1e-4 {
                            let d = length(sub(on_axis, on_tri));
                            let n = if d > 1e-5 {
                                scale(sub(on_axis, on_tri), 1.0 / d)
                            } else {
                                face
                            };
                            if d < radius + shell {
                                out.push(Touch {
                                    on_a: mid_model,
                                    on_b: on_tri,
                                    n,
                                    margin: radius + shell,
                                    surface,
                                });
                            }
                        }
                    }
                }
                // The triangle's edges reaching into a hull (a rail's long
                // edge under a bottle's base, a ridge of the ground): where
                // each enters and leaves the hull grown by the shells, the
                // hull is pushed off along its own face that faces the
                // triangle the most squarely, under that point.
                if let Shape::Hull {
                    planes, shell: s, ..
                } = shape
                {
                    let margin = s + shell;
                    let r = quat_mat(b.q);
                    for (e0, e1) in [(ta, tb), (tb, tc), (tc, ta)] {
                        let (l0, l1) = (b.to_model(e0), b.to_model(e1));
                        let Some((t0, t1)) = clip_segment(l0, l1, planes, margin) else {
                            continue;
                        };
                        for t in [t0, t1] {
                            let local = add(l0, scale(sub(l1, l0), t));
                            // Only a face the body is in front of (a rail's
                            // top under a bottle, not its side).
                            let to_body = normalize(sub(b.x, b.to_world(local)));
                            if dot(face, to_body) < 0.5 {
                                continue;
                            }
                            if let Some(touch) = edge_point_touch(
                                &r,
                                local,
                                b.to_world(local),
                                planes,
                                face,
                                margin,
                                surface,
                            ) {
                                out.push(touch);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Where body `i` touches body `j` (`j`'s side as model-space points
    /// of `j`).
    fn body_touches(&self, i: usize, j: usize) -> Vec<Touch> {
        let (a, b) = (&self.bodies[i], &self.bodies[j]);
        let surface = Surface {
            friction: b.setup.friction,
            restitution: b.setup.restitution,
        };
        let mut out = Vec::new();
        for sa in &a.setup.shapes {
            for sb in &b.setup.shapes {
                // a's corners and centres against b's hulls, b's against
                // a's (turned around).
                for (p, r) in feature_points(sa) {
                    let world = a.to_world(p);
                    if let Some(t) = point_in_shape(b, world, sb, r, surface) {
                        out.push(Touch {
                            on_a: p,
                            on_b: b.to_model(t.on_b),
                            ..t
                        });
                    }
                }
                for (p, r) in feature_points(sb) {
                    let world = b.to_world(p);
                    if let Some(t) = point_in_shape(a, world, sa, r, surface) {
                        // t: on_b is the point of b (world), n pushes b away
                        // from a; turn it around for a.
                        out.push(Touch {
                            on_a: a.to_model(t.on_b_surface()),
                            on_b: p,
                            n: scale(t.n, -1.0),
                            margin: t.margin,
                            surface,
                        });
                    }
                }
                // Round shapes against each other.
                if let (Some((a0, a1, ra)), Some((b0, b1, rb))) = (round(sa), round(sb)) {
                    let (wa0, wa1) = (a.to_world(a0), a.to_world(a1));
                    let (wb0, wb1) = (b.to_world(b0), b.to_world(b1));
                    let (pa, pb) = segment_segment_closest(wa0, wa1, wb0, wb1);
                    let d = length(sub(pa, pb));
                    if d < ra + rb && d > 1e-5 {
                        out.push(Touch {
                            on_a: a.to_model(pa),
                            on_b: b.to_model(pb),
                            n: scale(sub(pa, pb), 1.0 / d),
                            margin: ra + rb,
                            surface,
                        });
                    }
                }
            }
        }
        out
    }

    /// Where body `i` touches a walker (the walker's side as world points).
    fn mover_touches(&self, i: usize, m: &Mover) -> Vec<Touch> {
        let b = &self.bodies[i];
        let (s0, s1) = m.segment();
        let radius = m.radius + PUSH_SKIN;
        let surface = DEFAULT_SURFACE;
        // Quick reject.
        let c = closest_on_segment(b.x, s0, s1);
        if length(sub(b.x, c)) > b.reach + radius {
            return Vec::new();
        }
        let mut out = Vec::new();
        for shape in &b.setup.shapes {
            for (p, r) in feature_points(shape) {
                let world = b.to_world(p);
                let on = closest_on_segment(world, s0, s1);
                let d = length(sub(world, on));
                if d < r + radius && d > 1e-5 {
                    out.push(Touch {
                        on_a: p,
                        on_b: on,
                        n: scale(sub(world, on), 1.0 / d),
                        margin: r + radius,
                        surface,
                    });
                }
            }
            if let Shape::Hull { planes, shell, .. } = shape {
                for k in 0..=4 {
                    let p = add(s0, scale(sub(s1, s0), k as f32 / 4.0));
                    if let Some(t) = corner_in_hull(b, p, planes, shell + radius, surface) {
                        out.push(t);
                    }
                }
            }
        }
        out
    }
}

impl Touch {
    /// For a touch found by [`point_in_shape`]: the point on the shape's
    /// surface (world) where the outside point is pushed out to.
    fn on_b_surface(&self) -> Vec3 {
        // `on_a` holds it there (see `point_in_shape`).
        self.on_a
    }
}

/// A contact's friction: the square root of the two bodies' frictions
/// multiplied (the contact manager's new point, `00cfd800`, from each
/// entity's material at `+0x90`).
// Translated from 00cfd800 (decompiled, FalloutNV.exe 1.4.0.525)
pub fn combined_friction(a: f32, b: f32) -> f32 {
    (a * b).max(0.0).sqrt()
}

/// A contact's restitution: the square root of the two restitutions
/// multiplied (`+0x94`), kept in a byte as × 128 rounded (`00cfd800`).
// Translated from 00cfd800 (decompiled, FalloutNV.exe 1.4.0.525)
pub fn combined_restitution(a: f32, b: f32) -> f32 {
    let r = (a * b).max(0.0).sqrt();
    // FISTP to an integer, its low byte kept.
    f32::from((r * 128.0).round() as i32 as u8) / 128.0
}

/// The points of a shape that touch others here, with their radius:
/// corners (with the shell), sphere and capsule centres.
fn feature_points(shape: &Shape) -> Vec<(Vec3, f32)> {
    match shape {
        Shape::Hull {
            vertices, shell, ..
        }
        | Shape::Mesh {
            vertices, shell, ..
        } => vertices.iter().map(|&v| (v, *shell)).collect(),
        Shape::Sphere { center, radius } => vec![(*center, *radius)],
        Shape::Capsule { a, b, radius } => vec![(*a, *radius), (*b, *radius)],
    }
}

/// A sphere or capsule as a segment and a radius.
fn round(shape: &Shape) -> Option<(Vec3, Vec3, f32)> {
    match shape {
        Shape::Sphere { center, radius } => Some((*center, *center, *radius)),
        Shape::Capsule { a, b, radius } => Some((*a, *b, *radius)),
        _ => None,
    }
}

fn closest_on_segment(p: Vec3, a: Vec3, b: Vec3) -> Vec3 {
    let ab = sub(b, a);
    let l = dot(ab, ab);
    if l < 1e-12 {
        return a;
    }
    add(a, scale(ab, (dot(sub(p, a), ab) / l).clamp(0.0, 1.0)))
}

/// A point `p` of a body (with `margin`: its radius and the triangle's
/// shell) against a triangle whose face normal `face` points toward the
/// body: the point on the triangle side (world), the normal pushing the
/// body away and the margin, when they overlap. A point that has gone
/// through the face (up to `deep` behind it) still counts while it's over
/// the triangle.
fn point_triangle(
    p: Vec3,
    margin: f32,
    [a, b, c]: [Vec3; 3],
    face: Vec3,
    deep: f32,
) -> Option<(Vec3, Vec3, f32)> {
    let s = dot(face, sub(p, a));
    let on_plane = sub(p, scale(face, s));
    let inside = {
        let n = face;
        dot(cross(sub(b, a), sub(on_plane, a)), n) * dot(cross(sub(b, a), sub(c, a)), n) >= 0.0
            && dot(cross(sub(c, b), sub(on_plane, b)), n) * dot(cross(sub(c, b), sub(a, b)), n)
                >= 0.0
            && dot(cross(sub(a, c), sub(on_plane, c)), n) * dot(cross(sub(a, c), sub(b, c)), n)
                >= 0.0
    };
    if inside {
        return (s < margin && s > -deep).then_some((on_plane, face, margin));
    }
    let q = closest_on_triangle(p, a, b, c);
    let d = length(sub(p, q));
    (d < margin && d > 1e-5).then(|| (q, scale(sub(p, q), 1.0 / d), margin))
}

/// The part of segment `a`–`b` (model space) inside a hull's planes grown
/// by `margin`, as the range of the segment's parameter (Cyrus–Beck).
fn clip_segment(a: Vec3, b: Vec3, planes: &[[f32; 4]], margin: f32) -> Option<(f32, f32)> {
    if planes.is_empty() {
        return None;
    }
    let d = sub(b, a);
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for p in planes {
        let n = [p[0], p[1], p[2]];
        let start = dot(n, a) + p[3] - margin;
        let along = dot(n, d);
        if along.abs() < 1e-9 {
            if start > 0.0 {
                return None;
            }
            continue;
        }
        let t = -start / along;
        if along > 0.0 {
            t1 = t1.min(t);
        } else {
            t0 = t0.max(t);
        }
        if t0 > t1 {
            return None;
        }
    }
    Some((t0, t1))
}

/// A point of the world (`local` in the body's model space, `world` in the
/// world) within a hull's `margin`: the touch pushing the hull off it along
/// the hull's own face that faces the world's triangle (normal `face`,
/// pointing toward the body) the most squarely among those it's least
/// deep behind; none when no face of the hull faces the triangle.
fn edge_point_touch(
    r: &Mat3,
    local: Vec3,
    world: Vec3,
    planes: &[[f32; 4]],
    face: Vec3,
    margin: f32,
    surface: Surface,
) -> Option<Touch> {
    let mut best: Option<(f32, Vec3, Vec3)> = None;
    for p in planes {
        let n_local = [p[0], p[1], p[2]];
        let n = mat_vec(r, n_local);
        if dot(n, face) > -0.5 {
            continue;
        }
        let sd = dot(n_local, local) + p[3];
        if best.map_or(true, |(b, _, _)| sd > b) {
            best = Some((sd, n_local, n));
        }
    }
    let (sd, n_local, n) = best?;
    (sd < margin).then(|| Touch {
        on_a: sub(local, scale(n_local, sd)),
        on_b: world,
        n: scale(n, -1.0),
        margin,
        surface,
    })
}

/// A world point `p` inside body `b`'s hull (its planes, in the model's
/// space) grown by `margin`: pushes the body away from it, along the face
/// it's least deep behind.
fn corner_in_hull(
    b: &Rigid,
    p: Vec3,
    planes: &[[f32; 4]],
    margin: f32,
    surface: Surface,
) -> Option<Touch> {
    if planes.is_empty() {
        return None;
    }
    let local = b.to_model(p);
    let mut best = (f32::NEG_INFINITY, [0.0; 3]);
    for pl in planes {
        let n = [pl[0], pl[1], pl[2]];
        let sd = dot(n, local) + pl[3];
        if sd > best.0 {
            best = (sd, n);
        }
    }
    let (sd, n_local) = best;
    if sd >= margin {
        return None;
    }
    let r = quat_mat(b.q);
    let n = mat_vec(&r, n_local);
    // The hull's surface point under `p`, as a model-space point of the
    // body.
    let on_hull = sub(local, scale(n_local, sd));
    Some(Touch {
        on_a: on_hull,
        on_b: p,
        n: scale(n, -1.0),
        margin,
        surface,
    })
}

/// A world point `p` with radius `r` against shape `shape` of body `b`:
/// the touch pushing `p`'s side away from `b` (`n`), with `on_a` holding
/// the shape's surface point (world) and `on_b` the shape's surface point
/// too (filled by the caller).
fn point_in_shape(b: &Rigid, p: Vec3, shape: &Shape, r: f32, surface: Surface) -> Option<Touch> {
    match shape {
        Shape::Hull { planes, shell, .. } => {
            let t = corner_in_hull(b, p, planes, shell + r, surface)?;
            // corner_in_hull pushes b away from p; for p's side, the other
            // way, and the surface point in the world.
            let on_surface = b.to_world(t.on_a);
            Some(Touch {
                on_a: on_surface,
                on_b: on_surface,
                n: scale(t.n, -1.0),
                margin: t.margin,
                surface,
            })
        }
        Shape::Sphere { .. } | Shape::Capsule { .. } => {
            let (s0, s1, radius) = round(shape)?;
            let (w0, w1) = (b.to_world(s0), b.to_world(s1));
            let on = closest_on_segment(p, w0, w1);
            let d = length(sub(p, on));
            if d >= radius + r || d < 1e-5 {
                return None;
            }
            Some(Touch {
                on_a: on,
                on_b: on,
                n: scale(sub(p, on), 1.0 / d),
                margin: radius + r,
                surface,
            })
        }
        Shape::Mesh { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const I3: Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    fn floor(at: f32) -> Collider {
        let mut c = Collider::new();
        c.add(
            &[
                [-1000.0, -1000.0, at],
                [1000.0, -1000.0, at],
                [1000.0, 1000.0, at],
                [-1000.0, 1000.0, at],
            ],
            &[[0, 1, 2], [0, 2, 3]],
        );
        c
    }

    /// A box of half sizes `h` centred on its origin, of `mass`.
    fn crate_(reference: u32, h: Vec3, mass: f32) -> RigidSetup {
        let corners: Vec<Vec3> = (0..8)
            .map(|k| {
                [
                    if k & 1 == 0 { -h[0] } else { h[0] },
                    if k & 2 == 0 { -h[1] } else { h[1] },
                    if k & 4 == 0 { -h[2] } else { h[2] },
                ]
            })
            .collect();
        let planes = [
            [1.0, 0.0, 0.0, -h[0]],
            [-1.0, 0.0, 0.0, -h[0]],
            [0.0, 1.0, 0.0, -h[1]],
            [0.0, -1.0, 0.0, -h[1]],
            [0.0, 0.0, 1.0, -h[2]],
            [0.0, 0.0, -1.0, -h[2]],
        ];
        let k = mass / 3.0;
        RigidSetup {
            reference,
            layer: 4,
            mass,
            center: [0.0; 3],
            inertia: [
                [k * (h[1] * h[1] + h[2] * h[2]), 0.0, 0.0],
                [0.0, k * (h[0] * h[0] + h[2] * h[2]), 0.0],
                [0.0, 0.0, k * (h[0] * h[0] + h[1] * h[1])],
            ],
            linear_damping: 0.1,
            angular_damping: 0.05,
            friction: 0.5,
            restitution: 0.4,
            max_linear_speed: 1068.0 * HAVOK_UNIT,
            max_angular_speed: 31.57,
            motion: 4,
            wind: false,
            shapes: vec![Shape::hull(corners, &planes, 0.7)],
        }
    }

    #[test]
    fn the_clock_steps_as_the_game_does() {
        let mut c = Clock::new(0.016);
        // A 60 Hz frame: one step, 0.000667 s over.
        assert_eq!(c.advance(1.0 / 60.0), (1, 0.016));
        // A long frame: three steps at most, the rest held to one step.
        let (n, dt) = c.advance(0.2);
        assert_eq!((n, dt), (3, 0.016));
        assert!((c.left() - 0.016).abs() < 1e-6, "{}", c.left());
        // Less than half a step waits; half or more takes one of it all.
        let mut c = Clock::new(0.016);
        assert_eq!(c.advance(0.005), (0, 0.0));
        assert_eq!(c.advance(0.002), (0, 0.0));
        let (n, dt) = c.advance(0.0015);
        assert_eq!(n, 1);
        assert!((dt - 0.016).abs() < 1e-6, "{dt}");
        // Rounding up runs ahead: the rest is negative.
        let mut c = Clock::new(0.016);
        assert_eq!(c.advance(0.013).0, 1);
        assert!(c.left() < 0.0);
    }

    #[test]
    fn a_body_in_the_open_is_stepped_by_havoks_integrator() {
        // Far above the floor: nothing near it, so each step is exactly
        // `hkRigidMotionUtilApplyForcesAndStep` in game units.
        let c = floor(0.0);
        let mut w = RigidWorld::new();
        let i = w.add(crate_(1, [10.0; 3], 5.0), (I3, [0.0, 0.0, 5000.0]));
        w.wake(i);
        w.bodies[i].w = [0.0, 0.0, 3.0];
        let mut m = w.bodies[i].motion_now();
        let g = scale(crate::havok::GRAVITY, STEP * HAVOK_UNIT);
        for _ in 0..40 {
            assert_eq!(w.update(&c, STEP), 1);
            m.step(STEP, g, HAVOK_UNIT);
            assert!(w.bodies[i].free);
            assert_eq!(w.bodies[i].center(), m.center);
            assert_eq!(w.bodies[i].velocity(), m.linear_velocity);
            assert_eq!(w.bodies[i].spin(), m.angular_velocity);
        }
    }

    #[test]
    fn a_box_falls_and_rests_on_the_floor() {
        let mut w = RigidWorld::new();
        let i = w.add(crate_(1, [10.0, 10.0, 10.0], 5.0), (I3, [0.0, 0.0, 100.0]));
        w.wake(i);
        let c = floor(0.0);
        for _ in 0..240 {
            w.update(&c, 1.0 / 60.0);
        }
        let (_, t) = w.bodies[i].pose();
        // Half its size plus its shell above the floor.
        assert!((t[2] - 10.7).abs() < 0.5, "{t:?}");
        assert!(t[0].abs() < 1.0 && t[1].abs() < 1.0, "{t:?}");
        assert!(w.bodies[i].asleep && w.bodies[i].moved);
    }

    #[test]
    fn a_shot_knocks_a_bottle_off_a_rail() {
        // A rail 4 units wide at z 100; a tall thin body standing on it.
        let mut c = Collider::new();
        c.add(
            &[
                [-200.0, -2.0, 100.0],
                [200.0, -2.0, 100.0],
                [200.0, 2.0, 100.0],
                [-200.0, 2.0, 100.0],
            ],
            &[[0, 1, 2], [0, 2, 3]],
        );
        c.extend(&floor(0.0));
        let mut w = RigidWorld::new();
        let i = w.add(crate_(2, [1.5, 1.5, 12.0], 1.0), (I3, [0.0, 0.0, 112.7]));
        // Asleep where it was put, until pushed.
        for _ in 0..30 {
            w.update(&c, 1.0 / 60.0);
        }
        assert!(w.bodies[i].asleep);
        assert_eq!(w.bodies[i].pose().1, [0.0, 0.0, 112.7]);
        // A shot along +y high up.
        let j = crate::impulses::projectile_impulse(
            10.0,
            [0.0, 1.0, 0.0],
            10,
            1.0,
            &Default::default(),
        )
        .unwrap();
        w.apply_point_impulse(i, j, [0.0, 0.0, 120.0]);
        assert!(!w.bodies[i].asleep && w.bodies[i].moved);
        for _ in 0..300 {
            w.update(&c, 1.0 / 60.0);
        }
        let (_, t) = w.bodies[i].pose();
        // Off the rail, on the ground beyond it.
        assert!(t[2] < 20.0, "{t:?}");
        assert!(t[1] > 2.0, "{t:?}");
    }

    #[test]
    fn bodies_stack_and_a_walker_pushes_them() {
        let c = floor(0.0);
        let mut w = RigidWorld::new();
        let lower = w.add(crate_(3, [10.0; 3], 10.0), (I3, [0.0, 0.0, 10.7]));
        let upper = w.add(crate_(4, [10.0; 3], 10.0), (I3, [0.0, 0.0, 40.0]));
        w.wake(upper);
        for _ in 0..240 {
            w.update(&c, 1.0 / 60.0);
        }
        let top = w.bodies[upper].pose().1;
        assert!((top[2] - 32.1).abs() < 1.5, "{top:?}");
        // A walker coming from the west at 150 units a second pushes the
        // lower box east.
        let start = w.bodies[lower].pose().1;
        for k in 0..60 {
            let x = -40.0 + 150.0 * k as f32 / 60.0;
            w.set_movers(vec![Mover {
                feet: [x, 0.0, 0.0],
                radius: 20.0,
                height: 128.0,
                velocity: [150.0, 0.0, 0.0],
            }]);
            w.update(&c, 1.0 / 60.0);
        }
        let now = w.bodies[lower].pose().1;
        assert!(now[0] > start[0] + 20.0, "{start:?} → {now:?}");
    }

    /// A six-sided bottle as `clutter\junk\ssbottle02.nif`'s body has it
    /// (rounded): 4.5 across its corners, 17.8 tall to the shoulder, its
    /// neck to 29.1 narrowing to 2; mass 1, centre 12.3 up, inertia 59.1,
    /// 59.1, 7.7; damping, friction, restitution and limits as stored.
    fn bottle(reference: u32) -> RigidSetup {
        let ring = |r: f32, z: f32| -> Vec<Vec3> {
            (0..6)
                .map(|k| {
                    let a = k as f32 * std::f32::consts::FRAC_PI_3;
                    [r * a.cos(), r * a.sin(), z]
                })
                .collect()
        };
        let mut vertices = ring(4.5, 0.0);
        vertices.extend(ring(4.5, 17.8));
        vertices.extend(ring(2.0, 29.1));
        // Faces: bottom, top, the six sides of the body, the six of the
        // shoulder; planes through their corners, pointing out.
        let mut planes = vec![[0.0, 0.0, -1.0, 0.0], [0.0, 0.0, 1.0, -29.1]];
        for k in 0..6 {
            for (lo, hi) in [(0, 6), (6, 12)] {
                let (a, b, c) = (
                    vertices[lo + k],
                    vertices[lo + (k + 1) % 6],
                    vertices[hi + k],
                );
                let mut n = normalize(cross(sub(b, a), sub(c, a)));
                if dot(n, [a[0], a[1], 0.0]) < 0.0 {
                    n = scale(n, -1.0);
                }
                planes.push([n[0], n[1], n[2], -dot(n, a)]);
            }
        }
        RigidSetup {
            reference,
            layer: 10,
            mass: 1.0,
            center: [0.0, 0.0, 12.31],
            inertia: [[59.13, 0.0, 0.0], [0.0, 59.13, 0.0], [0.0, 0.0, 7.69]],
            linear_damping: 0.1,
            angular_damping: 0.05,
            friction: 0.5,
            restitution: 0.4,
            max_linear_speed: 1068.0 * HAVOK_UNIT,
            max_angular_speed: 31.57,
            motion: 4,
            wind: false,
            shapes: vec![Shape::hull(vertices, &planes, 0.7)],
        }
    }

    /// A rail 3 wide with its top at z 100 (two boxes meeting at x 0.5),
    /// over the ground at z 20 (the terrain's shell).
    fn fence() -> Collider {
        let mut c = Collider::new();
        for (x0, x1) in [(-200.0f32, 0.5f32), (0.5, 200.0)] {
            let v = [
                [x0, -1.5, 100.0],
                [x1, -1.5, 100.0],
                [x1, 1.5, 100.0],
                [x0, 1.5, 100.0],
                [x0, -1.5, 96.0],
                [x1, -1.5, 96.0],
                [x1, 1.5, 96.0],
                [x0, 1.5, 96.0],
            ];
            let t = [
                [0, 1, 2],
                [0, 2, 3],
                [4, 6, 5],
                [4, 7, 6],
                [0, 4, 5],
                [0, 5, 1],
                [3, 2, 6],
                [3, 6, 7],
            ];
            c.add_solid(&v, &t, 0.7, 0);
        }
        c.add_solid(
            &[
                [-1000.0, -1000.0, 20.0],
                [1000.0, -1000.0, 20.0],
                [1000.0, 1000.0, 20.0],
                [-1000.0, 1000.0, 20.0],
            ],
            &[[0, 1, 2], [0, 2, 3]],
            crate::TERRAIN_SHELL,
            0,
        );
        c
    }

    #[test]
    fn a_bottle_stands_on_a_narrow_rail_until_a_shot_tips_it_off() {
        bottle_shot_off_a_rail([0.0; 3]);
    }

    #[test]
    fn far_from_the_origin_a_shot_bottle_falls_the_same_way() {
        // Goodsprings' fence is at (-68233, 5050, 8438): there a
        // position's rounding is worth several units a second of speed
        // when velocities come from positions.
        bottle_shot_off_a_rail([-68233.0, 5050.0, 8338.0]);
    }

    /// Everything moved by `at`.
    fn shifted(c: &Collider, at: Vec3) -> Collider {
        let mut out = Collider::new();
        for k in 0..c.triangle_count() as u32 {
            let tri = c.triangle(k).map(|p| add(p, at));
            out.add_solid(&tri, &[[0, 1, 2]], c.shell(k), 0);
        }
        out
    }

    fn bottle_shot_off_a_rail(at: Vec3) {
        let c = shifted(&fence(), at);
        let mut w = RigidWorld::new();
        let i = w.add(bottle(9), (I3, add(at, [0.0, 0.0, 101.4])));
        // Woken, it stays standing on the rail (its base wider than the
        // rail: held by the rail's long edges) and falls asleep.
        w.apply_linear_impulse(i, [0.0, 0.0, 1e-4]);
        for _ in 0..120 {
            w.update(&c, 1.0 / 60.0);
        }
        let t = sub(w.bodies[i].pose().1, at);
        assert!(length(sub(t, [0.0, 0.0, 101.4])) < 0.1, "{t:?}");
        assert!(w.bodies[i].asleep);
        // A varmint rifle round (impact force 3: 9 Havok units on this
        // prop) high on it, heading north: it tips over and falls off
        // behind the rail, coming to rest on the ground.
        let j = crate::impulses::projectile_impulse(
            3.0,
            [0.0, 1.0, -0.14],
            10,
            1.0,
            &Default::default(),
        )
        .unwrap();
        w.apply_point_impulse(i, j, add(at, [0.0, 0.0, 120.0]));
        for _ in 0..400 {
            w.update(&c, 1.0 / 60.0);
        }
        let t = sub(w.bodies[i].pose().1, at);
        // Off northward, more than sideways (lying on its side it rolls a
        // little either way).
        assert!(t[1] > 10.0 && t[2] < 30.0 && t[0].abs() < t[1], "{t:?}");
        assert!(w.bodies[i].asleep && w.bodies[i].moved);
        // Lying on its side on the ground: its axis level.
        let (r, _) = w.bodies[i].pose();
        assert!(r[2][2].abs() < 0.2, "{r:?}");
    }

    #[test]
    fn a_rails_end_under_a_body_doesnt_push_it_sideways() {
        // A rail ending under a standing body (its end corners within the
        // body's footprint, inside its shell): the body stays put.
        let mut c = Collider::new();
        c.add(
            &[
                [-200.0, -2.0, 100.0],
                [1.0, -2.0, 100.0],
                [1.0, 2.0, 100.0],
                [-200.0, 2.0, 100.0],
            ],
            &[[0, 1, 2], [0, 2, 3]],
        );
        let mut w = RigidWorld::new();
        let i = w.add(crate_(8, [2.0, 2.0, 12.0], 1.0), (I3, [0.0, 0.0, 112.7]));
        w.apply_linear_impulse(i, [0.0, 0.0, 0.001]);
        for _ in 0..120 {
            w.update(&c, 1.0 / 60.0);
        }
        let t = w.bodies[i].pose().1;
        assert!(t[0].abs() < 0.5 && t[1].abs() < 0.5, "{t:?}");
        assert!((t[2] - 112.7).abs() < 0.5, "{t:?}");
    }

    #[test]
    fn surfaces_keep_their_bodies_values_through_extend() {
        let ice = Surface {
            friction: 0.1,
            restitution: 0.2,
        };
        let mut a = Collider::new();
        a.add_solid_surface(
            &[[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
            (0.7, 0, 3),
            Some(ice),
        );
        let mut all = floor(0.0);
        all.extend(&a);
        let last = all.triangle_count() as u32 - 1;
        assert_eq!(all.surface(last), Some(ice));
        assert_eq!(all.surface(0), None);
        assert_eq!(all.material(last), Some(3));
    }

    #[test]
    fn deltas_take_the_rest_pose_to_the_pose_now() {
        let mut w = RigidWorld::new();
        let i = w.add(crate_(5, [5.0; 3], 2.0), (I3, [10.0, 20.0, 30.0]));
        let turned = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        w.place(i, (turned, [0.0, 0.0, 5.0]), true);
        let (r, t) = w.delta(i);
        // The rest pose's origin goes to the new one.
        let moved = add(mat_vec(&r, [10.0, 20.0, 30.0]), t);
        assert!(length(sub(moved, [0.0, 0.0, 5.0])) < 1e-4, "{moved:?}");
        assert!(w.bodies[i].moved && w.bodies[i].asleep);
    }

    #[test]
    fn impulses_change_velocity_by_mass_in_havok_units() {
        let mut w = RigidWorld::new();
        let i = w.add(crate_(6, [5.0; 3], 2.0), (I3, [0.0; 3]));
        w.apply_linear_impulse(i, [4.0, 0.0, 0.0]);
        // 4 / 2 Havok units a second, in game units.
        assert!((w.bodies[i].velocity()[0] - 2.0 * HAVOK_UNIT).abs() < 1e-4);
        w.apply_angular_impulse(i, [0.0, 0.0, 1.0]);
        assert!(w.bodies[i].spin()[2] > 0.0);
        // Fixed bodies don't move.
        let mut fixed = crate_(7, [5.0; 3], 2.0);
        fixed.motion = 7;
        let j = w.add(fixed, (I3, [50.0, 0.0, 0.0]));
        w.apply_linear_impulse(j, [4.0, 0.0, 0.0]);
        assert_eq!(w.bodies[j].velocity(), [0.0; 3]);
        assert!(w.bodies[j].asleep);
    }

    #[test]
    fn walkers_push_only_bodies_lighter_than_the_move_limit() {
        let c = floor(0.0);
        let mut w = RigidWorld::new();
        let light = w.add(crate_(11, [10.0; 3], 20.0), (I3, [0.0, 0.0, 10.7]));
        let heavy = w.add(crate_(12, [10.0; 3], 100.0), (I3, [0.0, 200.0, 10.7]));
        for k in 0..60 {
            let x = -40.0 + 150.0 * k as f32 / 60.0;
            w.set_movers(vec![
                Mover {
                    feet: [x, 0.0, 0.0],
                    radius: 20.0,
                    height: 128.0,
                    velocity: [150.0, 0.0, 0.0],
                },
                Mover {
                    feet: [x, 200.0, 0.0],
                    radius: 20.0,
                    height: 128.0,
                    velocity: [150.0, 0.0, 0.0],
                },
            ]);
            w.update(&c, 1.0 / 60.0);
        }
        assert!(w.bodies[light].center()[0] > 20.0);
        assert!(w.bodies[heavy].center()[0].abs() < 0.5 && w.bodies[heavy].asleep);
    }

    /// As the viewer has it: the body's triangles in the collider (the
    /// walking character is kept off them) and the character as a mover.
    fn walk_into(h: Vec3, mass: f32) -> (Vec3, Vec3) {
        let mut c = floor(0.0);
        let setup = crate_(13, h, mass);
        let mut w = RigidWorld::new();
        let i = w.add(setup.clone(), (I3, [0.0, 0.0, h[2] + 0.7]));
        for shape in &setup.shapes {
            let (v, t, shell) = shape.triangles();
            let placed: Vec<Vec3> = v.iter().map(|p| add(*p, [0.0, 0.0, h[2] + 0.7])).collect();
            c.add_layered(&placed, &t, (shell, 13, crate::NO_MATERIAL), None, 4);
        }
        let shape = crate::CharacterShape::PLAYER;
        let mut walker = crate::Character::new([-100.0, 0.0, 0.0]);
        for _ in 0..90 {
            walker.update_controlled(&c, &shape, [280.0, 0.0], None, 1.0, 1.0 / 60.0);
            w.set_movers(vec![Mover {
                feet: walker.feet,
                radius: shape.radius,
                height: shape.height,
                velocity: [walker.horizontal[0], walker.horizontal[1], 0.0],
            }]);
            w.update(&c, 1.0 / 60.0);
            let (r, t) = w.delta(i);
            c.move_owner(13, &r, t);
        }
        (w.bodies[i].center(), walker.feet)
    }

    #[test]
    fn the_player_walking_into_clutter_pushes_it() {
        // A low, light box (a tumbleweed's size): pushed ahead.
        let (at, feet) = walk_into([15.0, 15.0, 15.0], 5.0);
        assert!(at[0] > 30.0, "{at:?} (walker at {feet:?})");
    }

    #[test]
    fn materials_combine_as_havok_does() {
        // A bottle (0.5, 0.4) on the ground (2.5, 0.4).
        assert!((combined_friction(0.5, 2.5) - 1.118_034).abs() < 1e-5);
        // sqrt(0.16) = 0.4 → 51.2 → 51 / 128.
        assert_eq!(combined_restitution(0.4, 0.4), 51.0 / 128.0);
        assert_eq!(combined_restitution(1.0, 1.0), 1.0);
    }

    #[test]
    fn a_falling_body_reports_one_contact_when_it_lands() {
        let c = floor(0.0);
        let mut w = RigidWorld::new();
        let i = w.add(crate_(10, [3.0; 3], 1.0), (I3, [0.0, 0.0, 60.0]));
        w.wake(i);
        let mut events = Vec::new();
        for _ in 0..120 {
            w.update(&c, 1.0 / 60.0);
            events.extend(w.take_contacts());
        }
        // Landing from about 56 units: sqrt(2 g h) ≈ 280 units a second.
        let first = events.first().copied().expect("a landing");
        assert!(first.speed > 200.0 && first.speed < 320.0, "{first:?}");
        assert_eq!((first.body, first.other_body), (i, None));
        assert!(first.triangle.is_some() && first.point[2] < 5.0);
        // Resting on the floor doesn't keep reporting.
        let late: Vec<_> = (0..60)
            .flat_map(|_| {
                w.update(&c, 1.0 / 60.0);
                w.take_contacts()
            })
            .collect();
        assert!(late.is_empty(), "{late:?}");
    }

    #[test]
    fn the_grab_spring_carries_a_body_to_its_target() {
        let c = floor(0.0);
        let mut w = RigidWorld::new();
        let i = w.add(crate_(8, [3.0; 3], 1.0), (I3, [0.0, 0.0, 3.7]));
        let s = crate::grab::GrabSettings::default();
        let (damping, elasticity, object_damping, max) = crate::grab::spring_values(10, &s);
        w.spring = Some(Spring {
            body: i,
            local: [0.0; 3],
            target: [20.0, 0.0, 60.0],
            damping,
            elasticity,
            max_relative_force: max,
            object_damping,
        });
        for _ in 0..180 {
            w.update(&c, 1.0 / 60.0);
        }
        // Held under the target, sagging by gravity × step² ÷ elasticity
        // (about 0.9 units).
        let at = w.bodies[i].center();
        assert!(length(sub(at, [20.0, 0.0, 60.0])) < 2.0, "{at:?}");
        assert!(!w.bodies[i].asleep && w.bodies[i].moved);
        // Let go: it falls back to the floor.
        w.spring = None;
        for _ in 0..240 {
            w.update(&c, 1.0 / 60.0);
        }
        assert!(w.bodies[i].center()[2] < 5.0, "{:?}", w.bodies[i].center());
        // Removing the held body ends the grab; others shift down.
        let j = w.add(crate_(9, [3.0; 3], 1.0), (I3, [50.0, 0.0, 3.7]));
        w.spring = Some(Spring {
            body: j,
            local: [0.0; 3],
            target: [0.0; 3],
            damping,
            elasticity,
            max_relative_force: max,
            object_damping,
        });
        w.remove(8);
        assert_eq!(w.spring.map(|s| s.body), Some(0));
        w.remove(9);
        assert!(w.spring.is_none());
    }
}
