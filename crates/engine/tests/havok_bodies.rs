//! Opt-in: real Havok bodies in a real `hkpWorld`, built and stepped by the exe's own translated
//! code over the player's FalloutNV.exe (its data sections give the vtables and constants). Nothing
//! here stands in for Havok: the world comes from the game's `bhkWorldCinfo` and `bhkWorld`
//! constructors with the settings a cell's world gets (`00552dc0`), the bodies from `hkpBoxShape`,
//! `hkpRigidBodyCinfo` and `hkpRigidBody`, and the step is `hkpWorld::stepDeltaTime` (the shipped
//! continuous simulation) or `hkpWorld::stepMultithreaded` (the job queue and worker pool that
//! `bhkMultithreadedSystem::InitMTSystem` builds; the workers run on the stepped thread host, so a
//! run repeats exactly). The one stand-in is outside Havok: `NiStream::RegisterLoader`, whose map
//! of NIF loaders Gamebryo's start-up makes, answers without registering.
//!
//! It is the proof harness for the physics step on spare cores: a world of stacked boxes falls onto
//! a ground slab and is stepped continuous and multithreaded; every body's motion (position,
//! rotation, velocities, deactivation state) is compared after every 20 steps, and a collision
//! listener on the world records each `contactPointAdded` (the callback the game's collision
//! listener, `FOCollisionListener`, hangs its sounds and damage on) with the bodies and the thread
//! it ran on.
//!
//! Needs the player's installation (nothing from it is stored or committed):
//!
//! ```text
//! NV_DATA="C:\...\Fallout New Vegas\Data" cargo test -p engine --test havok_bodies -- --ignored --nocapture
//! ```

use engine::{Engine, Ret};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// `operator new` (cdecl, the size).
const NEW: u32 = 0x0040_1000;
/// `bhkWorld::InitSystem`: the memory system, the base system, the collision filter, the
/// multithreaded system (`012677d0`).
const BHK_WORLD_INIT_SYSTEM: u32 = 0x00c6_8230;
/// `NiStream::RegisterLoader` (`(name, creator)`, answers 0).
const NI_STREAM_REGISTER_LOADER: u32 = 0x00a6_4900;
/// `NiStream::ms_pkLoaders`.
const NI_STREAM_LOADERS: u32 = 0x011f_4784;
/// `bhkWorldCinfo`'s constructor (0xF0 bytes; `+0xd1` the simulation type from `011afe5c`).
const BHK_WORLD_CINFO: u32 = 0x00c6_81c0;
/// `hkpWorldCinfo::setBroadPhaseWorldSize` (`(cinfo, size)`).
const SET_BROAD_PHASE_WORLD_SIZE: u32 = 0x00c9_0db0;
/// `bhkWorld`'s constructor (0xA0 bytes): `(this, cinfo)`.
const BHK_WORLD: u32 = 0x00c6_9010;
/// `bhkWorld`'s slot that answers its `hkpWorld`.
const SLOT_HK_WORLD: u32 = 0x94;
/// The Havok memory router of the calling thread (`+0x10` its allocator; slot 4 allocates).
const MEMORY_ROUTER: u32 = 0x00c8_5750;
/// `hkpBoxShape::hkpBoxShape`: `(this, half extents, radius)`, 0x30 bytes.
const BOX_SHAPE: u32 = 0x00c9_ddb0;
const BOX_SHAPE_SIZE: u32 = 0x30;
/// `hkpRigidBodyCinfo::hkpRigidBodyCinfo` (0xC0 bytes) and `hkpRigidBody::hkpRigidBody`
/// (`(this, cinfo)`, 0x220 bytes).
const BODY_CINFO: u32 = 0x00cf_5ea0;
const BODY: u32 = 0x00ca_8c90;
const BODY_SIZE: u32 = 0x220;
/// `hkpWorld::addEntity`: `(world, entity, activation)`.
const ADD_ENTITY: u32 = 0x00c9_14d0;
/// `hkpWorld::addCollisionListener` (the array at `+0x1a0`; `00553440` adds the game's there).
const ADD_COLLISION_LISTENER: u32 = 0x00c9_3a20;
/// `hkpWorld::stepDeltaTime`: `(world, seconds)`.
const STEP_DELTA_TIME: u32 = 0x00c9_1b10;
/// `hkpWorld::stepMultithreaded`: `(world, job queue, thread pool, seconds)`.
const STEP_MULTITHREADED: u32 = 0x00c9_3ae0;
/// `bhkMultithreadedSystem::AcquireMTResources`: `(system, flag)`; the system's global, its
/// worker count (the `iNumThreads:HAVOK` setting's value, +4), job queue (+0x14) and pool (+0x18).
const ACQUIRE_MT_RESOURCES: u32 = 0x00cb_f6b0;
const MT_SYSTEM: u32 = 0x0126_77d0;

