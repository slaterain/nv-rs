//! The game's own menus on screen (`ui::menu`, `ui::menus`): laid out from
//! the game's menu files and filled the way its menu classes fill them,
//! drawn over the scene with the HUD (`hud`), and driven by the mouse and
//! keyboard as the game's interface manager hands them on (`0070c4a0` in
//! FalloutNV.exe): the tile under the pointer is "moused over" and a click
//! goes to its menu; typed keys, the arrows and Enter go to the top menu.
//!
//! Each menu has a module of its own here, which takes its requests from
//! the game (`menus::Menu`, queued by scripts and the world) and hands the
//! player's choices back to the world.
//!
//! The pointer is the system's mouse pointer (the game moves its own by the
//! mouse's raw movement, at its own speed: `007118d0`); the game's cursor
//! picture (`sCursorFilename`, 32 × 32, drawn above every menu, `0070b240`)
//! is drawn where it is.

pub mod asks;
pub mod barter;
pub mod caravan;
pub mod chargen;
pub mod companion_wheel;
pub mod computers;
pub mod container;
pub mod dialog;
pub mod hacking;
pub mod levelup;
pub(crate) mod message;
pub mod recipe;
pub mod repair;
pub mod sleepwait;
pub mod start;
pub mod textedit;
pub mod traits;
pub mod vigor;

use std::collections::HashMap;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::render::camera::CameraOutputMode;
use bevy::render::render_resource::BlendState;
use bevy::window::PrimaryWindow;
use cellview::{Game, TextureData};
use ui::draw::{DrawItem, Textures};
use ui::menu::{Effect, Interface, MenuCode};
use ui::names::{kind, t};
use ui::TileId;

use crate::GameFiles;

/// The game's menus, built for the window's size, and the ones open.
#[derive(Resource, Default)]
pub struct GameMenus {
    pub(crate) screen: Option<Box<Screen>>,
    failed: bool,
    /// `--open-menu levelup:perks`: the next level-up menu's points are
    /// given and Continue pressed through the menu's own clicks.
    levelup_to_perks: bool,
    /// `--open-menu wait:N` / `sleep:N`: the next sleep/wait menu chooses
    /// N hours and presses Wait through its own click.
    sleepwait_hours: Option<u32>,
}

/// What the menus draw this frame, for the HUD's layer (`hud`), over the
/// HUD; and the open menus' class numbers, bottom first (the HUD shows
/// some of its pieces by them).
#[derive(Resource, Default)]
pub struct MenuDraw(pub Vec<DrawItem>, pub Vec<i32>);

/// The menus' tiles (one tile tree, as the game's), the interface's
/// pointer state, and the open menus, bottom first.
pub struct Screen {
    pub ui: ui::Ui,
    pub interface: Interface,
    pub open: Vec<OpenMenu>,
    size: UVec2,
    /// The cursor's picture.
    cursor: Option<TileId>,
    /// The fade to black (the fader manager's fader 0, `Interface\Faders\
    /// Black.dds` over the scene, under the menus: a sleep's), and its
    /// picture.
    pub fade: Option<world::living::sleep::Fade>,
    fader: Option<TileId>,
    /// The Rest control (T) held down (the sleep/wait menu cancels on it).
    rest_down: bool,
    /// The Pip-Boy's callback for the "how many?" open, if it asked
    /// (`asks`).
    quantity_owner: Option<u32>,
    /// The pointer this frame, in menu units.
    pub pointer: Option<(f32, f32)>,
    sizes: HashMap<String, Option<(u32, u32)>>,
    atlases: HashMap<String, Option<ui::Atlas>>,
    /// `nif` tiles' models (the start menu's pause background).
    models: HashMap<String, Option<Vec<start::ModelPiece>>>,
}

/// A menu on screen.
pub enum OpenMenu {
    Message(ui::menus::message::MessageMenu),
    Dialog(ui::menus::dialog::DialogMenu),
    Container(Box<container::ContainerScreen>),
    Barter(Box<barter::BarterScreen>),
    Recipe(Box<recipe::RecipeScreen>),
    Repair(Box<repair::RepairScreen>),
    CompanionWheel(Box<companion_wheel::WheelScreen>),
    Caravan(Box<caravan::CaravanScreen>),
    Quantity(ui::menus::quantity::QuantityMenu),
    LevelUp(Box<ui::menus::levelup::LevelUpMenu>),
    Traits(Box<ui::menus::traits::TraitMenu>),
    CharGen(Box<ui::menus::chargen::CharGenMenu>),
    TextEdit(Box<ui::menus::textedit::TextEditMenu>),
    SleepWait(Box<ui::menus::sleepwait::SleepWaitMenu>),
    Vigor(Box<vigor::VigorScreen>),
    Start(Box<start::StartScreen>),
    Hacking(Box<hacking::HackingScreen>),
    Computers(Box<computers::ComputersScreen>),
}

impl OpenMenu {
    fn code(&mut self) -> &mut dyn MenuCode {
        match self {
            OpenMenu::Message(m) => m,
            OpenMenu::Dialog(m) => m,
            OpenMenu::Container(c) => &mut c.menu,
            OpenMenu::Barter(b) => &mut b.menu,
            OpenMenu::Recipe(r) => &mut r.menu,
            OpenMenu::Repair(r) => &mut r.menu,
            OpenMenu::CompanionWheel(w) => &mut w.menu,
            OpenMenu::Caravan(c) => &mut c.menu,
            OpenMenu::Quantity(m) => m,
            OpenMenu::LevelUp(m) => &mut **m,
            OpenMenu::Traits(m) => &mut **m,
            OpenMenu::CharGen(m) => &mut **m,
            OpenMenu::TextEdit(m) => &mut **m,
            OpenMenu::SleepWait(m) => &mut **m,
            OpenMenu::Vigor(m) => &mut m.menu,
            OpenMenu::Start(m) => &mut m.menu,
            OpenMenu::Hacking(m) => &mut m.menu,
            OpenMenu::Computers(m) => &mut m.menu,
        }
    }

