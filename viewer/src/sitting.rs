//! People using furniture, picking idles and sandboxing, the game's way
//! (`world::furniture`, `world::idles`, `world::sandbox`, read from the
//! game's code). Each frame `ai::move_actors` hands every person here:
//!
//! - Furniture ([`furniture_frame`]): someone heading for a chair walks to
//!   its marker, turns to the marker's heading, and the sit procedure runs
//!   (the entry animation, moving them by its root, then the seated loop);
//!   getting up plays the exit the same way. Whoever the state has in
//!   furniture without that (a script's `Activate`, a loaded game) is
//!   seated at once.
//! - Idles ([`idles_frame`]): once a second of free time near the player,
//!   the idle tree is asked (`IdleClock`) and what it gives plays over
//!   everything.
//! - Sandbox packages ([`sandbox_frame`]): what to do next is chosen by
//!   the game's weights (`Sandbox::choose`), for the game's durations, and
//!   carried out (see [`Activity`]).

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::{Component, Resource};
use esm::{FormId, LoadOrder};
use world::ai::NavMesh;
use world::furniture::{self, Animations, MarkerSettings, SitState, Sitter, Step};
use world::idles::{procedures, Idle, IdleAsker, IdleClock, IdleMarker, IdleQuestion, IdleTree};
use world::sandbox::{self, activities, Candidate, Choice, Kind, Nearby, Sandbox};
use world::scripting::{Facts, GameState};

use crate::actors::ActorRig;
use crate::ai::{face, Chats, Walker};

/// The idle tree, and what's been read for sitting: animations, furniture
/// markers, marker settings, the sandbox settings.
#[derive(Resource)]
pub struct Seats {
    pub tree: IdleTree,
    roots: HashMap<String, Arc<Vec<FormId>>>,
    sequences: HashMap<String, Option<Arc<nif::Sequence>>>,
    markers: HashMap<FormId, Arc<Vec<nif::FurnitureMarker>>>,
    marker_settings: HashMap<u8, MarkerSettings>,
    pub sandbox: sandbox::Settings,
}

impl Seats {
    pub fn new(order: &LoadOrder) -> Seats {
        Seats {
            tree: IdleTree::load(order),
            roots: HashMap::new(),
            sequences: HashMap::new(),
            markers: HashMap::new(),
            marker_settings: HashMap::new(),
            sandbox: sandbox::Settings::read(order),
        }
    }

    /// An animation (a `.kf` under `meshes\`), read once.
    pub fn sequence(&mut self, game: &cellview::Game, model: &str) -> Option<Arc<nif::Sequence>> {
        load(game, &mut self.sequences, model)
    }

    /// Keep the same cached Arc on reload: special-idle request rejection
    /// compares sequence identity, not just its embedded display name.
    pub fn sequence_path(&self, sequence: &Arc<nif::Sequence>) -> Option<String> {
        self.sequences.iter().find_map(|(path, cached)| {
            cached
                .as_ref()
                .filter(|other| Arc::ptr_eq(sequence, other))
                .map(|_| path.clone())
        })
    }

    /// A piece of furniture's markers (by its base), read once.
    fn markers(&mut self, game: &cellview::Game, base: FormId) -> Arc<Vec<nif::FurnitureMarker>> {
        self.markers
            .entry(base)
            .or_insert_with(|| {
                Arc::new(
                    preview::furniture::markers(&game.assets, &game.order, base)
                        .unwrap_or_default(),
                )
            })
            .clone()
    }

    fn marker_settings(&mut self, order: &LoadOrder, number: u8) -> MarkerSettings {
        *self
            .marker_settings
            .entry(number)
            .or_insert_with(|| MarkerSettings::read(order, number))
    }

    /// The idle tree's roots for a skeleton.
    fn roots(&mut self, skeleton: &str) -> Arc<Vec<FormId>> {
        let tree = &self.tree;
        self.roots
            .entry(skeleton.to_ascii_lowercase())
            .or_insert_with(|| Arc::new(tree.roots_for(skeleton)))
            .clone()
    }
}

fn load(
    game: &cellview::Game,
    sequences: &mut HashMap<String, Option<Arc<nif::Sequence>>>,
    model: &str,
) -> Option<Arc<nif::Sequence>> {
    sequences
        .entry(model.to_ascii_lowercase())
        .or_insert_with(|| crate::viewmodel::sequence(game, model).map(Arc::new))
        .clone()
}

/// A placed piece of furniture's markers where it stands, and its `MNAM`,
/// from markers already read (`markers`: by base).
fn placed_markers(
    order: &LoadOrder,
    state: &GameState,
    markers: &HashMap<FormId, Arc<Vec<nif::FurnitureMarker>>>,
    furniture_ref: FormId,
) -> Option<(Vec<furniture::PlacedMarker>, u32)> {
    let base = world::scripting::base_of(order, furniture_ref)?;
    let m = markers.get(&base).filter(|m| !m.is_empty())?;
    let (_, _, at, heading) = state.place(order, furniture_ref)?;
    Some((
        furniture::place_markers(m, at, heading, reference_scale(order, furniture_ref)),
        furniture::marker_flags(order, base),
    ))
}

/// The animations' lengths and root travel, for the sit procedure.
struct Anims<'a> {
    game: &'a cellview::Game,
    sequences: &'a mut HashMap<String, Option<Arc<nif::Sequence>>>,
}

impl Animations for Anims<'_> {
    fn length(&mut self, model: &str) -> Option<f32> {
        let s = load(self.game, self.sequences, model)?;
        Some(s.stop - s.start)
    }

    fn root_offset(&mut self, model: &str, time: f32) -> [f32; 3] {
        load(self.game, self.sequences, model)
            .and_then(|s| s.root_offset(time))
            .unwrap_or([0.0; 3])
    }
}

/// A placed reference's scale (`XSCL`, else 1).
fn reference_scale(order: &LoadOrder, reference: FormId) -> f32 {
    order
        .get(reference)
        .and_then(|rr| rr.record().ok())
        .and_then(|r| {
            r.get(esm::FourCC::new(b"XSCL"))
                .filter(|s| s.data.len() >= 4)
                .map(|s| f32::from_le_bytes([s.data[0], s.data[1], s.data[2], s.data[3]]))
        })
        .unwrap_or(1.0)
}

/// Random numbers from one draw of the game state's dice.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        // xorshift64, as the state's.
        let mut x = self.0.max(1);
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

