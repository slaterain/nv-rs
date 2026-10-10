//! The order of the viewer's per-frame systems, from the game's frame
//! (`Main::OnIdle`, Xbox PDB; PC `0086e650`; `world::frame`,
//! docs/FRAME_SKELETON.md "PR 3 result").
//!
//! Each of `world::frame`'s stages is a system set ([`FrameSet::Stage`]),
//! run in the exe's order ([`world::frame::stages`]), and so is each of its
//! steps ([`FrameSet::Step`], an index into `world::frame::STEPS`, inside
//! its stage, in call order). A system that is (part of) a step goes in
//! the step's set; one that belongs to a stage without being one step goes
//! in the stage's set. Each set runs under its gate from `world::frame`
//! ([`world::frame::step_runs`], [`world::frame::stage_reached`]),
//! evaluated on one [`FrameState`] ([`ThisFrame`]) filled once per frame
//! by [`begin_frame`], the first system of the first stage.
//!
//! Two sets run ahead of the stages and one after them
//! ([`ViewerSet`]); see there for what they hold and why.
//!
//! The player's step (`Main::OnIdle_UpdatePlayer`, step 14) is split
//! further into its calls and `PlayerCharacter::Update`'s sub-steps
//! ([`PlayerSet`], `world::frame::player`, docs/FRAME_SKELETON.md "PR 4
//! result"), under their gates on [`ThisPlayer`].
//!
//! The world and time stage's callees (`TES::TestAllCells`, `Calendar::Update`,
//! the process lists' passes, the garbage collector, `BSTreeManager::Update`,
//! `Main::OnIdle_UpdateCurrentGridCell` and `TES::UpdateCurrentGridCell`) are
//! split into their sub-steps ([`WorldSet`], `world::frame::world_time`,
//! docs/FRAME_SKELETON.md "PR 5 result"), under their gates on [`ThisWorld`].

use bevy::prelude::*;
use world::frame::player::{self, PlayerState, UPDATE, UPDATE_PLAYER};
use world::frame::world_time::{self, WorldState, FUNCTIONS};
use world::frame::{self, FrameState, Stage, STEPS};

/// The game's frame as system sets.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameSet {
    /// One of `world::frame`'s stages.
    Stage(Stage),
    /// One of its steps (an index into `world::frame::STEPS`).
    Step(usize),
}

impl FrameSet {
    /// The set of the first step that calls `address`.
    pub fn step(address: u32) -> FrameSet {
        FrameSet::Step(frame::step_of(address).expect("a call of Main::OnIdle"))
    }

    /// The set of the step after the first step that calls `address`.
    pub fn after(address: u32) -> FrameSet {
        FrameSet::Step(frame::step_of(address).expect("a call of Main::OnIdle") + 1)
    }
}

/// `Main::OnIdle_UpdateTimer` (step 32).
pub const UPDATE_TIMER: u32 = 0x0086_f260;
/// `Main::OnIdle_HandleMenuBackground` (step 44).
pub const HANDLE_MENU_BACKGROUND: u32 = 0x0086_f450;
/// `BSTreeManager::Update` (step 77).
pub const TREE_MANAGER_UPDATE: u32 = 0x0066_52e0;

/// Viewer work outside the frame's stages.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewerSet {
    /// First: putting a newly loaded interior or exterior on screen and
    /// streaming the land around the player. In the exe, loading belongs to
    /// the frame (`TES::ShowLoadingMenu`, step 39; the grid cells' attach
    /// in `Main::OnIdle_UpdateCurrentGridCell`, step 78), but the viewer's
    /// loaders aren't split along those calls yet and other systems rely on
    /// a scene being installed at the start of the frame (a door or a
    /// script's `MoveTo` asks for it one frame, it is in place the next).
    Loading,
    /// Then the interface: the game's menus, the viewer's own menus,
    /// V.A.T.S. and the lockpicking menu, which take Bevy's input
    /// (`reset_all`) before the player's systems read it. In the exe this is
    /// `Interface::Idle` (through `Main::OnIdle_DoInterfaceIdle`), in stage 5
    /// with one thread or on the AI thread in stage 6; the player's update
    /// (stage 2) reads the controls `Main::OnIdle_PollControls` polled in the
    /// previous frame (step 33), after that frame's interface idle, so there
    /// too the interface meets the input before the player does. The
    /// player's update now stops in menu mode ([`PlayerSet`], PR 4), but
    /// the player systems ordered outside its gates (`view_input`, the
    /// mouse look) still read the input the menus clear, so it stays here
    /// until PR 7.
    Interface,
    /// Last: what is not in `Main::OnIdle`, the viewer's own tools
    /// (screenshots, help, the cursor, exposure, the F12 report, the frame
    /// rate, the window's focus, the present mode, the console key's switch
    /// to its free camera, billboards turned to Bevy's camera); and output
    /// whose place in the frame isn't traced, which only needs the frame's
    /// work done (the GPU's grass and water around the camera, sounds, music
    /// and the radio played).
    AfterFrame,
}