    fn tile(&self) -> TileId {
        match self {
            OpenMenu::Message(m) => m.menu,
            OpenMenu::Dialog(m) => m.menu,
            OpenMenu::Container(c) => c.menu.menu,
            OpenMenu::Barter(b) => b.menu.menu,
            OpenMenu::Recipe(r) => r.menu.menu,
            OpenMenu::Repair(r) => r.menu.menu,
            OpenMenu::CompanionWheel(w) => w.menu.menu,
            OpenMenu::Caravan(c) => c.menu.menu,
            OpenMenu::Quantity(m) => m.menu,
            OpenMenu::LevelUp(m) => m.menu,
            OpenMenu::Traits(m) => m.menu,
            OpenMenu::CharGen(m) => m.menu,
            OpenMenu::TextEdit(m) => m.menu,
            OpenMenu::SleepWait(m) => m.menu,
            OpenMenu::Vigor(m) => m.menu.menu,
            OpenMenu::Start(m) => m.menu.menu,
            OpenMenu::Hacking(m) => m.menu.menu,
            OpenMenu::Computers(m) => m.menu.menu,
        }
    }

    /// A service menu: one whose making hides the conversation
    /// (`00763ff0`: the barter menu `0072d250`, the recipe menu
    /// `00726ff0`) and whose closing brings it back (`007640a0`: `0072d6d0`,
    /// `00727430`). (A companion's trade, the repair menu and the face menu
    /// do too; they aren't shown here. Containers in mode 1 don't.)
    fn is_service(&self) -> bool {
        matches!(self, OpenMenu::Barter(_) | OpenMenu::Recipe(_))
    }

    /// A menu that draws a 3D scene into the HUD's picture, under the
    /// menus' tiles: the Vigor Tester's machine (`vigor::draw`), the Caravan
    /// table (`caravan_table`).
    fn draws_scene(&self) -> bool {
        matches!(self, OpenMenu::Vigor(_) | OpenMenu::Caravan(_))
    }

    fn closed(&self) -> bool {
        match self {
            OpenMenu::Message(m) => m.closed,
            OpenMenu::Dialog(m) => m.closed,
            OpenMenu::Container(c) => c.menu.closed,
            OpenMenu::Barter(b) => b.menu.closed,
            OpenMenu::Recipe(r) => r.menu.closed,
            OpenMenu::Repair(r) => r.menu.closed,
            OpenMenu::CompanionWheel(w) => w.menu.closed,
            OpenMenu::Caravan(c) => c.closed,
            OpenMenu::Quantity(m) => m.closed,
            OpenMenu::LevelUp(m) => m.closed,
            OpenMenu::Traits(m) => m.closed,
            OpenMenu::CharGen(m) => m.closed,
            OpenMenu::TextEdit(m) => m.closed,
            OpenMenu::SleepWait(m) => m.closed,
            OpenMenu::Vigor(m) => m.menu.closed,
            // Its requests (the load asked for, back to the game) are
            // carried out first (`start::frame`).
            OpenMenu::Start(m) => m.menu.closed && m.menu.requests.is_empty(),
            OpenMenu::Hacking(m) => m.menu.closed,
            OpenMenu::Computers(m) => m.menu.closed,
        }
    }
}

pub struct GameMenusPlugin;

impl Plugin for GameMenusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameMenus>()
            .init_resource::<MenuDraw>()
            .init_resource::<asks::PipboyAsks>()
            .init_resource::<StartMenu>()
            .init_resource::<FixedPointer>()
            .init_resource::<start::GameSettings>()
            .init_resource::<start::SaveFiles>()
            .init_resource::<FixedClicks>()
            .init_resource::<FixedKeys>()
            .init_resource::<hacking::HackingSounds>()
            .init_resource::<companion_wheel::WheelVoices>()
            .add_systems(
                Update,
                (
                    start_menu,
                    escape_opens_start_menu,
                    open_menus,
                    run_open_menus,
                    start_menu_frame,
                    draw_menus,
                )
                    .chain()
                    .before(crate::menus::run_menus),
            )
            .add_systems(Update, vigor::draw.after(run_open_menus))
            .add_systems(
                Update,
                compose_hud_over_scene.after(crate::menus::run_menus),
            )
            .add_systems(Update, hacking::play_sounds.after(run_open_menus))
            .add_systems(Update, companion_wheel::play_voices.after(run_open_menus));
    }
}

/// How the HUD's camera writes its picture this frame: over a menu's 3D
/// scene (drawn into the same picture first, by the scene's own camera at
/// order −5) it blends its tiles over it; otherwise it writes the picture
/// as it is.
fn hud_output(open: &[OpenMenu]) -> CameraOutputMode {
    if open.iter().any(OpenMenu::draws_scene) {
        CameraOutputMode::Write {
            blend_state: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            clear_color: ClearColorConfig::None,
        }
    } else {
        CameraOutputMode::default()
    }
}

fn blends(mode: &CameraOutputMode) -> bool {
    matches!(
        mode,
        CameraOutputMode::Write {
            blend_state: Some(_),
            ..
        }
    )
}

/// The one place that sets the HUD camera's output, for every menu that
/// draws a 3D scene under the menus. (Each menu used to set it itself, and
/// the Caravan table's "not open, so stop blending" undid the Vigor
/// Tester's every frame: the HUD's empty picture replaced the machine, so
/// the tester's menu worked but couldn't be seen.)
fn compose_hud_over_scene(
    menus: Res<GameMenus>,
    mut cameras: Query<&mut Camera, With<crate::hud::HudCamera>>,
) {
    let wanted = hud_output(menus.screen.as_deref().map_or(&[], |s| &s.open));
    for mut camera in &mut cameras {
        if blends(&camera.output_mode) != blends(&wanted) {
            camera.output_mode = wanted;
        }
    }
}