/// The gravity a cell's world gets (`00552dc0`: `(0, 0, [011ca158], 0)`; `00f4b550` sets the
/// global to -98.1 before the game starts), its broad phase size (`0102efc0`) and the float it
/// stores at the cinfo's `+0x54`.
const GRAVITY_Z: u32 = 0xc2c4_3334;
const GRAVITY_GLOBAL: u32 = 0x011c_a158;
const BROAD_PHASE_SIZE: f32 = 3250.0;

/// A body's motion in `hkpRigidBody`: 0x120 bytes at +0xE0 (the transform, the swept transform,
/// the velocities, the deactivation counters): the rotation's three columns of four floats at
/// +0xF0, the position at +0x120, the linear and angular velocities at +0x1B0 and +0x1C0.
const MOTION: u32 = 0xe0;
const MOTION_SIZE: u32 = 0x120;
const ROTATION: u32 = 0xf0;
const POSITION: u32 = 0x120;
const LINEAR_VELOCITY: u32 = 0x1b0;
const ANGULAR_VELOCITY: u32 = 0x1c0;
/// An entity's simulation island (+0xCC) and the world's active islands (array +0x28, count +0x2C):
/// a body whose island is not among them is deactivated (asleep).
const ENTITY_ISLAND: u32 = 0xcc;
const ACTIVE_ISLANDS: u32 = 0x28;

/// `bhkWorld`'s update-mode setter (`00c661d0`, the `iSimType:HAVOK` setting) and the global it sets
/// (the simulation type every new world gets).
const SET_UPDATE_MODE: u32 = 0x00c6_61d0;
const UPDATE_MODE: u32 = 0x011a_fe5c;

/// Where the boxes come to rest: half extent 10 plus the convex radius 0.05 above the slab's top
/// (0, plus its own radius 0.05) for the first layer (bodies 1 to 16), and a box higher again for
/// the second (17 to 32); how far a resting box may be from that, and the speeds below which an
/// awake box counts as resting (Havok units a second; radians a second).
const LAYER_HEIGHTS: [f32; 2] = [10.1, 30.2];
const HEIGHT_SLACK: f32 = 0.5;
const REST_LINEAR: f32 = 0.5;
const REST_ANGULAR: f32 = 0.05;

/// The simulation types (`hkpWorldCinfo::SimulationType`): 2 is what the game ships
/// (`iSimType:HAVOK` 1), 3 the multithreaded step.
const CONTINUOUS: u8 = 2;
const MULTITHREADED: u8 = 3;

/// Where the test listener's slots live (no exe code there).
const LISTENER_NOTHING: u32 = 0x0400_1000;
const LISTENER_ADDED: u32 = 0x0400_1010;

/// One step's length (the game's step at 60 frames a second) and how long a run is.
const SECONDS: f32 = 1.0 / 60.0;
const ROUNDS: usize = 12;
const STEPS_PER_ROUND: u32 = 20;

/// One `contactPointAdded`: the two bodies (entities) and the game thread it ran on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Contact {
    a: u32,
    b: u32,
    thread: u32,
}

/// The listener's record of the run in progress (the callback may run on a worker's stack).
static CONTACTS: Mutex<Vec<Contact>> = Mutex::new(Vec::new());
/// One run at a time: the listener's record is shared.
static SERIAL: Mutex<()> = Mutex::new(());