/// What a sandboxing person is doing for their current choice. Sitting
/// and sleeping are the game's (the furniture's sit procedure). Wandering
/// is the wander procedure (`008ed420`): a spot 32 to 0.75 × the area's
/// radius from its middle (how the game's navmesh search picks within that
/// ring isn't traced: here a random point of a random navmesh triangle
/// reaching into it), walked to within 50, then a pause by the wander timer
/// (`sandbox::WanderTimer`). An idle marker is walked to, faced and its
/// idles played one after another, in order or at random (`00901c70`).
/// Eating is the eat procedure (`008e3140`): food lying in the area is
/// walked to and taken, carried food used otherwise; a free chair nearby is
/// found and sat in, and seated they eat one, then ask for an eating idle
/// every 1–6 s; without a chair they eat where they stand. Talking starts a
/// conversation with the one chosen (`00901a20`, `ai::start_chat`).
#[derive(Debug, Clone, PartialEq)]
pub enum Activity {
    Furniture,
    Wander {
        timer: sandbox::WanderTimer,
    },
    IdleMarker {
        heading: f32,
        idles: IdleMarker,
        next: usize,
    },
    Eat(Eating),
    Talk {
        with: FormId,
    },
}

/// The eat procedure's progress.
#[derive(Debug, Clone, PartialEq)]
pub struct Eating {
    /// The food lying in the world to take first.
    pub food: Option<FormId>,
    /// The item they eat, once they hold it.
    pub item: Option<FormId>,
    /// The chair to eat in, once looked for.
    pub chair: Option<FormId>,
    pub searched: bool,
    pub eaten: bool,
    /// Seconds to the next eating idle.
    pub idle_in: f32,
}

/// What a person keeps for sitting, idles and sandboxing.
#[derive(Component, Default)]
pub struct Life {
    /// Their skeleton (for the idle tree's roots) and bound radius (for
    /// how near the player their idles play), once read.
    pub skeleton: Option<(String, f32)>,
    pub idles: IdleClock,
    /// A base loop the idle tree gave while standing.
    pub base_idle: Option<Arc<nif::Sequence>>,
    pub sandbox: Option<Sandbox>,
    pub activity: Option<Activity>,
    /// Getting up to go elsewhere: the package is looked at again once
    /// they're up.
    pub getting_up: bool,
}

/// What every person's frame needs.
pub struct Ctx<'a> {
    pub game: &'a cellview::Game,
    pub state: &'a mut GameState,
    pub seats: &'a mut Seats,
    pub mesh: &'a NavMesh,
    /// Turning and walking settings (`world::movement`).
    pub moves: &'a world::movement::MoveSettings,
    pub now: f32,
    pub dt: f32,
    /// In a fight (the fast exits, no idles).
    pub fighting: bool,
    /// Saying a line now (`IsTalking`).
    pub talking: bool,
}

/// The idle tree's view of someone, with `GetSitting`, `GetSleeping` and
/// `GetFurnitureMarkerID` as given.
fn question(
    walker: &Walker,
    life: &Life,
    order: &LoadOrder,
    (talking, fighting): (bool, bool),
    (sitting, sleeping, marker): (u8, u8, u8),
) -> IdleQuestion {
    let walking = walker.next < walker.path.len();
    let procedure = match &life.activity {
        _ if fighting => procedures::COMBAT,
        // A sandbox: by where it is and what it's doing, as the game's
        // `GetCurrentAIProcedure` reads it (`world::more_functions::
        // procedures::sandbox_number`).
        _ if life.sandbox.is_some() => {
            let sandbox = life.sandbox.as_ref().expect("checked");
            let phase = match sandbox.phase {
                world::sandbox::Phase::Init => 0,
                world::sandbox::Phase::GettingUp => 1,
                world::sandbox::Phase::Moving => 2,
                world::sandbox::Phase::Doing => 3,
            };
            world::more_functions::procedures::sandbox_number(
                phase,
                sandbox.choice.as_ref().map(|c| c.activity),
            )
        }
        Some(Activity::Wander { .. }) => procedures::WANDER,
        Some(Activity::Eat(_)) => procedures::EAT,
        Some(Activity::Talk { .. }) => procedures::DIALOGUE,
        Some(Activity::Furniture) if sleeping > 0 => procedures::SLEEP,
        _ if walking => procedures::TRAVEL,
        _ => procedures::NONE,
    };
    let used_item = match &life.activity {
        Some(Activity::Eat(e)) => e
            .item
            .or_else(|| e.food.and_then(|f| world::scripting::base_of(order, f))),
        _ => None,
    };
    IdleQuestion {
        sitting,
        sleeping,
        marker,
        procedure,
        moving: walking,
        talking,
        alerted: fighting,
        last_idle: life.idles.last,
        used_item,
        female: walker.female,
        player: false,
        child: world::idles::is_child(order, walker.reference),
    }
}

