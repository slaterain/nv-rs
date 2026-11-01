//! The long way: the navmesh info map (`NAVI`) and the high-level route
//! over it, as the game plans a path whose goal lies on a navmesh that isn't
//! loaded (`Pathfind.cpp`, `NavMeshInfoMap.cpp`; FalloutNV.exe 1.4.0.525).
//!
//! The game keeps one `NavMeshInfo` (Xbox PDB) per navmesh in memory at all
//! times, read from the `NAVI` record (`NavMeshInfoMap::Load` (Xbox PDB),
//! `006b5d80`): `NVMI` (`006b4b50`) gives the navmesh, its place (a
//! worldspace and grid square, or an interior cell), a rough position and a
//! "preferred" factor; `NVCI` (`006b51a0`) the navmeshes it joins (two
//! lists) and its doors. A path request between two navmeshes first
//! searches this map (`NavMeshInfoSearch`, an `AStarSearch` (Xbox PDB):
//! `006b8c50`, `006b9180`, edge costs `006b8490`) and turns the route into
//! "virtual" path nodes: the start, each navmesh passed (at its rough
//! position), the goal (`006c94c0`). Only the run of nodes from the
//! actor's on whose cells are attached gets a detailed navmesh path
//! (`006c9fc0`, `006ca0e0`); someone out of sight (not the high process)
//! walks the virtual nodes point to point (`VirtualActorPathHandler`
//! (Xbox PDB), `009ea8a0`).
//!
//! Not done (labelled where it matters): the "island" data of flag 0x20
//! entries; same-space `virtual_path` searches still omit door edges; and what a
//! failed search falls back to (`006c8f50` → `006c9b20`): here a failed
//! search gives no path.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use esm::{FormId, FourCC, LoadOrder};

use super::NavMesh;
use crate::cell::{le_f32, le_u32};
use crate::movement::PolyWalk;

const NAVI: FourCC = FourCC::new(b"NAVI");
const NVER: FourCC = FourCC::new(b"NVER");
const NVMI: FourCC = FourCC::new(b"NVMI");
const NVCI: FourCC = FourCC::new(b"NVCI");
const XNVP: FourCC = FourCC::new(b"XNVP");
const XNDP: FourCC = FourCC::new(b"XNDP");
const FNAM: FourCC = FourCC::new(b"FNAM");

/// A load-door connection. `near` and `far` are in different spaces: the
/// linked door's teleport destination and this door's teleport destination.
#[derive(Debug, Clone, Copy)]
struct DoorEdge {
    door: FormId,
    to: usize,
    near: [f32; 3],
    far: [f32; 3],
    minimal_use: bool,
    /// `0057b4b0`: a door reference with form flag 0x40.
    blocked_for_penalty: bool,
}

/// How the search treats a door the actor cannot unlock (`006b8490`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorDisposition {
    /// Exclude it (the default path request).
    RequireAccess,
    /// Use it without a lock penalty (`bIgnoreLocks`).
    IgnoreLocks,
    /// Use it with the door penalty.
    Penalize,
}

/// The changing reference and lock state supplied to the search.
#[derive(Debug, Clone, Copy)]
pub struct DoorAccess {
    pub disabled: bool,
    pub can_unlock: bool,
}

/// One navmesh's entry in the map (`NavMeshInfo` (Xbox PDB), 0x5c bytes).
#[derive(Debug, Clone, PartialEq)]
pub struct NavInfo {
    /// The navmesh (`NAVM`), +0.
    pub navmesh: FormId,
    /// Its place (`NVMI` +8): a worldspace, or an interior cell. +4.
    pub space: FormId,
    /// `NVMI` flags, +8 (0x20: island data follows the position).
    pub flags: u32,
    /// Its grid square (two i16), +0xc; outdoors only.
    pub square: Option<(i32, i32)>,
    /// Its rough position, +0x10: the virtual path node for it.
    pub position: [f32; 3],
    /// The preferred factor (`NVMI`'s last float, NVER > 9), +0x20: edges
    /// leaving it cost × clamp(1 − 100 × this, 0.1, 1) (`006b8490`).
    pub preferred: f32,
    /// `NVCI`'s first list, +0x24: navmeshes it joins; crossing to one
    /// costs three times the distance (`006b8490`).
    pub links: Vec<FormId>,
    /// `NVCI`'s second list (NVER > 10), +0x34: crossing costs the
    /// distance.
    pub cheap_links: Vec<FormId>,
    /// `NVCI`'s third list, +0x44: doors connecting this info to another.
    pub doors: Vec<FormId>,
}

/// Every navmesh's entry, by navmesh and by place.
#[derive(Debug, Clone, Default)]
pub struct NavInfoMap {
    pub infos: Vec<NavInfo>,
    by_navmesh: HashMap<FormId, usize>,
    /// By interior cell (`None`) or worldspace and square.
    by_place: HashMap<Place, Vec<usize>>,
    /// `NVCI`'s third list, resolved through the linked door's navmesh
    /// portal (`XNVP`/`XNDP`) to the other info (`006b8490`).
    door_edges: Vec<Vec<DoorEdge>>,
}

/// An interior cell (no square), or a worldspace and grid square.
type Place = (FormId, Option<(i32, i32)>);

/// An `NVMI` entry (`006b4b50`): flags, navmesh, place, grid (i16, i16),
/// position; then, with flag 0x20, island data; then (NVER > 9) the
/// preferred factor.
// Translated from 006b4b50 (decompiled, FalloutNV.exe 1.4.0.525).
fn read_nvmi(
    data: &[u8],
    version: u32,
    global: &dyn Fn(u32) -> FormId,
    exterior: &dyn Fn(FormId) -> bool,
) -> Option<NavInfo> {
    if data.len() < 28 {
        return None;
    }
    let flags = le_u32(data, 0);
    let navmesh = global(le_u32(data, 4));
    let space = global(le_u32(data, 8));
    // The grid word holds y, then x: WastelandNV's navmesh 001639F7 (cell
    // 000DEBB6, `XCLC` 1, −17; vertices near x 4096..8192, y −69632)
    // reads −17, 1.
    let gy = i16::from_le_bytes([data[12], data[13]]);
    let gx = i16::from_le_bytes([data[14], data[15]]);
    let position = [le_f32(data, 16), le_f32(data, 20), le_f32(data, 24)];
    // The island data's length is the loader's (`0069c8f0`); the factor is
    // the entry's last float.
    let preferred = if version > 9 && data.len() >= 32 {
        le_f32(data, data.len() - 4)
    } else {
        0.0
    };
    Some(NavInfo {
        navmesh,
        space,
        flags,
        square: exterior(space).then_some((i32::from(gx), i32::from(gy))),
        position,
        preferred,
        links: Vec::new(),
        cheap_links: Vec::new(),
        doors: Vec::new(),
    })
}