fn data() -> PathBuf {
    PathBuf::from(std::env::var("NV_DATA").expect("NV_DATA: the game's Data folder"))
}

/// The memory manager's static initializer (`00fb66a0`) in the host's form (`host::STAND_INS`): the thread
/// scrap-heap map and the Ni allocator set-up of `00866690`; the heaps are the engine's.
fn start_allocator(e: &mut Engine) {
    e.call(0x00aa_47e0, &[0x011f_6238 + 0x114, 0x25]);
    e.mem.set_u32(0x011f_6238 + 0x114, 0x010a_25f8);
    let holder = e.call(NEW, &[8]).u32();
    let allocator = e.call(NEW, &[4]).u32();
    let allocator = e.call(0x00aa_2230, &[allocator]).u32();
    let holder = e.call(0x00aa_21d0, &[holder, allocator]).u32();
    e.mem.set_u32(0x011f_6118, holder);
    e.call(0x00aa_2020, &[]);
}

/// A Havok object of `size` bytes from the calling thread's memory router, as the exe allocates
/// them (the size kept at `+4`).
fn havok_new(e: &mut Engine, size: u32) -> u32 {
    let router = e.call(MEMORY_ROUTER, &[]).u32();
    let allocator = e.mem.u32(router + 0x10);
    let object = e.vcall(allocator, 4, &[size]).u32();
    e.mem.set_u16(object + 4, size as u16);
    object
}

/// A float vector of four lanes in fresh memory.
fn vector(e: &mut Engine, lanes: [f32; 4]) -> u32 {
    let v = e.mem.alloc(16);
    for (i, lane) in (0u32..).zip(lanes) {
        e.mem.set_f32(v + 4 * i, lane);
    }
    v
}

/// The entity a contact event's body belongs to: the root of the `hkpCdBody` chain (`+0xC` the
/// parent) is an entity's collidable, whose signed byte at `+0x10` is the owner's offset.
fn entity_of(e: &Engine, cd_body: u32) -> u32 {
    let mut root = cd_body;
    while root != 0 && e.mem.u32(root + 0xc) != 0 {
        root = e.mem.u32(root + 0xc);
    }
    if root == 0 {
        return 0;
    }
    root.wrapping_add_signed(i32::from(e.mem.u8(root + 0x10) as i8))
}

fn listener_nothing(_: &mut Engine, _: &[u32]) -> Ret {
    Ret::default()
}

/// The listener's `contactPointAdded` (slot 0x10, as `FOCollisionListener`'s `00623cb0`):
/// records the event's two bodies (`+0`, `+4`) and the thread; accepts the point.
fn listener_added(e: &mut Engine, a: &[u32]) -> Ret {
    let event = a.get(1).copied().unwrap_or(0);
    let contact = Contact {
        a: entity_of(e, e.mem.u32(event)),
        b: entity_of(e, e.mem.u32(event + 4)),
        thread: engine_core::thread_host::current_id(e),
    };
    CONTACTS.lock().unwrap().push(contact);
    Ret::default()
}

/// A collision listener object: a vtable of eight slots, `contactPointAdded` at 0x10.
fn add_listener(e: &mut Engine, world: u32) {
    e.register(LISTENER_NOTHING, listener_nothing);
    e.register(LISTENER_ADDED, listener_added);
    let vtable = e.mem.alloc(0x20);
    for slot in 0..8 {
        e.mem.set_u32(vtable + 4 * slot, LISTENER_NOTHING);
    }
    e.mem.set_u32(vtable + 0x10, LISTENER_ADDED);
    let listener = e.mem.alloc(0x10);
    e.mem.set_u32(listener, vtable);
    e.call(ADD_COLLISION_LISTENER, &[world, listener]);
}