/// Someone's skeleton and bound radius: the radius is half their bounds'
/// (`OBND`) diagonal (the game uses its 3D's bound radius; taking the
/// record's bounds for it is a guess), 64 without.
fn skeleton_of(order: &LoadOrder, reference: FormId) -> (String, f32) {
    let base = world::scripting::base_of(order, reference);
    let skeleton = base
        .and_then(|b| world::actor_look(order, b))
        .map(|l| l.skeleton)
        .unwrap_or_default();
    let radius = base
        .and_then(|b| order.get(b))
        .and_then(|rr| rr.record().ok())
        .and_then(|r| {
            r.get(esm::FourCC::new(b"OBND"))
                .filter(|s| s.data.len() >= 12)
                .map(|s| {
                    let v = |i: usize| {
                        f32::from(i16::from_le_bytes([s.data[i * 2], s.data[i * 2 + 1]]))
                    };
                    let d = [v(3) - v(0), v(4) - v(1), v(5) - v(2)];
                    0.5 * (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
                })
        })
        .filter(|r| *r > 1.0)
        .unwrap_or(64.0);
    (skeleton, radius)
}

fn skeleton(life: &mut Life, order: &LoadOrder, reference: FormId) -> (String, f32) {
    life.skeleton
        .get_or_insert_with(|| skeleton_of(order, reference))
        .clone()
}

/// The idle the tree gives someone now, as (idle, `.kf`).
#[allow(clippy::too_many_arguments)]
fn pick_idle(
    order: &LoadOrder,
    state: &GameState,
    tree: &IdleTree,
    roots: &[FormId],
    walker: &Walker,
    life: &Life,
    flags: (bool, bool),
    values: (u8, u8, u8),
    seed: u64,
) -> Option<Idle> {
    let facts = Facts {
        order,
        state,
        speaker: None,
    };
    let about = question(walker, life, order, flags, values);
    let asker = IdleAsker::new(walker.reference, about, Some(&facts), seed);
    tree.evaluate(roots, &|i| asker.passes(i), &|id| life.idles.is_delayed(id))
        .cloned()
}

/// Starts using a placed piece of furniture: the nearest free marker its
/// `MNAM` allows is theirs (the state's `IsCurrentFurnitureRef` from now
/// on), and they walk to it. False when there's no marker free or no way
/// there.
pub fn begin_use(ctx: &mut Ctx, walker: &mut Walker, furniture_ref: FormId) -> bool {
    let me = walker.reference;
    let game = ctx.game;
    let order = &game.order;
    let Some(base) = world::scripting::base_of(order, furniture_ref) else {
        return false;
    };
    ctx.seats.markers(ctx.game, base);
    let Some((placed, flags)) = placed_markers(order, ctx.state, &ctx.seats.markers, furniture_ref)
    else {
        return false;
    };
    let taken = furniture::taken_markers(ctx.state, furniture_ref, me);
    let Some(marker) =
        furniture::nearest_free(&placed, flags, |i| taken.contains(&i), walker.position)
    else {
        return false;
    };
    let settings = ctx.seats.marker_settings(order, marker.number);
    let sitter = Sitter::new(
        furniture_ref,
        marker,
        settings,
        walker.position,
        walker.heading,
    );
    if sitter.in_reach(walker.position) {
        walker.clear_path();
    } else {
        match ctx.mesh.path(walker.position, marker.position) {
            Some(path) => walker.set_path(path, 0.0, true, ctx.moves),
            None => return false,
        }
    }
    println!(
        "{me} heads for {furniture_ref} (marker {}, number {})",
        marker.index, marker.number
    );
    walker.target = Some(marker.position);
    ctx.state.furniture.insert(me, furniture_ref);
    ctx.state.sitters.insert(me, sitter);
    true
}

/// Seated at once where the state has them (the game's "already seated"
/// path): the nearest free usable marker, else the first. False if the
/// furniture has none (they're let go).
fn seat_at_once(
    ctx: &mut Ctx,
    walker: &mut Walker,
    life: &mut Life,
    furniture_ref: FormId,
) -> bool {
    let me = walker.reference;
    let game = ctx.game;
    let order = &game.order;
    let (skeleton, _) = skeleton(life, order, me);
    let roots = ctx.seats.roots(&skeleton);
    if let Some(base) = world::scripting::base_of(order, furniture_ref) {
        ctx.seats.markers(ctx.game, base);
    }
    let taken = furniture::taken_markers(ctx.state, furniture_ref, me);
    let marker = placed_markers(order, ctx.state, &ctx.seats.markers, furniture_ref).and_then(
        |(p, flags)| {
            furniture::nearest_free(&p, flags, |i| taken.contains(&i), walker.position)
                .or_else(|| p.first().copied())
        },
    );
    let Some(marker) = marker else {
        ctx.state.stand(me);
        return false;
    };
    let settings = ctx.seats.marker_settings(order, marker.number);
    let seed = ctx.state.roll();
    let flags = (ctx.talking, ctx.fighting);
    let sitter = {
        let (state, tree) = (&*ctx.state, &ctx.seats.tree);
        let (w, l) = (&*walker, &*life);
        let mut pick = |sitting: u8, sleeping: u8, number: u8| {
            pick_idle(
                order,
                state,
                tree,
                &roots,
                w,
                l,
                flags,
                (sitting, sleeping, number),
                seed,
            )
            .map(|i| (i.form_id, i.model))
        };
        Sitter::seated(furniture_ref, marker, settings, walker.scale, &mut pick)
    };
    println!(
        "{me} is seated at {:.0},{:.0},{:.0} (marker {}){}",
        sitter.position[0],
        sitter.position[1],
        sitter.position[2],
        marker.number,
        if sitter.dynamic_idle.is_some() {
            ""
        } else {
            " (no seated idle)"
        }
    );
    walker.position = sitter.position;
    walker.heading = sitter.heading;
    walker.clear_path();
    ctx.state.sitters.insert(me, sitter);
    true
}

/// Lets go of a sandbox choice that couldn't be carried out: its target
/// weighs nothing till the next scan, and something else is chosen.
fn give_up(life: &mut Life) {
    if let Some(sb) = life.sandbox.as_mut() {
        sb.failed();
        sb.choice = None;
    }
    life.activity = None;
}

/// One frame of someone's furniture use: seated at once if the state has
/// them in furniture without the procedure; on the way, the turn to the
/// marker's heading; then the sit or stand procedure. Whether they're in
/// furniture now (sitting down, seated or getting up: not to walk or
/// fight). `rig` gets the seated loop and the entry or exit to play.
pub fn furniture_frame(
    ctx: &mut Ctx,
    walker: &mut Walker,
    life: &mut Life,
    rig: &mut ActorRig,
) -> bool {
    let me = walker.reference;
    let game = ctx.game;
    let order = &game.order;
    if let Some(&f) = ctx.state.furniture.get(&me) {
        if !ctx.state.sitters.contains_key(&me) && !seat_at_once(ctx, walker, life, f) {
            return false;
        }
    }
    let Some(mut sitter) = ctx.state.sitters.remove(&me) else {
        rig.dynamic_idle = life.base_idle.clone();
        return false;
    };
    if ctx.fighting {
        sitter.stand_up();
    }
    // On the way: walk the path, then turn to the marker's heading
    // (`00921350`), then sit.
    if sitter.state == SitState::Normal {
        rig.dynamic_idle = life.base_idle.clone();
        if walker.next < walker.path.len() {
            ctx.state.sitters.insert(me, sitter);
            return false;
        }
        if sitter.stand_requested {
            ctx.state.stand(me);
            return false;
        }
        if !sitter.in_reach(walker.position) {
            // No path (cleared when a script moved them): a new one.
            if walker.path.is_empty() {
                if let Some(path) = ctx.mesh.path(walker.position, sitter.marker.position) {
                    walker.set_path(path, 0.0, true, ctx.moves);
                    ctx.state.sitters.insert(me, sitter);
                    return false;
                }
            }
            println!(
                "{me} can't reach {} (at {:.0},{:.0},{:.0}, its marker at {:.0},{:.0},{:.0}; path {}/{})",
                sitter.furniture,
                walker.position[0],
                walker.position[1],
                walker.position[2],
                sitter.marker.position[0],
                sitter.marker.position[1],
                sitter.marker.position[2],
                walker.next,
                walker.path.len()
            );
            ctx.state.stand(me);
            give_up(life);
            return false;
        }
        // The turn to the marker's heading: in place, the game's turn
        // (`00921350` → `008bb5c0`).
        if face(walker, sitter.marker.heading, ctx.dt, false, ctx.moves) {
            ctx.state.sitters.insert(me, sitter);
            return true;
        }
        sitter.position = walker.position;
        sitter.heading = walker.heading;
    }
    // The procedure's step, the tree asked with its sit state.
    let (skeleton, _) = skeleton(life, order, me);
    let roots = ctx.seats.roots(&skeleton);
    let seated_idle = life.idles.playing.is_some();
    // The dice only when the tree will be asked (sitting down or getting
    // up), so a run repeats however fast frames come.
    let asks = sitter.state == SitState::Normal
        || (sitter.state.is_settled() && sitter.stand_requested && !seated_idle);
    let seed = if asks { ctx.state.roll() } else { 0 };
    let flags = (ctx.talking, ctx.fighting);
    let before = sitter.state;
    let step = {
        let state = &*ctx.state;
        let Seats {
            tree, sequences, ..
        } = &mut *ctx.seats;
        let tree = &*tree;
        let (w, l) = (&*walker, &*life);
        let mut pick = |sitting: u8, sleeping: u8, number: u8| {
            pick_idle(
                order,
                state,
                tree,
                &roots,
                w,
                l,
                flags,
                (sitting, sleeping, number),
                seed,
            )
            .map(|i| (i.form_id, i.model))
        };
        let mut anims = Anims {
            game: ctx.game,
            sequences,
        };
        sitter.update(ctx.dt, seated_idle, &mut pick, &mut anims)
    };
    if sitter.state != before {
        println!(
            "{:.1} s: {me}: {} -> {}{}",
            ctx.now,
            before.name(),
            sitter.state.name(),
            sitter
                .playing
                .as_ref()
                .map(|p| format!(", playing {}", p.model))
                .unwrap_or_default()
        );
    }
    walker.position = sitter.position;
    walker.heading = sitter.heading;
    match step {
        Step::Released | Step::Failed => {
            if step == Step::Failed {
                println!("{me} has no way to sit in {}", sitter.furniture);
                give_up(life);
            }
            ctx.state.stand(me);
            rig.dynamic_idle = life.base_idle.clone();
            rig.overlay = None;
            if life.getting_up {
                life.getting_up = false;
                walker.evaluate = true;
            }
            false
        }
        Step::Busy | Step::Settled => {
            rig.dynamic_idle = sitter
                .dynamic_idle
                .as_ref()
                .and_then(|(_, model)| ctx.seats.sequence(ctx.game, model));
            rig.overlay = sitter
                .playing
                .as_ref()
                .and_then(|p| Some((ctx.seats.sequence(ctx.game, &p.model)?, p.elapsed)));
            ctx.state.sitters.insert(me, sitter);
            true
        }
    }
}

/// One frame of someone's idles: the playing one goes on; free near the
/// player (not walking, fighting, sitting down or getting up), the tree is
/// asked once a second ([`IdleClock`]). What it gives plays over
/// everything (`rig.overlay`), unless sitting down or getting up plays.
/// Walking or a fight stops an idle (a guess: how the game cuts them
/// short isn't traced). At an idle marker, its idles play instead of the
/// tree's ([`sandbox_frame`]).
pub fn idles_frame(ctx: &mut Ctx, walker: &Walker, life: &mut Life, rig: &mut ActorRig) {
    // The process does not ask for a free idle while a requested one
    // owns its animation (008dafd0). Its KF, not IdleClock, advances it.
    if rig.scripted_idle.is_some() {
        life.idles.stop();
        rig.overlay = None;
    }
    let me = walker.reference;
    let game = ctx.game;
    let order = &game.order;
    let walking = walker.next < walker.path.len();
    if ctx.fighting || walking {
        life.idles.stop();
    }
    life.idles.advance(ctx.dt);
    let sitter = ctx.state.sitters.get(&me);
    let values = sitter.map_or((0, 0, 0), |s| s.question());
    let entering = sitter.is_some_and(|s| s.playing.is_some() || !s.state.is_settled());
    let (skeleton, radius) = skeleton(life, order, me);
    let near = ctx.state.player_position.is_some_and(|p| {
        let d = (0..3)
            .map(|k| (p[k] - walker.position[k]).powi(2))
            .sum::<f32>();
        d.sqrt() <= world::idles::IDLE_ANIMATION_DISTANCE * radius / 64.0
    });
    let at_marker = matches!(life.activity, Some(Activity::IdleMarker { .. }));
    let free = near
        && !walking
        && !ctx.fighting
        && !entering
        && !life.getting_up
        && !at_marker
        && rig.scripted_idle.is_none();
    if life.idles.due(ctx.dt, free) {
        let roots = ctx.seats.roots(&skeleton);
        let seed = ctx.state.roll();
        let flags = (ctx.talking, ctx.fighting);
        let found = pick_idle(
            order,
            ctx.state,
            &ctx.seats.tree,
            &roots,
            walker,
            life,
            flags,
            values,
            seed,
        );
        if let Some(idle) = found {
            let standing = sitter_is_none(ctx.state, me);
            play(ctx, life, &idle, standing);
        }
    }
    if !ctx
        .state
        .sitters
        .get(&me)
        .is_some_and(|s| s.playing.is_some())
    {
        rig.overlay = life
            .idles
            .playing
            .as_ref()
            .and_then(|p| Some((ctx.seats.sequence(ctx.game, &p.model)?, p.elapsed)));
    }
    // What scripts ask about them (`world::more_functions`): walking
    // forward along their path (whether they run isn't told), the last idle
    // played, the procedure.
    let about = question(walker, life, order, (ctx.talking, ctx.fighting), values);
    world::more_functions::report(
        ctx.state,
        me,
        world::more_functions::Seen {
            movement: if walking {
                world::more_functions::movement::FORWARD
            } else {
                0
            },
            last_idle: life.idles.last,
            procedure: (about.procedure != procedures::NONE).then_some(about.procedure),
            swimming: false,
            idle_playing: rig.scripted_idle.is_some() || life.idles.playing.is_some(),
        },
    );
}

fn sitter_is_none(state: &GameState, me: FormId) -> bool {
    !state.sitters.contains_key(&me)
}

/// Plays an idle: a base-loop one (section 0) becomes the standing loop
/// (seated people's comes from their seat); any other plays over
/// everything its rolled number of times.
fn play(ctx: &mut Ctx, life: &mut Life, idle: &Idle, standing: bool) {
    if idle.group() == 0 {
        if standing {
            life.base_idle = ctx.seats.sequence(ctx.game, &idle.model);
        }
        return;
    }
    let Some(seq) = ctx.seats.sequence(ctx.game, &idle.model) else {
        return;
    };
    let r = ctx.state.roll();
    life.idles.start(idle, seq.stop - seq.start, |lo, hi| {
        lo + (r % u64::from(hi.saturating_sub(lo) + 1)) as u8
    });
    if let Some(p) = &life.idles.playing {
        println!(
            "{:.1} s: plays the idle {} ({}, {:.1} s, {} more times)",
            ctx.now,
            idle.editor_id,
            idle.model.rsplit(['\\', '/']).next().unwrap_or_default(),
            p.length,
            p.loops_left
        );
    }
}

/// The game's clock as the sandbox counts it: days passed, and the hour.
fn clock(order: &LoadOrder, state: &GameState) -> (i32, f32) {
    (
        state.global(order, "GameDaysPassed").unwrap_or(0.0) as i32,
        state.global(order, "GameHour").unwrap_or(12.0),
    )
}

/// Whether someone's idle marker has an idle that passes for them, with
/// its parents (`00479fb0`).
fn marker_usable(ctx: &mut Ctx, walker: &Walker, life: &Life, marker: &IdleMarker) -> bool {
    let game = ctx.game;
    let order = &game.order;
    let seed = ctx.state.roll();
    let facts = Facts {
        order,
        state: ctx.state,
        speaker: None,
    };
    let about = question(walker, life, order, (ctx.talking, ctx.fighting), (0, 0, 0));
    let asker = IdleAsker::new(walker.reference, about, Some(&facts), seed);
    marker
        .idles
        .iter()
        .any(|&id| ctx.seats.tree.passes_with_parents(id, &|i| asker.passes(i)))
}

/// What's around a sandboxing person for [`Sandbox::scan`]: the
/// references in the cell they're in (outdoors, their square) and people
/// brought there, with what each is; and whether they carry food.
fn nearby_list(ctx: &mut Ctx, walker: &Walker, life: &Life) -> (Vec<Nearby>, bool) {
    let me = walker.reference;
    let game = ctx.game;
    let order = &game.order;
    let Some((space, cell, _, _)) = ctx.state.place(order, me) else {
        return (Vec::new(), false);
    };
    let mut refs: Vec<FormId> = order
        .references_in_cell(cell)
        .into_iter()
        .filter(|rr| !rr.entry.header.is_deleted())
        .map(|rr| rr.form_id)
        .collect();
    refs.extend(world::ai::moved_into(order, ctx.state, space));
    let child = world::idles::is_child(order, me);
    let mut out = Vec::new();
    for r in refs {
        if r == me
            || ctx.state.dead.contains(&r)
            || !world::enabled_now(order, r, &ctx.state.disabled)
            || sandbox::ignored(order, r)
            || (child && !sandbox::child_can_use(order, r))
        {
            continue;
        }
        let Some(kind) = sandbox::kind_of(order, r) else {
            continue;
        };
        let Some((there, _, position, _)) = ctx.state.place(order, r) else {
            continue;
        };
        if there != space {
            continue;
        }
        let usable = match kind {
            Kind::Furniture(_) => world::scripting::base_of(order, r)
                .is_some_and(|b| !ctx.seats.markers(ctx.game, b).is_empty()),
            Kind::IdleMarker => {
                IdleMarker::load(order, r).is_some_and(|m| marker_usable(ctx, walker, life, &m))
            }
            _ => true,
        };
        out.push(Nearby {
            reference: r,
            kind,
            position,
            allowed: sandbox::may_use(order, ctx.state, me, r),
            usable,
        });
    }
    let food = ctx.state.inventory(order, me).iter().any(|(item, n)| {
        *n > 0
            && order
                .get(*item)
                .is_some_and(|r| matches!(r.entry.header.kind.as_bytes(), b"INGR" | b"ALCH"))
    });
    (out, food)
}

/// Where a candidate's target is now, if it can be used (`009f4700`'s
/// checks): in the same place and enabled; furniture with a usable marker
/// free (`00568260`); people alive and not fighting or in a conversation
/// (the game asks whether they can talk, `008b06d0`).
fn current_position(
    order: &LoadOrder,
    state: &GameState,
    markers: &HashMap<FormId, Arc<Vec<nif::FurnitureMarker>>>,
    me: FormId,
    here: Option<FormId>,
    c: &Candidate,
) -> Option<[f32; 3]> {
    let t = c.target?;
    let (space, _, position, _) = state.place(order, t)?;
    if Some(space) != here || !world::enabled_now(order, t, &state.disabled) {
        return None;
    }
    match c.activity {
        activities::SIT | activities::SLEEP => {
            let (placed, flags) = placed_markers(order, state, markers, t)?;
            let taken = furniture::taken_markers(state, t, me);
            furniture::first_free(&placed, flags, |i| taken.contains(&i))?;
        }
        activities::DIALOGUE if state.dead.contains(&t) || state.combat.contains_key(&t) => {
            return None;
        }
        _ => {}
    }
    Some(position)
}

/// The nearest sitting furniture with a free marker within
/// `fAIFindBedChairsDistance` (8000, `00922670`) of someone, for eating.
fn nearest_free_chair(ctx: &mut Ctx, walker: &Walker) -> Option<FormId> {
    let me = walker.reference;
    let game = ctx.game;
    let order = &game.order;
    let (space, cell, _, _) = ctx.state.place(order, me)?;
    let reach = world::scripting::game_setting(order, "fAIFindBedChairsDistance").unwrap_or(8000.0);
    let mut best: Option<(f32, FormId)> = None;
    for rr in order.references_in_cell(cell) {
        let r = rr.form_id;
        let Some(Kind::Furniture(mnam)) = sandbox::kind_of(order, r) else {
            continue;
        };
        if mnam & furniture::SIT_FURNITURE == 0
            || !world::enabled_now(order, r, &ctx.state.disabled)
        {
            continue;
        }
        if let Some(b) = world::scripting::base_of(order, r) {
            ctx.seats.markers(ctx.game, b);
        }
        let c = Candidate {
            activity: activities::SIT,
            target: Some(r),
            weight: 1,
        };
        let Some(p) = current_position(order, ctx.state, &ctx.seats.markers, me, Some(space), &c)
        else {
            continue;
        };
        let d = (0..3)
            .map(|k| (p[k] - walker.position[k]).powi(2))
            .sum::<f32>()
            .sqrt();
        if d <= reach && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, r));
        }
    }
    best.map(|b| b.1)
}