/// `--menu-pointer X,Y`: the menus' pointer held at this pixel of a 1920 ×
/// 1080 picture (for screenshots, which have no mouse).
#[derive(Resource, Default)]
pub struct FixedPointer(pub Option<(f32, f32)>);

/// `--menu-click S,...`: left clicks at these seconds (pressed one frame,
/// let go the next), for testing with `FixedPointer`.
#[derive(Resource, Default)]
pub struct FixedClicks {
    pub at: Vec<f64>,
    pub release: bool,
}

/// `--menu-keys S:K,...`: keys typed into the top menu at these seconds
/// (a character, or left, right, up, down), for testing.
#[derive(Resource, Default)]
pub struct FixedKeys(pub Vec<(f64, String)>);

/// `--open-menu NAME[:ID]`: a menu to open once the place has loaded, for
/// testing.
#[derive(Resource, Default)]
pub struct StartMenu(pub Option<String>);

/// A form by editor ID or hexadecimal form ID.
fn form_named(order: &esm::LoadOrder, id: &str) -> Option<esm::FormId> {
    order.form_by_editor_id(id).or_else(|| {
        u32::from_str_radix(id.trim_start_matches("0x"), 16)
            .ok()
            .map(esm::FormId)
            .filter(|f| order.get(*f).is_some())
    })
}

/// A reference's name (its base's `FULL`).
fn reference_name(order: &esm::LoadOrder, reference: esm::FormId) -> String {
    world::scripting::base_of(order, reference)
        .and_then(|b| order.get(b))
        .and_then(|r| r.record().ok())
        .and_then(|r| r.full_name())
        .unwrap_or_default()
}

/// Opens the `--open-menu` menu once the player is in the place.
#[allow(clippy::too_many_arguments)]
fn start_menu(
    game: Res<GameFiles>,
    mut start: ResMut<StartMenu>,
    mut queue: ResMut<crate::menus::Menus>,
    player: Res<crate::walk::Player>,
    mut menus: ResMut<GameMenus>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<crate::dialogue::DialogueState>,
    commands: Res<crate::scripts::StartCommands>,
) {
    // After `--run`'s lines (a menu opened first would hold them back).
    if !player.ready || !commands.0.is_empty() {
        return;
    }
    let Some(asked) = start.0.take() else {
        return;
    };
    // The player goes up a level now (`world::experience::level_up`);
    // `levelup:perks` then gives the points and goes on to the perks.
    if asked == "levelup" || asked == "levelup:perks" {
        let up = world::experience::level_up(&game.0.order, &mut state.0);
        println!("--open-menu: level {}.", up.level);
        menus.levelup_to_perks = asked.ends_with(":perks");
        queue.push(crate::menus::Menu::LevelUp(up));
        return;
    }
    let order = &game.0.order;
    let (name, id) = asked.split_once(':').unwrap_or((asked.as_str(), ""));
    // The sleep/wait menu in either mode (as `ShowSleepWaitMenu` opens it,
    // without the refusals).
    if name == "sleep" || name == "wait" {
        println!("--open-menu: the {name} menu.");
        menus.sleepwait_hours = id.parse::<u32>().ok().filter(|n| (1..=24).contains(n));
        queue.push(crate::menus::Menu::SleepWait {
            sleep: name == "sleep",
        });
        return;
    }
    // "How many?" on its own (the game only opens it from other menus).
    if name == "quantity" {
        let most = id.parse::<i32>().unwrap_or(20);
        let size = window_size(&windows);
        if let Some(screen) = size.and_then(|s| screen(&mut menus, &game.0, s)) {
            println!("--open-menu: how many, up to {most}.");
            container::open_quantity(screen, &game.0, most);
            queue.game_open = true;
        }
        return;
    }
    let form = form_named(order, id);
    let request = match (name, form) {
        ("container", Some(r)) => crate::menus::Menu::Container(r, reference_name(order, r)),
        ("barter", Some(r)) => crate::menus::Menu::Barter(r),
        // A recipe category's crafting menu for the player, as
        // `ShowRecipeMenu` would open it.
        ("recipes", Some(r)) => crate::menus::Menu::Recipe {
            actor: world::dialogue::PLAYER_REF,
            category: r,
        },
        // A vendor's repairs, as `ShowRepairMenu` on them would open them.
        ("repair", Some(r)) => crate::menus::Menu::RepairServices(r),
        // A companion's things, as `OpenTeammateContainer` would open them.
        ("teammate", Some(r)) => crate::menus::Menu::Teammate(r),
        // A companion's wheel, as using them would bring it up.
        ("wheel", Some(r)) => crate::menus::Menu::CompanionWheel(r),
        // A placed terminal's own screen, as getting in would open it.
        ("terminal", Some(r)) => {
            crate::menus::Menu::Terminal(world::scripting::base_of(order, r).unwrap_or(r), r)
        }
        // A placed terminal's hacking menu, as using it would open it.
        ("hacking", Some(r)) => {
            crate::menus::Menu::Hacking(world::scripting::base_of(order, r).unwrap_or(r), r)
        }
        _ => {
            println!("--open-menu: don't know how to open '{asked}'.");
            return;
        }
    };
    println!("--open-menu: opening {asked}.");
    queue.push(request);
}

/// Every text game setting (`GMST` named `s…`), for `&-sName;`.
fn text_settings(order: &esm::LoadOrder) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for rr in order.records_of_type(esm::FourCC::new(b"GMST")) {
        let Ok(record) = rr.record() else {
            continue;
        };
        let Some(name) = record.get(esm::sig::EDID).map(|s| s.zstring()) else {
            continue;
        };
        if name.starts_with('s') {
            if let Some(text) = record.get(esm::sig::DATA).map(|s| s.zstring()) {
                out.insert(name, text);
            }
        }
    }
    out
}

/// The cursor's picture, a game setting (`00f6ccc0`).
const CURSOR_DEFAULT: &str = "Interface\\Icons\\Misc\\cursor.dds";