/// One box body: half extents `half`, at `at`, of `mass` (0: fixed), on collision layer `layer`.
fn box_body(e: &mut Engine, half: [f32; 3], at: [f32; 3], mass: f32, layer: u32) -> u32 {
    let extents = vector(e, [half[0], half[1], half[2], 0.0]);
    let shape = havok_new(e, BOX_SHAPE_SIZE);
    e.call(BOX_SHAPE, &[shape, extents, 0.05f32.to_bits()]);
    let cinfo = e.mem.alloc(0xc0);
    e.call(BODY_CINFO, &[cinfo]);
    e.mem.set_u32(cinfo, layer);
    e.mem.set_u32(cinfo + 4, shape);
    for (i, lane) in (0u32..).zip(at) {
        e.mem.set_f32(cinfo + 0x10 + 4 * i, lane);
    }
    if mass == 0.0 {
        // MOTION_FIXED.
        e.mem.set_u8(cinfo + 0xb4, 5);
    } else {
        // A box's inertia: m/3 * (b^2 + c^2) on the diagonal (half extents a, b, c);
        // MOTION_BOX_INERTIA.
        let [a, b, c] = half;
        let k = mass / 3.0;
        e.mem.set_f32(cinfo + 0x50, k * (b * b + c * c));
        e.mem.set_f32(cinfo + 0x64, k * (a * a + c * c));
        e.mem.set_f32(cinfo + 0x78, k * (a * a + b * b));
        e.mem.set_f32(cinfo + 0x90, mass);
        e.mem.set_u8(cinfo + 0xb4, 3);
    }
    let body = havok_new(e, BODY_SIZE);
    e.call(BODY, &[body, cinfo]);
    body
}

/// A world as the game makes a cell's (`00552dc0`), of simulation type `simulation`.
fn world(e: &mut Engine, simulation: u8) -> u32 {
    e.call(BHK_WORLD_INIT_SYSTEM, &[]);
    // The game builds it in a 0x110-byte slot of its frame; the world's `Init` reads `cinfo + 0xF0`.
    let cinfo = e.mem.alloc(0x110);
    e.call(BHK_WORLD_CINFO, &[cinfo]);
    e.mem.set_u32(cinfo + 0x10, 0);
    e.mem.set_u32(cinfo + 0x14, 0);
    // The global the static initializer `00f4b550` sets (its value written here if the run skipped it).
    let gravity = match e.mem.u32(GRAVITY_GLOBAL) {
        0 => GRAVITY_Z,
        set => set,
    };
    e.mem.set_u32(cinfo + 0x18, gravity);
    e.mem.set_u32(cinfo + 0x1c, 0);
    e.call(
        SET_BROAD_PHASE_WORLD_SIZE,
        &[cinfo, BROAD_PHASE_SIZE.to_bits()],
    );
    e.mem.set_f32(cinfo + 0x54, 1.0);
    e.mem.set_u8(cinfo + 0xd1, simulation);
    let bhk = e.call(NEW, &[0xa0]).u32();
    e.call(BHK_WORLD, &[bhk, cinfo]);
    let world = e.vcall(bhk, SLOT_HK_WORLD, &[]).u32();
    assert_ne!(world, 0, "no hkpWorld");
    assert_eq!(e.mem.u32(world + 0xf0), u32::from(simulation));
    world
}

/// A world of simulation type `simulation` with `workers` worker threads: a fixed ground slab and
/// `n * n` boxes in `layers` stacked layers above it, offset so they tumble and touch.
struct Scene {
    e: Engine,
    world: u32,
    bodies: Vec<u32>,
}