/// A walk to a wander spot (`008ed420`): a point of the navmesh between
/// 32 and 0.75 × the radius from the middle (`world::ai::wander_ring`),
/// at random (how the game's search picks within the ring isn't traced:
/// here a random point of a random triangle reaching into it, tried a few
/// times), walked to within 50. How far away it is, if there's a way.
fn wander_to(ctx: &mut Ctx, walker: &mut Walker, center: [f32; 3], radius: f32) -> Option<f32> {
    let mesh = ctx.mesh;
    let (near, far) = world::ai::wander_ring(radius);
    if near > far {
        return None;
    }
    let flat = |p: [f32; 3]| (p[0] - center[0]).hypot(p[1] - center[1]);
    let corners = |t: &world::ai::NavTriangle| t.vertices.map(|v| mesh.vertices[v]);
    let reaching: Vec<[[f32; 3]; 3]> = mesh
        .triangles
        .iter()
        .map(corners)
        .filter(|c| c.iter().any(|p| flat(*p) <= far) || holds(c, center))
        .collect();
    if reaching.is_empty() {
        return None;
    }
    let mut dice = Dice(ctx.state.roll());
    let mut unit = || (dice.next() % 1_000_000) as f32 / 1_000_000.0;
    for _ in 0..16 {
        let [a, b, c] = reaching[(unit() * reaching.len() as f32) as usize % reaching.len()];
        let (mut u, mut v) = (unit(), unit());
        if u + v > 1.0 {
            (u, v) = (1.0 - u, 1.0 - v);
        }
        let goal = [0, 1, 2].map(|k| a[k] + u * (b[k] - a[k]) + v * (c[k] - a[k]));
        let d = flat(goal);
        if d < near || d > far {
            continue;
        }
        if let Some(path) = mesh.path(walker.position, goal) {
            let d = crate::ai::distance(walker.position, goal);
            walker.set_path(path, 50.0, true, ctx.moves);
            return Some(d);
        }
    }
    None
}

