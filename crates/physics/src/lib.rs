//! Collision and walking, in game units (z up).
//!
//! [`Collider`] holds a cell's solid surfaces as one triangle set, bucketed
//! on a grid for quick lookups; [`shapes`] turns the models' Havok shapes
//! (hulls, spheres, capsules) into triangles for it. [`Character`] is a
//! walking capsule that moves through a collider: it falls, lands, slides
//! along walls, climbs low steps and follows the floor down stairs.
//!
//! ```
//! use physics::{Character, CharacterShape, Collider};
//!
//! // A floor: two triangles at z = 0.
//! let mut floor = Collider::new();
//! floor.add(&[[-500.0, -500.0, 0.0], [500.0, -500.0, 0.0], [500.0, 500.0, 0.0], [-500.0, 500.0, 0.0]],
//!           &[[0, 1, 2], [0, 2, 3]]);
//! let shape = CharacterShape::PLAYER;
//! let mut player = Character::new([0.0, 0.0, 50.0]);
//! for _ in 0..120 {
//!     player.update(&floor, &shape, [0.0, 0.0], false, 1.0 / 60.0);
//! }
//! assert!(player.on_ground && player.feet[2].abs() < 1.0);
//! ```

pub mod contacts;
pub mod grab;
pub mod havok;
pub mod impulses;
pub mod layers;
pub mod ragdoll;
pub mod rigid;
pub mod shapes;
mod vec;
pub mod view_caster;
pub mod wind;

use std::collections::{HashMap, HashSet};

use vec::*;

pub type Vec3 = [f32; 3];

/// Size of the grid's buckets, in game units.
const BUCKET: f32 = 128.0;

/// Solid surfaces as triangles, for collision and ray casts.
///
/// Each triangle can stand out from its plane by a shell (Havok's convex
/// radius: every shape in the game's models has 0.1 Havok units, 0.7 game
/// units, around it) and can belong to an object whose collision can be
/// switched off and on (a door that opens). Other characters, which move,
/// are kept apart from the triangles ([`Collider::set_people`]).
#[derive(Debug, Clone, Default)]
pub struct Collider {
    vertices: Vec<Vec3>,
    triangles: Vec<Triangle>,
    /// Bucket (x, y) to the triangles overlapping it.
    grid: HashMap<(i32, i32), Vec<u32>>,
    /// What changes while walking (kept aside, so a collider stays small).
    live: Box<Live>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Triangle {
    corners: [u32; 3],
    /// How far its surface stands out (game units).
    shell: f32,
    /// The object it belongs to, 0 for none.
    owner: u32,
    /// The Havok material of the shape it came from ([NO_MATERIAL] when
    /// none was given): what a shot striking it sounds and looks like.
    material: u32,
    /// Its rigid body's friction and restitution: 1 + an index into
    /// `Live::surfaces`, 0 for none given.
    surface: u16,
    /// Its body's Havok layer ([`ANY_LAYER`]: not given), which decides
    /// what casts meet it ([`Collider::raycast_layer`]).
    layer: u8,
    /// The placed reference it comes from, 0 for none
    /// ([`Collider::add_placed`]).
    reference: u32,
}

/// A triangle added without a Havok layer: every cast meets it.
pub const ANY_LAYER: u8 = u8::MAX;

/// How a surface rubs and bounces: its rigid body's friction and
/// restitution (`nif::RigidBodyInfo`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Surface {
    pub friction: f32,
    pub restitution: f32,
}

#[derive(Debug, Clone, Default)]
struct Live {
    /// The surfaces triangles were added with (see `Triangle::surface`).
    surfaces: Vec<Surface>,
    /// Objects whose triangles are switched off.
    hidden: HashSet<u32>,
    /// Other characters, as upright cylinders.
    people: Vec<Person>,
    /// Owned triangles' vertices where they were added (an object that
    /// moves, such as a door's leaf, is put back from these), and the
    /// owned triangles themselves, by owner.
    rest: HashMap<u32, Vec<(u32, Vec3)>>,
    owned: HashMap<u32, Vec<u32>>,
    /// The triangles of each placed reference ([`Collider::add_placed`]).
    placed: HashMap<u32, Vec<u32>>,
}

/// Another character a walking one runs into: the game's character
/// controllers collide with each other (layer 30 with itself), and the
/// contact callback (`00c711d0`) flattens each such contact to a vertical
/// wall (its normal's z set to 0), so nobody stands on anybody.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Person {
    pub feet: Vec3,
    pub radius: f32,
    pub height: f32,
}

/// A triangle added without a Havok material.
pub const NO_MATERIAL: u32 = u32::MAX;

fn bucket(v: f32) -> i32 {
    (v / BUCKET).floor() as i32
}

/// Whether a triangle's box is further than `reach` (and a margin, for
/// rounding) from a segment's box along some axis: then no point of it is
/// within `reach` of the segment, and the exact test can be skipped.
fn surely_beyond(t: [Vec3; 3], a: Vec3, b: Vec3, reach: f32) -> bool {
    const MARGIN: f32 = 0.05;
    (0..3).any(|k| {
        let (t0, t1) = (
            t[0][k].min(t[1][k]).min(t[2][k]),
            t[0][k].max(t[1][k]).max(t[2][k]),
        );
        let (s0, s1) = (a[k].min(b[k]), a[k].max(b[k]));
        t0 - s1 > reach + MARGIN || s0 - t1 > reach + MARGIN
    })
}

/// [`bucket_keys`] without making a list: each key in the same order.
fn for_bucket(a: Vec3, b: Vec3, c: Vec3, mut f: impl FnMut((i32, i32))) {
    let (x0, x1) = (a[0].min(b[0]).min(c[0]), a[0].max(b[0]).max(c[0]));
    let (y0, y1) = (a[1].min(b[1]).min(c[1]), a[1].max(b[1]).max(c[1]));
    for bx in bucket(x0)..=bucket(x1) {
        for by in bucket(y0)..=bucket(y1) {
            f((bx, by));
        }
    }
}

/// The buckets a triangle's x-y box overlaps.
fn bucket_keys(a: Vec3, b: Vec3, c: Vec3) -> Vec<(i32, i32)> {
    let (x0, x1) = (a[0].min(b[0]).min(c[0]), a[0].max(b[0]).max(c[0]));
    let (y0, y1) = (a[1].min(b[1]).min(c[1]), a[1].max(b[1]).max(c[1]));
    let mut keys = Vec::new();
    for bx in bucket(x0)..=bucket(x1) {
        for by in bucket(y0)..=bucket(y1) {
            keys.push((bx, by));
        }
    }
    keys
}