/// The player stage as system sets (`world::frame::player`, Phase 1 PR 4):
/// inside step 14's set (`Main::OnIdle_UpdatePlayer`, `0086f940`) each of
/// its calls in call order, and inside the call of
/// `PlayerCharacter::Update` (slot +0x2f8) each of that function's
/// sub-steps in call order; each under its gate on [`ThisPlayer`].
///
/// A player system that is (part of) a sub-step goes in the sub-step's set.
/// One that belongs at a sub-step but also does work while that sub-step's
/// gate is closed (in menu mode, in V.A.T.S.'s menu) is ordered at the
/// sub-step from outside its set, so that it keeps running: see where each
/// is added in `main.rs`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerSet {
    /// A call of `0086f940` (an index into `world::frame::player::UPDATE_PLAYER`).
    Call(usize),
    /// A sub-step of `PlayerCharacter::Update` (an index into
    /// `world::frame::player::UPDATE`).
    Update(usize),
}

impl PlayerSet {
    /// The set of the first call of `0086f940` to `address`.
    pub fn call(address: u32) -> PlayerSet {
        PlayerSet::Call(player::call_of(address).expect("a call of Main::OnIdle_UpdatePlayer"))
    }

    /// The set of the call of `PlayerCharacter::Update`.
    pub fn update() -> PlayerSet {
        PlayerSet::Call(player::update_call())
    }

    /// The set of the sub-step of `PlayerCharacter::Update` whose call is at
    /// `site`.
    pub fn at(site: u32) -> PlayerSet {
        PlayerSet::Update(Self::index(site))
    }

    /// The set of the sub-step after the one at `site`.
    pub fn after(site: u32) -> PlayerSet {
        PlayerSet::Update(Self::index(site) + 1)
    }

    fn index(site: u32) -> usize {
        player::step_at(site).expect("a sub-step of PlayerCharacter::Update")
    }
}

/// `Main::OnIdle_UpdatePlayer` (step 14).
pub const UPDATE_PLAYER_STEP: u32 = player::UPDATE_PLAYER_ADDRESS;
/// `PlayerCharacter::UpdateFlyCamera` (`0086f940`'s call at `0086f9c7`).
pub const UPDATE_FLY_CAMERA: u32 = 0x0094_66d0;
/// `PlayerCharacter::UpdateHeadingAndLooking`'s call in
/// `PlayerCharacter::Update`.
pub const HEADING_AND_LOOKING_AT: u32 = 0x0093_f8d9;
/// The attack's call (`00948310`) on the free branch.
pub const ATTACK_AT: u32 = 0x0094_20fc;
/// The move vector given to the player's mover (`009ea570`) on the free
/// branch; its slot +0x14 then moves the player.
pub const MOVE_AT: u32 = 0x0094_280b;
/// The animation update of the view the player is in (the second
/// `Actor::UpdateAnimationMovement` of the free branch).
pub const OWN_VIEW_ANIMATION_AT: u32 = 0x0094_3806;

/// The world and time stage's callees as system sets
/// (`world::frame::world_time`, Phase 1 PR 5): each sub-step of each
/// modelled function, in call order inside the set of the frame step that
/// calls the function (its first, the one the viewer's thread count reaches:
/// the process lists are called twice, threads > 1 first) or of its
/// caller's sub-step (`TES::UpdateCurrentGridCell` inside
/// `Main::OnIdle_UpdateCurrentGridCell`'s call of it); each under its gate
/// on [`ThisWorld`].
///
/// As for the player's sets, a system that belongs at a sub-step but also
/// does work while the sub-step's gate is closed is ordered at it from
/// outside its set: see where each is added.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldSet {
    /// Sub-step `.1` of the function at `.0` (an index into its
    /// `world::frame::world_time::Function::steps`).
    Sub(u32, usize),
}

impl WorldSet {
    /// The set of `function`'s sub-step whose call is at `site`.
    pub fn at(function: u32, site: u32) -> WorldSet {
        WorldSet::Sub(function, Self::index(function, site))
    }

    /// The set of the sub-step after the one at `site`.
    pub fn after(function: u32, site: u32) -> WorldSet {
        WorldSet::Sub(function, Self::index(function, site) + 1)
    }

    fn index(function: u32, site: u32) -> usize {
        let f = world_time::function(function).expect("a modelled function");
        world_time::step_at(f, site).expect("a sub-step")
    }
}