impl Screen {
    fn build(game: &Game, size: UVec2) -> Screen {
        let mut read = |p: &str| game.assets.read(p).ok().flatten();
        let ini = |s: &str, k: &str| game.settings.get(s, k).map(str::to_string);
        let mut ui = ui::game::new_ui(&mut read, &ini, text_settings(&game.order), size.x, size.y);
        for w in &ui.warnings {
            println!("  Menus: {w}");
        }
        // The cursor (`0070b240`): an image named "Cursor", depth 2000, 32 ×
        // 32, its picture `sCursorFilename`, white, hidden until the
        // menus need it.
        let file = ui
            .setting_text("sCursorFilename")
            .unwrap_or_else(|| CURSOR_DEFAULT.to_string());
        let cursor = ui
            .load_menu(
                format!(
                    "<image name=\"Cursor\"><depth>2000</depth><width>32</width><height>32</height><filename>{file}</filename><x>0</x><y>0</y><alpha>255</alpha><red>255</red><green>255</green><blue>255</blue></image>"
                )
                .as_bytes(),
                &mut |_| None,
            )
            .ok();
        // The fader (`00700960`): a quad over the whole screen with
        // `Data\Textures\Interface\Faders\Black.dds`, under the menus;
        // hidden until a sleep fades the screen.
        let fader = ui
            .load_menu(
                "<image name=\"Fader\"><depth>1</depth><width><copy src=\"screen()\" trait=\"width\"/></width><height><copy src=\"screen()\" trait=\"height\"/></height><filename>Interface\\Faders\\Black.dds</filename><zoom>-1</zoom><x>0</x><y>0</y><alpha>0</alpha><red>255</red><green>255</green><blue>255</blue><visible>&false;</visible></image>"
                    .as_bytes(),
                &mut |_| None,
            )
            .ok();
        Screen {
            ui,
            interface: Interface::default(),
            open: Vec::new(),
            size,
            cursor,
            fade: None,
            fader,
            rest_down: false,
            quantity_owner: None,
            pointer: None,
            sizes: HashMap::new(),
            atlases: HashMap::new(),
            models: HashMap::new(),
        }
    }

    /// The menu tiles open, bottom first.
    fn menu_tiles(&self) -> Vec<TileId> {
        self.open.iter().map(OpenMenu::tile).collect()
    }

    /// Loads a menu file for a menu's code (`ui::menu::load`), above the
    /// menus open.
    pub fn load(
        &mut self,
        game: &Game,
        path: &str,
        code: &mut dyn MenuCode,
    ) -> Result<TileId, String> {
        let open = self.menu_tiles();
        let depth = ui::menu::next_depth(&mut self.ui, &open);
        let mut read = |p: &str| game.assets.read(p).ok().flatten();
        let tile = ui::menu::load(&mut self.ui, &mut read, path, code, depth)?;
        for w in self.ui.warnings.drain(..) {
            println!("  {path}: {w}");
        }
        Ok(tile)
    }

    /// The message menu, if it's open.
    pub fn message(&mut self) -> Option<&mut ui::menus::message::MessageMenu> {
        self.open.iter_mut().find_map(|m| match m {
            OpenMenu::Message(m) => Some(m),
            _ => None,
        })
    }
}

impl GameMenus {
    /// The menus, once built for the window (`None` before the first frame
    /// or when the game's menu files can't be read).
    pub fn screen(&mut self) -> Option<&mut Screen> {
        self.screen.as_deref_mut()
    }
}

/// Texture sizes and atlases from the game's files.
struct Files<'a> {
    game: &'a Game,
    sizes: &'a mut HashMap<String, Option<(u32, u32)>>,
    atlases: &'a mut HashMap<String, Option<ui::Atlas>>,
}

impl Textures for Files<'_> {
    fn size(&mut self, path: &str) -> Option<(u32, u32)> {
        let game = self.game;
        *self.sizes.entry(path.to_string()).or_insert_with(|| {
            let bytes = game.assets.read(path).ok().flatten()?;
            let t = TextureData::from_dds(path, bytes).ok()?;
            Some((t.width, t.height))
        })
    }

    fn atlas(&mut self, path: &str) -> Option<ui::Atlas> {
        let game = self.game;
        self.atlases
            .entry(path.to_string())
            .or_insert_with(|| {
                let bytes = game.assets.read(path).ok().flatten()?;
                Some(ui::Atlas::parse(&String::from_utf8_lossy(&bytes)))
            })
            .clone()
    }
}

/// The window's size in pixels.
fn window_size(windows: &Query<&Window, With<PrimaryWindow>>) -> Option<UVec2> {
    let w = windows.single().ok()?;
    let size = UVec2::new(w.physical_width(), w.physical_height());
    (size.x > 0 && size.y > 0).then_some(size)
}

/// The menus built for the window (again when its size changes).
fn screen<'a>(menus: &'a mut GameMenus, game: &Game, size: UVec2) -> Option<&'a mut Screen> {
    if menus.failed {
        return None;
    }
    if menus.screen.as_ref().is_none_or(|s| s.size != size) {
        if menus.screen.as_ref().is_some_and(|s| !s.open.is_empty()) {
            // Menus open keep their layout until they close.
        } else {
            let built = Screen::build(game, size);
            if built.ui.fonts.iter().all(Option::is_none) {
                println!("The game's menus can't be shown: no fonts found.");
                menus.failed = true;
                return None;
            }
            menus.screen = Some(Box::new(built));
        }
    }
    menus.screen.as_deref_mut()
}

/// Whether the game's own menus show a request (the rest are the viewer's
/// panel, `menus`).
pub fn takes(m: &crate::menus::Menu) -> bool {
    message::takes(m)
        || container::takes(m)
        || barter::takes(m)
        || recipe::takes(m)
        || repair::takes(m)
        || companion_wheel::takes(m)
        || caravan::takes(m)
        || levelup::takes(m)
        || traits::takes(m)
        || chargen::takes(m)
        || textedit::takes(m)
        || sleepwait::takes(m)
        || vigor::takes(m)
        || hacking::takes(m)
        || computers::takes(m)
}