impl Collider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds triangles (indices into `vertices`); out-of-range ones and ones
    /// with a corner that isn't a finite number are skipped.
    pub fn add(&mut self, vertices: &[Vec3], triangles: &[[u32; 3]]) {
        self.add_solid(vertices, triangles, 0.0, 0);
    }

    /// [`Collider::add`] with a shell around each triangle (game units) and
    /// the object they belong to (0 for none; see
    /// [`Collider::set_hidden`]).
    pub fn add_solid(&mut self, vertices: &[Vec3], triangles: &[[u32; 3]], shell: f32, owner: u32) {
        self.add_solid_material(vertices, triangles, shell, owner, NO_MATERIAL);
    }

    /// [`Self::add`], the triangles being of a Havok material (what a shot
    /// striking them looks and sounds like).
    pub fn add_with_material(&mut self, vertices: &[Vec3], triangles: &[[u32; 3]], material: u32) {
        self.add_solid_material(vertices, triangles, 0.0, 0, material);
    }

    /// [`Self::add_solid`] with the triangles' Havok material.
    pub fn add_solid_material(
        &mut self,
        vertices: &[Vec3],
        triangles: &[[u32; 3]],
        shell: f32,
        owner: u32,
        material: u32,
    ) {
        self.add_solid_surface(vertices, triangles, (shell, owner, material), None);
    }

    /// [`Self::add_solid_material`] (shell, owner and material) with the
    /// friction and restitution of the body they belong to, which rigid
    /// bodies resting on them rub and bounce against
    /// ([`crate::rigid`]).
    pub fn add_solid_surface(
        &mut self,
        vertices: &[Vec3],
        triangles: &[[u32; 3]],
        parts: (f32, u32, u32),
        surface: Option<Surface>,
    ) {
        self.add_layered(vertices, triangles, parts, surface, ANY_LAYER);
    }

    /// [`Self::add_solid_surface`] with the Havok layer of the body the
    /// triangles belong to (`nif::CollisionPart::layer`). An owner is also
    /// the placed reference they come from ([`Self::add_placed`]).
    pub fn add_layered(
        &mut self,
        vertices: &[Vec3],
        triangles: &[[u32; 3]],
        parts: (f32, u32, u32),
        surface: Option<Surface>,
        layer: u8,
    ) {
        self.add_placed(vertices, triangles, parts, surface, layer, parts.1);
    }

    /// [`Self::add_layered`] for the collision of a placed reference
    /// (`reference`, 0 for none): what the crosshair's pick
    /// ([`view_caster`]) takes a triangle for, whether or not the triangles
    /// have an owner that moves or switches them off.
    pub fn add_placed(
        &mut self,
        vertices: &[Vec3],
        triangles: &[[u32; 3]],
        (shell, owner, material): (f32, u32, u32),
        surface: Option<Surface>,
        layer: u8,
        reference: u32,
    ) {
        let surface = surface.map_or(0, |s| self.surface_index(s));
        let base = self.vertices.len() as u32;
        self.vertices.extend_from_slice(vertices);
        if owner != 0 {
            let rest = self.live.rest.entry(owner).or_default();
            rest.extend(
                vertices
                    .iter()
                    .enumerate()
                    .map(|(i, &v)| (base + i as u32, v)),
            );
        }
        for t in triangles {
            if t.iter().any(|&i| i as usize >= vertices.len()) {
                continue;
            }
            self.push_triangle(
                [t[0] + base, t[1] + base, t[2] + base],
                shell,
                owner,
                (material, surface, layer, reference),
            );
        }
    }

    /// 1 + the index of a surface in the table (added if new; the table is
    /// small: a few values per place). 0 once it's full.
    fn surface_index(&mut self, s: Surface) -> u16 {
        let table = &mut self.live.surfaces;
        if let Some(i) = table.iter().position(|&t| t == s) {
            return i as u16 + 1;
        }
        if table.len() >= usize::from(u16::MAX - 1) {
            return 0;
        }
        table.push(s);
        table.len() as u16
    }

    /// The friction and restitution a triangle was added with, if any.
    pub fn surface(&self, index: u32) -> Option<Surface> {
        let s = self.triangles.get(index as usize)?.surface;
        (s > 0)
            .then(|| self.live.surfaces.get(usize::from(s) - 1).copied())
            .flatten()
    }

    /// Moves an object's triangles (`owner`, see [`Collider::add_solid`])
    /// to where a rigid move puts their added positions: `v = rotation ×
    /// rest + translation` (a 3 × 3 row-major rotation). A door's leaf
    /// follows its animation this way.
    pub fn move_owner(&mut self, owner: u32, rotation: &[[f32; 3]; 3], translation: Vec3) {
        if owner == 0 {
            return;
        }
        let Some(rest) = self.live.rest.get(&owner) else {
            return;
        };
        let Some(owned) = self.live.owned.get(&owner) else {
            return;
        };
        // Out of their old buckets first: each bucket they're in looked
        // through once (a triangle is only in its own buckets, so the lists
        // come out as taking them out one at a time leaves them).
        let owned: Vec<u32> = owned.clone();
        let lo = owned.iter().copied().min().unwrap_or(0);
        let hi = owned.iter().copied().max().unwrap_or(0);
        // An owner's triangles are added together: when they're one run
        // of numbers, being in the run is being one of them.
        let run = (hi - lo) as usize + 1 == owned.len();
        let leaving: HashSet<u32> = if run {
            HashSet::new()
        } else {
            owned.iter().copied().collect()
        };
        let mut buckets: Vec<(i32, i32)> = Vec::new();
        for &t in &owned {
            let [a, b, c] = self.triangle(t);
            for_bucket(a, b, c, |key| {
                if !buckets.contains(&key) {
                    buckets.push(key);
                }
            });
        }
        for key in &buckets {
            if let Some(list) = self.grid.get_mut(key) {
                list.retain(|&i| i < lo || i > hi || (!run && !leaving.contains(&i)));
            }
        }
        for &(i, p) in rest {
            let r = [
                rotation[0][0] * p[0] + rotation[0][1] * p[1] + rotation[0][2] * p[2],
                rotation[1][0] * p[0] + rotation[1][1] * p[1] + rotation[1][2] * p[2],
                rotation[2][0] * p[0] + rotation[2][1] * p[1] + rotation[2][2] * p[2],
            ];
            self.vertices[i as usize] = add(r, translation);
        }
        // Into their new ones: each bucket's in the owner's order, as
        // putting them in one at a time does.
        let mut arriving: Vec<((i32, i32), Vec<u32>)> = Vec::new();
        for &t in &owned {
            let [a, b, c] = self.triangle(t);
            for_bucket(a, b, c, |key| {
                match arriving.iter_mut().find(|(k, _)| *k == key) {
                    Some((_, list)) => list.push(t),
                    None => arriving.push((key, vec![t])),
                }
            });
        }
        for (key, list) in arriving {
            self.grid.entry(key).or_default().extend(list);
        }
    }

    /// One triangle over vertices already added. Triangles without an area
    /// (their corners in a line: the game's meshes have a few) have no
    /// surface to touch, and the closest-point tests divide by their area,
    /// so they're left out.
    fn push_triangle(
        &mut self,
        tri: [u32; 3],
        shell: f32,
        owner: u32,
        (material, surface, layer, reference): (u32, u16, u8, u32),
    ) {
        let [a, b, c] = tri.map(|i| self.vertices[i as usize]);
        if [a, b, c].iter().flatten().any(|v| !v.is_finite()) {
            return;
        }
        let area = length(cross(sub(b, a), sub(c, a)));
        // Also leaves out a NaN area.
        if area.is_nan() || area <= 1e-6 {
            return;
        }
        let index = self.triangles.len() as u32;
        self.triangles.push(Triangle {
            corners: tri,
            shell,
            owner,
            material,
            surface,
            layer,
            reference,
        });
        if owner != 0 {
            self.live.owned.entry(owner).or_default().push(index);
        }
        if reference != 0 {
            self.live.placed.entry(reference).or_default().push(index);
        }
        for key in bucket_keys(a, b, c) {
            self.grid.entry(key).or_default().push(index);
        }
    }

    /// Adds every triangle of another collider (the squares of an outdoor
    /// area, gathered into one), with its shells, owners and switched-off
    /// objects.
    ///
    /// Its triangles were checked as they went into it and its buckets are
    /// those of its triangles where they are now, so they're taken over as
    /// they are, renumbered (each bucket and owner's list in triangle
    /// order, as pushing them in one at a time puts them), rather than
    /// measured again one by one: outdoors this is redone for every loaded
    /// square whenever one comes or goes.
    pub fn extend(&mut self, other: &Collider) {
        let base = self.vertices.len() as u32;
        let first = self.triangles.len() as u32;
        self.vertices.extend_from_slice(&other.vertices);
        let surfaces: Vec<u16> = other
            .live
            .surfaces
            .iter()
            .map(|&s| self.surface_index(s))
            .collect();
        self.triangles
            .extend(other.triangles.iter().map(|tri| Triangle {
                corners: tri.corners.map(|i| i + base),
                surface: match tri.surface {
                    0 => 0,
                    s => surfaces.get(usize::from(s) - 1).copied().unwrap_or(0),
                },
                ..*tri
            }));
        let renumbered = |list: &[u32]| {
            let mut list: Vec<u32> = list.iter().map(|&t| t + first).collect();
            list.sort_unstable();
            list
        };
        for (&key, list) in &other.grid {
            if !list.is_empty() {
                self.grid.entry(key).or_default().extend(renumbered(list));
            }
        }
        for (&owner, list) in &other.live.owned {
            if !list.is_empty() {
                self.live
                    .owned
                    .entry(owner)
                    .or_default()
                    .extend(renumbered(list));
            }
        }
        // The placed objects' lists (the crosshair's view caster) the same
        // way.
        for (&reference, list) in &other.live.placed {
            if !list.is_empty() {
                self.live
                    .placed
                    .entry(reference)
                    .or_default()
                    .extend(renumbered(list));
            }
        }
        self.live.hidden.extend(other.live.hidden.iter().copied());
        for (&owner, rest) in &other.live.rest {
            self.live
                .rest
                .entry(owner)
                .or_default()
                .extend(rest.iter().map(|&(i, v)| (i + base, v)));
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// The Havok material a triangle was added with, if any.
    pub fn material(&self, index: u32) -> Option<u32> {
        self.triangles
            .get(index as usize)
            .map(|t| t.material)
            .filter(|&m| m != NO_MATERIAL)
    }

    /// The corners of a triangle.
    pub fn triangle(&self, index: u32) -> [Vec3; 3] {
        self.triangles[index as usize]
            .corners
            .map(|i| self.vertices[i as usize])
    }

    /// How far a triangle's surface stands out from it.
    pub fn shell(&self, index: u32) -> f32 {
        self.triangles[index as usize].shell
    }

    /// The object a triangle belongs to (0 for none).
    pub fn owner(&self, index: u32) -> u32 {
        self.triangles[index as usize].owner
    }

    fn switched_off(&self, index: u32) -> bool {
        !self.live.hidden.is_empty() && self.live.hidden.contains(&self.owner(index))
    }

    /// Switches an object's triangles off (`true`: an open door) or back on.
    pub fn set_hidden(&mut self, owner: u32, hidden: bool) {
        if hidden && owner != 0 {
            self.live.hidden.insert(owner);
        } else {
            self.live.hidden.remove(&owner);
        }
    }

    pub fn is_hidden(&self, owner: u32) -> bool {
        self.live.hidden.contains(&owner)
    }

    /// Whether any triangle belongs to this object.
    pub fn owns(&self, owner: u32) -> bool {
        owner != 0 && self.live.owned.get(&owner).is_some_and(|t| !t.is_empty())
    }

    /// The other characters walkers run into, replacing the last list.
    pub fn set_people(&mut self, people: Vec<Person>) {
        self.live.people = people;
    }

    pub fn people(&self) -> &[Person] {
        &self.live.people
    }

    /// Triangles whose buckets overlap the box from `lo` to `hi` (x and y;
    /// z isn't bucketed), each once; switched-off objects' are left out.
    pub fn near(&self, lo: Vec3, hi: Vec3) -> Vec<u32> {
        let mut out = Vec::new();
        for bx in bucket(lo[0])..=bucket(hi[0]) {
            for by in bucket(lo[1])..=bucket(hi[1]) {
                if let Some(list) = self.grid.get(&(bx, by)) {
                    out.extend_from_slice(list);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        if !self.live.hidden.is_empty() {
            out.retain(|&t| !self.switched_off(t));
        }
        out
    }

    /// The nearest surface along a ray from `origin` in `direction` (unit
    /// length), within `max` units: its distance and triangle.
    pub fn raycast(&self, origin: Vec3, direction: Vec3, max: f32) -> Option<(f32, u32)> {
        self.cast(origin, direction, max, false, None)
    }

    /// [`Collider::raycast`] for a cast on Havok layer `layer`: triangles
    /// whose body's layer that layer doesn't touch (the game's collision
    /// filter, [`layers::Filter::layers_touch`] with the cast first, as the
    /// ray filter `00c84930` asks it) are passed through. A shot (layer 6,
    /// `PROJECTILE`) goes through a `TRANSPARENT` (3) chain-link fence that
    /// stops a walker.
    pub fn raycast_layer(
        &self,
        origin: Vec3,
        direction: Vec3,
        max: f32,
        layer: u8,
    ) -> Option<(f32, u32)> {
        self.cast(origin, direction, max, false, Some(layer))
    }

    /// The Havok layer a triangle's body is on ([`ANY_LAYER`]: not given).
    pub fn layer(&self, index: u32) -> u8 {
        self.triangles[index as usize].layer
    }

    /// [`Collider::raycast`], also meeting switched-off objects (an open
    /// door, to close it again).
    pub fn raycast_including_hidden(
        &self,
        origin: Vec3,
        direction: Vec3,
        max: f32,
    ) -> Option<(f32, u32)> {
        self.cast(origin, direction, max, true, None)
    }

    fn cast(
        &self,
        origin: Vec3,
        direction: Vec3,
        max: f32,
        hidden: bool,
        layer: Option<u8>,
    ) -> Option<(f32, u32)> {
        let filter = layer.map(|l| (l, layers::Filter::shared()));
        let passes = |t: u32| {
            filter.is_some_and(|(l, f)| {
                let tl = self.triangles[t as usize].layer;
                tl != ANY_LAYER && !f.layers_touch(l, tl)
            })
        };
        // Walk the buckets the ray crosses, a step at a time.
        let steps = (max / (BUCKET * 0.5)).ceil().max(1.0) as usize;
        let mut best: Option<(f32, u32)> = None;
        let mut seen = std::collections::HashSet::new();
        for s in 0..=steps {
            let along = (s as f32 * BUCKET * 0.5).min(max);
            let p = add(origin, scale(direction, along));
            if best.is_some_and(|(t, _)| t < along - BUCKET) {
                break;
            }
            for bx in bucket(p[0]) - 1..=bucket(p[0]) + 1 {
                for by in bucket(p[1]) - 1..=bucket(p[1]) + 1 {
                    let Some(list) = self.grid.get(&(bx, by)) else {
                        continue;
                    };
                    for &t in list {
                        if !seen.insert(t) || (!hidden && self.switched_off(t)) || passes(t) {
                            continue;
                        }
                        let [a, b, c] = self.triangle(t);
                        if let Some(d) = ray_triangle(origin, direction, a, b, c) {
                            if d <= max && best.map_or(true, |(bd, _)| d < bd) {
                                best = Some((d, t));
                            }
                        }
                    }
                }
            }
        }
        best
    }

    /// A sphere of `radius` whose centre moves from `origin` along
    /// `direction` (unit length) for up to `max` units: where it first
    /// touches a surface, with the point it touches. Each triangle's shell
    /// counts as part of it, and switched-off objects (open doors) are
    /// passed through, as for [`Collider::raycast`]. A sphere already
    /// touching something gives distance 0 and the nearest point of what
    /// it touches. (The game's cameras are kept out of walls with such a
    /// cast, `fCameraCasterSize` wide: `0094a0c0`.)
    pub fn spherecast(
        &self,
        origin: Vec3,
        direction: Vec3,
        max: f32,
        radius: f32,
    ) -> Option<SphereHit> {
        let steps = (max / (BUCKET * 0.5)).ceil().max(1.0) as usize;
        let mut best: Option<SphereHit> = None;
        let mut seen = std::collections::HashSet::new();
        for s in 0..=steps {
            let along = (s as f32 * BUCKET * 0.5).min(max);
            let p = add(origin, scale(direction, along));
            if best.is_some_and(|h| h.distance < along - BUCKET) {
                break;
            }
            for bx in bucket(p[0]) - 1..=bucket(p[0]) + 1 {
                for by in bucket(p[1]) - 1..=bucket(p[1]) + 1 {
                    let Some(list) = self.grid.get(&(bx, by)) else {
                        continue;
                    };
                    for &t in list {
                        if !seen.insert(t) || self.switched_off(t) {
                            continue;
                        }
                        let [a, b, c] = self.triangle(t);
                        let shell = self.shell(t);
                        let r = radius + shell;
                        if let Some((d, on_triangle)) =
                            sphere_sweep_triangle(origin, direction, max, r, a, b, c)
                        {
                            if best.map_or(true, |h| d < h.distance) {
                                // The touching point is on the shell's
                                // surface, toward the sphere's centre.
                                let centre = add(origin, scale(direction, d));
                                let point = add(
                                    on_triangle,
                                    scale(normalize(sub(centre, on_triangle)), shell),
                                );
                                best = Some(SphereHit {
                                    distance: d,
                                    point,
                                    triangle: t,
                                });
                            }
                        }
                    }
                }
            }
        }
        best
    }

    /// The placed reference a triangle comes from (0 for none).
    pub fn reference(&self, index: u32) -> u32 {
        self.triangles[index as usize].reference
    }

    /// Every triangle a sphere of `radius` cast from `origin` along
    /// `direction` (unit length) for up to `max` units touches, as an
    /// all-hits collector gathers a Havok linear cast's: each with where it
    /// is touched, nearest first. Only triangles whose body's layer the
    /// cast's `layer` touches ([`layers::Filter::layers_touch`], the cast
    /// first; those added without a layer always count), not switched off.
    pub fn spherecast_all(
        &self,
        origin: Vec3,
        direction: Vec3,
        max: f32,
        radius: f32,
        layer: u8,
    ) -> Vec<SphereHit> {
        let filter = layers::Filter::shared();
        let lo = [0, 1, 2].map(|i| origin[i].min(origin[i] + direction[i] * max) - radius);
        let hi = [0, 1, 2].map(|i| origin[i].max(origin[i] + direction[i] * max) + radius);
        let mut out = Vec::new();
        for t in self.near(lo, hi) {
            let tl = self.layer(t);
            if tl != ANY_LAYER && !filter.layers_touch(layer, tl) {
                continue;
            }
            let [a, b, c] = self.triangle(t);
            let shell = self.shell(t);
            if let Some((d, on_triangle)) =
                sphere_sweep_triangle(origin, direction, max, radius + shell, a, b, c)
            {
                let centre = add(origin, scale(direction, d));
                let point = add(
                    on_triangle,
                    scale(normalize(sub(centre, on_triangle)), shell),
                );
                out.push(SphereHit {
                    distance: d,
                    point,
                    triangle: t,
                });
            }
        }
        out.sort_by(|a, b| a.distance.total_cmp(&b.distance));
        out
    }

    /// Where a ray from `origin` along `direction` (unit length) first meets
    /// one placed reference's triangles ([`Self::add_placed`]), however far,
    /// passing everything else; `None` when it misses them or they're
    /// switched off.
    pub fn raycast_reference(&self, origin: Vec3, direction: Vec3, reference: u32) -> Option<f32> {
        let tris = self.live.placed.get(&reference)?;
        tris.iter()
            .filter(|&&t| !self.switched_off(t))
            .filter_map(|&t| {
                let [a, b, c] = self.triangle(t);
                ray_triangle(origin, direction, a, b, c)
            })
            .min_by(f32::total_cmp)
    }
}

/// Where a ray from `o` along `d` (unit length) first meets a capsule (the
/// segment `a b` grown by `r`); 0 when it starts inside.
pub fn ray_capsule(o: Vec3, d: Vec3, a: Vec3, b: Vec3, r: f32) -> Option<f32> {
    let inside = {
        let p = closest_on_segment(o, a, b);
        dist2(o, p) <= r * r
    };
    if inside {
        return Some(0.0);
    }
    let mut best: Option<f32> = None;
    let mut consider = |t: f32| {
        if t >= 0.0 && best.map_or(true, |b| t < b) {
            best = Some(t);
        }
    };
    if let Some((t, _)) = sphere_sweep_segment(o, d, r, a, b) {
        consider(t);
    }
    for c in [a, b] {
        if let Some(t) = ray_sphere(o, d, c, r) {
            consider(t);
        }
    }
    best
}

/// A sphere of radius `r` moving from `o` along `d` (unit length) up to
/// `max`: when it first touches a capsule (segment `a b`, radius `cr`) and
/// the point of the capsule's surface it touches; already touching, 0 and
/// the capsule's surface point nearest the start.
pub fn sphere_sweep_capsule(
    o: Vec3,
    d: Vec3,
    max: f32,
    r: f32,
    (a, b, cr): (Vec3, Vec3, f32),
) -> Option<(f32, Vec3)> {
    let t = ray_capsule(o, d, a, b, r + cr)?;
    if t > max {
        return None;
    }
    let centre = add(o, scale(d, t));
    let axis = closest_on_segment(centre, a, b);
    let toward = sub(centre, axis);
    let n = if dot(toward, toward) > 1e-12 {
        normalize(toward)
    } else {
        scale(d, -1.0)
    };
    Some((t, add(axis, scale(n, cr))))
}

/// The point of segment `a b` nearest `p`.
fn closest_on_segment(p: Vec3, a: Vec3, b: Vec3) -> Vec3 {
    let e = sub(b, a);
    let ee = dot(e, e);
    if ee < 1e-12 {
        return a;
    }
    let u = (dot(sub(p, a), e) / ee).clamp(0.0, 1.0);
    add(a, scale(e, u))
}

/// Where a swept sphere first touches something ([`Collider::spherecast`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphereHit {
    /// How far the sphere's centre travelled from the origin.
    pub distance: f32,
    /// The point on the surface it touches.
    pub point: Vec3,
    pub triangle: u32,
}

/// A sphere of radius `r` moving from `o` along `d` (unit length) up to
/// `max`: the distance at which it first touches triangle `a b c` and the
/// point of the triangle it touches, if it does. Already touching: 0 and
/// the nearest point.
fn sphere_sweep_triangle(
    o: Vec3,
    d: Vec3,
    max: f32,
    r: f32,
    a: Vec3,
    b: Vec3,
    c: Vec3,
) -> Option<(f32, Vec3)> {
    let nearest = closest_on_triangle(o, a, b, c);
    if dist2(o, nearest) <= r * r {
        return Some((0.0, nearest));
    }
    let mut best: Option<(f32, Vec3)> = None;
    let mut consider = |t: f32, p: Vec3| {
        if t >= 0.0 && t <= max && best.map_or(true, |(bt, _)| t < bt) {
            best = Some((t, p));
        }
    };
    // The face: the sphere's near side reaches the plane while approaching
    // it, at a point inside the triangle. (Overlapping the plane already
    // and sliding in from outside the triangle is an edge or corner
    // touch, below.)
    let n = normalize(cross(sub(b, a), sub(c, a)));
    let s0 = dot(n, sub(o, a));
    let nd = dot(n, d);
    if nd.abs() > 1e-9 && s0 * nd < 0.0 {
        let side = if s0 >= 0.0 { 1.0 } else { -1.0 };
        let t = (side * r - s0) / nd;
        if t >= 0.0 {
            let centre = add(o, scale(d, t));
            let p = sub(centre, scale(n, side * r));
            if point_in_triangle(p, a, b, c, n) {
                consider(t, p);
            }
        }
    }
    for (p, q) in [(a, b), (b, c), (c, a)] {
        if let Some((t, point)) = sphere_sweep_segment(o, d, r, p, q) {
            consider(t, point);
        }
    }
    for v in [a, b, c] {
        if let Some(t) = ray_sphere(o, d, v, r) {
            consider(t, v);
        }
    }
    best
}

/// Whether `p`, on the plane of triangle `a b c` with normal `n`, lies
/// inside it (edges included).
fn point_in_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3, n: Vec3) -> bool {
    dot(cross(sub(b, a), sub(p, a)), n) >= -1e-6
        && dot(cross(sub(c, b), sub(p, b)), n) >= -1e-6
        && dot(cross(sub(a, c), sub(p, c)), n) >= -1e-6
}

/// A sphere of radius `r` moving from `o` along `d`: when it first touches
/// the segment `p q` (its side, not its ends), and the point on the segment
/// it touches.
fn sphere_sweep_segment(o: Vec3, d: Vec3, r: f32, p: Vec3, q: Vec3) -> Option<(f32, Vec3)> {
    let e = sub(q, p);
    let ee = dot(e, e);
    if ee < 1e-12 {
        return None;
    }
    let m = sub(o, p);
    let me = dot(m, e);
    let de = dot(d, e);
    // The parts of the offset and the motion across the segment's line.
    let w = sub(m, scale(e, me / ee));
    let v = sub(d, scale(e, de / ee));
    let qa = dot(v, v);
    if qa < 1e-12 {
        return None;
    }
    let qb = 2.0 * dot(v, w);
    let qc = dot(w, w) - r * r;
    let disc = qb * qb - 4.0 * qa * qc;
    if disc < 0.0 {
        return None;
    }
    let t = (-qb - disc.sqrt()) / (2.0 * qa);
    if t < 0.0 {
        return None;
    }
    let u = (me + de * t) / ee;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    Some((t, add(p, scale(e, u))))
}

/// When a ray from `o` along `d` (unit length) first reaches the surface of
/// a sphere of radius `r` at `centre`.
fn ray_sphere(o: Vec3, d: Vec3, centre: Vec3, r: f32) -> Option<f32> {
    let m = sub(o, centre);
    let b = dot(m, d);
    let c = dot(m, m) - r * r;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    (t >= 0.0).then_some(t)
}

/// Möller–Trumbore, both faces: the distance along the ray, if it hits.
fn ray_triangle(origin: Vec3, dir: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let p = cross(dir, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-9 {
        return None;
    }
    let inv = 1.0 / det;
    let s = sub(origin, a);
    let u = dot(s, p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(s, e1);
    let v = dot(dir, q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = dot(e2, q) * inv;
    (t >= 0.0).then_some(t)
}

/// Game units per Havok unit (the exe's 69.99125671 game units a metre over
/// 10).
pub const HAVOK_UNIT: f32 = 6.999_125_7;

/// How far the ground's surface stands out from the terrain's triangles: the
/// game builds each square's terrain as a sampled height field wrapped in a
/// triangle collection with a radius of 0.5 Havok units (`00cb0820`, the
/// float at `01016248`), 3.5 game units; models' shapes have 0.1.
pub const TERRAIN_SHELL: f32 = 0.5 * HAVOK_UNIT;

/// How the ground rubs and bounces: each square's land body is built
/// (`00621f60`) from Havok's default body info (`00c8f510`, restitution
/// 0.4) as a fixed body (mass 0, motion 5) with its friction set to
/// `[Landscape] fLandFriction` (2.5 in the exe and `Fallout.ini`).
pub const LAND_SURFACE: Surface = Surface {
    friction: 2.5,
    restitution: 0.4,
};

/// The Havok world's gravity, (0, 0, −98.1) Havok units a second squared
/// (`00f4b550`: 10 × −9.81, put on z by both world builders), in game
/// units: 686.61.
pub const GRAVITY: f32 = 98.1 * HAVOK_UNIT;

/// A walking character's size: a vertical capsule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterShape {
    pub radius: f32,
    /// From the feet to the top of the head.
    pub height: f32,
    /// The highest ledge it walks up without jumping.
    pub step: f32,
    /// Units per second squared, downward.
    pub gravity: f32,
    /// Steepest ground it stands on: the cosine of the slope.
    pub max_slope_cos: f32,
    /// How far above the feet the shape's bottom floats.
    pub lift: f32,
}

impl CharacterShape {
    /// The player, and every person (the game gives people one fixed size,
    /// not scaled by height; read from its code, `00c72410`): half extents
    /// 23 × 17.5 × 64, so radius (23 + 17.5) / 2 = 20.25 and height 128;
    /// steps up to 31 (`00c6eb00`); ground no steeper than 47°
    /// (`011b0148`, cosine 0.682); the world's gravity, multiplied by one.
    /// The shape's bottom floats 0.1 Havok units (0.7) above the feet
    /// (twice the 0.05 keep distance, `00c72410`), which is just the shell
    /// every model's collision has around it (0.1 Havok units, see
    /// [`Collider`]): standing on a model, the feet are at its triangles.
    /// The game's shape is an eight-sided hull with a cone underneath 31.7
    /// units tall that slides over step edges; this is a capsule with the
    /// same radius and height (the step rule stands in for the cone).
    pub const PLAYER: CharacterShape = CharacterShape {
        radius: 20.25,
        height: 128.0,
        step: 31.0,
        gravity: GRAVITY,
        // cos 47°.
        max_slope_cos: 0.681_998_4,
        lift: 0.1 * HAVOK_UNIT,
    };

    /// The capsule's axis: the centres of its bottom and top spheres, for
    /// feet at `feet`.
    fn axis(&self, feet: Vec3) -> (Vec3, Vec3) {
        let bottom = self.lift + self.radius;
        let top = (self.lift + self.height - self.radius).max(bottom);
        (add(feet, [0.0, 0.0, bottom]), add(feet, [0.0, 0.0, top]))
    }
}

/// A walking character.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Character {
    /// The bottom of the capsule, in game units.
    pub feet: Vec3,
    /// Vertical speed, units per second (up positive).
    pub vertical_speed: f32,
    pub on_ground: bool,
    /// The height of the point it last stood on. With the rounded bottom
    /// resting on a ledge's edge, that's the ledge's top, a little above
    /// the feet; steps are measured from it, so a ledge higher than a step
    /// can't be climbed a bit at a time.
    pub ground: f32,
    /// The height the feet were at when it last left the ground (jumping
    /// or stepping off), and on landing how far it fell from there (the
    /// game measures falls this way, not from the top of a jump:
    /// `00c70550`); taken by whoever applies fall damage.
    pub left_ground_at: f32,
    pub fell: Option<f32>,
    /// Horizontal velocity, units per second, as [`Character::update_controlled`]
    /// keeps it from one update to the next.
    pub horizontal: [f32; 2],
}

/// The most the on-ground state changes the velocity in one update: 500
/// Havok units a second (`01013d84`, the movement input's maximum velocity
/// change, `00cd4800`), in game units.
pub const GROUND_MAX_VELOCITY_CHANGE: f32 = 500.0 * HAVOK_UNIT;
/// The in-air state's: 2000 Havok units a second (`01013970`, stored by
/// its constructor `00cd3f90`).
pub const AIR_MAX_VELOCITY_CHANGE: f32 = 2000.0 * HAVOK_UNIT;

/// Moves `current` toward `desired` by `gain` of the gap, the gap first
/// cut to `max_change` long (the controller's movement input, as the
/// ground and air states use it, `00cd4800` and `00cd3fb0`).
pub fn blend_velocity(
    current: [f32; 2],
    desired: [f32; 2],
    gain: f32,
    max_change: f32,
) -> [f32; 2] {
    let mut diff = [desired[0] - current[0], desired[1] - current[1]];
    let len = (diff[0] * diff[0] + diff[1] * diff[1]).sqrt();
    if len > max_change {
        let k = max_change / len;
        diff = [diff[0] * k, diff[1] * k];
    }
    [current[0] + gain * diff[0], current[1] + gain * diff[1]]
}

/// A surface the capsule rests on or is pushed by.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Support {
    /// The face's normal, turned toward the capsule.
    normal: Vec3,
    /// The height of the point touched.
    height: f32,
}

/// Longest time step the controller takes at once, in seconds.
const MAX_STEP: f32 = 1.0 / 120.0;
/// How many times a step pushes the capsule out of what it overlaps.
const RESOLVE_PASSES: usize = 4;

impl Character {
    pub fn new(feet: Vec3) -> Self {
        Self {
            feet,
            vertical_speed: 0.0,
            on_ground: false,
            ground: feet[2],
            left_ground_at: feet[2],
            fell: None,
            horizontal: [0.0; 2],
        }
    }

    /// One controller update of `dt` seconds wanting to go at `desired`
    /// (x, y; units per second), as the game's character states set the
    /// velocity once per update before it's integrated:
    ///
    /// - jumping (asked, and standing): upward at `jump_speed`, the
    ///   horizontal velocity set to the ground's own (still ground: 0) —
    ///   the jumping state `00cd4280` replaces it, it doesn't keep the
    ///   run-up;
    /// - on the ground: the wanted velocity (gain 1, `00cd4800`);
    /// - in the air: `air_gain` of the way from the current velocity to the
    ///   wanted one (`00cd3fb0`; `world::locomotion::air_gain`, 0.3).
    ///
    /// Whether the in-air state also runs in the update a jump starts
    /// isn't traced (the jumping state hands over through `00c6cba0`); here
    /// it doesn't. On slopes the game works the velocity out in the
    /// ground's plane and Havok's proxy solver slides it; neither is
    /// modelled (the horizontal velocity is used as it is).
    pub fn update_controlled(
        &mut self,
        collider: &Collider,
        shape: &CharacterShape,
        desired: [f32; 2],
        jump_speed: Option<f32>,
        air_gain: f32,
        dt: f32,
    ) {
        let jumping = self.on_ground && jump_speed.is_some_and(|s| s > 0.0);
        if jumping {
            self.horizontal = [0.0; 2];
        } else if self.on_ground {
            self.horizontal =
                blend_velocity(self.horizontal, desired, 1.0, GROUND_MAX_VELOCITY_CHANGE);
        } else {
            self.horizontal =
                blend_velocity(self.horizontal, desired, air_gain, AIR_MAX_VELOCITY_CHANGE);
        }
        let velocity = self.horizontal;
        let before = self.feet;
        self.update_with_jump(collider, shape, velocity, jump_speed, dt);
        // What's kept is the velocity the move ended with (Havok's proxy
        // leaves the solved velocity in the controller): stopped or slid by
        // what it ran into.
        if dt > 0.0 {
            self.horizontal = [
                (self.feet[0] - before[0]) / dt.min(0.25),
                (self.feet[1] - before[1]) / dt.min(0.25),
            ];
        }
    }

    /// Moves for `dt` seconds wanting to go at `velocity` (x, y; units per
    /// second), jumping at `jump_speed` (units per second up) if asked and
    /// standing on something.
    pub fn update(
        &mut self,
        collider: &Collider,
        shape: &CharacterShape,
        velocity: [f32; 2],
        jump: bool,
        dt: f32,
    ) {
        self.update_with_jump(collider, shape, velocity, jump.then_some(0.0), dt);
    }

    /// As [`Character::update`], with a jump speed (units per second up)
    /// when jumping.
    pub fn update_with_jump(
        &mut self,
        collider: &Collider,
        shape: &CharacterShape,
        velocity: [f32; 2],
        jump: Option<f32>,
        dt: f32,
    ) {
        if let (Some(speed), true) = (jump, self.on_ground) {
            if speed > 0.0 {
                self.vertical_speed = speed;
                self.on_ground = false;
                self.left_ground_at = self.feet[2];
            }
        }
        let mut left = dt.clamp(0.0, 0.25);
        while left > 1e-6 {
            let h = left.min(MAX_STEP);
            left -= h;
            // Stepping off an edge, the rounded bottom rolls over it before
            // letting go; the point last stood on is where it left.
            let (was_on_ground, z) = (self.on_ground, self.feet[2].max(self.ground));
            self.substep(collider, shape, velocity, h);
            if was_on_ground && !self.on_ground {
                self.left_ground_at = z;
            } else if !was_on_ground && self.on_ground {
                self.fell = Some(self.left_ground_at - self.feet[2]);
            }
        }
    }

    fn substep(
        &mut self,
        collider: &Collider,
        shape: &CharacterShape,
        velocity: [f32; 2],
        dt: f32,
    ) {
        let horizontal = [velocity[0] * dt, velocity[1] * dt, 0.0];
        let was_on_ground = self.on_ground;

        // Horizontal: straight, and stepped up then down; keep whichever
        // gets further.
        if dot(horizontal, horizontal) > 0.0 {
            let straight = self.moved(collider, shape, horizontal).0;
            let lifted = add(self.feet, [0.0, 0.0, shape.step]);
            let up = Character {
                feet: lifted,
                ..*self
            }
            .moved(collider, shape, [0.0; 3])
            .0;
            let mut stepped = None;
            if (up[2] - lifted[2]).abs() < 0.01 {
                let across = Character { feet: up, ..*self }
                    .moved(collider, shape, horizontal)
                    .0;
                let (down, ground) = Character {
                    feet: across,
                    ..*self
                }
                .swept_down(collider, shape, shape.step);
                // Landed on something no lower than it started and no more
                // than a step above where it stood: the step's edge counts
                // (the rounded bottom rests on it first).
                if ground.is_some_and(|g| {
                    g.normal[2] > 0.0
                        && g.height <= self.ground.max(self.feet[2]) + shape.step + 0.01
                }) && down[2] >= self.feet[2] - 0.01
                {
                    stepped = Some(down);
                }
            }
            let gain = |p: Vec3| {
                let d = sub(p, self.feet);
                d[0] * horizontal[0] + d[1] * horizontal[1]
            };
            self.feet = match stepped {
                Some(s) if was_on_ground && gain(s) > gain(straight) + 0.01 => s,
                _ => straight,
            };
        }

        // Vertical: gravity, then land or hit the ceiling.
        if !self.on_ground {
            self.vertical_speed -= shape.gravity * dt;
        }
        let (after, ground) = if self.on_ground {
            // Stay on the floor going down slopes and stairs.
            self.swept_down(collider, shape, shape.step)
        } else {
            self.moved(collider, shape, [0.0, 0.0, self.vertical_speed * dt])
        };
        let stands_on = |g: Option<Support>| g.filter(|g| g.normal[2] >= shape.max_slope_cos);
        let standing = stands_on(ground);
        if self.on_ground && standing.is_none() {
            // Walked off an edge: only the free fall moves it.
            self.on_ground = false;
            let (after, ground) = self.moved(collider, shape, [0.0; 3]);
            self.feet = after;
            if let Some(g) = stands_on(ground) {
                self.on_ground = true;
                self.ground = g.height;
            }
            return;
        }
        self.feet = after;
        if let Some(g) = standing {
            self.on_ground = true;
            self.vertical_speed = 0.0;
            self.ground = g.height;
        } else {
            self.on_ground = false;
            if ground.is_some_and(|g| g.normal[2] < -0.5) && self.vertical_speed > 0.0 {
                self.vertical_speed = 0.0;
            }
        }
    }

    /// Where the capsule ends up moving by `delta` and being pushed out of
    /// everything it overlaps, and the most upward-facing push it got (the
    /// surface it rests on), if any.
    fn moved(
        &self,
        collider: &Collider,
        shape: &CharacterShape,
        delta: Vec3,
    ) -> (Vec3, Option<Support>) {
        // In pieces of at most half the radius, so nothing is jumped
        // through and contacts are met in order.
        let pieces = (length(delta) / (shape.radius * 0.5)).ceil().max(1.0) as usize;
        let piece = scale(delta, 1.0 / pieces as f32);
        let mut feet = self.feet;
        let mut best: Option<Support> = None;
        for _ in 0..pieces {
            let (next, support) = Character { feet, ..*self }.pushed_out(collider, shape, piece);
            feet = next;
            if let Some(s) = support {
                if best.map_or(true, |b| s.normal[2] > b.normal[2]) {
                    best = Some(s);
                }
            }
        }
        (feet, best)
    }

    /// Lowers the capsule by up to `distance`, stopping where it first
    /// touches something (a sweep, not a push: landing on a step's edge
    /// stays on the step). Returns where it stops and the face it rests on.
    fn swept_down(
        &self,
        collider: &Collider,
        shape: &CharacterShape,
        distance: f32,
    ) -> (Vec3, Option<Support>) {
        const INCREMENT: f32 = 0.5;
        let steps = (distance / INCREMENT).ceil().max(1.0) as usize;
        let piece = distance / steps as f32;
        let mut feet = self.feet;
        // The triangles near it are the same at every step (the grid is
        // across x and y, which the sweep keeps): found once.
        let candidates = self.touch_candidates(collider, shape);
        for _ in 0..steps {
            let next = [feet[0], feet[1], feet[2] - piece];
            if let Some(support) = (Character {
                feet: next,
                ..*self
            })
            .touching_among(collider, shape, &candidates)
            {
                return (feet, Some(support));
            }
            feet = next;
        }
        (feet, None)
    }

    /// The triangles near enough for [`Character::touching_among`] to look at.
    fn touch_candidates(&self, collider: &Collider, shape: &CharacterShape) -> Vec<u32> {
        let r = shape.radius;
        let lo = [
            self.feet[0] - r - 1.0,
            self.feet[1] - r - 1.0,
            self.feet[2] - 1.0,
        ];
        let hi = [
            self.feet[0] + r + 1.0,
            self.feet[1] + r + 1.0,
            self.feet[2] + shape.lift + shape.height + 1.0,
        ];
        collider.near(lo, hi)
    }

    /// The face the capsule overlaps most among `candidates` (in their
    /// order), turned toward it, if any.
    fn touching_among(
        &self,
        collider: &Collider,
        shape: &CharacterShape,
        candidates: &[u32],
    ) -> Option<Support> {
        let r = shape.radius;
        let (bottom, top) = shape.axis(self.feet);
        let mut best: Option<Support> = None;
        for &t in candidates {
            let [a, b, c] = collider.triangle(t);
            if surely_beyond([a, b, c], bottom, top, r + collider.shell(t)) {
                continue;
            }
            let (on_axis, on_triangle) = segment_triangle_closest(bottom, top, a, b, c);
            let gap = sub(on_axis, on_triangle);
            let d = length(gap);
            // A triangle with no area gives no distance: not touched.
            let shell = collider.shell(t);
            if d.is_nan() || d >= r + shell - 0.01 {
                continue;
            }
            let face = normalize(cross(sub(b, a), sub(c, a)));
            let toward = if d > 1e-6 { gap } else { sub(bottom, a) };
            let normal = if dot(face, toward) >= 0.0 {
                face
            } else {
                scale(face, -1.0)
            };
            // Prefer what it stands on, and of that the highest point.
            let better = best.map_or(true, |b| {
                normal[2] > b.normal[2] + 1e-4
                    || (normal[2] > b.normal[2] - 1e-4 && on_triangle[2] > b.height)
            });
            if better {
                best = Some(Support {
                    normal,
                    height: stood_height(on_triangle, shell, shape),
                });
            }
        }
        best
    }

    /// Moves by `delta` (short) and pushes the capsule out of everything it
    /// overlaps.
    fn pushed_out(
        &self,
        collider: &Collider,
        shape: &CharacterShape,
        delta: Vec3,
    ) -> (Vec3, Option<Support>) {
        let mut feet = add(self.feet, delta);
        let r = shape.radius;
        let mut best: Option<Support> = None;
        // The deepest overlap first, then look again: pushing out of a face
        // also clears its neighbors' edges, which would otherwise push the
        // capsule sideways.
        let lo = [feet[0] - r - 4.0, feet[1] - r - 4.0, feet[2] - 4.0];
        let hi = [
            feet[0] + r + 4.0,
            feet[1] + r + 4.0,
            feet[2] + shape.lift + shape.height + 4.0,
        ];
        let candidates: Vec<u32> = collider
            .near(lo, hi)
            .into_iter()
            .filter(|&t| {
                let [a, b, c] = collider.triangle(t);
                a[2].max(b[2]).max(c[2]) >= lo[2] && a[2].min(b[2]).min(c[2]) <= hi[2]
            })
            .collect();
        for _ in 0..RESOLVE_PASSES * 2 {
            let (bottom, top) = shape.axis(feet);
            let mut deepest: Option<(f32, Vec3, Support)> = None;
            for &t in &candidates {
                let [a, b, c] = collider.triangle(t);
                if surely_beyond([a, b, c], bottom, top, r + collider.shell(t)) {
                    continue;
                }
                let (on_axis, on_triangle) = segment_triangle_closest(bottom, top, a, b, c);
                let gap = sub(on_axis, on_triangle);
                let d = length(gap);
                // A triangle with no area (two corners the same) gives no
                // distance at all: it's nothing to push against.
                let shell = collider.shell(t);
                let reach = r + shell;
                if d.is_nan() || d >= reach - 1e-4 {
                    continue;
                }
                let face = normalize(cross(sub(b, a), sub(c, a)));
                let n = if d > 1e-6 {
                    scale(gap, 1.0 / d)
                } else {
                    // The axis passes through the triangle: push along its
                    // face normal, toward the side the capsule came from.
                    if dot(face, sub(self.feet, a)) >= 0.0 {
                        face
                    } else {
                        scale(face, -1.0)
                    }
                };
                let depth = reach - d;
                if deepest.map_or(true, |(dd, _, _)| depth > dd) {
                    // What supports the capsule is the surface it touches,
                    // so report the face's normal (turned toward the
                    // capsule), not the push: resting on a step's edge is
                    // standing on the step.
                    let normal = if dot(face, n) >= 0.0 {
                        face
                    } else {
                        scale(face, -1.0)
                    };
                    deepest = Some((
                        depth,
                        n,
                        Support {
                            normal,
                            height: stood_height(on_triangle, shell, shape),
                        },
                    ));
                }
            }
            // Other people: pushed apart sideways only (the game flattens
            // these contacts to walls), wherever their heights overlap.
            for p in collider.people() {
                let below = feet[2] + shape.lift + shape.height < p.feet[2];
                let above = feet[2] + shape.lift > p.feet[2] + p.height;
                if below || above {
                    continue;
                }
                let apart = [feet[0] - p.feet[0], feet[1] - p.feet[1], 0.0];
                let d = length(apart);
                let reach = r + p.radius;
                if d >= reach - 1e-4 {
                    continue;
                }
                let n = if d > 1e-6 {
                    scale(apart, 1.0 / d)
                } else {
                    // Right on top of them: back the way it came.
                    let back = [-delta[0], -delta[1], 0.0];
                    if length(back) > 1e-6 {
                        normalize(back)
                    } else {
                        [1.0, 0.0, 0.0]
                    }
                };
                let depth = reach - d;
                if deepest.map_or(true, |(dd, _, _)| depth > dd) {
                    deepest = Some((
                        depth,
                        n,
                        Support {
                            normal: n,
                            height: p.feet[2],
                        },
                    ));
                }
            }
            let Some((depth, n, support)) = deepest else {
                break;
            };
            feet = add(feet, scale(n, depth + 1e-3));
            if best.map_or(true, |b| support.normal[2] > b.normal[2]) {
                best = Some(support);
            }
        }
        (feet, best)
    }
}

/// The height the feet are at standing on a point of a triangle: its shell's
/// surface above the point, less how far the shape floats above the feet
/// (the same on models' collision: the feet are at the triangles).
fn stood_height(on_triangle: Vec3, shell: f32, shape: &CharacterShape) -> f32 {
    on_triangle[2] + shell - shape.lift
}

/// The closest points between segment `p`–`q` and triangle `a b c`: one on
/// the segment, one on the triangle.
pub(crate) fn segment_triangle_closest(
    p: Vec3,
    q: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
) -> (Vec3, Vec3) {
    // The segment crossing the triangle: distance zero there.
    let dir = sub(q, p);
    let len = length(dir);
    if len > 1e-9 {
        let unit = scale(dir, 1.0 / len);
        if let Some(t) = ray_triangle(p, unit, a, b, c) {
            if t <= len {
                let hit = add(p, scale(unit, t));
                return (hit, hit);
            }
        }
    }
    let mut best = (p, closest_on_triangle(p, a, b, c));
    let mut best_d = dist2(best.0, best.1);
    let mut consider = |s: Vec3, t: Vec3| {
        let d = dist2(s, t);
        if d < best_d {
            best_d = d;
            best = (s, t);
        }
    };
    consider(q, closest_on_triangle(q, a, b, c));
    for (e0, e1) in [(a, b), (b, c), (c, a)] {
        let (s, t) = segment_segment_closest(p, q, e0, e1);
        consider(s, t);
    }
    best
}

/// The point of triangle `a b c` nearest `p` (Ericson, Real-Time Collision
/// Detection, 5.1.5).
pub(crate) fn closest_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return add(a, scale(ab, d1 / (d1 - d3)));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return add(a, scale(ac, d2 / (d2 - d6)));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return add(b, scale(sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6))));
    }
    let denom = 1.0 / (va + vb + vc);
    add(a, add(scale(ab, vb * denom), scale(ac, vc * denom)))
}