/// The wander procedure's spot-to-spot loop (`008ed420`), standing: the
/// timer (`sandbox::WanderTimer`) let go within 5 of the middle; at 10 or
/// less a new spot ([`wander_to`]) and the timer set to `pause` when it's
/// 50 or more away. When no spot can be reached the timer stays, so it's
/// tried again next frame, as the code does (they stand meanwhile).
fn wander_about(
    ctx: &mut Ctx,
    walker: &mut Walker,
    timer: &mut sandbox::WanderTimer,
    middle: [f32; 3],
    radius: f32,
    pause: f32,
) {
    let to_middle = crate::ai::distance(walker.position, middle);
    if timer.idle(ctx.dt, to_middle) {
        if let Some(d) = wander_to(ctx, walker, middle, radius) {
            timer.chose(d, pause);
        }
    }
}

/// A wander package (type 5) once the walk to its place is over, each
/// frame (`008ed420` on the package's list 1, `world::ai::wander_step`):
/// with a radius under 60 they stand, turned to an `XMarkerHeading`'s
/// heading if the place is one; farther than the radius + 250 from the
/// middle they walk back to the place (the travel again); otherwise from
/// spot to spot ([`wander_about`]) with the wander pause by their energy.
/// The middle: the place's reference (or editor location), their own
/// position for "in a cell", where they stood when the package began for
/// "near the current location" (as for sandboxes; inferred).
pub fn wander_package_frame(ctx: &mut Ctx, walker: &mut Walker, life: &mut Life) {
    if walker.on_path() {
        return;
    }
    let game = ctx.game;
    let order = &game.order;
    let me = walker.reference;
    if ctx.state.furniture.contains_key(&me) || ctx.state.sitters.contains_key(&me) {
        return;
    }
    let Some(package) = walker
        .package
        .and_then(|p| world::ai::Package::load(order, p))
    else {
        return;
    };
    let interior = ctx.state.player_world.is_none();
    let (radius, own) = world::ai::wander_radius(order, &package, interior);
    let place = world::ai::destination(order, ctx.state, me, &package);
    let middle = if own {
        walker.position
    } else {
        place.map_or(walker.home, |(to, _)| to)
    };
    let at_place = place.is_none_or(|(to, r)| world::movement::arrived(walker.position, to, r));
    let to_middle = crate::ai::distance(walker.position, middle);
    match world::ai::wander_step(radius, own, at_place, to_middle) {
        world::ai::WanderStep::Back => {
            life.activity = None;
            let (to, r) = place.unwrap_or((middle, radius));
            if let Some(path) = ctx.mesh.path(walker.position, to) {
                println!("{me} wanders back to their package's place");
                walker.set_path(path, r, true, ctx.moves);
            }
        }
        world::ai::WanderStep::Stand => {
            life.activity = None;
            if let Some(h) = world::ai::marker_heading(order, ctx.state, &package) {
                let off = world::movement::wrap_pi(walker.heading - h).abs();
                if walker.facing.is_none() && off > world::movement::ONE_DEGREE {
                    walker.facing = Some(h);
                }
            }
        }
        world::ai::WanderStep::Wander => {
            let mut timer = match &life.activity {
                Some(Activity::Wander { timer }) => *timer,
                _ => sandbox::WanderTimer::default(),
            };
            let energy = sandbox::energy(order, me);
            let pause = sandbox::wander_pause(&ctx.seats.sandbox, energy);
            wander_about(ctx, walker, &mut timer, middle, radius, pause);
            life.activity = Some(Activity::Wander { timer });
        }
    }
}