/// The world's requests become menus.
#[allow(clippy::too_many_arguments)]
fn open_menus(
    game: Res<GameFiles>,
    mut menus: ResMut<GameMenus>,
    mut queue: ResMut<crate::menus::Menus>,
    mut player: ResMut<crate::walk::Player>,
    mut state: ResMut<crate::dialogue::DialogueState>,
    mut sounds: ResMut<crate::sounds::SoundRequests>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time<bevy::time::Real>>,
    mut asks: ResMut<asks::PipboyAsks>,
    hacking_sounds: Res<hacking::HackingSounds>,
    scripts: Res<crate::scripts::Scripts>,
) {
    let Some(size) = window_size(&windows) else {
        return;
    };
    let wanted = menus.levelup_to_perks;
    let mut to_perks = wanted;
    let sleepwait_hours = menus.sleepwait_hours.take();
    let Some(screen) = screen(&mut menus, &game.0, size) else {
        queue.game_ready = false;
        return;
    };
    queue.game_ready = true;
    while let Some(request) = queue.take_next(takes) {
        let before = screen.open.len();
        if message::takes(&request) {
            message::open(screen, &game.0, request);
        } else if hacking::takes(&request) {
            hacking::open(screen, &game.0, &state.0, request, &hacking_sounds);
        } else if computers::takes(&request) {
            computers::open(screen, &game.0, &state.0, request);
        } else if vigor::takes(&request) {
            sounds
                .0
                .extend(vigor::open(screen, &game.0, &mut state.0, request));
        } else if traits::takes(&request) {
            traits::open(screen, &game.0, &mut state.0, request);
        } else if chargen::takes(&request) {
            chargen::open(screen, &game.0, &mut state.0, request);
        } else if textedit::takes(&request) {
            textedit::open(screen, &game.0, &state.0, time.elapsed_secs_f64() * 1000.0);
        } else if sleepwait::takes(&request) {
            sounds
                .0
                .extend(sleepwait::open(screen, &game.0, &state.0, request));
            if let Some(hours) = sleepwait_hours {
                sleepwait::choose_and_wait(screen, hours);
            }
        } else if levelup::takes(&request) {
            sounds
                .0
                .extend(levelup::open(screen, &game.0, &mut state.0, request));
            if std::mem::take(&mut to_perks) {
                levelup::give_points_and_continue(screen, &game.0, &mut state.0);
            }
        } else if barter::takes(&request) {
            sounds
                .0
                .extend(barter::open(screen, &game.0, &mut state.0, request));
        } else if recipe::takes(&request) {
            recipe::open(screen, &game.0, &mut state.0, request);
        } else if repair::takes(&request) {
            repair::open(screen, &game.0, &mut state.0, request);
        } else if companion_wheel::takes(&request) {
            companion_wheel::open(screen, &game.0, &scripts.0, &mut state.0, request);
        } else if caravan::takes(&request) {
            caravan::open(screen, &game.0, &mut state.0, request);
        } else {
            sounds
                .0
                .extend(container::open(screen, &game.0, &mut state.0, request));
        }
        // A service menu made: the conversation fades out under it.
        if screen.open.len() > before && screen.open.last().is_some_and(OpenMenu::is_service) {
            dialog::service_opened(screen);
        }
        player.ready = false;
    }
    // The Pip-Boy's questions (over it, so the player isn't ready anyway).
    asks::open(screen, &game.0, &mut asks);
    queue.game_open = !screen.open.is_empty();
    if wanted && !to_perks {
        menus.levelup_to_perks = false;
    }
}

/// What the menus need each frame from Bevy: the pointer, buttons, wheel
/// and keys.
#[derive(bevy::ecs::system::SystemParam)]
pub struct MenuInput<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    scroll: Res<'w, AccumulatedMouseScroll>,
    keys: ResMut<'w, ButtonInput<KeyCode>>,
    typed: EventReader<'w, 's, KeyboardInput>,
    time: Res<'w, Time<bevy::time::Real>>,
    fixed: Res<'w, FixedPointer>,
    clicks: ResMut<'w, FixedClicks>,
    fixed_keys: ResMut<'w, FixedKeys>,
}

/// A key as the game's interface turns it into a menu code (`007154b0`):
/// characters as themselves, Backspace, the arrows, Home, End, Delete,
/// Enter, Page Up and Page Down as the interface's own codes.
fn key_code(key: &Key) -> Option<u32> {
    use ui::menu::key;
    Some(match key {
        Key::Character(c) => {
            let ch = c.chars().next()?;
            if !ch.is_ascii() {
                return None;
            }
            ch as u32
        }
        Key::Space => u32::from(b' '),
        Key::Backspace => key::BACKSPACE,
        Key::ArrowLeft => key::LEFT,
        Key::ArrowRight => key::RIGHT,
        Key::ArrowUp => key::UP,
        Key::ArrowDown => key::DOWN,
        Key::Home => key::HOME,
        Key::End => key::END,
        Key::Delete => key::DELETE,
        Key::Enter => key::ENTER,
        Key::PageUp => key::PAGE_UP,
        Key::PageDown => key::PAGE_DOWN,
        _ => return None,
    })
}