/// The closest points between segments `p1`–`q1` and `p2`–`q2` (Ericson,
/// 5.1.9).
pub(crate) fn segment_segment_closest(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> (Vec3, Vec3) {
    let d1 = sub(q1, p1);
    let d2 = sub(q2, p2);
    let r = sub(p1, p2);
    let a = dot(d1, d1);
    let e = dot(d2, d2);
    let f = dot(d2, r);
    let (s, t);
    if a <= 1e-12 && e <= 1e-12 {
        return (p1, p2);
    }
    if a <= 1e-12 {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = dot(d1, r);
        if e <= 1e-12 {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = dot(d1, d2);
            let denom = a * e - b * b;
            let mut s0 = if denom != 0.0 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t0 = (b * s0 + f) / e;
            if t0 < 0.0 {
                t0 = 0.0;
                s0 = (-c / a).clamp(0.0, 1.0);
            } else if t0 > 1.0 {
                t0 = 1.0;
                s0 = ((b - c) / a).clamp(0.0, 1.0);
            }
            s = s0;
            t = t0;
        }
    }
    (add(p1, scale(d1, s)), add(p2, scale(d2, t)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [`Collider::extend`] as it was: every triangle pushed in again, its
    /// buckets measured again.
    fn extend_one_at_a_time(c: &mut Collider, other: &Collider) {
        let base = c.vertices.len() as u32;
        c.vertices.extend_from_slice(&other.vertices);
        let surfaces: Vec<u16> = other
            .live
            .surfaces
            .iter()
            .map(|&s| c.surface_index(s))
            .collect();
        for tri in &other.triangles {
            let surface = match tri.surface {
                0 => 0,
                s => surfaces.get(usize::from(s) - 1).copied().unwrap_or(0),
            };
            c.push_triangle(
                tri.corners.map(|i| i + base),
                tri.shell,
                tri.owner,
                (tri.material, surface, tri.layer, tri.reference),
            );
        }
        c.live.hidden.extend(other.live.hidden.iter().copied());
        for (&owner, rest) in &other.live.rest {
            c.live
                .rest
                .entry(owner)
                .or_default()
                .extend(rest.iter().map(|&(i, v)| (i + base, v)));
        }
    }

    /// Squares of ground and objects, some owned, some with surfaces, some
    /// triangles without area or with a corner that isn't a number, one
    /// owner moved after.
    fn square(seed: u32, at: [f32; 2]) -> Collider {
        let mut rng = seed.wrapping_mul(2654435761) | 1;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            (rng % 10_000) as f32 / 10_000.0
        };
        let mut c = Collider::new();
        for part in 0..12u32 {
            let mut vertices = Vec::new();
            let mut tris = Vec::new();
            for t in 0..20u32 {
                let x = at[0] + next() * 4096.0;
                let y = at[1] + next() * 4096.0;
                let size = 10.0 + next() * 900.0;
                vertices.push([x, y, next() * 100.0]);
                vertices.push([x + size, y + next() * size, next() * 100.0]);
                vertices.push([x + next() * size, y + size, next() * 100.0]);
                let b = 3 * t;
                tris.push(match t % 9 {
                    // Without area, then out of range.
                    4 => [b, b, b + 1],
                    8 => [b, b + 1, 9999],
                    _ => [b, b + 1, b + 2],
                });
            }
            if part == 5 {
                vertices[0] = [f32::NAN, 0.0, 0.0];
            }
            let owner = if part % 3 == 0 { 0 } else { 0x100 + part % 4 };
            let surface = (part % 2 == 0).then_some(Surface {
                friction: 0.1 * (part % 3) as f32,
                restitution: 0.5,
            });
            c.add_layered(
                &vertices,
                &tris,
                (part as f32, owner, part),
                surface,
                (part % 5) as u8,
            );
        }
        c.set_hidden(0x101, true);
        let turn = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        c.move_owner(0x102, &turn, [500.0, -300.0, 0.0]);
        c
    }

    /// Gathering the outdoor squares into one collider takes their
    /// triangles and buckets over as they are: the same collider as putting
    /// every triangle in again.
    #[test]
    fn gathered_squares_are_as_if_put_in_one_at_a_time() {
        let squares: Vec<Collider> = (0..5)
            .map(|i| square(i + 1, [4096.0 * (i % 3) as f32, 4096.0 * (i / 3) as f32]))
            .collect();
        let mut fast = Collider::new();
        let mut slow = Collider::new();
        for s in &squares {
            fast.extend(s);
            extend_one_at_a_time(&mut slow, s);
        }
        assert!(slow.triangles.len() > 500);
        assert_eq!(fast.vertices.len(), slow.vertices.len());
        for (a, b) in fast.vertices.iter().zip(&slow.vertices) {
            assert!(a == b || (a[0].is_nan() && b[0].is_nan()));
        }
        assert_eq!(fast.triangles, slow.triangles);
        assert_eq!(fast.grid, slow.grid);
        assert_eq!(fast.live.owned, slow.live.owned);
        assert_eq!(fast.live.placed, slow.live.placed);
        assert_eq!(fast.live.surfaces, slow.live.surfaces);
        assert_eq!(fast.live.hidden, slow.live.hidden);
        assert_eq!(fast.live.rest.len(), slow.live.rest.len());
        // And gathered again from the gathered one (moved owners' buckets
        // aren't in triangle order there).
        let mut twice = Collider::new();
        let mut twice_slow = Collider::new();
        let turn = [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        fast.move_owner(0x103, &turn, [10.0, 20.0, 0.0]);
        twice.extend(&fast);
        extend_one_at_a_time(&mut twice_slow, &fast);
        assert_eq!(twice.triangles, twice_slow.triangles);
        assert_eq!(twice.grid, twice_slow.grid);
        assert_eq!(twice.live.owned, twice_slow.live.owned);
        assert_eq!(twice.live.placed, twice_slow.live.placed);
    }

    /// The shell the game's models have around their collision.
    const SHELL: f32 = 0.1 * HAVOK_UNIT;

    /// A room: floor at z = 0 from -500 to 500, a wall at x = 200, a 16-unit
    /// step at x from -200 to -100, and a 64-unit block at y = 200..300;
    /// every surface with a model's shell.
    fn room() -> Collider {
        let mut c = Collider::new();
        let quad =
            |c: &mut Collider, v: [Vec3; 4]| c.add_solid(&v, &[[0, 1, 2], [0, 2, 3]], SHELL, 0);
        quad(
            &mut c,
            [
                [-500.0, -500.0, 0.0],
                [500.0, -500.0, 0.0],
                [500.0, 500.0, 0.0],
                [-500.0, 500.0, 0.0],
            ],
        );
        quad(
            &mut c,
            [
                [200.0, -500.0, 0.0],
                [200.0, 500.0, 0.0],
                [200.0, 500.0, 300.0],
                [200.0, -500.0, 300.0],
            ],
        );
        // The step: top at z = 16 and its front face at x = -100.
        quad(
            &mut c,
            [
                [-200.0, -500.0, 16.0],
                [-100.0, -500.0, 16.0],
                [-100.0, 500.0, 16.0],
                [-200.0, 500.0, 16.0],
            ],
        );
        quad(
            &mut c,
            [
                [-100.0, -500.0, 0.0],
                [-100.0, 500.0, 0.0],
                [-100.0, 500.0, 16.0],
                [-100.0, -500.0, 16.0],
            ],
        );
        // The block's front face at y = 200, 64 high.
        quad(
            &mut c,
            [
                [-50.0, 200.0, 0.0],
                [50.0, 200.0, 0.0],
                [50.0, 200.0, 64.0],
                [-50.0, 200.0, 64.0],
            ],
        );
        quad(
            &mut c,
            [
                [-50.0, 200.0, 64.0],
                [50.0, 200.0, 64.0],
                [50.0, 300.0, 64.0],
                [-50.0, 300.0, 64.0],
            ],
        );
        // A bench: 32 high (a third over a step), front face at y = -200.
        quad(
            &mut c,
            [
                [-50.0, -200.0, 0.0],
                [50.0, -200.0, 0.0],
                [50.0, -200.0, 32.0],
                [-50.0, -200.0, 32.0],
            ],
        );
        quad(
            &mut c,
            [
                [-50.0, -200.0, 32.0],
                [50.0, -200.0, 32.0],
                [50.0, -300.0, 32.0],
                [-50.0, -300.0, 32.0],
            ],
        );
        c
    }

    #[test]
    fn does_not_climb_a_ledge_higher_than_a_step_bit_by_bit() {
        // The rounded bottom can rest on the bench's edge 22 units up (under
        // a step); from there the bench's top is only 10 higher. Steps count
        // from the point stood on, so it stays blocked.
        let c = room();
        let mut p = Character::new([0.0, -100.0, 0.0]);
        run(&c, &mut p, [0.0, -200.0], 2.0);
        let r = CharacterShape::PLAYER.radius;
        assert!(p.feet[2] < 1.0, "climbed: {:?}", p.feet);
        assert!(p.feet[1] > -200.0 + r - 1.0, "went through: {:?}", p.feet);
    }

    fn run(c: &Collider, player: &mut Character, velocity: [f32; 2], seconds: f32) {
        let steps = (seconds * 60.0) as usize;
        for _ in 0..steps {
            player.update(c, &CharacterShape::PLAYER, velocity, false, 1.0 / 60.0);
        }
    }

    #[test]
    fn a_flat_sliver_triangle_is_ignored() {
        // Game collision has triangles with no area: two corners the same
        // (the Prospector Saloon's, about 40, −581, 3516), or three in a
        // line. One near the capsule made its position NaN.
        let mut c = room();
        c.add(
            &[[0.0, 0.0, 10.0], [0.0, 0.0, 10.0], [1.9, -3.2, 10.7]],
            &[[0, 1, 2]],
        );
        c.add(
            &[[-10.0, 0.0, 10.0], [0.0, 0.0, 10.0], [10.0, 0.0, 10.0]],
            &[[0, 1, 2]],
        );
        let mut p = Character::new([0.0, 0.0, 0.0]);
        run(&c, &mut p, [0.0, 0.0], 0.5);
        assert!(p.feet.iter().all(|v| v.is_finite()), "{:?}", p.feet);
        run(&c, &mut p, [50.0, 0.0], 0.5);
        assert!(p.feet.iter().all(|v| v.is_finite()), "{:?}", p.feet);
    }

    #[test]
    fn falls_onto_the_floor_and_stands_there() {
        let c = room();
        let mut p = Character::new([0.0, 0.0, 100.0]);
        run(&c, &mut p, [0.0, 0.0], 2.0);
        assert!(p.on_ground);
        assert!(p.feet[2].abs() < 0.5, "{:?}", p.feet);
    }

    #[test]
    fn stops_at_a_wall_its_radius_and_the_shell_away() {
        let c = room();
        let mut p = Character::new([0.0, 0.0, 0.0]);
        run(&c, &mut p, [300.0, 0.0], 2.0);
        let r = CharacterShape::PLAYER.radius;
        assert!(
            (p.feet[0] - (200.0 - r - SHELL)).abs() < 0.1,
            "{:?}",
            p.feet
        );
        // Standing on the floor's shell, floating its own lift above the
        // feet: the feet are at the floor.
        assert!(p.feet[2].abs() < 0.05, "{:?}", p.feet);
    }

    #[test]
    fn triangles_without_area_are_left_out() {
        // Corners in a line (the saloon's collision has some): the closest
        // point test divides by the area, which made the walk NaN.
        let mut c = room();
        let before = c.triangle_count();
        c.add(
            &[[-50.0, 0.0, 10.0], [0.0, 0.0, 10.0], [50.0, 0.0, 10.0]],
            &[[0, 1, 2]],
        );
        assert_eq!(c.triangle_count(), before);
        let mut p = Character::new([0.0, -60.0, 0.0]);
        run(&c, &mut p, [0.0, 100.0], 1.0);
        assert!(p.feet.iter().all(|v| v.is_finite()), "{:?}", p.feet);
        assert!((p.feet[1] - 40.0).abs() < 1.0, "{:?}", p.feet);
    }

    #[test]
    fn switched_off_objects_dont_block() {
        // A door's leaf across the way, owned by the door (form 0x123).
        let mut c = room();
        c.add_solid(
            &[
                [-100.0, 100.0, 0.0],
                [100.0, 100.0, 0.0],
                [100.0, 100.0, 200.0],
                [-100.0, 100.0, 200.0],
            ],
            &[[0, 1, 2], [0, 2, 3]],
            SHELL,
            0x123,
        );
        assert!(c.owns(0x123) && !c.owns(0x124));
        let mut p = Character::new([0.0, 0.0, 0.0]);
        run(&c, &mut p, [0.0, 100.0], 1.0);
        assert!(p.feet[1] < 100.0 - 20.0, "closed: {:?}", p.feet);
        // Open: walked through, and rays pass.
        c.set_hidden(0x123, true);
        assert!(c
            .raycast([0.0, 0.0, 50.0], [0.0, 1.0, 0.0], 150.0)
            .is_none());
        // Still found by a ray that looks for it (to close it again).
        let (_, t) = c
            .raycast_including_hidden([0.0, 0.0, 50.0], [0.0, 1.0, 0.0], 150.0)
            .unwrap();
        assert_eq!(c.owner(t), 0x123);
        let mut p = Character::new([0.0, 0.0, 0.0]);
        run(&c, &mut p, [0.0, 100.0], 1.5);
        assert!(p.feet[1] > 120.0, "open: {:?}", p.feet);
        // Merged colliders keep both the owners and what's switched off.
        let mut merged = Collider::new();
        merged.extend(&c);
        assert!(merged.is_hidden(0x123) && merged.owns(0x123));
    }

    #[test]
    fn people_block_sideways_only() {
        let mut c = room();
        c.set_people(vec![Person {
            feet: [0.0, 100.0, 0.0],
            radius: 20.25,
            height: 128.0,
        }]);
        let mut p = Character::new([0.0, 0.0, 0.0]);
        run(&c, &mut p, [0.0, 200.0], 1.0);
        let r = CharacterShape::PLAYER.radius;
        assert!(
            (p.feet[1] - (100.0 - r - 20.25)).abs() < 0.5,
            "{:?}",
            p.feet
        );
        // Dropped on top of them, nobody stands on anybody: pushed off
        // sideways to the floor.
        let mut p = Character::new([0.0, 101.0, 140.0]);
        run(&c, &mut p, [0.0, 0.0], 2.0);
        assert!(p.on_ground && p.feet[2].abs() < 0.05, "{:?}", p.feet);
        let apart = ((p.feet[0]).powi(2) + (p.feet[1] - 100.0).powi(2)).sqrt();
        assert!(apart >= r + 20.25 - 0.5, "{apart}");
    }

    #[test]
    fn walks_up_a_low_step_but_not_a_high_block() {
        let c = room();
        let mut p = Character::new([0.0, 0.0, 0.0]);
        run(&c, &mut p, [-200.0, 0.0], 1.0);
        assert!(
            (p.feet[2] - 16.0).abs() < 0.5,
            "onto the step: {:?}",
            p.feet
        );
        assert!(p.feet[1].abs() < 0.5, "straight on: {:?}", p.feet);
        let mut p = Character::new([0.0, 100.0, 0.0]);
        run(&c, &mut p, [0.0, 200.0], 2.0);
        let r = CharacterShape::PLAYER.radius;
        assert!(
            p.feet[1] < 200.0 - r + 1.0 && p.feet[2] < 1.0,
            "blocked: {:?}",
            p.feet
        );
    }

    #[test]
    fn walks_straight_across_a_triangulated_floor() {
        // The floor quad's diagonal runs through the start: its edge
        // mustn't push the capsule sideways.
        let c = room();
        let mut p = Character::new([0.0, 0.0, 0.0]);
        run(&c, &mut p, [-60.0, 0.0], 1.0);
        assert!(
            p.feet[1].abs() < 0.01 && (p.feet[0] + 60.0).abs() < 1.0,
            "{:?}",
            p.feet
        );
    }

    #[test]
    fn walks_off_a_ledge_and_falls() {
        let c = room();
        // On top of the 64-unit block, walking off its front edge.
        let mut p = Character::new([0.0, 250.0, 64.0]);
        run(&c, &mut p, [0.0, 0.0], 0.2);
        assert!(
            p.on_ground && (p.feet[2] - 64.0).abs() < 0.5,
            "{:?}",
            p.feet
        );
        p.fell = None;
        run(&c, &mut p, [0.0, -200.0], 1.5);
        assert!(p.on_ground && p.feet[2].abs() < 0.5, "{:?}", p.feet);
        // The fall is counted from where it stepped off.
        let fell = p.fell.expect("landed");
        assert!((fell - 64.0).abs() < 1.0, "{fell}");
    }

    #[test]
    fn jumps_and_comes_back_down() {
        let c = room();
        let mut p = Character::new([0.0, 0.0, 0.0]);
        run(&c, &mut p, [0.0, 0.0], 0.5);
        p.update_with_jump(
            &c,
            &CharacterShape::PLAYER,
            [0.0, 0.0],
            Some(500.0),
            1.0 / 60.0,
        );
        let mut highest: f32 = 0.0;
        for _ in 0..120 {
            p.update(&c, &CharacterShape::PLAYER, [0.0, 0.0], false, 1.0 / 60.0);
            highest = highest.max(p.feet[2]);
        }
        assert!(highest > 50.0, "{highest}");
        assert!(p.on_ground && p.feet[2].abs() < 0.5);
        // A jump on flat ground is no fall: measured from where it left
        // the ground, not from the top.
        assert!(p.fell.is_some_and(|f| f.abs() < 0.5), "{:?}", p.fell);
    }

    #[test]
    fn a_running_jump_starts_from_standing_and_steers_three_tenths_a_frame() {
        let c = room();
        let shape = CharacterShape::PLAYER;
        let dt = 1.0 / 60.0;
        let mut p = Character::new([-50.0, 0.0, 0.0]);
        // Settle, then run east at 308.
        for _ in 0..30 {
            p.update_controlled(&c, &shape, [0.0, 0.0], None, 0.3, dt);
        }
        p.update_controlled(&c, &shape, [308.0, 0.0], None, 0.3, dt);
        assert!(p.on_ground && (p.horizontal[0] - 308.0).abs() < 0.5);
        // The jump's update: the run-up is dropped.
        let x = p.feet[0];
        p.update_controlled(&c, &shape, [308.0, 0.0], Some(296.5), 0.3, dt);
        assert!(!p.on_ground);
        assert!((p.feet[0] - x).abs() < 1e-3, "{}", p.feet[0] - x);
        // Then 0.3 of the gap each update: 92.4, then 157.1.
        p.update_controlled(&c, &shape, [308.0, 0.0], None, 0.3, dt);
        assert!((p.horizontal[0] - 92.4).abs() < 0.1, "{:?}", p.horizontal);
        p.update_controlled(&c, &shape, [308.0, 0.0], None, 0.3, dt);
        assert!((p.horizontal[0] - 157.08).abs() < 0.1, "{:?}", p.horizontal);
        // Letting go in the air slows it the same way, not at once.
        p.update_controlled(&c, &shape, [0.0, 0.0], None, 0.3, dt);
        assert!((p.horizontal[0] - 109.96).abs() < 0.1, "{:?}", p.horizontal);
        assert_eq!(
            blend_velocity([0.0, 0.0], [10000.0, 0.0], 1.0, 500.0),
            [500.0, 0.0]
        );
    }

    #[test]
    fn the_game_jump_rises_its_height() {
        // `fJumpHeightMin` 64: launched at √(2 g h), about 296.5 a second.
        let speed = (2.0 * GRAVITY * 64.0).sqrt();
        assert!((speed - 296.5).abs() < 0.2, "{speed}");
        let c = room();
        let mut p = Character::new([0.0, 0.0, 0.0]);
        run(&c, &mut p, [0.0, 0.0], 0.5);
        p.update_with_jump(&c, &CharacterShape::PLAYER, [0.0, 0.0], Some(speed), 1e-4);
        let mut highest: f32 = 0.0;
        for _ in 0..1200 {
            p.update(&c, &CharacterShape::PLAYER, [0.0, 0.0], false, 1.0 / 1200.0);
            highest = highest.max(p.feet[2]);
        }
        assert!((highest - 64.0).abs() < 1.0, "{highest}");
    }

    #[test]
    fn merged_colliders_keep_every_surface() {
        let mut merged = Collider::new();
        merged.extend(&room());
        let mut far = Collider::new();
        far.add(
            &[[5000.0, 0.0, 0.0], [5100.0, 0.0, 0.0], [5000.0, 100.0, 0.0]],
            &[[0, 1, 2]],
        );
        merged.extend(&far);
        assert_eq!(merged.triangle_count(), room().triangle_count() + 1);
        let down = [0.0, 0.0, -1.0];
        assert!(merged.raycast([5010.0, 10.0, 50.0], down, 100.0).is_some());
        assert!(merged.raycast([0.0, 0.0, 50.0], down, 100.0).is_some());
    }

    #[test]
    fn rays_hit_the_nearest_surface() {
        let c = room();
        let (d, _) = c
            .raycast([0.0, 0.0, 50.0], [1.0, 0.0, 0.0], 1000.0)
            .unwrap();
        assert!((d - 200.0).abs() < 1e-3);
        assert!(c
            .raycast([0.0, 0.0, 50.0], [1.0, 0.0, 0.0], 100.0)
            .is_none());
    }

    #[test]
    fn an_owned_object_moves_from_its_added_place() {
        // A door's leaf: a wall across y = 100 belonging to 0x904, and a
        // wall of nobody's at x = 300.
        let mut c = Collider::new();
        let quad = |c: &mut Collider, v: [[f32; 3]; 4], owner: u32| {
            c.add_solid(&v, &[[0, 1, 2], [0, 2, 3]], 0.0, owner)
        };
        quad(
            &mut c,
            [
                [-40.0, 100.0, 0.0],
                [40.0, 100.0, 0.0],
                [40.0, 100.0, 200.0],
                [-40.0, 100.0, 200.0],
            ],
            0x904,
        );
        quad(
            &mut c,
            [
                [300.0, -40.0, 0.0],
                [300.0, 40.0, 0.0],
                [300.0, 40.0, 200.0],
                [300.0, -40.0, 200.0],
            ],
            0,
        );
        let north = |c: &Collider| c.raycast([0.0, 0.0, 100.0], [0.0, 1.0, 0.0], 500.0);
        let east = |c: &Collider| c.raycast([0.0, 0.0, 100.0], [1.0, 0.0, 0.0], 500.0);
        assert!(north(&c).is_some() && east(&c).is_some());
        // Turned a quarter turn about the origin (x → y) and moved 1000
        // north: it's off the ray and bucketed where it now is.
        let turn = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        c.move_owner(0x904, &turn, [0.0, 1000.0, 0.0]);
        assert!(north(&c).is_none());
        let there = c.raycast([-200.0, 1000.0, 100.0], [1.0, 0.0, 0.0], 500.0);
        assert!(
            there.is_some_and(|(d, _)| (d - 100.0).abs() < 1e-3),
            "{there:?}"
        );
        // From its added place, not from where it was: back to the start.
        c.move_owner(
            0x904,
            &[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [0.0; 3],
        );
        assert!(north(&c).is_some_and(|(d, _)| (d - 100.0).abs() < 1e-3));
        // Nobody's wall never moved; an unknown owner does nothing.
        assert!(east(&c).is_some_and(|(d, _)| (d - 300.0).abs() < 1e-3));
        c.move_owner(0x999, &turn, [0.0; 3]);
        c.move_owner(0, &turn, [0.0; 3]);
        assert!(east(&c).is_some());
        // Gathered into another collider, the object still moves there.
        let mut all = Collider::new();
        all.extend(&c);
        all.move_owner(0x904, &turn, [0.0, 1000.0, 0.0]);
        assert!(north(&all).is_none());
    }

    /// The box test only skips triangles the exact test finds out of reach.
    #[test]
    fn surely_beyond_never_skips_a_triangle_within_reach() {
        let mut seed = 12345u64;
        let mut unit = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 33) as f32) / (1u64 << 31) as f32 * 2.0 - 1.0
        };
        let mut skipped = 0;
        for _ in 0..200_000 {
            let mut p = || [unit() * 120.0, unit() * 120.0, unit() * 120.0];
            let (a, b, c) = (p(), p(), p());
            let bottom = [unit() * 60.0, unit() * 60.0, unit() * 60.0];
            let top = add(bottom, [0.0, 0.0, unit().abs() * 80.0]);
            let reach = 20.0 + unit().abs() * 20.0;
            if surely_beyond([a, b, c], bottom, top, reach) {
                skipped += 1;
                let (on_axis, on_triangle) = segment_triangle_closest(bottom, top, a, b, c);
                let d = length(sub(on_axis, on_triangle));
                assert!(
                    d.is_nan() || d >= reach,
                    "{a:?} {b:?} {c:?} {bottom:?} {top:?} {reach} {d}"
                );
            }
        }
        assert!(skipped > 10_000);
    }

    /// A seeded number from -1 to 1 (the tests' own, no library).
    fn lcg(seed: &mut u64) -> f32 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((*seed >> 33) as f32) / (1u64 << 31) as f32 * 2.0 - 1.0
    }

    /// Where the outdoor numbers are largest (Goodsprings, about -68000,
    /// 5800, 8480; f32 steps of 1/128 there) and just past the margin, the
    /// box test still only skips what both exact tests (the touch's and
    /// the push's, the stricter) leave out.
    #[test]
    fn surely_beyond_never_skips_a_triangle_within_reach_outdoors() {
        let mut seed = 777u64;
        let origin = [-68250.0, 5800.0, 8480.0];
        let reach = CharacterShape::PLAYER.radius + 0.7;
        let mut skipped = 0;
        for i in 0..200_000 {
            let mut u = || lcg(&mut seed);
            let bottom = add(origin, [u() * 40.0, u() * 40.0, u() * 40.0]);
            let top = add(bottom, [0.0, 0.0, 87.0]);
            // A triangle whose box starts just beyond reach along one axis.
            let k = i % 3;
            let gap = reach + 0.05 + u().abs() * 0.02;
            let mut corner = |_| {
                let mut p = add(bottom, [u() * 60.0, u() * 60.0, u() * 60.0]);
                let start = if k == 2 { top[k] } else { bottom[k] };
                p[k] = start + gap + u().abs() * 30.0;
                p
            };
            let (a, b, c) = (corner(0), corner(1), corner(2));
            if surely_beyond([a, b, c], bottom, top, reach) {
                skipped += 1;
                let (on_axis, on_triangle) = segment_triangle_closest(bottom, top, a, b, c);
                let d = length(sub(on_axis, on_triangle));
                assert!(
                    d.is_nan() || d >= reach - 1e-4,
                    "{a:?} {b:?} {c:?} {bottom:?} {top:?} {reach} {d}"
                );
            }
        }
        assert!(skipped > 100_000);
    }

    /// The downward sweep with its candidates found once gives what
    /// finding them again at every step (the way before) gives: the grid
    /// is across x and y, which the sweep keeps.
    #[test]
    fn one_candidate_list_sweeps_as_one_per_step() {
        let mut seed = 4242u64;
        let origin = [-68250.0, 5800.0, 8480.0];
        let shape = CharacterShape::PLAYER;
        for _ in 0..40 {
            let mut c = Collider::new();
            let mut vertices = Vec::new();
            let mut triangles = Vec::new();
            for _ in 0..300 {
                let mut u = || lcg(&mut seed);
                let middle = add(origin, [u() * 400.0, u() * 400.0, u() * 150.0]);
                let base = vertices.len() as u32;
                for _ in 0..3 {
                    let mut u = || lcg(&mut seed);
                    vertices.push(add(middle, [u() * 60.0, u() * 60.0, u() * 20.0]));
                }
                triangles.push([base, base + 1, base + 2]);
            }
            c.add(&vertices, &triangles);
            for _ in 0..50 {
                let mut u = || lcg(&mut seed);
                let feet = add(origin, [u() * 300.0, u() * 300.0, 200.0 + u() * 100.0]);
                let distance = 50.0 + u().abs() * 400.0;
                let walker = Character::new(feet);
                let found = walker.swept_down(&c, &shape, distance);
                // The way before: the triangles near looked up at each step.
                let steps = (distance / 0.5).ceil().max(1.0) as usize;
                let piece = distance / steps as f32;
                let mut at = feet;
                let mut expected = (at, None);
                for _ in 0..steps {
                    let next = [at[0], at[1], at[2] - piece];
                    let there = Character {
                        feet: next,
                        ..walker
                    };
                    let near = there.touch_candidates(&c, &shape);
                    if let Some(s) = there.touching_among(&c, &shape, &near) {
                        expected = (at, Some(s));
                        break;
                    }
                    at = next;
                    expected = (at, None);
                }
                assert_eq!(found, expected, "{feet:?} {distance}");
            }
        }
    }

    /// Moving an owner leaves every bucket's list as taking its triangles
    /// out one at a time and putting them back does (the order queries see
    /// them in).
    #[test]
    fn moving_an_owner_leaves_the_buckets_as_one_at_a_time() {
        let mut c = Collider::new();
        // Ground under everything, and an object of many triangles on it.
        let mut ground = Vec::new();
        let mut tris = Vec::new();
        for i in 0..20 {
            let x = i as f32 * 50.0 - 500.0;
            let b = ground.len() as u32;
            ground.extend([[x, -500.0, 0.0], [x + 60.0, -500.0, 0.0], [x, 500.0, 0.0]]);
            tris.push([b, b + 1, b + 2]);
        }
        c.add(&ground, &tris);
        let mut body = Vec::new();
        let mut body_tris = Vec::new();
        for i in 0..12 {
            let a = i as f32 * 0.5;
            let b = body.len() as u32;
            body.extend([
                [a.cos() * 40.0, a.sin() * 40.0, 10.0],
                [a.cos() * 90.0, a.sin() * 90.0, 30.0],
                [0.0, 0.0, 60.0],
            ]);
            body_tris.push([b, b + 1, b + 2]);
        }
        c.add_solid_surface(&body, &body_tris, (0.0, 0x77, NO_MATERIAL), None);
        let one_at_a_time = |c: &mut Collider, r: &[[f32; 3]; 3], t: Vec3| {
            let owned = c.live.owned[&0x77].clone();
            for &tri in &owned {
                let [a, b, cc] = c.triangle(tri);
                for key in bucket_keys(a, b, cc) {
                    if let Some(list) = c.grid.get_mut(&key) {
                        list.retain(|&i| i != tri);
                    }
                }
            }
            for &(i, p) in &c.live.rest[&0x77].clone() {
                let q = [0, 1, 2].map(|k| r[k][0] * p[0] + r[k][1] * p[1] + r[k][2] * p[2]);
                c.vertices[i as usize] = add(q, t);
            }
            for &tri in &owned {
                let [a, b, cc] = c.triangle(tri);
                for key in bucket_keys(a, b, cc) {
                    c.grid.entry(key).or_default().push(tri);
                }
            }
        };
        let mut expected = c.clone();
        let turn = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        for (r, t) in [(turn, [300.0, 20.0, 0.0]), (turn, [-280.0, 500.0, 5.0])] {
            c.move_owner(0x77, &r, t);
            one_at_a_time(&mut expected, &r, t);
            assert_eq!(c.grid, expected.grid);
            assert_eq!(c.vertices, expected.vertices);
        }
    }

    #[test]
    fn a_swept_sphere_stops_its_radius_and_the_shell_short_of_a_wall() {
        let c = room();
        let hit = c
            .spherecast([0.0, 0.0, 50.0], [1.0, 0.0, 0.0], 1000.0, 10.0)
            .unwrap();
        // The wall at x = 200 stands SHELL out of its triangles: the
        // sphere's centre stops 10 + SHELL before it, touching the wall
        // straight ahead.
        assert!(
            (hit.distance - (200.0 - 10.0 - SHELL)).abs() < 1e-3,
            "{hit:?}"
        );
        assert!((hit.point[0] - (200.0 - SHELL)).abs() < 1e-3, "{hit:?}");
        assert!(hit.point[1].abs() < 1e-3 && (hit.point[2] - 50.0).abs() < 1e-3);
        // Out of reach: nothing.
        assert!(c
            .spherecast([0.0, 0.0, 50.0], [1.0, 0.0, 0.0], 150.0, 10.0)
            .is_none());
        // Already touching: distance 0 and the nearest point.
        let touching = c
            .spherecast([195.0, 0.0, 50.0], [1.0, 0.0, 0.0], 100.0, 10.0)
            .unwrap();
        assert_eq!(touching.distance, 0.0);
        assert!((touching.point[0] - (200.0 - SHELL)).abs() < 1e-3);
    }

    #[test]
    fn a_swept_sphere_catches_a_walls_edge_a_ray_would_miss() {
        let c = room();
        // Past the wall's end (y = 500): a ray misses, but a sphere 5 units
        // beyond the edge touches it.
        assert!(c
            .raycast([0.0, 505.0, 50.0], [1.0, 0.0, 0.0], 1000.0)
            .is_none());
        let hit = c
            .spherecast([0.0, 505.0, 50.0], [1.0, 0.0, 0.0], 1000.0, 10.0)
            .unwrap();
        let r = 10.0 + SHELL;
        let expected = 200.0 - (r * r - 5.0f32 * 5.0).sqrt();
        assert!((hit.distance - expected).abs() < 1e-2, "{hit:?}");
        // Touched on the edge's shell, SHELL out from the edge toward the
        // sphere's centre.
        let edge = [200.0, 500.0, 50.0];
        let off = sub(hit.point, edge);
        assert!((length(off) - SHELL).abs() < 1e-3, "{hit:?}");
        assert!(off[0] < 0.0 && off[1] > 0.0, "{hit:?}");
        // Far enough past it: nothing.
        assert!(c
            .spherecast([0.0, 512.0, 50.0], [1.0, 0.0, 0.0], 1000.0, 10.0)
            .is_none());
    }

    #[test]
    fn a_swept_sphere_passes_switched_off_objects() {
        let mut c = room();
        c.add_solid(
            &[
                [-100.0, 100.0, 0.0],
                [100.0, 100.0, 0.0],
                [100.0, 100.0, 200.0],
                [-100.0, 100.0, 200.0],
            ],
            &[[0, 1, 2], [0, 2, 3]],
            SHELL,
            0x123,
        );
        let hit = c
            .spherecast([0.0, 0.0, 50.0], [0.0, 1.0, 0.0], 400.0, 10.0)
            .unwrap();
        assert_eq!(c.owner(hit.triangle), 0x123);
        c.set_hidden(0x123, true);
        assert!(c
            .spherecast([0.0, 0.0, 50.0], [0.0, 1.0, 0.0], 150.0, 10.0)
            .is_none());
    }

    #[test]
    fn shots_pass_a_transparent_fence_that_stops_walkers() {
        // A chain-link fence on layer 3 (TRANSPARENT) 100 units north, a
        // wall on layer 1 (STATIC) behind it.
        let mut c = Collider::new();
        let plane = |y: f32| {
            [
                [-100.0, y, 0.0],
                [100.0, y, 0.0],
                [100.0, y, 200.0],
                [-100.0, y, 200.0],
            ]
        };
        let quad = [[0, 1, 2], [0, 2, 3]];
        c.add_layered(&plane(100.0), &quad, (0.0, 0x31, NO_MATERIAL), None, 3);
        c.add_layered(&plane(200.0), &quad, (0.0, 0x32, NO_MATERIAL), None, 1);
        let north = [0.0, 1.0, 0.0];
        // Unfiltered (and for the character, layer 30) the fence is met.
        let (d, t) = c.raycast([0.0, 0.0, 50.0], north, 500.0).unwrap();
        assert!((d - 100.0).abs() < 1e-3 && c.owner(t) == 0x31);
        let (_, t) = c.raycast_layer([0.0, 0.0, 50.0], north, 500.0, 30).unwrap();
        assert_eq!(c.owner(t), 0x31);
        // A projectile's cast (layer 6) goes through to the wall.
        let (d, t) = c.raycast_layer([0.0, 0.0, 50.0], north, 500.0, 6).unwrap();
        assert!((d - 200.0).abs() < 1e-3 && c.owner(t) == 0x32, "{d}");
        assert_eq!(c.layer(t), 1);
        // Triangles added without a layer stop every cast.
        let mut plain = Collider::new();
        plain.add(&plane(100.0), &quad);
        assert!(plain
            .raycast_layer([0.0, 0.0, 50.0], north, 500.0, 6)
            .is_some());
    }
}