/// Whether a triangle holds a point, seen from above.
fn holds(c: &[[f32; 3]; 3], p: [f32; 3]) -> bool {
    let side =
        |a: [f32; 3], b: [f32; 3]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    let (s0, s1, s2) = (side(c[0], c[1]), side(c[1], c[2]), side(c[2], c[0]));
    (s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0) || (s0 <= 0.0 && s1 <= 0.0 && s2 <= 0.0)
}

/// Sets out on a sandbox choice.
fn start_activity(
    ctx: &mut Ctx,
    walker: &mut Walker,
    life: &mut Life,
    c: Choice,
    chats: &mut Chats,
    social: &world::social::SocialSettings,
) {
    let me = walker.reference;
    let game = ctx.game;
    let order = &game.order;
    println!(
        "{:.1} s: {me} sandboxes: {}{} for {:.1} game minutes",
        ctx.now,
        activities::name(c.activity),
        c.target.map(|t| format!(" {t}")).unwrap_or_default(),
        c.duration
    );
    let started = match c.activity {
        activities::SIT | activities::SLEEP => {
            let ok = c.target.is_some_and(|t| begin_use(ctx, walker, t));
            if ok {
                life.activity = Some(Activity::Furniture);
            }
            ok
        }
        activities::EAT => {
            life.activity = Some(Activity::Eat(Eating {
                food: c.target,
                item: None,
                chair: None,
                searched: false,
                eaten: false,
                idle_in: 0.0,
            }));
            true
        }
        activities::WANDER => {
            life.activity = Some(Activity::Wander {
                timer: sandbox::WanderTimer::default(),
            });
            true
        }
        activities::IDLE_MARKER => {
            let marker = c
                .target
                .and_then(|t| Some((IdleMarker::load(order, t)?, ctx.state.place(order, t)?)));
            match marker {
                Some((idles, (_, _, at, heading))) => {
                    life.activity = Some(Activity::IdleMarker {
                        heading,
                        idles,
                        next: usize::MAX,
                    });
                    // Within 10 of the marker (`00901c70`).
                    if (0..2).all(|k| (at[k] - walker.position[k]).abs() < 10.0) {
                        true
                    } else {
                        match ctx.mesh.path(walker.position, at) {
                            Some(path) => {
                                walker.set_path(path, 10.0, true, ctx.moves);
                                true
                            }
                            None => false,
                        }
                    }
                }
                None => false,
            }
        }
        // A conversation with the one chosen (`00901a20`), its timer started
        // again.
        activities::DIALOGUE => match c.target {
            Some(with) => {
                life.activity = Some(Activity::Talk { with });
                let seated = ctx.state.sitters.contains_key(&me);
                let ok = crate::ai::start_chat(ctx, walker, chats, with, None, seated);
                if ok {
                    let mut dice = Dice(ctx.state.roll());
                    let unit = (dice.next() % 1_000_000) as f32 / 1_000_000.0;
                    if let Some(s) = walker.social.as_mut() {
                        let (lo, hi) = social.conversation_timer;
                        s.conversation = lo + (hi - lo) * unit;
                    }
                }
                ok
            }
            None => false,
        },
        _ => false,
    };
    if !started {
        println!("{me} can't {}", activities::name(c.activity));
        give_up(life);
    }
}