/// `ProcessLists::RunActorScripts` (step 60).
pub const RUN_ACTOR_SCRIPTS: u32 = 0x0097_8550;
/// `TESObjectREFR::RunScript`'s call in it.
pub const RUN_SCRIPT_AT: u32 = 0x0097_85bf;
/// The wind update's call (`006658b0`) in `BSTreeManager::Update`.
pub const TREE_WIND_AT: u32 = 0x0066_54dd;
/// `Main::OnIdle_UpdateCurrentGridCell` (step 78).
pub const UPDATE_CURRENT_GRID_CELL_STEP: u32 = 0x0086_fbe0;
/// `TES::UpdateCurrentGridCell`.
pub const UPDATE_CURRENT_GRID_CELL: u32 = 0x0045_2580;
/// `GridCellArray::SetCenter`'s call (the grid array's slot +0x10) in
/// `TES::UpdateCurrentGridCell`: the cells detached and attached around the
/// new centre.
pub const GRID_SET_CENTER_AT: u32 = 0x0045_2c40;
/// `BGSTerrainManager::Update`'s call in `TES::UpdateCurrentGridCell`.
pub const TERRAIN_UPDATE_AT: u32 = 0x0045_2d9f;

/// The world and time stage's inputs, filled with [`ThisFrame`]
/// ([`begin_frame`]).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct ThisWorld(pub WorldState);

/// The world stage's inputs from the frame's and the viewer's values. Those
/// the viewer has no source for are fixed (docs/FRAME_SKELETON.md, "PR 5
/// result"):
///
/// - the world runs, menu mode, the frozen world, the cell tests: the
///   frame's ([`ThisFrame`]);
/// - an interior loaded, the sky: whether the viewer is indoors (no
///   `exterior::Exterior`); the scene graph's camera is there;
/// - the rest at `WorldState::default`: no new game's loading menu, no cell
///   test's walk, the calendar's carries, radiation, resting and the garbage
///   queues not tracked (nothing in the viewer sits under those gates), the
///   data handler there, the position inside the grid's centre cell (the
///   viewer's squares stream on their own, `exterior::stream_squares`, which
///   is ordered at the grid's sub-steps but not under their gates), no
///   script running.
pub fn viewer_world_state(frame: &FrameState, indoors: bool) -> WorldState {
    WorldState {
        interior_loaded: indoors,
        sky: !indoors,
        ..world_time::from_frame(frame)
    }
}

/// The frame's inputs to the gates, filled once per frame ([`begin_frame`]).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ThisFrame(pub FrameState);

/// The player stage's inputs, filled with [`ThisFrame`] ([`begin_frame`]).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct ThisPlayer(pub PlayerState);

/// The player stage's inputs from the frame's and the viewer's values. Those
/// the viewer has no source for are fixed (docs/FRAME_SKELETON.md, "PR 4
/// result"):
///
/// - menu mode and the Pip-Boy's opening: the frame's ([`ThisFrame`]);
/// - the fly camera: the viewer's free camera (the ` key,
///   `walk::Player::walking` off), which is `TFC` without an argument: the
///   player isn't updated, the world runs (`world_frozen` stays off);
/// - dialogue (`Interface::InDialog`): the dialogue menu up;
/// - the rest at `PlayerState::default`: no position request (the viewer's
///   doors and `MoveTo` load in `ViewerSet::Loading`), the player's 3D there
///   (its systems test `walk::Player::ready` themselves), not
///   AI-controlled, not dead (the exe's dead player takes the controlled
///   branch, but the viewer's death countdown is in `combat::player_attack`,
///   which sits on the free branch, and would stop), not knocked down, no
///   fade, no forced activation, no muzzle flash.
pub fn viewer_player_state(frame: &FrameState, walking: bool, in_dialogue: bool) -> PlayerState {
    PlayerState {
        menu_flag: frame.menu_flag(),
        pipboy_opening: frame.pipboy_opening,
        fly_camera: !walking,
        in_dialogue,
        ..PlayerState::default()
    }
}

impl Default for ThisFrame {
    fn default() -> ThisFrame {
        ThisFrame(viewer_frame_state(FrameInputs::default()))
    }
}

/// The values the viewer has for the frame's inputs.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FrameInputs {
    /// Tab and Alt held.
    pub alt_tab_held: bool,
    /// A menu is up: the game's own, the viewer's, the Pip-Boy, the
    /// lockpicking menu (`menus::Menus::is_open`), the dialogue menu, the
    /// V.A.T.S. menu.
    pub menu_up: bool,
    /// The V.A.T.S. manager's mode (`vats::Vats::manager_mode`).
    pub vats_mode: u8,
    /// The class of the top game menu on screen (`game_menus::MenuDraw`).
    pub top_menu: Option<i32>,
}