/// The open menus get the pointer and the keys, then hand what the player
/// did back to the game.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_open_menus(
    game: Res<GameFiles>,
    mut menus: ResMut<GameMenus>,
    mut queue: ResMut<crate::menus::Menus>,
    mut input: MenuInput,
    mut state: ResMut<crate::dialogue::DialogueState>,
    mut player: ResMut<crate::walk::Player>,
    conversation: Res<crate::dialogue::Conversation>,
    mut sounds: ResMut<crate::sounds::SoundRequests>,
    scripts: Res<crate::scripts::Scripts>,
    mut asks: ResMut<asks::PipboyAsks>,
    mut hacking_sounds: ResMut<hacking::HackingSounds>,
    mut hud_messages: ResMut<crate::hud::HudMessages>,
    mut wheel_voices: ResMut<companion_wheel::WheelVoices>,
) {
    let Some(screen) = menus.screen.as_deref_mut() else {
        input.typed.clear();
        return;
    };
    if screen.open.is_empty() {
        input.typed.clear();
        return;
    }
    let now = input.time.elapsed_secs_f64();
    let dt = input.time.delta_secs();
    dialog::update(screen, &game.0.order, dt);
    container::update(screen);
    textedit::update(screen, now * 1000.0);
    vigor::update(screen, dt);
    companion_wheel::update(screen, &mut wheel_voices, now * 1000.0);
    // The Rest control (T) this frame, for the sleep/wait menu.
    let mut rest_pressed = false;
    {
        let Screen {
            ui,
            interface,
            open,
            rest_down,
            pointer: here,
            ..
        } = &mut *screen;
        let Some(top) = open.last_mut() else {
            return;
        };
        let menu = top.tile();
        // The pointer, in menu units.
        let k = ui.screen_size.resolution_converter();
        let pointer = match input.fixed.0 {
            // A 1920 × 1080 picture's pixels: the screen is 960 units high.
            Some((x, y)) => Some((x * 960.0 / 1080.0, y * 960.0 / 1080.0)),
            None => input
                .windows
                .single()
                .ok()
                .and_then(|w| w.physical_cursor_position().map(|p| (p.x * k, p.y * k))),
        };
        *here = pointer;
        // `--menu-click`: pressed this frame, let go the next.
        let release = std::mem::take(&mut input.clicks.release);
        let press = {
            let due = input.clicks.at.iter().position(|&t| t <= now);
            due.map(|i| input.clicks.at.remove(i)).is_some()
        };
        input.clicks.release = press;
        if let Some((x, y)) = pointer {
            if let OpenMenu::Vigor(v) = top {
                v.pointer = Some(Vec2::new(x / k, y / k));
            }
            interface.pointer(
                ui,
                menu,
                top.code(),
                x,
                y,
                input.mouse.pressed(MouseButton::Left) || press,
                input.mouse.just_pressed(MouseButton::Left) || press,
                input.mouse.just_released(MouseButton::Left) || release,
                now,
            );
        }
        if let OpenMenu::Vigor(v) = top {
            sounds.0.extend(v.inputs(&game.0, &mut state.0));
        }
        // A meter's bar held under the pointer follows it (`007cf6a0`, id
        // 0x66).
        if let (OpenMenu::Start(s), Some((x, _)), true) =
            (&mut *top, pointer, input.mouse.pressed(MouseButton::Left))
        {
            let bar = interface.dragging.or(interface.over).filter(|&o| {
                ui.has(o, t::ID) && ui.number(o, t::ID) as i32 == ui::menus::start::id::BAR
            });
            if let Some(bar) = bar {
                s.menu.drag_meter(ui, bar, x);
            }
        }
        let notches = match input.scroll.unit {
            MouseScrollUnit::Line => input.scroll.delta.y,
            MouseScrollUnit::Pixel => input.scroll.delta.y / 40.0,
        }
        .round() as i32;
        interface.wheel(ui, menu, top.code(), notches);
        // Keys.
        let shift =
            input.keys.pressed(KeyCode::ShiftLeft) || input.keys.pressed(KeyCode::ShiftRight);
        let alt = input.keys.pressed(KeyCode::AltLeft) || input.keys.pressed(KeyCode::AltRight);
        let events: Vec<KeyboardInput> = input.typed.read().cloned().collect();
        for e in events {
            if e.key_code == KeyCode::KeyT {
                match e.state {
                    ButtonState::Pressed => {
                        rest_pressed |= !e.repeat;
                        *rest_down = true;
                    }
                    ButtonState::Released => *rest_down = false,
                }
            }
            if e.state != ButtonState::Pressed {
                continue;
            }
            // LoveTesterMenu polls scan code1e (A), or10 (Q) in French,
            // for Done (00791ab0). This is separate from its XML bindings.
            if let OpenMenu::Vigor(v) = top {
                let french = game
                    .0
                    .settings
                    .get("General", "sLanguage")
                    .is_some_and(|s| s.eq_ignore_ascii_case("FRENCH"));
                if e.key_code == if french { KeyCode::KeyQ } else { KeyCode::KeyA } {
                    v.menu
                        .intents
                        .push(ui::menus::vigor::InputIntent::TileClick(4));
                    sounds.0.extend(v.inputs(&game.0, &mut state.0));
                    continue;
                }
            }
            // Tab or Escape leave the hacking and terminal menus and the
            // companion wheel (their code 10).
            if matches!(
                top,
                OpenMenu::Hacking(_) | OpenMenu::Computers(_) | OpenMenu::CompanionWheel(_)
            ) && matches!(e.logical_key, Key::Escape | Key::Tab)
            {
                use ui::menu::MenuCode;
                match top {
                    OpenMenu::CompanionWheel(w) => {
                        w.menu
                            .special_key(ui, ui::menus::companion_wheel::LEAVE, now * 1000.0);
                    }
                    OpenMenu::Hacking(h) => {
                        h.menu
                            .special_key(ui, ui::menus::hacking::LEAVE, now * 1000.0);
                    }
                    OpenMenu::Computers(c) => {
                        c.menu
                            .special_key(ui, ui::menus::computers::LEAVE, now * 1000.0);
                    }
                    _ => {}
                }
                continue;
            }
            // The Escape control: the message box hears it as a 1 (`0070c4a0`).
            if e.logical_key == Key::Escape {
                // The start menu takes the Escape control as Back
                // (`007cf5e0`).
                if let OpenMenu::Start(s) = top {
                    s.menu.escape(ui, now);
                    continue;
                }
                // The recipe menu's Exit is its X key.
                if top.code().class() == ui::menus::recipe::CLASS {
                    interface.key(ui, menu, top.code(), u32::from(b'X'), false, false, now);
                }
                if top.code().class() == ui::menus::message::CLASS {
                    interface.key(
                        ui,
                        menu,
                        top.code(),
                        ui::menu::key::ESCAPE_TO_MESSAGE,
                        false,
                        false,
                        now,
                    );
                }
                continue;
            }
            if let Some(code) = key_code(&e.logical_key) {
                if let OpenMenu::Vigor(v) = top {
                    let (used, played) = v.key(code, &game.0, &mut state.0);
                    sounds.0.extend(played);
                    if used {
                        continue;
                    }
                }
                interface.key(ui, menu, top.code(), code, shift, alt, now);
                if let OpenMenu::Vigor(v) = top {
                    sounds.0.extend(v.inputs(&game.0, &mut state.0));
                }
            }
        }
        // `--menu-keys`: the ones due.
        while let Some(i) = input.fixed_keys.0.iter().position(|(t, _)| *t <= now) {
            let (_, k) = input.fixed_keys.0.remove(i);
            let code = match k.to_ascii_lowercase().as_str() {
                "left" => Some(ui::menu::key::LEFT),
                "right" => Some(ui::menu::key::RIGHT),
                "up" => Some(ui::menu::key::UP),
                "down" => Some(ui::menu::key::DOWN),
                "enter" => Some(ui::menu::key::ENTER),
                _ => k.chars().next().map(|c| c as u32),
            };
            if let Some(code) = code {
                println!("--menu-keys: {k}");
                interface.key(ui, menu, top.code(), code, false, false, now);
            }
        }
        // The menus have the keys and buttons.
        input.keys.reset_all();
        // Sounds the interface asked for.
        let order = &game.0.order;
        for effect in interface.effects.drain(..) {
            match effect {
                Effect::Sound(name) => {
                    if let Some(id) = order.form_by_editor_id(&name) {
                        sounds.0.push(id);
                    }
                }
            }
        }
    }
    // The hacking and terminal menus' frames (`00767c90`, `00758470`),
    // after the pointer.
    hacking::update(screen, now * 1000.0);
    computers::update(screen, now * 1000.0);
    // The sleep/wait menu's frame: its clicks carried out, the hours pass
    // (`007c0580`).
    let rest_held = screen.rest_down;
    sounds.0.extend(sleepwait::frame(
        screen,
        &game.0,
        &mut state.0,
        dt,
        rest_pressed,
        rest_held,
    ));
    // What the menus did.
    let order = &game.0.order;
    message::after(&mut screen.open, &mut state.0, &mut sounds, order);
    sounds
        .0
        .extend(container::after(screen, &game.0, &scripts.0, &mut state.0));
    sounds
        .0
        .extend(barter::after(screen, &game.0, &mut state.0));
    hud_messages
        .queue
        .extend(recipe::after(screen, &game.0, &mut state.0));
    sounds
        .0
        .extend(repair::after(screen, &game.0, &mut state.0));
    sounds.0.extend(companion_wheel::after(
        screen,
        &game.0,
        &scripts.0,
        &mut state.0,
        &mut wheel_voices,
        now * 1000.0,
    ));
    sounds
        .0
        .extend(caravan::after(screen, &game.0, &mut state.0, now * 1000.0));
    sounds
        .0
        .extend(levelup::after(screen, &game.0, &mut state.0));
    sounds
        .0
        .extend(traits::after(screen, &game.0, &mut state.0));
    sounds
        .0
        .extend(chargen::after(screen, &game.0, &mut state.0));
    textedit::after(screen, &mut state.0);
    asks::after(screen, &mut asks);
    for m in hacking::after(
        screen,
        order,
        &mut state.0,
        &mut hacking_sounds,
        now * 1000.0,
    ) {
        queue.push(m);
    }
    for n in computers::after(
        screen,
        order,
        &scripts.0,
        &mut state.0,
        &mut hacking_sounds,
        now * 1000.0,
    ) {
        println!("{n}");
        hud_messages.queue.push(n);
    }
    // Closed menus leave the screen; a service menu closing brings the
    // conversation back (`007640a0`).
    let mut i = 0;
    while i < screen.open.len() {
        if screen.open[i].closed() {
            let m = screen.open.remove(i);
            screen.ui.detach(m.tile());
            if screen.open.is_empty() {
                screen.interface.over = None;
                screen.interface.focus = None;
            }
            if m.is_service() {
                dialog::service_closed(screen);
            }
        } else {
            i += 1;
        }
    }
    let open = &screen.open;
    queue.game_open = !open.is_empty();
    // (A box over the Pip-Boy closing leaves the Pip-Boy up.)
    if open.is_empty() && conversation.0.is_none() && !queue.pipboy {
        player.ready = true;
    }
}