/// One frame of a sandbox package (`00929fc0`): back to the area if
/// they've strayed; when it's time for something else (and no sitting or
/// idle-marker idle is playing, `009f4300`), up from any seat first, then
/// a new choice (`Sandbox::choose`, the area scanned when due; talking
/// weighs nothing while their conversation timer runs) and off to it; the
/// current activity carried on.
pub fn sandbox_frame(
    ctx: &mut Ctx,
    walker: &mut Walker,
    life: &mut Life,
    chats: &mut Chats,
    social: &world::social::SocialSettings,
) {
    let me = walker.reference;
    let Some(mut sb) = life.sandbox.take() else {
        return;
    };
    let game = ctx.game;
    let order = &game.order;
    let now_clock = clock(order, ctx.state);
    let sit = ctx.state.sitters.get(&me).map(|s| s.state);
    let seated = sit.is_some_and(|s| s != SitState::Normal);
    let walking = walker.on_path();
    if !seated && !walking && sb.strayed(walker.position) {
        if let Some(path) = ctx.mesh.path(walker.position, sb.center) {
            println!("{me} goes back to their sandbox area");
            walker.set_path(path, 0.0, true, ctx.moves);
        }
        sb.choice = None;
        life.activity = None;
        life.sandbox = Some(sb);
        return;
    }
    let idle_busy = life.idles.playing.is_some()
        && matches!(
            life.activity,
            Some(Activity::Furniture) | Some(Activity::IdleMarker { .. })
        );
    if !idle_busy && sb.time_for_something_else(now_clock) {
        if seated {
            // Up first (`Actor::StandUp`); the choice waits until they are.
            if let Some(s) = ctx.state.sitters.get_mut(&me) {
                s.stand_up();
            }
            life.sandbox = Some(sb);
            return;
        }
        if sit == Some(SitState::Normal) {
            ctx.state.stand(me);
        }
        life.activity = None;
        walker.clear_path();
        let nearby = if sb.scan_due(ctx.now) {
            Some(nearby_list(ctx, walker, life))
        } else {
            None
        };
        sb.conversation_wait = walker.social.as_ref().is_some_and(|s| s.conversation > 0.0);
        let mut dice = Dice(ctx.state.roll());
        let choice = {
            let state = &*ctx.state;
            let markers = &ctx.seats.markers;
            let here = state.place(order, me).map(|p| p.0);
            let current = |c: &Candidate| current_position(order, state, markers, me, here, c);
            sb.choose(
                me,
                &ctx.seats.sandbox,
                ctx.now,
                now_clock,
                &mut || dice.next(),
                nearby.as_ref().map(|(l, f)| (l.as_slice(), *f)),
                &current,
            )
        };
        if let Some(c) = choice {
            start_activity(ctx, walker, life, c, chats, social);
            // A choice that couldn't start is chosen again next frame.
            if life.activity.is_none() {
                sb.failed();
                sb.choice = None;
            }
        }
    }
    carry_on(ctx, walker, life, &mut sb);
    life.sandbox = Some(sb);
}