fn scene(simulation: u8, workers: u32, n: u32, layers: u32) -> Scene {
    let exe = engine::exe::installed_exe(&data());
    engine::exe::verify(&exe).unwrap_or_else(|e| panic!("{e}"));
    let mut e = Engine::with_exe(&exe).expect("checked installation");
    // The C runtime's start as the host runs it (`mainCRTStartup`): `_heap_init(1)`, `__mtinit` (the
    // locks `atexit` takes), `_RTC_Initialize`, `_ioinit`, `_FPinit` and `_initp_misc_cfltcvt_tab`.
    e.call(0x00ed_db6a, &[1]);
    e.call(0x00ed_1327, &[]);
    e.call(0x00ed_db1e, &[]);
    e.call(0x00ed_7e18, &[]);
    let fp_init = e.mem.u32(0x010f_1f1c);
    if fp_init != 0 {
        e.call(fp_init, &[1]);
    }
    e.call(0x00ed_248e, &[]);
    // The exe's static initializers as the host runs them (the settings, Havok's singletons and its
    // reference lock, the gravity global).
    engine::static_init::run(&mut e, &[(0x00fb_66a0, start_allocator)]);
    // The one stand-in: `bhkWorld::InitSystem` registers its NIF stream loaders with
    // `NiStream::RegisterLoader`, whose map Gamebryo's start-up makes (not run here); loading
    // NIF files is no part of this test.
    if e.mem.u32(NI_STREAM_LOADERS) == 0 {
        e.register(NI_STREAM_REGISTER_LOADER, |_, _| Ret::default());
    }
    // A player's `iSimType` cannot reach the multithreaded step: the port keeps 4 continuous (the
    // exe gives it type 3). The last setting leaves what the game ships (1: continuous); the worlds
    // here set their type themselves.
    for (setting, simulation_type) in [(4, CONTINUOUS), (0, 1), (1, CONTINUOUS)] {
        e.call(SET_UPDATE_MODE, &[setting]);
        assert_eq!(e.mem.u8(UPDATE_MODE), simulation_type, "iSimType {setting}");
    }
    let world = world(&mut e, simulation);
    let system = e.mem.u32(MT_SYSTEM);
    assert_ne!(system, 0, "no multithreaded system");
    e.mem.set_u32(system + 4, workers);
    add_listener(&mut e, world);
    // Layer 1 (static) for the ground, 4 (clutter) for the boxes.
    let ground = box_body(&mut e, [400.0, 400.0, 5.0], [0.0, 0.0, -5.0], 0.0, 1);
    e.call(ADD_ENTITY, &[world, ground, 1]);
    let mut bodies = vec![ground];
    for z in 0..layers {
        for y in 0..n {
            for x in 0..n {
                let at = [
                    (x as f32 - n as f32 / 2.0) * 22.0 + z as f32 * 1.5,
                    (y as f32 - n as f32 / 2.0) * 22.0 - z as f32 * 1.1,
                    12.0 + z as f32 * 21.0,
                ];
                let body = box_body(&mut e, [10.0, 10.0, 10.0], at, 10.0, 4);
                e.call(ADD_ENTITY, &[world, body, 1]);
                bodies.push(body);
            }
        }
    }
    Scene { e, world, bodies }
}

/// Steps the world once by [`SECONDS`], continuous or multithreaded by its type, the way
/// `bhkWorld::Update` (`00c6ae70`) does.
fn step(e: &mut Engine, world: u32) {
    if e.mem.u32(world + 0xf0) == u32::from(MULTITHREADED) {
        let system = e.mem.u32(MT_SYSTEM);
        e.call(ACQUIRE_MT_RESOURCES, &[system, 0]);
        let queue = e.mem.u32(system + 0x14);
        let pool = e.mem.u32(system + 0x18);
        e.call(STEP_MULTITHREADED, &[world, queue, pool, SECONDS.to_bits()]);
    } else {
        e.call(STEP_DELTA_TIME, &[world, SECONDS.to_bits()]);
    }
}

/// What a run leaves: every body's motion bytes after each round, each step's contacts (the
/// entities as body numbers, in the order they were added), and at its end every body's position
/// and whether it is still awake.
struct Run {
    motions: Vec<Vec<Vec<u8>>>,
    contacts: Vec<Vec<Contact>>,
    positions: Vec<[f32; 3]>,
    awake: Vec<bool>,
}

/// Whether `body`'s simulation island is one of the world's active islands.
fn is_awake(e: &Engine, world: u32, body: u32) -> bool {
    let island = e.mem.u32(body + ENTITY_ISLAND);
    let islands = e.mem.u32(world + ACTIVE_ISLANDS);
    let count = e.mem.u32(world + ACTIVE_ISLANDS + 4);
    (0..count).any(|i| e.mem.u32(islands + 4 * i) == island)
}

/// Four float lanes of a motion block (`at` a body offset), from the bytes a round kept.
fn lanes(motion: &[u8], at: u32) -> [f32; 4] {
    let start = (at - MOTION) as usize;
    std::array::from_fn(|i| {
        f32::from_le_bytes(motion[start + 4 * i..start + 4 * i + 4].try_into().unwrap())
    })
}