/// The Escape control opens the start menu as the pause menu
/// (`0070c4a0` at `0070e651` → `007cb7d0(1, 0)`) when no other menu has
/// the keys (the game's menus closed, no conversation, no lock; the
/// Pip-Boy may be up: the start menu opens over it). The game stops
/// (virtual time paused) while it's up. Saving is offered unless the game
/// refuses (`00850fe0`: its refusals in combat and the like aren't
/// followed here).
#[allow(clippy::too_many_arguments)]
fn escape_opens_start_menu(
    game: Res<GameFiles>,
    mut menus: ResMut<GameMenus>,
    mut queue: ResMut<crate::menus::Menus>,
    keys: Res<ButtonInput<KeyCode>>,
    conversation: Res<crate::dialogue::Conversation>,
    mut settings: ResMut<start::GameSettings>,
    mut virt: ResMut<Time<Virtual>>,
    mut player: ResMut<crate::walk::Player>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    let Some(size) = window_size(&windows) else {
        return;
    };
    if queue.others_open() || conversation.0.is_some() {
        return;
    }
    let Some(screen) = screen(&mut menus, &game.0, size) else {
        return;
    };
    if !screen.open.is_empty() {
        return;
    }
    let was_paused = virt.is_paused();
    if start::open(screen, &game.0, &mut settings, true, was_paused) {
        virt.pause();
        queue.game_open = true;
        player.ready = false;
    }
}