/// The thread count `main` leaves (`0086a950`-`0086a990`): the processor
/// count (`GetSystemInfo`; here the standard library's), raised to 2 when
/// it is 1. Whether the INI can lower it afterwards is not traced.
pub fn thread_count() -> i32 {
    let n = std::thread::available_parallelism().map_or(1, |n| n.get());
    i32::try_from(n).unwrap_or(i32::MAX).max(2)
}

/// The frame's inputs from the viewer's values. Those the viewer has no
/// source for are fixed at the normal PC case (docs/FRAME_SKELETON.md, "PR 3
/// result"):
///
/// - fader 1 (`FaderManager::IsFaderVisible(1)`): not visible. The viewer's
///   two faders (the sleep's and the menus' fade to black) are fader 0.
/// - the frozen world (`Main` +7, the console's `TFC` 1, 2 or 5): not
///   frozen. The viewer has no console window, and its free camera (the `
///   key, `walk::toggle_walking`) is its own, not `TFC`: the world runs.
/// - the interface mode (`[011d8a80]`+0xc): 1, game mode. Menu mode comes
///   from `menu_up`; only the memory free's gate (step 13, open) reads the
///   number itself.
/// - the thread count: [`thread_count`], at least 2 as `main` leaves it.
///
/// Also fixed: the console is hidden (no console window), the Pip-Boy's
/// opening is part of `menu_up` (`Menus::pipboy`), and the requests and
/// loading inputs of steps no viewer system implements keep
/// `FrameState::default`'s values.
pub fn viewer_frame_state(v: FrameInputs) -> FrameState {
    FrameState {
        alt_tab_held: v.alt_tab_held,
        in_menu_mode: v.menu_up,
        pipboy_opening: false,
        fader_visible: false,
        console_visible: false,
        interface_mode: 1,
        world_frozen: false,
        threads: thread_count(),
        vats_mode: u32::from(v.vats_mode),
        top_menu: v.top_menu.and_then(|c| u32::try_from(c).ok()).unwrap_or(0),
        ..FrameState::default()
    }
}

/// Fills [`ThisFrame`]: the menu state queries at the start of
/// `Main::OnIdle` (steps 3-12), with the Tab and Alt test (`0086e682`,
/// `0086e69a`: `GetAsyncKeyState` on Tab and either Alt).
#[allow(clippy::too_many_arguments)]
pub fn begin_frame(
    keys: Res<ButtonInput<KeyCode>>,
    menus: Res<crate::menus::Menus>,
    conversation: Res<crate::dialogue::Conversation>,
    vats: Option<Res<crate::vats::Vats>>,
    draw: Res<crate::game_menus::MenuDraw>,
    walker: Res<crate::walk::Player>,
    exterior: Option<Res<crate::exterior::Exterior>>,
    (mut this, mut this_player, mut this_world): (
        ResMut<ThisFrame>,
        ResMut<ThisPlayer>,
        ResMut<ThisWorld>,
    ),
) {
    let vats_mode = vats.as_ref().map_or(0, |v| v.manager_mode());
    let in_dialogue = conversation.0.as_ref().is_some_and(|t| !t.is_line_only());
    this.0 = viewer_frame_state(FrameInputs {
        alt_tab_held: keys.pressed(KeyCode::Tab)
            && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight)),
        menu_up: menus.is_open() || in_dialogue || vats.is_some_and(|v| v.in_menu()),
        vats_mode,
        top_menu: draw.1.last().copied(),
    });
    this_player.0 = viewer_player_state(&this.0, walker.walking, in_dialogue);
    this_world.0 = viewer_world_state(&this.0, exterior.is_none());
}

/// Configures the sets in `Update`: the viewer's two sets ahead, the
/// stages in the exe's order, each step inside its stage in call order,
/// the viewer's last set after; each stage and step under its gate on
/// [`ThisFrame`].
pub fn configure(app: &mut App) {
    app.init_resource::<ThisFrame>();
    let stages = frame::stages();
    app.configure_sets(
        Update,
        (
            ViewerSet::Loading,
            ViewerSet::Interface,
            FrameSet::Stage(stages[0]),
        )
            .chain(),
    );
    for pair in stages.windows(2) {
        app.configure_sets(
            Update,
            FrameSet::Stage(pair[0]).before(FrameSet::Stage(pair[1])),
        );
    }
    app.configure_sets(
        Update,
        FrameSet::Stage(*stages.last().expect("stages")).before(ViewerSet::AfterFrame),
    );
    for stage in stages {
        app.configure_sets(
            Update,
            FrameSet::Stage(stage)
                .run_if(move |this: Res<ThisFrame>| frame::stage_reached(&this.0, stage)),
        );
    }
    for (i, step) in STEPS.iter().enumerate() {
        app.configure_sets(
            Update,
            FrameSet::Step(i)
                .in_set(FrameSet::Stage(step.stage))
                .run_if(move |this: Res<ThisFrame>| frame::step_runs(&this.0, i)),
        );
        if i + 1 < STEPS.len() && STEPS[i + 1].stage == step.stage {
            app.configure_sets(Update, FrameSet::Step(i).before(FrameSet::Step(i + 1)));
        }
    }
    configure_player(app);
    configure_world(app);
}