/// An `NVCI` entry's three lists (`006b51a0`), after its navmesh: each a
/// count and form IDs; the second only for NVER > 10.
// Translated from 006b51a0 (decompiled, FalloutNV.exe 1.4.0.525).
#[allow(clippy::type_complexity)]
fn read_nvci(
    data: &[u8],
    version: u32,
    global: &dyn Fn(u32) -> FormId,
) -> Option<(FormId, Vec<FormId>, Vec<FormId>, Vec<FormId>)> {
    let mut at = 0usize;
    let word = |at: &mut usize| -> Option<u32> {
        let v = data.get(*at..*at + 4)?;
        *at += 4;
        Some(u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
    };
    let navmesh = global(word(&mut at)?);
    let list = |at: &mut usize| -> Option<Vec<FormId>> {
        let n = word(at)?;
        (0..n).map(|_| word(at).map(global)).collect()
    };
    let links = list(&mut at)?;
    let cheap = if version > 10 {
        list(&mut at)?
    } else {
        Vec::new()
    };
    let doors = list(&mut at)?;
    Some((navmesh, links, cheap, doors))
}

impl NavInfoMap {
    /// Every `NAVI` record's entries, every plugin's version of it in load
    /// order: a plugin's version holds only its own navmeshes' entries
    /// (the DLCs override `FalloutNV.esm`'s 00014B92 with a handful), and
    /// the loader adds each version's entries to the one map (`006b5d80`
    /// runs per record read); a later entry for a navmesh replaces an
    /// earlier one.
    pub fn load(order: &LoadOrder) -> NavInfoMap {
        let mut map = NavInfoMap::default();
        let ids: Vec<FormId> = order.records_of_type(NAVI).map(|rr| rr.form_id).collect();
        for rr in ids.into_iter().flat_map(|id| order.versions(id)) {
            if rr.entry.header.is_deleted() {
                continue;
            }
            let Ok(record) = rr.record() else { continue };
            let global = |raw: u32| rr.plugin.to_global(FormId(raw));
            let exterior = |space: FormId| {
                order
                    .get(space)
                    .is_some_and(|r| r.entry.header.kind == esm::sig::WRLD)
            };
            // `NVER` (the loader's default 1 when absent).
            let version = record.get(NVER).map_or(1, |s| le_u32(&s.data, 0));
            for sub in &record.subrecords {
                if sub.kind == NVMI {
                    if let Some(info) = read_nvmi(&sub.data, version, &global, &exterior) {
                        map.insert(info);
                    }
                } else if sub.kind == NVCI && version > 4 {
                    if let Some((navmesh, links, cheap, doors)) =
                        read_nvci(&sub.data, version, &global)
                    {
                        if let Some(&i) = map.by_navmesh.get(&navmesh) {
                            let info = &mut map.infos[i];
                            info.links = links;
                            info.cheap_links = cheap;
                            info.doors = doors;
                        }
                    }
                }
            }
        }
        map.resolve_door_edges(order);
        map
    }

    fn resolve_door_edges(&mut self, order: &LoadOrder) {
        self.door_edges = vec![Vec::new(); self.infos.len()];
        for (from, info) in self.infos.iter().enumerate() {
            for &door in &info.doors {
                let Some(rr) = order.get(door) else { continue };
                let Some(teleport) = crate::placement::teleport_of(&rr) else {
                    continue;
                };
                let Some(linked) = order.get(teleport.door) else {
                    continue;
                };
                let Some(back) = crate::placement::teleport_of(&linked) else {
                    continue;
                };
                let Ok(record) = linked.record_shared() else {
                    continue;
                };
                // `0069dfd0` loads the navmesh form from the portal's first
                // word; `006b8490` looks up that navmesh's info.
                let Some(portal) = record.get(XNVP).or_else(|| record.get(XNDP)) else {
                    continue;
                };
                if portal.data.len() < 8 {
                    continue;
                }
                let navmesh = linked.plugin.to_global(FormId(le_u32(&portal.data, 0)));
                let Some(to) = self.of_navmesh(navmesh) else {
                    continue;
                };
                if self.infos[to].flags & 0x10 != 0 {
                    continue;
                }
                let base = crate::scripting::base_of(order, door).and_then(|base| order.get(base));
                let is_door = base
                    .as_ref()
                    .is_some_and(|base| base.entry.header.kind.as_bytes() == b"DOOR");
                let minimal_use = base
                    .and_then(|base| base.record().ok())
                    .and_then(|base| base.get(FNAM).and_then(|s| s.data.first().copied()))
                    .is_some_and(|flags| flags & crate::doors::MINIMAL_USE != 0);
                self.door_edges[from].push(DoorEdge {
                    door,
                    to,
                    near: back.position,
                    far: teleport.position,
                    minimal_use,
                    blocked_for_penalty: is_door && rr.entry.header.flags & 0x40 != 0,
                });
            }
        }
    }

    fn insert(&mut self, info: NavInfo) {
        let key = (info.space, info.square);
        if let Some(&i) = self.by_navmesh.get(&info.navmesh) {
            let old = (self.infos[i].space, self.infos[i].square);
            if let Some(list) = self.by_place.get_mut(&old) {
                list.retain(|&j| j != i);
            }
            self.infos[i] = info;
            self.by_place.entry(key).or_default().push(i);
            return;
        }
        let i = self.infos.len();
        self.by_navmesh.insert(info.navmesh, i);
        self.infos.push(info);
        self.by_place.entry(key).or_default().push(i);
    }

    /// The entry of a navmesh.
    pub fn of_navmesh(&self, navmesh: FormId) -> Option<usize> {
        self.by_navmesh.get(&navmesh).copied()
    }

    /// The entries of an interior cell (`square` `None`) or a worldspace's
    /// grid square.
    pub fn in_place(&self, space: FormId, square: Option<(i32, i32)>) -> &[usize] {
        self.by_place
            .get(&(space, square))
            .map_or(&[], Vec::as_slice)
    }

    /// Whether two entries are outdoors in the same worldspace
    /// (`00690800` gives an entry's worldspace, none indoors).
    fn same_world(&self, a: usize, b: usize) -> bool {
        let (a, b) = (&self.infos[a], &self.infos[b]);
        a.square.is_some() && b.square.is_some() && a.space == b.space
    }

    /// The route from navmesh `start` (the walker at `from`) to navmesh
    /// `goal` (the goal at `to`): the entries passed, start and goal
    /// included, and whether the goal was reached. A search that fails
    /// gives the route to the entry it got nearest to (least estimate;
    /// `006b8f40` called with "closest" set by `006b8c50`).
    ///
    /// A*: a node's cost so far `g`, its estimate `h` = 0.1 × the distance
    /// from its rough position to `to` when both ends are outdoors in one
    /// worldspace, else 1; the open list ordered by g + h (`006b9710`; its
    /// buckets, `006f4790`, put every node with h ≥ 0 in one sorted list;
    /// a new node goes before those with the same g + h). Leaving entry `n`
    /// (at `from` for the start) for a joined entry `m` (at `to` for the
    /// goal) costs the distance × k (× 3 for `links`), k = clamp(1 − 100 ×
    /// n's preferred factor, 0.1, 1); an entry reached more cheaply is
    /// taken up again. A neighbour improved through the node it was reached
    /// from ends the search as failed (`006b9180`'s parent test).
    // Translated from 006b8c50, 006b9180 and 006b8490 (decompiled, FalloutNV.exe 1.4.0.525).
    pub fn search(
        &self,
        start: usize,
        from: [f32; 3],
        goal: usize,
        to: [f32; 3],
    ) -> (bool, Vec<usize>) {
        let (found, route, _) = self.search_edges(start, from, goal, to, None);
        (found, route)
    }

    /// Search including the load-door edges. The returned edge list is
    /// aligned with the route; the first entry is `None` at start.
    // Translated from 006b8490 (decompiled, FalloutNV.exe 1.4.0.525).
    pub fn search_with_doors(
        &self,
        start: usize,
        from: [f32; 3],
        goal: usize,
        to: [f32; 3],
        disposition: DoorDisposition,
        access: impl Fn(FormId) -> DoorAccess,
    ) -> (bool, Vec<usize>, Vec<Option<FormId>>) {
        self.search_edges(start, from, goal, to, Some((disposition, &access)))
    }

    fn search_edges(
        &self,
        start: usize,
        from: [f32; 3],
        goal: usize,
        to: [f32; 3],
        doors: Option<(DoorDisposition, &dyn Fn(FormId) -> DoorAccess)>,
    ) -> (bool, Vec<usize>, Vec<Option<FormId>>) {
        #[derive(Clone, Copy)]
        struct Node {
            info: usize,
            g: f32,
            h: f32,
            parent: Option<usize>,
            parent_door: Option<FormId>,
            /// Open-list entry version (stale heap entries are skipped).
            version: u32,
            open: bool,
        }
        #[derive(PartialEq)]
        struct Entry {
            f: f32,
            seq: u64,
            node: usize,
            version: u32,
        }
        impl Eq for Entry {}
        impl Ord for Entry {
            fn cmp(&self, o: &Self) -> Ordering {
                // Least f first; among equals the newest (inserted before
                // them in the game's list).
                o.f.total_cmp(&self.f).then(self.seq.cmp(&o.seq))
            }
        }
        impl PartialOrd for Entry {
            fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
                Some(self.cmp(o))
            }
        }
        let same_world = self.same_world(start, goal);
        let estimate = |info: usize| {
            if same_world {
                distance(self.infos[info].position, to) * 0.1
            } else {
                1.0
            }
        };
        let place_of = |info: usize, is_from: bool| {
            if is_from && info == start {
                from
            } else if !is_from && info == goal {
                to
            } else {
                self.infos[info].position
            }
        };
        let mut nodes: Vec<Node> = Vec::new();
        let mut node_of: HashMap<usize, usize> = HashMap::new();
        let mut open = BinaryHeap::new();
        let mut seq = 0u64;
        let push = |open: &mut BinaryHeap<Entry>, nodes: &mut [Node], n: usize, seq: &mut u64| {
            nodes[n].version += 1;
            nodes[n].open = true;
            *seq += 1;
            open.push(Entry {
                f: nodes[n].g + nodes[n].h,
                seq: *seq,
                node: n,
                version: nodes[n].version,
            });
        };
        // The start node: cost 0, its estimate as above (`006b8ec0`).
        let h0 = if same_world {
            distance(self.infos[start].position, to) * 0.1
        } else {
            1.0
        };
        nodes.push(Node {
            info: start,
            g: 0.0,
            h: h0,
            parent: None,
            parent_door: None,
            version: 0,
            open: false,
        });
        node_of.insert(start, 0);
        push(&mut open, &mut nodes, 0, &mut seq);
        let pop = |open: &mut BinaryHeap<Entry>, nodes: &mut [Node]| loop {
            let e = open.pop()?;
            if nodes[e.node].open && nodes[e.node].version == e.version {
                nodes[e.node].open = false;
                return Some(e.node);
            }
        };
        let Some(mut current) = pop(&mut open, &mut nodes) else {
            return (false, Vec::new(), Vec::new());
        };
        let mut best = current;
        let found = loop {
            if nodes[current].info == goal {
                best = current;
                break true;
            }
            // The connections (`006b8490`): new nodes and cheaper ways.
            let here = nodes[current].info;
            let g = nodes[current].g;
            let k = (1.0 - self.infos[here].preferred * 100.0).clamp(0.1, 1.0);
            let a = place_of(here, true);
            let mut improved = Vec::new();
            let mut neighbors: Vec<(usize, f32, Option<FormId>, f32)> = self.infos[here]
                .links
                .iter()
                .filter_map(|&navmesh| self.of_navmesh(navmesh).map(|m| (m, 3.0)))
                .chain(
                    self.infos[here]
                        .cheap_links
                        .iter()
                        .filter_map(|&navmesh| self.of_navmesh(navmesh).map(|m| (m, 1.0))),
                )
                .map(|(m, mult)| {
                    (
                        m,
                        distance(a, place_of(m, false)) * k * mult + g,
                        None,
                        estimate(m),
                    )
                })
                .collect();
            if let Some((disposition, access)) = doors {
                for edge in self.door_edges.get(here).into_iter().flatten() {
                    let state = access(edge.door);
                    if state.disabled {
                        continue;
                    }
                    if disposition == DoorDisposition::Penalize && edge.blocked_for_penalty {
                        continue;
                    }
                    let mut penalty = if state.can_unlock {
                        0.0
                    } else {
                        match disposition {
                            DoorDisposition::RequireAccess => continue,
                            DoorDisposition::IgnoreLocks => 0.0,
                            DoorDisposition::Penalize => 409_600.0,
                        }
                    };
                    if edge.minimal_use {
                        penalty = 409_600.0;
                    }
                    let cost = door_edge_cost(a, place_of(edge.to, false), edge, k, g, penalty);
                    let h = if same_world {
                        nodes[current].h * 2.0
                    } else {
                        1.0
                    };
                    neighbors.push((edge.to, cost, Some(edge.door), h));
                }
            }
            for (m, cost, via_door, h) in neighbors {
                match node_of.get(&m) {
                    None => {
                        let n = nodes.len();
                        nodes.push(Node {
                            info: m,
                            g: cost,
                            h,
                            parent: None,
                            parent_door: None,
                            version: 0,
                            open: false,
                        });
                        node_of.insert(m, n);
                        improved.push((n, via_door));
                    }
                    Some(&n) if cost < nodes[n].g => {
                        nodes[n].g = cost;
                        improved.push((n, via_door));
                    }
                    Some(_) => {}
                }
            }
            let mut failed = false;
            for (n, via_door) in improved {
                nodes[n].open = false;
                if nodes[current].parent == Some(n) {
                    failed = true;
                    break;
                }
                nodes[n].parent = Some(current);
                nodes[n].parent_door = via_door;
                push(&mut open, &mut nodes, n, &mut seq);
                if nodes[n].h < nodes[best].h {
                    best = n;
                }
            }
            if failed {
                break false;
            }
            match pop(&mut open, &mut nodes) {
                Some(n) => current = n,
                None => break false,
            }
        };
        // The route back from the goal (or the nearest node) (`00996ae0`).
        let mut route = Vec::new();
        let mut route_doors = Vec::new();
        let mut at = Some(best);
        while let Some(n) = at {
            route.push(nodes[n].info);
            route_doors.push(nodes[n].parent_door);
            at = nodes[n].parent;
            if route.len() > nodes.len() {
                break;
            }
        }
        route.reverse();
        route_doors.reverse();
        (found, route, route_doors)
    }
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f32>().sqrt()
}