/// The length of a vector's first three lanes.
fn length(v: [f32; 4]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// A difference that a NaN cannot hide: NaN on either side is an infinite gap.
fn gap(a: f32, b: f32) -> f32 {
    let d = (a - b).abs();
    if d.is_nan() {
        f32::INFINITY
    } else {
        d
    }
}

/// Holds what "at rest on the slab" means for a run's end: every box at its layer's height
/// (within [`HEIGHT_SLACK`]), and every box asleep or slower than [`REST_LINEAR`] and
/// [`REST_ANGULAR`] in the last round. Prints the heights and the awake boxes.
fn assert_at_rest(name: &str, run: &Run) {
    let last = run.motions.last().expect("a round");
    let mut heights = [f32::MAX, f32::MIN, f32::MAX, f32::MIN];
    for (i, p) in run.positions.iter().enumerate().skip(1) {
        let layer = (i - 1) / 16;
        let want = LAYER_HEIGHTS[layer];
        heights[2 * layer] = heights[2 * layer].min(p[2]);
        heights[2 * layer + 1] = heights[2 * layer + 1].max(p[2]);
        assert!(
            gap(p[2], want) <= HEIGHT_SLACK,
            "{name}: box {i} rests at {p:?}, not at its layer's height {want}"
        );
        let linear = length(lanes(&last[i], LINEAR_VELOCITY));
        let angular = length(lanes(&last[i], ANGULAR_VELOCITY));
        assert!(
            !run.awake[i] || (linear < REST_LINEAR && angular < REST_ANGULAR),
            "{name}: box {i} is awake and moving ({linear} units/s, {angular} rad/s)"
        );
    }
    let awake: Vec<usize> = (1..run.awake.len()).filter(|&i| run.awake[i]).collect();
    println!(
        "{name}: first layer at {:.3}..{:.3}, second at {:.3}..{:.3}; awake boxes at the end: {awake:?}",
        heights[0], heights[1], heights[2], heights[3]
    );
}

fn run(simulation: u8, workers: u32) -> Run {
    let Scene {
        mut e,
        world,
        bodies,
    } = scene(simulation, workers, 4, 2);
    let number: BTreeMap<u32, u32> = (0u32..).zip(&bodies).map(|(i, &b)| (b, i)).collect();
    let mut motions = Vec::new();
    let mut contacts = Vec::new();
    for _ in 0..ROUNDS {
        for _ in 0..STEPS_PER_ROUND {
            CONTACTS.lock().unwrap().clear();
            step(&mut e, world);
            let made = std::mem::take(&mut *CONTACTS.lock().unwrap());
            contacts.push(
                made.into_iter()
                    .map(|c| Contact {
                        a: number.get(&c.a).copied().unwrap_or(u32::MAX),
                        b: number.get(&c.b).copied().unwrap_or(u32::MAX),
                        thread: c.thread,
                    })
                    .collect(),
            );
        }
        motions.push(
            bodies
                .iter()
                .map(|&b| e.mem.bytes(b + MOTION, MOTION_SIZE))
                .collect(),
        );
    }
    let positions = bodies
        .iter()
        .map(|&b| [0, 4, 8].map(|lane| e.mem.f32(b + POSITION + lane)))
        .collect();
    let awake = bodies.iter().map(|&b| is_awake(&e, world, b)).collect();
    // The worker threads stay parked on the thread host; the engine is not torn down under them.
    std::mem::forget(e);
    Run {
        motions,
        contacts,
        positions,
        awake,
    }
}

/// The largest difference of the first three lanes of the vectors at `offsets` between two runs'
/// bodies after each round (NaN counts as an infinite gap).
fn largest_gap(a: &Run, b: &Run, offsets: &[u32]) -> Vec<f32> {
    a.motions
        .iter()
        .zip(&b.motions)
        .map(|(x, y)| {
            x.iter()
                .zip(y)
                .flat_map(|(p, q)| {
                    offsets.iter().flat_map(move |&at| {
                        let (p, q) = (lanes(p, at), lanes(q, at));
                        (0..3).map(move |lane| gap(p[lane], q[lane]))
                    })
                })
                .fold(0.0, f32::max)
        })
        .collect()
}

fn pairs(contacts: &[Contact]) -> Vec<(u32, u32)> {
    contacts
        .iter()
        .map(|c| (c.a.min(c.b), c.a.max(c.b)))
        .collect()
}

#[test]
#[ignore = "needs the player's FalloutNV.exe (set NV_DATA)"]
fn the_boxes_fall_and_come_to_rest_on_the_ground() {
    let _one = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let run = run(CONTINUOUS, 1);
    assert_at_rest("continuous", &run);
    let total: usize = run.contacts.iter().map(Vec::len).sum();
    println!(
        "continuous: {total} contact points added over {} steps",
        run.contacts.len()
    );
    assert!(total > 0, "no contact point was added");
}

#[test]
#[ignore = "needs the player's FalloutNV.exe (set NV_DATA)"]
fn the_multithreaded_step_leaves_the_bodies_within_a_centimetre_of_the_continuous_one() {
    let _one = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let single = run(CONTINUOUS, 1);
    let multi = run(MULTITHREADED, 3);
    let gaps = largest_gap(&single, &multi, &[POSITION]);
    let turns = largest_gap(
        &single,
        &multi,
        &[ROTATION, ROTATION + 0x10, ROTATION + 0x20],
    );
    let differing: Vec<usize> = single
        .motions
        .iter()
        .zip(&multi.motions)
        .map(|(x, y)| x.iter().zip(y).filter(|(p, q)| p != q).count())
        .collect();
    println!("largest position gap after each 20 steps: {gaps:?}");
    println!("largest rotation-matrix entry gap after each 20 steps: {turns:?}");
    println!("bodies whose motion bytes differ after each 20 steps: {differing:?}");
    let mut threads: BTreeMap<u32, usize> = BTreeMap::new();
    for c in multi.contacts.iter().flatten() {
        *threads.entry(c.thread).or_default() += 1;
    }
    println!("multithreaded contact points by thread: {threads:?}");
    let mut same_order = 0;
    let mut same_set = 0;
    for (s, m) in single.contacts.iter().zip(&multi.contacts) {
        let (ps, pm) = (pairs(s), pairs(m));
        if ps == pm {
            same_order += 1;
        }
        let (mut ss, mut sm) = (ps, pm);
        ss.sort_unstable();
        sm.sort_unstable();
        if ss == sm {
            same_set += 1;
        }
    }
    let steps = single.contacts.len();
    println!("steps with the same contact points: {same_set} of {steps}; in the same order: {same_order}");
    // What the first run found (FalloutNV.exe 1.4.0.525): the two steps are not bit for bit the same. Every box's
    // motion bytes differ from the first 20 steps on, but the boxes come to rest at most 0.07 Havok units (7 mm)
    // from where the continuous step leaves them and stay there; the contact points are the same in 238 of the
    // 240 steps (in the same order in 236), and some are added on a worker thread. Held here as found.
    for (round, gap) in gaps.iter().enumerate() {
        assert!(
            *gap < 0.1,
            "after {} steps the bodies are {gap} apart",
            (round + 1) * 20
        );
    }
    assert!(
        gaps[ROUNDS - 1] <= gaps[0] + 0.01,
        "the gap grows: {gaps:?}"
    );
    assert!(
        same_set * 100 >= steps * 98,
        "the contact points differ in {} steps",
        steps - same_set
    );
    assert!(
        threads.len() > 1,
        "every contact point was added on the stepping thread: {threads:?}"
    );
    // Both come to rest on the slab, the same boxes asleep, turned alike: a rotation-matrix entry
    // within 0.02 (about a degree).
    assert_at_rest("continuous", &single);
    assert_at_rest("multithreaded", &multi);
    assert_eq!(
        single.awake, multi.awake,
        "the two steps leave different boxes awake"
    );
    for (round, turn) in turns.iter().enumerate() {
        assert!(
            *turn < 0.02,
            "after {} steps the boxes are turned {turn} apart",
            (round + 1) * 20
        );
    }
}