/// The player stage's sets ([`PlayerSet`]): the calls of `0086f940` inside
/// its step, the sub-steps of `PlayerCharacter::Update` inside its call,
/// each in call order under its gate on [`ThisPlayer`].
fn configure_player(app: &mut App) {
    app.init_resource::<ThisPlayer>();
    let step = FrameSet::step(UPDATE_PLAYER_STEP);
    for i in 0..UPDATE_PLAYER.len() {
        app.configure_sets(
            Update,
            PlayerSet::Call(i)
                .in_set(step)
                .run_if(move |this: Res<ThisPlayer>| player::call_runs(&this.0, i)),
        );
        if i + 1 < UPDATE_PLAYER.len() {
            app.configure_sets(Update, PlayerSet::Call(i).before(PlayerSet::Call(i + 1)));
        }
    }
    for i in 0..UPDATE.len() {
        app.configure_sets(
            Update,
            PlayerSet::Update(i)
                .in_set(PlayerSet::update())
                .run_if(move |this: Res<ThisPlayer>| player::step_runs(&this.0, i)),
        );
        if i + 1 < UPDATE.len() {
            app.configure_sets(
                Update,
                PlayerSet::Update(i).before(PlayerSet::Update(i + 1)),
            );
        }
    }
    // The sub-steps also follow the calls before the update's and come
    // before the calls after it as an order of their own (the hierarchy
    // alone doesn't order them), so a system ordered at a sub-step from
    // outside the sets keeps its place when the update's sets are empty.
    let call = player::update_call();
    app.configure_sets(
        Update,
        (
            PlayerSet::Update(0).after(PlayerSet::Call(call - 1)),
            PlayerSet::Update(UPDATE.len() - 1).before(PlayerSet::Call(call + 1)),
        ),
    );
}

/// The world stage's sets ([`WorldSet`]): each modelled function's
/// sub-steps inside the set of its call (a frame step, or its caller's
/// sub-step), in call order under their gates on [`ThisWorld`].
fn configure_world(app: &mut App) {
    app.init_resource::<ThisWorld>();
    for f in &FUNCTIONS {
        let address = f.address;
        let (step, around) = if f.caller == world_time::ON_IDLE {
            let step = frame::step_of(address).expect("a call of Main::OnIdle");
            (Some(FrameSet::Step(step)), None)
        } else {
            let caller = world_time::function(f.caller).expect("a modelled caller");
            let k = world_time::step_at(caller, f.call_sites[0]).expect("the caller's call");
            let parent = WorldSet::Sub(f.caller, k);
            let before = k.checked_sub(1).map(|j| WorldSet::Sub(f.caller, j));
            let after = (k + 1 < caller.steps.len()).then_some(WorldSet::Sub(f.caller, k + 1));
            (None, Some((parent, before, after)))
        };
        let n = f.steps.len();
        for i in 0..n {
            let set = WorldSet::Sub(address, i);
            match (step, around) {
                (Some(step), _) => app.configure_sets(Update, set.in_set(step)),
                (None, Some((parent, _, _))) => app.configure_sets(Update, set.in_set(parent)),
                (None, None) => unreachable!(),
            };
            app.configure_sets(
                Update,
                set.run_if(move |this: Res<ThisWorld>| {
                    let f = world_time::function(address).expect("modelled");
                    world_time::step_runs(&this.0, f, i)
                }),
            );
            if i + 1 < n {
                app.configure_sets(
                    Update,
                    WorldSet::Sub(address, i).before(WorldSet::Sub(address, i + 1)),
                );
            }
        }
        // A nested function's sub-steps also follow its caller's sub-step
        // before the call and come before the one after it (as the player's
        // update's do), so a system ordered at one of them from outside the
        // sets keeps its place when they are empty.
        if let Some((_, before, after)) = around {
            if let Some(b) = before {
                app.configure_sets(Update, WorldSet::Sub(address, 0).after(b));
            }
            if let Some(a) = after {
                app.configure_sets(Update, WorldSet::Sub(address, n - 1).before(a));
            }
        }
    }
}

/// The frame's sets and [`begin_frame`].
pub struct FrameOrderPlugin;