/// The current activity, each frame: wanderers pause and move on, at an
/// idle marker they face its heading and play its idles, eaters go on
/// eating.
fn carry_on(ctx: &mut Ctx, walker: &mut Walker, life: &mut Life, sb: &mut Sandbox) {
    let game = ctx.game;
    let order = &game.order;
    match life.activity.clone() {
        // The wander procedure for a sandbox (`008ed420` with package type
        // 12: no "under 60 stand" and no radius + 250 tests, those are wander
        // packages').
        Some(Activity::Wander { mut timer }) => {
            if walker.on_path() {
                return;
            }
            // No spot (a radius under 43 leaves an empty ring): they stand,
            // tried again next frame; the procedure doesn't fail the
            // activity, which lasts its time.
            let pause = sandbox::wander_pause(&ctx.seats.sandbox, sb.energy);
            wander_about(ctx, walker, &mut timer, sb.center, sb.radius, pause);
            life.activity = Some(Activity::Wander { timer });
        }
        Some(Activity::IdleMarker {
            heading,
            idles,
            next,
        }) => {
            if walker.on_path() {
                return;
            }
            if face(walker, heading, ctx.dt, false, ctx.moves) {
                return;
            }
            if life.idles.playing.is_some() {
                return;
            }
            // Its usable idles, the next in order or one at random.
            let seed = ctx.state.roll();
            let usable: Vec<Idle> = {
                let facts = Facts {
                    order,
                    state: ctx.state,
                    speaker: None,
                };
                let about = question(walker, life, order, (ctx.talking, ctx.fighting), (0, 0, 0));
                let asker = IdleAsker::new(walker.reference, about, Some(&facts), seed);
                idles
                    .idles
                    .iter()
                    .filter(|&&id| ctx.seats.tree.passes_with_parents(id, &|i| asker.passes(i)))
                    .filter_map(|&id| ctx.seats.tree.get(id).cloned())
                    .collect()
            };
            if usable.is_empty() {
                return;
            }
            let i = if idles.in_sequence() {
                if next == usize::MAX || next + 1 >= usable.len() {
                    0
                } else {
                    next + 1
                }
            } else {
                (seed % usable.len() as u64) as usize
            };
            let standing = sitter_is_none(ctx.state, walker.reference);
            play(ctx, life, &usable[i], standing);
            life.activity = Some(Activity::IdleMarker {
                heading,
                idles,
                next: i,
            });
        }
        Some(Activity::Eat(e)) => eat_frame(ctx, walker, life, sb, e),
        // The conversation runs in `ai::chat_frame`.
        _ => {}
    }
}

/// One frame of eating (the eat procedure `008e3140`, as findings §5a read
/// it): food lying in the area is walked to (within its reach, as the
/// activate procedure measures: `world::movement::target_reach`) and taken
/// (`world::sandbox::take_food`), else food carried is used; then a free
/// chair is looked for once and sat in; seated, one is eaten
/// (`world::sandbox::eat`) and an eating idle asked for every random 1–6 s
/// with the food as the idle's item; without a chair they eat where they
/// stand. No food: the activity fails.
fn eat_frame(ctx: &mut Ctx, walker: &mut Walker, life: &mut Life, sb: &mut Sandbox, mut e: Eating) {
    let game = ctx.game;
    let order = &game.order;
    let me = walker.reference;
    if walker.on_path() {
        return;
    }
    let fail = |life: &mut Life, sb: &mut Sandbox| {
        println!("{me} has nothing to eat.");
        sb.failed();
        sb.choice = None;
        life.activity = None;
    };
    if e.item.is_none() {
        match e.food {
            Some(food) => {
                let Some((_, _, at, _)) = ctx.state.place(order, food) else {
                    fail(life, sb);
                    return;
                };
                let reach = world::movement::target_reach(
                    0,
                    world::ai::spot_of(order, ctx.state, food),
                    world::ai::min_location_radius(order),
                );
                let radius = walker
                    .kit
                    .as_ref()
                    .map_or(world::combat_ai::PERSON_RADIUS, |k| k.radius);
                if world::movement::within_distance(
                    walker.position,
                    128.0,
                    Some(radius),
                    at,
                    reach,
                    true,
                ) {
                    e.item = sandbox::take_food(order, ctx.state, me, food);
                    if e.item.is_none() {
                        fail(life, sb);
                        return;
                    }
                    println!("{:.1} s: {me} takes {food} to eat.", ctx.now);
                } else {
                    match ctx.mesh.path(walker.position, at) {
                        Some(path) => {
                            walker.set_path(path, reach, true, ctx.moves);
                            life.activity = Some(Activity::Eat(e));
                        }
                        None => fail(life, sb),
                    }
                    return;
                }
            }
            None => {
                e.item = sandbox::carried_food(order, ctx.state, me);
                if e.item.is_none() {
                    fail(life, sb);
                    return;
                }
            }
        }
    }
    if !e.searched {
        e.searched = true;
        e.chair = nearest_free_chair(ctx, walker);
        if let Some(chair) = e.chair {
            if !begin_use(ctx, walker, chair) {
                e.chair = None;
            }
        }
        life.activity = Some(Activity::Eat(e));
        return;
    }
    let seated = ctx
        .state
        .sitters
        .get(&me)
        .is_some_and(|s| s.state.is_settled());
    let item = e.item.expect("set above");
    if !e.eaten && (e.chair.is_none() || seated) {
        if sandbox::eat(order, ctx.state, me, item) {
            println!("{:.1} s: {me} eats {item}.", ctx.now);
        }
        e.eaten = true;
        e.idle_in = 0.0;
    }
    // Seated and eaten: an eating idle every 1–6 s.
    if e.eaten && seated {
        e.idle_in -= ctx.dt;
        if e.idle_in <= 0.0 && life.idles.playing.is_none() {
            let (lo, hi) = sandbox::EATING_IDLE_EVERY;
            let mut dice = Dice(ctx.state.roll());
            e.idle_in = lo + (hi - lo) * ((dice.next() % 1_000_000) as f32 / 1_000_000.0);
            life.activity = Some(Activity::Eat(e.clone()));
            let (skeleton, _) = skeleton(life, order, me);
            let roots = ctx.seats.roots(&skeleton);
            let values = ctx
                .state
                .sitters
                .get(&me)
                .map_or((0, 0, 0), |s| s.question());
            let seed = ctx.state.roll();
            let found = pick_idle(
                order,
                ctx.state,
                &ctx.seats.tree,
                &roots,
                walker,
                life,
                (ctx.talking, ctx.fighting),
                values,
                seed,
            );
            if let Some(idle) = found {
                play(ctx, life, &idle, false);
            }
        }
    }
    life.activity = Some(Activity::Eat(e));
}