/// The two walks are measured in their own spaces. `006b8490` stores the
/// sum and each following arithmetic result as a float, in this order.
// Translated from 006b8adc-006b8b78 (FalloutNV.exe 1.4.0.525).
fn door_edge_cost(a: [f32; 3], b: [f32; 3], edge: &DoorEdge, k: f32, g: f32, penalty: f32) -> f32 {
    let distance = |p: [f32; 3], q: [f32; 3]| {
        (0..3)
            .map(|i| f64::from(p[i] - q[i]).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    let walks = (distance(edge.near, a) + distance(b, edge.far)) as f32;
    let scaled = (f64::from(walks) * f64::from(k)) as f32;
    let scaled = (f64::from(scaled) * 3.0) as f32;
    let with_path = (f64::from(scaled) + f64::from(g)) as f32;
    (f64::from(with_path) + f64::from(penalty)) as f32
}

/// A node of a planned path (`VirtualPathingNode` (Xbox PDB)): where it
/// is, the grid square of its cell outdoors (a navmesh's own square, else
/// the point's) and the navmesh it's on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualNode {
    pub position: [f32; 3],
    pub square: Option<(i32, i32)>,
    pub navmesh: FormId,
}

/// The navmesh info map, loaded on first use, and the navmeshes read to
/// tell which one a point is on.
#[derive(Default)]
pub struct NavInfos {
    map: Option<NavInfoMap>,
    meshes: HashMap<Place, NavMesh>,
}

impl NavInfos {
    pub fn map(&mut self, order: &LoadOrder) -> &NavInfoMap {
        self.map.get_or_insert_with(|| NavInfoMap::load(order))
    }

    fn interior(order: &LoadOrder, space: FormId) -> bool {
        order
            .get(space)
            .is_some_and(|r| r.entry.header.kind == esm::sig::CELL)
    }

    /// The entry of the navmesh a point in `space` (an interior cell or a
    /// worldspace) is on: the only one of its cell (or grid square), else
    /// the one whose triangles hold it (`PathingLocation`'s resolution,
    /// `006dd6f0`, looks through the cell's navmeshes for the triangle),
    /// else the one whose rough position is nearest (unresolved: the
    /// game's handling of a point on none isn't traced).
    pub fn info_at(&mut self, order: &LoadOrder, space: FormId, p: [f32; 3]) -> Option<usize> {
        let square = (!Self::interior(order, space)).then(|| crate::square_of(p));
        let candidates: Vec<usize> = self.map(order).in_place(space, square).to_vec();
        match candidates.len() {
            0 => return None,
            1 => return Some(candidates[0]),
            _ => {}
        }
        let map = self.map.as_ref().expect("loaded above");
        let navmeshes: Vec<FormId> = candidates.iter().map(|&i| map.infos[i].navmesh).collect();
        let mesh = self
            .meshes
            .entry((space, square))
            .or_insert_with(|| NavMesh::load_navmeshes(order, &navmeshes));
        if let Some(i) = mesh.navmesh_at(p).and_then(|n| map.of_navmesh(n)) {
            return Some(i);
        }
        candidates.into_iter().min_by(|&a, &b| {
            distance(map.infos[a].position, p).total_cmp(&distance(map.infos[b].position, p))
        })
    }

    /// The planned path from `from` to `to` in `space` (`006c94c0`): the
    /// start, the rough position of each navmesh the route passes, the
    /// goal; just the two ends when both are on one navmesh. `None` when
    /// either end is on no known navmesh or the search fails.
    pub fn virtual_path(
        &mut self,
        order: &LoadOrder,
        space: FormId,
        from: [f32; 3],
        to: [f32; 3],
    ) -> Option<Vec<VirtualNode>> {
        let start = self.info_at(order, space, from)?;
        let goal = self.info_at(order, space, to)?;
        let outdoors = !Self::interior(order, space);
        let map = self.map(order);
        let end = |p: [f32; 3], info: usize| VirtualNode {
            position: p,
            square: outdoors.then(|| crate::square_of(p)),
            navmesh: map.infos[info].navmesh,
        };
        if start == goal {
            return Some(vec![end(from, start), end(to, goal)]);
        }
        let (found, route) = map.search(start, from, goal, to);
        if !found || route.len() < 2 {
            return None;
        }
        let mut nodes = vec![end(from, start)];
        nodes.extend(route[1..route.len() - 1].iter().map(|&i| VirtualNode {
            position: map.infos[i].position,
            square: map.infos[i].square,
            navmesh: map.infos[i].navmesh,
        }));
        nodes.push(end(to, goal));
        Some(nodes)
    }

    /// The first `NVCI` door on the game's navmesh-info route from a point
    /// in one place to a point in another (`006b8c50`, `006b8490`).
    pub fn first_door_toward(
        &mut self,
        order: &LoadOrder,
        state: &crate::scripting::GameState,
        actor: FormId,
        to_space: FormId,
        to: [f32; 3],
    ) -> Option<FormId> {
        let (from_space, _, from, _) = state.place(order, actor)?;
        let start = self.info_at(order, from_space, from)?;
        let goal = self.info_at(order, to_space, to)?;
        let map = self.map(order);
        let (found, route, doors) = map.search_with_doors(
            start,
            from,
            goal,
            to,
            DoorDisposition::RequireAccess,
            |door| DoorAccess {
                disabled: !crate::placement::enabled_now(order, door, &state.disabled),
                can_unlock: actor_can_unlock(order, state, actor, door),
            },
        );
        if !found || route.len() < 2 {
            return None;
        }
        doors.into_iter().skip(1).flatten().next()
    }
}

/// The form named by a reference or cell subrecord.
fn sub_form(order: &LoadOrder, form: FormId, kind: FourCC) -> Option<FormId> {
    let rr = order.get(form)?;
    let data = rr.subrecord(kind).ok()??;
    (data.len() >= 4)
        .then(|| rr.plugin.to_global(FormId(le_u32(&data, 0))))
        .filter(|id| id.0 != 0)
}

fn ownership_rank(order: &LoadOrder, form: FormId) -> Option<i32> {
    let data = order.get(form)?.subrecord(FourCC::new(b"XRNK")).ok()??;
    (data.len() >= 4)
        .then(|| i32::from_le_bytes(data[..4].try_into().expect("length checked")))
        .filter(|rank| *rank != -1)
}

/// `0047d680`: rank on the actor base, including script changes kept by the
/// world state. Missing membership is -1.
fn actor_faction_rank(
    order: &LoadOrder,
    state: &crate::scripting::GameState,
    actor: FormId,
    base: FormId,
    faction: FormId,
) -> i32 {
    if let Some(rank) = state.faction_changes.get(&(actor, faction)) {
        return i32::from(*rank);
    }
    if actor == crate::dialogue::PLAYER_REF && faction == crate::factions::PLAYER_FACTION {
        return 0;
    }
    let Some((rr, record)) = crate::actor::data_record(order, base, crate::actor::USE_FACTIONS)
    else {
        return -1;
    };
    let rank = record
        .get_all(FourCC::new(b"SNAM"))
        .filter(|s| s.data.len() >= 5)
        .find(|s| rr.plugin.to_global(FormId(le_u32(&s.data, 0))) == faction)
        .map_or(-1, |s| i32::from(s.data[4] as i8));
    rank
}

/// `00546a40`/`00546c20`: a cell's owner, falling back to its encounter
/// zone (including the worldspace's zone when the cell has none).
fn cell_owner(
    order: &LoadOrder,
    state: &crate::scripting::GameState,
    space: FormId,
    cell: FormId,
) -> Option<FormId> {
    let explicit = state
        .set_by_scripts
        .cell_owners
        .get(&cell)
        .copied()
        .or_else(|| sub_form(order, cell, FourCC::new(b"XOWN")))
        .filter(|id| id.0 != 0);
    explicit.or_else(|| {
        let xezn = FourCC::new(b"XEZN");
        let zone = sub_form(order, cell, xezn).or_else(|| sub_form(order, space, xezn))?;
        (zone.0 != 0x1e)
            .then(|| sub_form(order, zone, esm::sig::DATA))
            .flatten()
    })
}

/// `00546ee0`: whether an NPC base owns a cell, including its required
/// faction rank.
fn actor_owns_cell(
    order: &LoadOrder,
    state: &crate::scripting::GameState,
    actor: FormId,
    base: FormId,
    space: FormId,
    cell: FormId,
) -> bool {
    let Some(owner) = cell_owner(order, state, space, cell) else {
        return false;
    };
    let Some(kind) = order.get(owner).map(|rr| rr.entry.header.kind) else {
        return false;
    };
    if kind.as_bytes() == b"NPC_" {
        return owner == base;
    }
    if kind.as_bytes() != b"FACT"
        || !order
            .get(base)
            .is_some_and(|rr| rr.entry.header.kind.as_bytes() == b"NPC_")
    {
        return false;
    }
    actor_faction_rank(order, state, actor, base, owner) >= ownership_rank(order, cell).unwrap_or(0)
}

/// `005679f0`: the reference's rank, its paired door's, its encounter
/// zone's, then its cell's. The no-zone record (0x1e) is ignored.
fn door_ownership_rank(
    order: &LoadOrder,
    door: FormId,
    linked: Option<FormId>,
    space: FormId,
    cell: FormId,
) -> i32 {
    let xezn = FourCC::new(b"XEZN");
    let zone = sub_form(order, door, xezn)
        .or_else(|| sub_form(order, cell, xezn))
        .or_else(|| sub_form(order, space, xezn))
        .filter(|id| id.0 != 0x1e);
    ownership_rank(order, door)
        .or_else(|| linked.and_then(|id| ownership_rank(order, id)))
        .or_else(|| {
            zone.and_then(|id| order.get(id))
                .and_then(|rr| rr.subrecord(esm::sig::DATA).ok().flatten())
                .and_then(|data| data.get(4).copied())
                .map(|rank| i32::from(rank as i8))
                .filter(|rank| *rank != -1)
        })
        .or_else(|| ownership_rank(order, cell))
        .unwrap_or(0)
}

/// `00502450`/`00502310`: a key, ownership, or an alarmed guard may open a
/// locked door. A lockpick item sets the used-item flag, so `006b8490`
/// still treats that door as unavailable in the normal request.
fn actor_can_unlock(
    order: &LoadOrder,
    state: &crate::scripting::GameState,
    actor: FormId,
    door: FormId,
) -> bool {
    let Some(lock) = crate::locks::lock_now(order, state, door) else {
        return true;
    };
    if lock
        .key
        .is_some_and(|key| state.item_count(order, actor, key) > 0)
    {
        return true;
    }
    let Some(base) = crate::scripting::base_of(order, actor) else {
        return false;
    };
    let Some((space, cell, _, _)) = state.place(order, door) else {
        return false;
    };
    let linked = order
        .get(door)
        .as_ref()
        .and_then(crate::placement::teleport_of)
        .map(|t| t.door);
    if actor_owns_cell(order, state, actor, base, space, cell) {
        return linked
            .and_then(|id| state.place(order, id))
            .map(|(other_space, other, _, _)| {
                cell_owner(order, state, other_space, other).is_none()
                    || actor_owns_cell(order, state, actor, base, other_space, other)
            })
            .unwrap_or(true);
    }
    // `00567790` for a door takes its own owner, then the linked door's;
    // unlike ordinary objects it does not inherit the cell's owner here.
    let owner = crate::script_functions::own_owner(order, state, door)
        .filter(|id| id.0 != 0)
        .or_else(|| {
            linked
                .and_then(|id| crate::script_functions::own_owner(order, state, id))
                .filter(|id| id.0 != 0)
        });
    if let Some(owner) = owner {
        if owner == base {
            return true;
        }
        if order
            .get(owner)
            .is_some_and(|rr| rr.entry.header.kind.as_bytes() == b"FACT")
        {
            if actor_faction_rank(order, state, actor, base, owner)
                >= door_ownership_rank(order, door, linked, space, cell)
            {
                return true;
            }
        } else {
            let xglb = FourCC::new(b"XGLB");
            let global = sub_form(order, door, xglb)
                .or_else(|| linked.and_then(|id| sub_form(order, id, xglb)))
                .or_else(|| sub_form(order, cell, xglb));
            if global
                .and_then(|id| state.globals.get(&id))
                .is_some_and(|value| *value != 0.0)
            {
                return true;
            }
        }
    }
    let class = order
        .get(base)
        .filter(|rr| rr.entry.header.kind.as_bytes() == b"NPC_")
        .and_then(|_| sub_form(order, base, FourCC::new(b"CNAM")));
    let guard = class
        .and_then(|id| order.get(id))
        .and_then(|rr| rr.subrecord(esm::sig::DATA).ok().flatten())
        .is_some_and(|data| {
            let at = if data.len() == 0x18 { 0x0c } else { 0x10 };
            data.get(at).is_some_and(|flags| flags & 2 != 0)
        });
    guard
        && super::current_package(order, state, actor)
            .is_some_and(|package| package.kind == super::kinds::ALARM)
}

/// The run of nodes from `first` whose cells are attached (`006c9fc0`):
/// the last node's index, or `None` when `first`'s isn't. (A teleport
/// door node would end the run; routes here have none.) Only this run gets
/// a detailed navmesh path (`006ca0e0`).
// Translated from 006c9fc0 (decompiled, FalloutNV.exe 1.4.0.525).
pub fn attached_run(
    nodes: &[VirtualNode],
    first: usize,
    attached: impl Fn(&VirtualNode) -> bool,
) -> Option<usize> {
    if !attached(nodes.get(first)?) {
        return None;
    }
    let mut last = first;
    while last + 1 < nodes.len() && attached(&nodes[last + 1]) {
        last += 1;
    }
    Some(last)
}

/// Someone out of sight walks `distance` along the nodes still ahead
/// (`009ea8a0`): point to point, facing each leg, the nodes reached taken
/// off; partway along the last one, done when within `radius` of the end.
// Translated from 009ea8a0 (decompiled, FalloutNV.exe 1.4.0.525).
pub fn walk_nodes(
    nodes: &mut Vec<[f32; 3]>,
    from: [f32; 3],
    heading: f32,
    distance_left: f32,
    radius: f32,
) -> PolyWalk {
    let mut at = from;
    let mut facing = heading;
    let mut left = distance_left;
    while let Some(&next) = nodes.first() {
        let d = distance(at, next);
        if d > 1e-3 {
            facing = crate::movement::heading_to(at, next);
        }
        let last = nodes.len() == 1;
        if d <= left {
            left -= d;
            at = next;
            nodes.remove(0);
            continue;
        }
        let f = if d > 0.0 { left / d } else { 0.0 };
        at = [0, 1, 2].map(|k| at[k] + (next[k] - at[k]) * f);
        return PolyWalk {
            at,
            heading: facing,
            left: 0.0,
            done: last && d - left < radius,
        };
    }
    PolyWalk {
        at,
        heading: facing,
        left,
        done: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(navmesh: u32, x: f32, links: &[u32], cheap: &[u32]) -> NavInfo {
        NavInfo {
            navmesh: FormId(navmesh),
            space: FormId(0x3C),
            flags: 0,
            square: Some(((x / 4096.0).floor() as i32, 0)),
            position: [x, 0.0, 0.0],
            preferred: 0.0,
            links: links.iter().map(|&l| FormId(l)).collect(),
            cheap_links: cheap.iter().map(|&l| FormId(l)).collect(),
            doors: Vec::new(),
        }
    }

    fn map(infos: Vec<NavInfo>) -> NavInfoMap {
        let mut m = NavInfoMap::default();
        for i in infos {
            m.insert(i);
        }
        m
    }

    #[test]
    fn the_first_list_costs_three_times_the_second() {
        // A at 0, C at 1000; B at 500 off to the side (y 400). A–C joined
        // directly by the first list (3 × 1000), A–B–C by the second (2 ×
        // about 640): the search goes round by B.
        let mut b = info(2, 500.0, &[], &[1, 3]);
        b.position[1] = 400.0;
        let m = map(vec![
            info(1, 0.0, &[3], &[2]),
            b,
            info(3, 1000.0, &[1], &[2]),
        ]);
        let (found, route) = m.search(0, [0.0; 3], 2, [1000.0, 0.0, 0.0]);
        assert!(found);
        assert_eq!(route, [0, 1, 2]);
        // With A–C in the second list too, straight there.
        let m = map(vec![
            info(1, 0.0, &[], &[2, 3]),
            info(2, 500.0, &[], &[1, 3]),
            info(3, 1000.0, &[], &[1, 2]),
        ]);
        let (_, route) = m.search(0, [0.0; 3], 2, [1000.0, 0.0, 0.0]);
        assert_eq!(route, [0, 2]);
    }

    #[test]
    fn door_access_disposition_and_minimal_use_change_the_route() {
        let mut m = map(vec![
            info(1, 0.0, &[], &[]),
            info(2, 100.0, &[], &[]),
            info(3, 200.0, &[], &[]),
            info(4, 10.0, &[], &[]),
        ]);
        let edge = |door, to, near, far, minimal_use| DoorEdge {
            door: FormId(door),
            to,
            near: [near, 0.0, 0.0],
            far: [far, 0.0, 0.0],
            minimal_use,
            blocked_for_penalty: false,
        };
        m.door_edges = vec![
            vec![
                edge(12, 3, 5.0, 10.0, false),
                edge(10, 1, 0.0, 100.0, false),
            ],
            vec![edge(11, 3, 110.0, 10.0, false)],
            vec![],
            vec![],
        ];
        let access = |blocked: bool, disabled: bool| {
            move |door| DoorAccess {
                can_unlock: !(blocked && door == FormId(12)),
                disabled: disabled && door == FormId(12),
            }
        };
        let (found, route, doors) = m.search_with_doors(
            0,
            [0.0; 3],
            3,
            [10.0, 0.0, 0.0],
            DoorDisposition::RequireAccess,
            access(false, false),
        );
        assert!(found);
        assert_eq!(route, [0, 3]);
        assert_eq!(doors, [None, Some(FormId(12))]);

        let (found, route, doors) = m.search_with_doors(
            0,
            [0.0; 3],
            3,
            [10.0, 0.0, 0.0],
            DoorDisposition::RequireAccess,
            access(true, false),
        );
        assert!(found);
        assert_eq!(route, [0, 1, 3]);
        m.door_edges[0][0].minimal_use = false;
        m.door_edges[0][0].blocked_for_penalty = true;
        let (_, route, _) = m.search_with_doors(
            0,
            [0.0; 3],
            3,
            [10.0, 0.0, 0.0],
            DoorDisposition::Penalize,
            access(false, false),
        );
        assert_eq!(route, [0, 1, 3]);
        assert_eq!(doors, [None, Some(FormId(10)), Some(FormId(11))]);
        for disposition in [DoorDisposition::IgnoreLocks, DoorDisposition::Penalize] {
            let (_, route, _) = m.search_with_doors(
                0,
                [0.0; 3],
                3,
                [10.0, 0.0, 0.0],
                disposition,
                access(true, false),
            );
            assert_eq!(
                route,
                if disposition == DoorDisposition::IgnoreLocks {
                    vec![0, 3]
                } else {
                    vec![0, 1, 3]
                }
            );
        }
        let (_, route, _) = m.search_with_doors(
            0,
            [0.0; 3],
            3,
            [10.0, 0.0, 0.0],
            DoorDisposition::IgnoreLocks,
            access(false, true),
        );
        assert_eq!(route, [0, 1, 3]);
        m.door_edges[0][0].minimal_use = true;
        let (_, route, _) = m.search_with_doors(
            0,
            [0.0; 3],
            3,
            [10.0, 0.0, 0.0],
            DoorDisposition::RequireAccess,
            access(false, false),
        );
        assert_eq!(route, [0, 1, 3]);
    }

    #[test]
    fn a_door_costs_the_two_walks_in_their_respective_spaces() {
        let edge = DoorEdge {
            door: FormId(1),
            to: 1,
            near: [4.0, 0.0, 0.0],
            far: [5000.0, 0.0, 0.0],
            minimal_use: false,
            blocked_for_penalty: false,
        };
        assert_eq!(
            door_edge_cost([0.0; 3], [5008.0, 0.0, 0.0], &edge, 0.5, 7.0, 0.0),
            25.0
        );
    }

    #[test]
    fn a_door_node_uses_twice_the_parent_heuristic_in_one_worldspace() {
        let mut m = map(vec![
            info(1, 0.0, &[], &[3]),
            info(2, 2000.0, &[], &[]),
            info(3, 100.0, &[], &[]),
        ]);
        let edge = |door, to, near, far| DoorEdge {
            door: FormId(door),
            to,
            near: [near, 0.0, 0.0],
            far: [far, 0.0, 0.0],
            minimal_use: false,
            blocked_for_penalty: false,
        };
        m.door_edges = vec![
            vec![edge(10, 1, 0.0, 2000.0)],
            vec![edge(11, 2, 2000.0, 100.0)],
            vec![],
        ];
        let (found, route, _) = m.search_with_doors(
            0,
            [0.0; 3],
            2,
            [100.0, 0.0, 0.0],
            DoorDisposition::RequireAccess,
            |_| DoorAccess {
                disabled: false,
                can_unlock: true,
            },
        );
        assert!(found);
        // A direct cheap link costs 100; both door walks cost zero. The
        // door-node heuristic is 20, rather than 190 from the far info's
        // rough position, so the search considers the door route first.
        assert_eq!(route, [0, 1, 2]);
    }

    #[test]
    fn linked_door_portal_selects_the_destination_and_skips_flagged_infos() {
        use testdata::quest_ids::*;
        let data = testdata::quests("navinfo-door-portal");
        let order =
            LoadOrder::from_data_dir(data.path(), &esm::ActivePlugins::OfficialOnly).unwrap();
        let mut m = NavInfoMap::load(&order);
        let start = m.of_navmesh(FormId(NAVMESH)).unwrap();
        let destination = m.of_navmesh(FormId(NAVMESH2)).unwrap();
        let decoy = m.of_navmesh(FormId(NAVMESH3)).unwrap();
        assert!(m.infos[decoy].doors.contains(&FormId(LINK_DOOR2)));
        let edge = m.door_edges[start]
            .iter()
            .find(|e| e.door == FormId(LINK_DOOR))
            .unwrap();
        assert_eq!(edge.to, destination);
        m.infos[destination].flags |= 0x10;
        m.resolve_door_edges(&order);
        assert!(!m.door_edges[start]
            .iter()
            .any(|e| e.door == FormId(LINK_DOOR)));
    }

    #[test]
    fn a_preferred_navmesh_makes_leaving_it_cheaper() {
        // Two ways from A to D, by B or by C, equally long; B is preferred
        // (factor 0.005: k = 1 − 0.5), so its leg costs half.
        let mut b = info(2, 500.0, &[], &[1, 4]);
        b.position[1] = 300.0;
        b.preferred = 0.005;
        let mut c = info(3, 500.0, &[], &[1, 4]);
        c.position[1] = -300.0;
        let m = map(vec![
            info(1, 0.0, &[], &[3, 2]),
            b,
            c,
            info(4, 1000.0, &[], &[2, 3]),
        ]);
        let (found, route) = m.search(0, [0.0; 3], 3, [1000.0, 0.0, 0.0]);
        assert!(found);
        assert_eq!(route, [0, 1, 3]);
    }

    #[test]
    fn a_search_that_fails_gives_the_way_to_the_nearest_entry() {
        // D can't be reached: the route ends at C, nearest to it.
        let m = map(vec![
            info(1, 0.0, &[], &[2]),
            info(2, 400.0, &[], &[1, 3]),
            info(3, 800.0, &[], &[2]),
            info(4, 2000.0, &[], &[]),
        ]);
        let (found, route) = m.search(0, [0.0; 3], 3, [2000.0, 0.0, 0.0]);
        assert!(!found);
        assert_eq!(route, [0, 1, 2]);
    }

    #[test]
    fn entries_read_as_the_loader_reads_them() {
        let global = |raw: u32| FormId(raw);
        let exterior = |space: FormId| space == FormId(0xDA726);
        let mut nvmi = 0x20u32.to_le_bytes().to_vec();
        nvmi.extend(0x1639F7u32.to_le_bytes());
        nvmi.extend(0xDA726u32.to_le_bytes());
        nvmi.extend((-17i16).to_le_bytes());
        nvmi.extend(1i16.to_le_bytes());
        for v in [6070.5f32, -67421.0, 9407.0] {
            nvmi.extend(v.to_le_bytes());
        }
        // Island data (bounds, no vertices or triangles), then the factor.
        nvmi.extend([0u8; 24]);
        nvmi.extend([0u8; 4]);
        nvmi.extend(0.25f32.to_le_bytes());
        let i = read_nvmi(&nvmi, 11, &global, &exterior).unwrap();
        assert_eq!(i.square, Some((1, -17)));
        assert_eq!(i.position, [6070.5, -67421.0, 9407.0]);
        assert_eq!(i.preferred, 0.25);
        assert_eq!(
            read_nvmi(&nvmi, 9, &global, &exterior).unwrap().preferred,
            0.0
        );
        // Three lists from NVER 11; before that no second list.
        let words = |w: &[u32]| w.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>();
        let (n, a, b, d) = read_nvci(&words(&[9, 2, 10, 11, 1, 12, 1, 13]), 11, &global).unwrap();
        assert_eq!(
            (n, a.len(), b, d),
            (FormId(9), 2, vec![FormId(12)], vec![FormId(13)])
        );
        let (_, a, b, d) = read_nvci(&words(&[9, 1, 10, 1, 13]), 10, &global).unwrap();
        assert_eq!((a, b, d), (vec![FormId(10)], vec![], vec![FormId(13)]));
    }

    #[test]
    fn walking_the_nodes_takes_off_those_reached() {
        let mut nodes = vec![[100.0, 0.0, 0.0], [100.0, 100.0, 0.0]];
        let w = walk_nodes(&mut nodes, [0.0; 3], 0.0, 150.0, 20.0);
        assert_eq!(nodes, [[100.0, 100.0, 0.0]]);
        assert_eq!((w.at, w.done), ([100.0, 50.0, 0.0], false));
        // Within the radius of the end: done.
        let w = walk_nodes(&mut nodes, w.at, w.heading, 40.0, 20.0);
        assert!(w.done && (w.at[1] - 90.0).abs() < 1e-4, "{w:?}");
    }
}