impl Plugin for FrameOrderPlugin {
    fn build(&self, app: &mut App) {
        configure(app);
        app.add_systems(
            Update,
            begin_frame.in_set(FrameSet::Stage(Stage::FrameStart)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What ran, in order.
    #[derive(Resource, Default)]
    struct Ran(Vec<&'static str>);

    fn note(name: &'static str) -> impl FnMut(ResMut<Ran>) {
        move |mut ran: ResMut<Ran>| ran.0.push(name)
    }

    fn stage_name(stage: Stage) -> &'static str {
        match stage {
            Stage::FrameStart => "FrameStart",
            Stage::Player => "Player",
            Stage::Housekeeping => "Housekeeping",
            Stage::WorldAndTime => "WorldAndTime",
            Stage::InterfaceAndScene => "InterfaceAndScene",
            Stage::AiStart => "AiStart",
            Stage::Render => "Render",
            Stage::AiJoin => "AiJoin",
        }
    }

    /// An app with the sets and nothing else; the state is set by hand.
    fn frame_app(state: FrameState) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        configure(&mut app);
        app.insert_resource(ThisFrame(state)).init_resource::<Ran>();
        app
    }

    /// The configured order of the sets is `world::frame`'s stage order,
    /// with the viewer's sets ahead and after; and steps run in call order
    /// inside their stage. Systems are added in reverse so that only the
    /// sets can give the order.
    #[test]
    fn sets_run_in_the_frames_order() {
        let mut app = frame_app(FrameState::default());
        app.add_systems(Update, note("AfterFrame").in_set(ViewerSet::AfterFrame));
        for stage in frame::stages().into_iter().rev() {
            app.add_systems(
                Update,
                note(stage_name(stage)).in_set(FrameSet::Stage(stage)),
            );
        }
        app.add_systems(Update, note("Interface").in_set(ViewerSet::Interface))
            .add_systems(Update, note("Loading").in_set(ViewerSet::Loading));
        app.update();
        let mut want = vec!["Loading", "Interface"];
        want.extend(frame::stages().into_iter().map(stage_name));
        want.push("AfterFrame");
        assert_eq!(app.world().resource::<Ran>().0, want);
        // The exe's stage order, as STEPS reaches the stages.
        let mut from_steps: Vec<Stage> = STEPS.iter().map(|s| s.stage).collect();
        from_steps.dedup();
        assert_eq!(frame::stages(), from_steps);

        // Steps: the tree manager (77) after the menu background (44)
        // after the timer (32), whatever order they're added in, and each
        // with its stage's other systems.
        let mut app = frame_app(FrameState::default());
        app.add_systems(
            Update,
            note("77").in_set(FrameSet::step(TREE_MANAGER_UPDATE)),
        )
        .add_systems(
            Update,
            note("44").in_set(FrameSet::step(HANDLE_MENU_BACKGROUND)),
        )
        .add_systems(Update, note("32").in_set(FrameSet::step(UPDATE_TIMER)))
        .add_systems(
            Update,
            note("Player").in_set(FrameSet::Stage(Stage::Player)),
        );
        app.update();
        assert_eq!(
            app.world().resource::<Ran>().0,
            ["Player", "32", "44", "77"]
        );
    }

    /// A system in a gated set doesn't run while its gate is closed: menu
    /// mode stops `Calendar::Update` (step 56, the world runs), not the rest
    /// of its stage; Tab and Alt held stop every stage after the first.
    #[test]
    fn gates_stop_their_sets() {
        let calendar = FrameSet::step(0x0086_7a40);
        let add = |app: &mut App| {
            app.add_systems(
                Update,
                note("start").in_set(FrameSet::Stage(Stage::FrameStart)),
            )
            .add_systems(Update, note("calendar").in_set(calendar))
            .add_systems(
                Update,
                note("world")
                    .in_set(FrameSet::Stage(Stage::WorldAndTime))
                    .after(calendar),
            )
            .add_systems(Update, note("after").in_set(ViewerSet::AfterFrame));
        };
        let game = viewer_frame_state(FrameInputs::default());

        let mut a = frame_app(game);
        add(&mut a);
        a.update();
        assert_eq!(
            a.world().resource::<Ran>().0,
            ["start", "calendar", "world", "after"]
        );

        let menu = viewer_frame_state(FrameInputs {
            menu_up: true,
            ..FrameInputs::default()
        });
        let mut a = frame_app(menu);
        add(&mut a);
        a.update();
        assert_eq!(a.world().resource::<Ran>().0, ["start", "world", "after"]);

        let held = viewer_frame_state(FrameInputs {
            alt_tab_held: true,
            ..FrameInputs::default()
        });
        let mut a = frame_app(held);
        add(&mut a);
        a.update();
        assert_eq!(a.world().resource::<Ran>().0, ["start", "after"]);
    }

    /// The fixed inputs: fader 1 hidden, the world not frozen, game mode's
    /// interface mode, two threads or more; V.A.T.S. playback reaches the
    /// field-of-view gate.
    #[test]
    fn fixed_inputs() {
        let s = viewer_frame_state(FrameInputs::default());
        assert!(!s.fader_visible && !s.world_frozen && !s.console_visible);
        assert_eq!(s.interface_mode, 1);
        assert!(s.threads >= 2);
        assert!(s.world_runs() && s.process_lists());
        let playback = viewer_frame_state(FrameInputs {
            vats_mode: world::vats::mode::PLAYBACK,
            ..FrameInputs::default()
        });
        assert!(!frame::Gate::NotVatsPlayback.holds(&playback));
        let sleeping = viewer_frame_state(FrameInputs {
            top_menu: Some(1012),
            ..FrameInputs::default()
        });
        assert!(frame::Gate::SleepWaitMenuTop.holds(&sleeping));
    }

    /// An app with the sets, the frame's state and the player's.
    fn player_app(player: PlayerState) -> App {
        let mut app = frame_app(FrameState::default());
        app.insert_resource(ThisPlayer(player));
        app
    }

    /// The player's sets run in the exe's order inside step 14: the calls of
    /// `0086f940` in call order, `PlayerCharacter::Update`'s sub-steps
    /// inside its call, in theirs; a system ordered at a sub-step from
    /// outside its set runs there. Systems are added in reverse.
    #[test]
    fn player_sets_run_in_the_exes_order() {
        let mut app = player_app(PlayerState::default());
        app.add_systems(
            Update,
            note("after step")
                .in_set(FrameSet::Stage(Stage::Player))
                .after(FrameSet::step(UPDATE_PLAYER_STEP)),
        )
        .add_systems(
            Update,
            note("animation").in_set(PlayerSet::at(OWN_VIEW_ANIMATION_AT)),
        )
        .add_systems(Update, note("move").in_set(PlayerSet::at(MOVE_AT)))
        .add_systems(Update, note("attack").in_set(PlayerSet::at(ATTACK_AT)))
        .add_systems(
            Update,
            note("look")
                .in_set(FrameSet::step(UPDATE_PLAYER_STEP))
                .after(PlayerSet::at(HEADING_AND_LOOKING_AT))
                .before(PlayerSet::after(HEADING_AND_LOOKING_AT)),
        )
        .add_systems(
            Update,
            note("heading").in_set(PlayerSet::at(HEADING_AND_LOOKING_AT)),
        )
        .add_systems(Update, note("cell").in_set(PlayerSet::call(0x0043_6aa0)))
        .add_systems(Update, note("request").in_set(PlayerSet::call(0x0093_bea0)))
        .add_systems(
            Update,
            note("before step")
                .in_set(FrameSet::Stage(Stage::Player))
                .before(FrameSet::step(UPDATE_PLAYER_STEP)),
        );
        app.update();
        assert_eq!(
            app.world().resource::<Ran>().0,
            [
                "before step",
                "request",
                "heading",
                "look",
                "attack",
                "move",
                "animation",
                "cell",
                "after step"
            ]
        );
    }

    /// The gates: in menu mode only the grenade hold of the player's update
    /// runs (`0086f968`, `0086f974`), with the fly camera only its update
    /// (`0086f98f`); a system ordered from outside the sets runs either way.
    #[test]
    fn player_gates_stop_their_sets() {
        let add = |app: &mut App| {
            app.add_systems(Update, note("grenade").in_set(PlayerSet::call(0x0094_81d0)))
                .add_systems(
                    Update,
                    note("fly").in_set(PlayerSet::call(UPDATE_FLY_CAMERA)),
                )
                .add_systems(Update, note("move").in_set(PlayerSet::at(MOVE_AT)))
                .add_systems(
                    Update,
                    note("look")
                        .in_set(FrameSet::step(UPDATE_PLAYER_STEP))
                        .after(PlayerSet::at(HEADING_AND_LOOKING_AT))
                        .before(PlayerSet::after(HEADING_AND_LOOKING_AT)),
                );
        };
        let run = |state: PlayerState| {
            let mut app = player_app(state);
            add(&mut app);
            app.update();
            app.world().resource::<Ran>().0.clone()
        };
        let menu = viewer_frame_state(FrameInputs {
            menu_up: true,
            ..FrameInputs::default()
        });
        assert_eq!(
            run(viewer_player_state(&FrameState::default(), true, false)),
            ["look", "move"]
        );
        assert_eq!(
            run(viewer_player_state(&menu, true, false)),
            ["grenade", "look"]
        );
        assert_eq!(
            run(viewer_player_state(&FrameState::default(), false, false)),
            ["fly", "look"]
        );
        // Menu mode wins over the fly camera.
        assert_eq!(
            run(viewer_player_state(&menu, false, false)),
            ["grenade", "look"]
        );
    }

    /// The player stage's fixed inputs: the free branch with nothing pending.
    #[test]
    fn player_fixed_inputs() {
        let s = viewer_player_state(&FrameState::default(), true, false);
        assert!(s.updates() && !s.controlled() && !s.position_request && s.has_3d);
        assert!(!s.fading && !s.forced_activation && !s.knocked_or_paralysed);
        assert!(!viewer_player_state(&FrameState::default(), false, false).updates());
        assert!(viewer_player_state(&FrameState::default(), true, true).in_dialogue);
    }

    /// An app with the sets, the frame's state and the world stage's.
    fn world_app(frame: FrameState, indoors: bool) -> App {
        let mut app = frame_app(frame);
        app.insert_resource(ThisWorld(viewer_world_state(&frame, indoors)));
        app
    }

    /// Adds the world stage's systems the way `main.rs` and `trees.rs`
    /// place them, in reverse.
    fn add_world_systems(app: &mut App) {
        let tree = TREE_MANAGER_UPDATE;
        let grid = UPDATE_CURRENT_GRID_CELL;
        app.add_systems(
            Update,
            note("grid query").in_set(WorldSet::at(UPDATE_CURRENT_GRID_CELL_STEP, 0x0086_fc28)),
        )
        .add_systems(
            Update,
            note("distant land")
                .in_set(FrameSet::step(UPDATE_CURRENT_GRID_CELL_STEP))
                .after(WorldSet::at(grid, TERRAIN_UPDATE_AT))
                .before(WorldSet::after(grid, TERRAIN_UPDATE_AT)),
        )
        .add_systems(
            Update,
            note("squares")
                .in_set(FrameSet::step(UPDATE_CURRENT_GRID_CELL_STEP))
                .after(WorldSet::at(grid, GRID_SET_CENTER_AT))
                .before(WorldSet::after(grid, GRID_SET_CENTER_AT)),
        )
        .add_systems(
            Update,
            note("sway")
                .in_set(FrameSet::step(tree))
                .after(WorldSet::at(tree, TREE_WIND_AT))
                .before(WorldSet::after(tree, TREE_WIND_AT)),
        )
        .add_systems(
            Update,
            note("wind").in_set(WorldSet::at(tree, TREE_WIND_AT)),
        )
        .add_systems(
            Update,
            note("doors")
                .in_set(FrameSet::Stage(Stage::WorldAndTime))
                .after(FrameSet::step(RUN_ACTOR_SCRIPTS))
                .before(FrameSet::after(RUN_ACTOR_SCRIPTS)),
        )
        .add_systems(
            Update,
            note("scripts").in_set(WorldSet::at(RUN_ACTOR_SCRIPTS, RUN_SCRIPT_AT)),
        )
        .add_systems(Update, note("calendar").in_set(FrameSet::step(0x0086_7a40)));
    }

    /// The world stage's sets run in the exe's order: the calendar (step 56),
    /// the scripts (step 60), what follows them, the tree manager's wind and
    /// the sway after it (step 77), the grid's squares and distant land at
    /// `TES::UpdateCurrentGridCell`'s sub-steps, then the rest of
    /// `Main::OnIdle_UpdateCurrentGridCell` (step 78).
    #[test]
    fn world_sets_run_in_the_exes_order() {
        let mut app = world_app(FrameState::default(), false);
        add_world_systems(&mut app);
        app.update();
        assert_eq!(
            app.world().resource::<Ran>().0,
            [
                "calendar",
                "scripts",
                "doors",
                "wind",
                "sway",
                "squares",
                "distant land",
                "grid query"
            ]
        );
    }

    /// The gates: in menu mode the calendar, the wind and the grid's update
    /// stop (`0086e946`, `006653a9`, `0086fbf1`); the scripts' step, the
    /// sway and the loaders ordered outside the gates run.
    #[test]
    fn world_gates_stop_their_sets() {
        let menu = viewer_frame_state(FrameInputs {
            menu_up: true,
            ..FrameInputs::default()
        });
        let mut app = world_app(menu, false);
        add_world_systems(&mut app);
        app.update();
        assert_eq!(
            app.world().resource::<Ran>().0,
            ["scripts", "doors", "sway", "squares", "distant land"]
        );
    }

    /// The world stage's fixed inputs: the frame's modes, indoors or out from
    /// the viewer, the rest at the normal case.
    #[test]
    fn world_fixed_inputs() {
        let game = viewer_frame_state(FrameInputs::default());
        let out = viewer_world_state(&game, false);
        assert!(out.world_runs && out.sky && !out.interior_loaded && out.scene_camera);
        assert_eq!(out.grid, world_time::GridMove::Inside);
        let inside = viewer_world_state(&game, true);
        assert!(inside.interior_loaded && !inside.sky);
        let menu = viewer_frame_state(FrameInputs {
            menu_up: true,
            ..FrameInputs::default()
        });
        let s = viewer_world_state(&menu, false);
        assert!(s.menu_flag && !s.world_runs);
    }
}