/// The start menu's frame: its fades and saving, then what it asked for:
/// back to the game (time going again), a save or load for the save
/// system, the program's end, the settings' new values (the volumes into
/// the music's decks).
#[allow(clippy::too_many_arguments)]
pub(crate) fn start_menu_frame(
    game: Res<GameFiles>,
    mut menus: ResMut<GameMenus>,
    mut settings: ResMut<start::GameSettings>,
    mut saves: ResMut<start::SaveFiles>,
    mut virt: ResMut<Time<Virtual>>,
    time: Res<Time<bevy::time::Real>>,
    mut exit: EventWriter<AppExit>,
    mut music: ResMut<crate::music::Music>,
    mut sounds: ResMut<crate::sounds::SoundRequests>,
) {
    let Some(screen) = menus.screen.as_deref_mut() else {
        return;
    };
    let out = start::frame(screen, &game.0, &mut settings, time.elapsed_secs_f64());
    for s in out.sounds {
        if let Some(id) = game.0.order.form_by_editor_id(&s) {
            sounds.0.push(id);
        }
    }
    if let Some(was_paused) = out.resume {
        if !was_paused {
            virt.unpause();
        }
    }
    if let Some(s) = out.save {
        saves.0 = Some(s);
    }
    for (setting, v) in out.applied {
        use ui::menus::start::Setting;
        match setting {
            Setting::MasterVolume => crate::music::set_volumes(&mut music, |vol| vol.master = v),
            Setting::MusicVolume => crate::music::set_volumes(&mut music, |vol| vol.music = v),
            Setting::RadioVolume => crate::music::set_volumes(&mut music, |vol| vol.radio = v),
            _ => {}
        }
    }
    if out.exit {
        println!("Exit Game.");
        exit.write(AppExit::Success);
    }
}

/// The open menus' pictures and the cursor, for the HUD's layer.
fn draw_menus(
    game: Res<GameFiles>,
    mut menus: ResMut<GameMenus>,
    mut draw: ResMut<MenuDraw>,
    windows: Query<&Window, With<PrimaryWindow>>,
    fixed: Res<FixedPointer>,
    time: Res<Time<bevy::time::Real>>,
    pipboy: Option<Res<crate::pipboy::Pipboy>>,
) {
    let Some(screen) = menus.screen.as_deref_mut() else {
        if !draw.0.is_empty() {
            draw.0.clear();
        }
        if !draw.1.is_empty() {
            draw.1.clear();
        }
        return;
    };
    let tiles = screen.menu_tiles();
    // Each menu's fade alpha (the conversation's under a service menu).
    let alphas: Vec<f32> = screen
        .open
        .iter()
        .map(|m| match m {
            OpenMenu::Dialog(d) => d.fade.alpha(),
            _ => 1.0,
        })
        .collect();
    let mut items: Vec<DrawItem> = Vec::new();
    // The fade to black moves every frame (`007011d0`) and is drawn under
    // the menus while it lasts.
    if let Some(fade) = screen.fade.as_mut() {
        fade.frame(time.delta_secs());
        if !fade.on {
            screen.fade = None;
        }
    }
    {
        let mut files = Files {
            game: &game.0,
            sizes: &mut screen.sizes,
            atlases: &mut screen.atlases,
        };
        if let Some(fader) = screen.fader {
            let alpha = screen.fade.map_or(0.0, |f| f.alpha);
            screen.ui.set_number(fader, t::ALPHA, alpha * 255.0);
            screen
                .ui
                .set_number(fader, t::VISIBLE, f32::from(alpha > 0.0));
            if alpha > 0.0 {
                items.extend(ui::draw_list(&mut screen.ui, fader, &mut files, &|_| None));
            }
        }
        for (&menu, &alpha) in tiles.iter().zip(&alphas) {
            let first = items.len();
            let drawn = ui::draw_list(&mut screen.ui, menu, &mut files, &|_| None);
            items.extend(ui::draw::faded(&mut screen.ui, drawn, alpha));
            // A start menu's `nif` tile (the pause background).
            start::background_draws(
                &mut screen.ui,
                &screen.open,
                &game.0,
                &mut screen.models,
                &mut items,
                menu,
                first,
            );
        }
        // The cursor over everything while a menu is open (the Pip-Boy
        // too: it moves over the screen and the model's buttons,
        // `007f8720`, except where DATA's map hides it, `0079a130`) and
        // the pointer is in the window.
        let wanted = !tiles.is_empty()
            || pipboy
                .as_ref()
                .is_some_and(|p| p.open && !p.cursor_hidden());
        if let Some(cursor) = screen.cursor {
            let k = screen.ui.screen_size.resolution_converter();
            let at = match fixed.0 {
                Some((x, y)) => Some(Vec2::new(x, y) * (960.0 / 1080.0) / k),
                None => windows
                    .single()
                    .ok()
                    .and_then(|w| w.physical_cursor_position()),
            };
            match (at, wanted) {
                (Some(p), true) => {
                    screen.ui.set_number(cursor, t::X, p.x * k);
                    screen.ui.set_number(cursor, t::Y, p.y * k);
                    screen.ui.set_number(cursor, t::VISIBLE, 1.0);
                    items.extend(ui::draw_list(&mut screen.ui, cursor, &mut files, &|_| None));
                }
                _ => screen.ui.set_number(cursor, t::VISIBLE, 0.0),
            }
        }
    }
    let _ = kind::IMAGE;
    if draw.0 != items {
        draw.0 = items;
    }
    // The Pip-Boy's menu first while it's up: the menus over it opened
    // from it, and the HUD keeps the pieces the Pip-Boy's pages leave on
    // (`ui::hud::parts_for_menu`: the messages).
    let classes: Vec<i32> = pipboy
        .as_ref()
        .and_then(|p| p.menu_class())
        .into_iter()
        .chain(screen.open.iter_mut().map(|m| m.code().class()))
        .collect();
    if draw.1 != classes {
        draw.1 = classes;
    }
}
