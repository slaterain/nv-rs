//! The launcher's window: the terminal's screens, typed in a character at a time as the game types
//! them, the highlight moved with the arrow keys or the mouse, Enter or a click to choose; what is
//! typed goes to the prompt. Play runs on a thread of its own and asks the window through
//! [`crate::session`]; the window steps aside while the game plays and comes back when it ends
//! badly (a good end closes the launcher). The picture is drawn again only when something on it
//! changes: a typed character, the cursor's blink, an input.

use crate::gpu::{self, Gpu};
use crate::intro::Intro;
use crate::place::{self, Place};
use crate::screen::{self, Look, Screen};
use crate::session::{self, Event};
use crate::settings;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};

/// How long the game's card may take to be read (a child process: a Vulkan driver that hangs or
/// crashes takes only that process down).
const CARD_LIMIT: Duration = Duration::from_secs(15);

/// The game's look when the game is there and every piece reads, else the plain one (the reason
/// in the log).
fn look_of(game: Option<&Path>, fallout_ini: &Path, plain: bool) -> Look {
    if plain {
        return Look::plain();
    }
    let Some(folder) = game else {
        return Look::plain();
    };
    let Some(g) = crate::game::Game::at(folder) else {
        eprintln!("look: no game at {}: the port's own look", folder.display());
        return Look::plain();
    };
    match Look::from_game(&g, &[fallout_ini.to_path_buf()]) {
        Ok(l) => l,
        Err(why) => {
            eprintln!("look: {why}: the port's own look");
            Look::plain()
        }
    }
}

/// The profile's files as the screens read them (text one character a byte).
fn profile_files(fallout_ini: &Path) -> settings::Files {
    let read = |p: &Path| {
        std::fs::read(p)
            .map(|b| b.iter().map(|&c| c as char).collect())
            .unwrap_or_default()
    };
    settings::Files {
        ini: read(fallout_ini),
        prefs: read(&fallout_ini.with_file_name("FalloutPrefs.ini")),
    }
}

/// The game's card as far as it is known: still asking, the answer (a card and its API, or no
/// card at all), or no answer (the question hung or crashed in the graphics driver).
#[derive(Clone, Debug, PartialEq)]
pub enum Card {
    Asking,
    Known(Option<(String, String)>),
    NoAnswer,
}

/// The start screen's entries: Play, Play with mods (when there are mods), Exit.
pub const PLAY: &str = "Play";
pub const PLAY_WITH_MODS: &str = "Play with mods";
pub const EXIT: &str = "Exit";

/// The start screen. Its status names what the GAME will use: the renderer the play profile
/// chooses and, on Vulkan, the card and API the game's own rule picks (`gpu::game_card`), never
/// the launcher's own drawing device; on the real Direct3D 9 the card of the last start's adapter
/// table.
pub fn start_screen(place: &Place, card: &Card) -> Screen {
    let mut welcome = Vec::new();
    let mut entries = Vec::new();
    match &place.game {
        None => welcome.push(
            "Your game was not found yet. Type or paste its folder below, or drop the folder \
             onto this window, or choose Play to look for it."
                .to_string(),
        ),
        Some(folder) => {
            let name = folder
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| folder.display().to_string());
            welcome.push(format!("Game: {name}"));
            let renderer = settings::Row::Renderer.current(&profile_files(&place.fallout_ini));
            welcome.push(if !renderer.starts_with("Vulkan") {
                match &place.adapter_card {
                    Some(name) => format!("Renderer: Direct3D 9, {name}"),
                    None => "Renderer: Direct3D 9".into(),
                }
            } else {
                match card {
                    Card::Asking => "Renderer: Vulkan".into(),
                    Card::Known(Some((name, api))) => format!("Renderer: {api}, {name}"),
                    // No card for wgpu: on Windows the game takes the real Direct3D 9 by itself.
                    Card::Known(None) if cfg!(windows) => {
                        "Renderer: Direct3D 9 (no Vulkan card)".into()
                    }
                    Card::Known(None) => "Renderer: no graphics card found".into(),
                    // The game still starts on Vulkan; its driver may be what hangs.
                    Card::NoAnswer if cfg!(windows) => {
                        "Renderer: Vulkan (its driver did not answer; Settings' Renderer can \
                         choose Direct3D 9)"
                            .into()
                    }
                    Card::NoAnswer => "Renderer: Vulkan (its driver did not answer)".into(),
                }
            });
        }
    }
    entries.push(PLAY.to_string());
    if place.mods {
        entries.push(PLAY_WITH_MODS.to_string());
    }
    entries.push(SETTINGS.to_string());
    entries.push(EXIT.to_string());
    Screen {
        title: "-nv-rs-".into(),
        welcome,
        entries,
        ..Default::default()
    }
}

/// A text from Play's steps (paragraphs split at line breaks) with its entries.
pub fn message_screen(text: &str, entries: &[String]) -> Screen {
    Screen {
        title: "-nv-rs-".into(),
        welcome: text.split('\n').map(str::to_string).collect(),
        entries: entries.to_vec(),
        ..Default::default()
    }
}

/// The lines of play.sh's output kept on screen (scrolled back with the wheel or Page Up).
const OUTPUT_LINES: usize = 2000;

/// `text` (a piece of a program's output) added to `lines` as a terminal shows it: the piece
/// carries on the last line, each line break starts a new one, a carriage return before it goes;
/// only the last [`OUTPUT_LINES`] stay.
pub fn append_output(lines: &mut Vec<String>, text: &str) {
    for (i, piece) in text.split('\n').enumerate() {
        if i > 0 || lines.is_empty() {
            lines.push(String::new());
        }
        if let Some(last) = lines.last_mut() {
            last.push_str(piece.trim_end_matches('\r'));
        }
    }
    let extra = lines.len().saturating_sub(OUTPUT_LINES);
    lines.drain(..extra);
}

/// Esc's entry: the screen's Back or No, else its last (Exit, OK); none without entries.
pub fn back_entry(entries: &[String]) -> Option<usize> {
    let last = entries.len().checked_sub(1)?;
    Some(
        entries
            .iter()
            .position(|x| x == "Back" || x == "No")
            .unwrap_or(last),
    )
}

/// How much of the monitor's height the first window may take: on Windows 80%, leaving room for
/// its title bar and the taskbar; elsewhere 88%.
pub const HEIGHT_SHARE: f64 = if cfg!(windows) { 0.80 } else { 0.88 };

/// The first window's size, in the monitor's logical units: the terminal's 4:3 shape at most
/// 960 by 720, and no more than [`HEIGHT_SHARE`] of the monitor's height and 90% of its width (a
/// scaled laptop screen is often under 800 high), so the prompt at the bottom shows.
pub fn first_size(monitor: Option<(f64, f64)>) -> (f64, f64) {
    let (w, h) = (f64::from(screen::WIDTH), f64::from(screen::HEIGHT));
    let Some((mw, mh)) = monitor.filter(|&(mw, mh)| mw > 0.0 && mh > 0.0) else {
        return (w, h);
    };
    let height = h.min(mh * HEIGHT_SHARE).min(mw * 0.9 * 3.0 / 4.0);
    ((height * 4.0 / 3.0).floor(), height.floor())
}

/// Where a window of `size` stands centred on a monitor at `origin` of `monitor`'s size (all in
/// the same pixels).
pub fn centred(origin: (i32, i32), monitor: (u32, u32), size: (u32, u32)) -> (i32, i32) {
    let half = |m: u32, s: u32| (i64::from(m) - i64::from(s)).max(0) as i32 / 2;
    (
        origin.0 + half(monitor.0, size.0),
        origin.1 + half(monitor.1, size.1),
    )
}

/// The first window inside a monitor's work area `work` (x, y, width, height, without the taskbar,
/// in physical pixels): its inner size `inner` made smaller (its 4:3 shape kept) until it fits
/// with its frame `frame` (the title bar and borders), and its OUTER rectangle centred there.
/// Returns the inner size and the outer rectangle's position.
pub fn fit_in(
    work: (i32, i32, u32, u32),
    frame: (u32, u32),
    inner: (u32, u32),
) -> ((u32, u32), (i32, i32)) {
    let room = (
        work.2.saturating_sub(frame.0).max(1),
        work.3.saturating_sub(frame.1).max(1),
    );
    let (mut w, mut h) = inner;
    if h > room.1 {
        (w, h) = (room.1 * 4 / 3, room.1);
    }
    if w > room.0 {
        (w, h) = (room.0, room.0 * 3 / 4);
    }
    let at = centred(
        (work.0, work.1),
        (work.2, work.3),
        (w + frame.0, h + frame.1),
    );
    ((w, h), at)
}

/// The monitor's work area (x, y, width, height in physical pixels): on Windows without the
/// taskbar; elsewhere the whole monitor (the window manager keeps its own panels clear).
fn work_area(m: &winit::monitor::MonitorHandle) -> (i32, i32, u32, u32) {
    let whole = (
        m.position().x,
        m.position().y,
        m.size().width,
        m.size().height,
    );
    #[cfg(windows)]
    {
        use winit::platform::windows::MonitorHandleExtWindows;
        #[repr(C)]
        struct MonitorInfo {
            size: u32,
            monitor: [i32; 4],
            work: [i32; 4],
            flags: u32,
        }
        #[link(name = "user32")]
        extern "system" {
            fn GetMonitorInfoW(monitor: isize, info: *mut MonitorInfo) -> i32;
        }
        let mut info = MonitorInfo {
            size: std::mem::size_of::<MonitorInfo>() as u32,
            monitor: [0; 4],
            work: [0; 4],
            flags: 0,
        };
        // SAFETY: `info` is a MONITORINFO with its size set, alive through the call.
        if unsafe { GetMonitorInfoW(m.hmonitor(), &mut info) } != 0 {
            let [l, t, r, b] = info.work;
            if r > l && b > t {
                return (l, t, (r - l) as u32, (b - t) as u32);
            }
        }
    }
    whole
}

/// How long the game must have been running before the launcher's window is minimised on
/// Wayland (where a minimised window cannot be brought back by the program): a start that fails
/// at once leaves it on screen.
pub const WAYLAND_MINIMISE_AFTER: Duration = Duration::from_secs(5);

/// What the window's surface gave for a frame (wgpu's answer without the texture).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Got {
    Texture,
    Outdated,
    Lost,
    Timeout,
    Occluded,
    Validation,
}

/// What a redraw does next.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FrameStep {
    Draw,
    /// The surface set up again for the window, and drawn at the next redraw.
    Reconfigure,
    /// Nothing this time; the next redraw tries again.
    Skip,
    /// The device or the surface is gone: the drawing chosen again ([`gpu::choose`]), else the
    /// launcher goes on without its window.
    ChooseAgain,
}

/// The next step for a frame the surface answered `got`, with the device `lost` or not: a lost
/// device or surface is never drawn on, whatever the surface said.
pub fn frame_step(lost: bool, got: Got) -> FrameStep {
    match got {
        _ if lost => FrameStep::ChooseAgain,
        Got::Texture => FrameStep::Draw,
        Got::Outdated => FrameStep::Reconfigure,
        Got::Lost | Got::Validation => FrameStep::ChooseAgain,
        Got::Timeout | Got::Occluded => FrameStep::Skip,
    }
}

/// Whether the window can show a frame: not stepped aside (`hidden`), not minimised, not of no
/// size. Otherwise nothing is drawn or asked for (a surface that stays outdated would else be
/// asked again nonstop beside the game).
pub fn drawable(hidden: bool, minimised: bool, size: (u32, u32)) -> bool {
    !hidden && !minimised && size.0 > 0 && size.1 > 0
}

/// How long after a frame that was not drawn (its surface set up again, or skipped) the next try
/// waits: never nonstop, and never left for good (the intro's last picture, a message that came
/// in the same moment, must still be drawn).
pub const RETRY_AFTER: Duration = Duration::from_millis(100);

/// When a frame not drawn at `now` is tried again: at once only after a drawing chosen again.
pub fn retry_after(step: FrameStep, now: Instant) -> Option<Instant> {
    match step {
        FrameStep::Draw | FrameStep::ChooseAgain => None,
        FrameStep::Reconfigure | FrameStep::Skip => Some(now + RETRY_AFTER),
    }
}

/// What a frame's step leaves for the next wake, for `key` (what shows now) at `now`: what was
/// last put on screen and when a frame not drawn is tried again. A frame drawn: its key, no
/// retry. A frame not drawn: nothing counts as drawn (so a change the key does not track is
/// drawn too) and a retry after [`RETRY_AFTER`]. A drawing chosen again: nothing drawn, no wait.
pub fn frame_outcome(
    step: FrameStep,
    key: (usize, bool),
    now: Instant,
) -> (Option<(usize, bool)>, Option<Instant>) {
    match step {
        FrameStep::Draw => (Some(key), None),
        _ => (None, retry_after(step, now)),
    }
}

/// What the event loop knows before it waits ([`next_wake`]).
pub struct Waiting {
    /// The window can show a frame ([`drawable`]: not stepped aside, not minimised, not 0x0).
    pub can_draw: bool,
    /// Text is still being typed in (or the intro runs): woken every character.
    pub typing: bool,
    /// How long the screen has shown, for the cursor's blink.
    pub shown_ms: u64,
    pub drawn: Option<(usize, bool)>,
    pub key: (usize, bool),
    pub retry_at: Option<Instant>,
    pub now: Instant,
}

/// When the event loop wakes next (`None`: only for an event) and whether a redraw is asked now.
/// Nothing to draw (minimised, 0x0, stepped aside): no wake, no redraw; restoring the window
/// sends an event (its size) that draws it again. Typing: every character (the menus' 6 ms);
/// otherwise at the cursor's next blink; a frame not drawn is tried again once its short wait is
/// over, even with nothing else to wake for.
pub fn next_wake(w: &Waiting) -> (Option<Instant>, bool) {
    if !w.can_draw {
        return (None, false);
    }
    let next = if w.typing {
        w.now + Duration::from_millis(crate::intro::ms_per_char(crate::intro::DISPLAY_RATE_MENUS))
    } else {
        w.now + Duration::from_millis(screen::BLINK_MS - w.shown_ms % screen::BLINK_MS)
    };
    let next = match w.retry_at {
        Some(at) if w.drawn != Some(w.key) => next.min(at),
        _ => next,
    };
    (Some(next), wants_redraw(w.drawn, w.key, w.retry_at, w.now))
}

/// The redraw's bookkeeping, kept apart from the window so that what the window does with it is
/// tested: what was last put on screen and when a frame not drawn is tried again. The redraw
/// tells it each frame's step ([`FrameState::on_frame`]), a new screen that something else shows
/// ([`FrameState::changed`]), and the event loop asks it when to wake ([`FrameState::wake`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameState {
    pub drawn: Option<(usize, bool)>,
    pub retry_at: Option<Instant>,
}

impl FrameState {
    /// A frame's `step` for `key` (what shows) at `now` ([`frame_outcome`]).
    pub fn on_frame(&mut self, step: FrameStep, key: (usize, bool), now: Instant) {
        (self.drawn, self.retry_at) = frame_outcome(step, key, now);
    }

    /// Something else shows (a new screen, a resize): drawn at the next wake, whatever its key.
    pub fn changed(&mut self) {
        self.drawn = None;
    }

    /// When the event loop wakes next and whether it asks for a redraw now ([`next_wake`]).
    pub fn wake(
        &self,
        can_draw: bool,
        typing: bool,
        shown_ms: u64,
        key: (usize, bool),
        now: Instant,
    ) -> (Option<Instant>, bool) {
        next_wake(&Waiting {
            can_draw,
            typing,
            shown_ms,
            drawn: self.drawn,
            key,
            retry_at: self.retry_at,
            now,
        })
    }
}

/// Whether a redraw is asked for `now`: what shows (`key`) is not what was last put on screen
/// (`drawn`), and no wait after a frame not drawn (`retry_at`) is still running.
pub fn wants_redraw(
    drawn: Option<(usize, bool)>,
    key: (usize, bool),
    retry_at: Option<Instant>,
    now: Instant,
) -> bool {
    drawn != Some(key) && retry_at.is_none_or(|at| now >= at)
}

/// The span within which the drawing is chosen again at most [`CHOOSE_AGAIN_LIMIT`] times.
pub const CHOOSE_AGAIN_SPAN: Duration = Duration::from_secs(1);
/// How often within [`CHOOSE_AGAIN_SPAN`] the drawing may be chosen again.
pub const CHOOSE_AGAIN_LIMIT: usize = 2;

/// Whether the drawing may be chosen again `now`, given when it was (`recent`): a driver that
/// loses each new device at once goes to the no-window way instead of a loop.
pub fn may_choose_again(recent: &[Instant], now: Instant) -> bool {
    recent
        .iter()
        .filter(|t| now.duration_since(**t) < CHOOSE_AGAIN_SPAN)
        .count()
        < CHOOSE_AGAIN_LIMIT
}

/// Play's work, `work`, run so that its end is always known: a panic in it ends as Rust's
/// panic code (101), so `Ended` is sent and a closed launcher still ends.
pub fn work_code(work: impl FnOnce() -> i32) -> i32 {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).unwrap_or(101)
}

/// The kinds of screen, for what Enter and Esc do there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Intro,
    Start,
    Refused,
    Waiting,
    Message,
    Folder,
    /// play.sh's output while it runs (typing answers its questions).
    Output,
    /// play.sh has ended badly: Back or Exit.
    Ended,
    /// The Settings list (its rows, Testing, Back).
    Settings,
    /// The Testing list (its rows, Back).
    Testing,
    /// One row's choices (and Back).
    Choices,
}

/// What a choice does.
#[derive(Debug, PartialEq)]
pub enum Act {
    EndIntro,
    Play {
        mods: bool,
        game: Option<PathBuf>,
    },
    Exit,
    Answer(usize),
    Folder(Option<PathBuf>),
    TypeLine(String),
    BackToStart,
    OpenSettings,
    OpenTesting,
    /// The row at this place of the list on screen.
    OpenChoices(usize),
    /// The choice of the row whose choices show.
    Save(String),
    /// From a row's choices back to its list.
    BackToList,
    Nothing,
}

/// The Settings list's entry that opens the Testing list, and the start screen's for Settings.
pub const TESTING_ENTRY: &str = "Testing";
pub const SETTINGS: &str = "Settings";
pub const BACK: &str = "Back";

/// Whether typing (and so a paste) goes to the prompt on a screen of `kind`: the start screen's
/// folder, Play's folder question and play.sh's output only; never a menu, a message or the
/// intro.
pub fn takes_typing(kind: Kind) -> bool {
    matches!(kind, Kind::Start | Kind::Folder | Kind::Output)
}

/// How long a paste's answer may take: one that comes later is dropped (the player has moved on).
pub const PASTE_WAIT: Duration = Duration::from_millis(500);

/// Whether the clipboard's answer for paste `id`, arriving `now` on a screen of `kind`, is added
/// at the prompt: it is the paste asked last (`asked`: its id, when, on what screen), within
/// [`PASTE_WAIT`], and the screen is still of the kind it was asked on.
pub fn accept_paste(
    asked: Option<(u64, Instant, Kind)>,
    id: u64,
    now: Instant,
    kind: Kind,
) -> bool {
    asked.is_some_and(|(asked, at, on)| {
        asked == id && now.saturating_duration_since(at) <= PASTE_WAIT && on == kind
    })
}

/// Enter on a screen of `kind` with `typed` at the prompt and the entry `chosen` (its index and
/// text) highlighted.
pub fn on_enter(kind: Kind, typed: &str, chosen: usize, entry: &str) -> Act {
    let typed_folder = (!typed.trim().is_empty()).then(|| session::typed_folder(typed));
    match kind {
        Kind::Intro => Act::EndIntro,
        Kind::Start if typed_folder.is_some() => Act::Play {
            mods: false,
            game: typed_folder,
        },
        Kind::Start if entry == PLAY => Act::Play {
            mods: false,
            game: None,
        },
        Kind::Start if entry == PLAY_WITH_MODS => Act::Play {
            mods: true,
            game: None,
        },
        Kind::Start if entry == SETTINGS => Act::OpenSettings,
        Kind::Start | Kind::Refused | Kind::Ended if entry == EXIT => Act::Exit,
        Kind::Ended => Act::BackToStart,
        Kind::Start | Kind::Refused | Kind::Waiting => Act::Nothing,
        Kind::Message => Act::Answer(chosen),
        Kind::Folder => Act::Folder(typed_folder),
        Kind::Output => Act::TypeLine(typed.to_string()),
        Kind::Settings if entry == TESTING_ENTRY => Act::OpenTesting,
        Kind::Settings if entry == BACK => Act::BackToStart,
        Kind::Testing if entry == BACK => Act::OpenSettings,
        Kind::Settings | Kind::Testing => Act::OpenChoices(chosen),
        Kind::Choices if entry == BACK => Act::BackToList,
        Kind::Choices => Act::Save(entry.to_string()),
    }
}

/// A choice's name without its note: "Vulkan (recommended)" is "Vulkan" in a list.
pub fn short(choice: &str) -> &str {
    choice.split(" (").next().unwrap_or(choice)
}

/// The Settings list's (or the Testing list's) entries for what `shown` says: each row with its
/// value, then (the main list) Testing, then Back.
pub fn settings_entries(
    rows: &[settings::Row],
    shown: &settings::Files,
    testing: bool,
) -> Vec<String> {
    let mut e: Vec<String> = rows
        .iter()
        .map(|r| format!("{}: {}", r.label(), short(&r.current(shown))))
        .collect();
    if !testing {
        e.push(TESTING_ENTRY.into());
    }
    e.push(BACK.into());
    e
}

/// Left and Right on a row: the choice before or after the one it has, round the list.
/// "Game's setting" is passed over (it writes nothing, so stepping onto it would leave the row
/// where it was and Left/Right stuck): from it, Right gives the first other choice and Left the
/// last; from a value the list lacks, the same.
pub fn stepped_choice(choices: &[String], now: &str, forward: bool) -> Option<String> {
    let choices: Vec<&String> = choices
        .iter()
        .filter(|c| c.as_str() != settings::OWN)
        .collect();
    let n = choices.len();
    let Some(i) = choices.iter().position(|c| c.as_str() == now) else {
        let end = if forward {
            choices.first()
        } else {
            choices.last()
        };
        return end.map(|c| (*c).clone());
    };
    let next = if forward {
        (i + 1) % n
    } else {
        (i + n - 1) % n
    };
    Some(choices[next].clone())
}

/// Esc on a screen of `kind`: its Back, No or Exit ([`back_entry`]), whatever is typed at the
/// prompt (typed text is never sent by Esc).
pub fn on_escape(kind: Kind, entries: &[String]) -> Act {
    if kind == Kind::Waiting || kind == Kind::Output {
        return Act::Nothing;
    }
    match back_entry(entries) {
        Some(i) => on_enter(kind, "", i, &entries[i]),
        None => Act::Nothing,
    }
}

/// What the window shows now.
enum Mode {
    /// The game's intro (the first start only).
    Intro(Intro),
    Start,
    /// Play's waiting screen, with the step's line.
    Waiting,
    /// A message or question of Play's; the reply goes back with the entry chosen.
    Message(Option<Sender<usize>>),
    /// Play asks for the game's folder.
    Folder(Option<Sender<Option<PathBuf>>>),
    /// play.sh's output (Linux, macOS); its exit code once it has ended.
    Output(Option<i32>),
    /// nv-rs may not write in its own folder: only Exit.
    Refused,
    /// The Settings list (`testing`: the Testing list).
    Settings {
        testing: bool,
    },
    /// The choices of the row at `index` of the Settings (or Testing) list.
    Choices {
        testing: bool,
        index: usize,
    },
}

impl Mode {
    fn kind(&self) -> Kind {
        match self {
            Mode::Intro(_) => Kind::Intro,
            Mode::Start => Kind::Start,
            Mode::Waiting => Kind::Waiting,
            Mode::Message(_) => Kind::Message,
            Mode::Folder(_) => Kind::Folder,
            Mode::Output(None) => Kind::Output,
            Mode::Output(Some(_)) => Kind::Ended,
            Mode::Refused => Kind::Refused,
            Mode::Settings { testing: false } => Kind::Settings,
            Mode::Settings { testing: true } => Kind::Testing,
            Mode::Choices { .. } => Kind::Choices,
        }
    }
}

/// The rows of the Settings list, or of the Testing list.
fn rows_of(testing: bool) -> &'static [settings::Row] {
    if testing {
        &settings::TESTING
    } else {
        &settings::ROWS
    }
}

struct App {
    look: Look,
    place: Place,
    proxy: EventLoopProxy<Event>,
    /// The launcher's own drawing API's instance (kept as long as its device).
    instance: Option<wgpu::Instance>,
    window: Option<Arc<winit::window::Window>>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    gpu: Option<Gpu>,
    mode: Mode,
    screen: Screen,
    started: Instant,
    finished: bool,
    rows: Option<screen::Rows>,
    mouse: (f64, f64),
    card: Card,
    /// The action named on the command line, started as soon as the window is there.
    action: Option<Vec<OsString>>,
    /// play.sh's output so far, in lines (the last one may be a question without its line break).
    output: Vec<String>,
    hidden: bool,
    /// Wayland: when the hidden window is minimised (the game has run a moment by then).
    minimise_at: Option<Instant>,
    /// Play's work is running (its one-game claim held).
    working: bool,
    /// The window was closed while Play ran: the launcher ends when Play ends.
    closed: bool,
    /// What the last drawn picture showed ([`App::frame_key`]) and when a frame not drawn is tried
    /// again ([`FrameState`]).
    frame: FrameState,
    /// When the drawing was last chosen again ([`may_choose_again`]).
    chosen_again: Vec<Instant>,
    /// The window could not be made or drawn in ([`cannot_draw`] said why).
    no_window: bool,
    exit_code: i32,
    /// The keyboard's Ctrl, Shift, Alt and Command keys now (for the paste key).
    modifiers: winit::keyboard::ModifiersState,
    /// Pastes asked so far (each one's id), and the one being read: its id, when, on what screen.
    pastes: u64,
    asked_paste: Option<(u64, Instant, Kind)>,
    /// The main monitor's size for the resolution row, as the game sees it.
    desktop: (u32, u32),
}

/// Starts Play's work on a thread of its own with `args` (Play.exe's, or play.sh's on Linux and
/// macOS); it always sends `Ended` with its exit code ([`work_code`]: 101 after a panic).
fn start_work(args: Vec<OsString>) {
    std::thread::spawn(move || {
        let code = work_code(move || {
            #[cfg(windows)]
            {
                let args: Vec<String> = args
                    .iter()
                    .map(|a| a.to_string_lossy().into_owned())
                    .collect();
                crate::play::run_play(&args)
            }
            #[cfg(not(windows))]
            match crate::play_unix::script() {
                Some(script) => crate::play_unix::run(&script, &args),
                None => 127,
            }
        });
        session::send(Event::Ended(code));
    });
}

/// Play's arguments, the same for Play.exe's steps and play.sh: `--mods` for Play with mods, and
/// `--game FOLDER` for a folder the player typed or dropped (kept as the system's own bytes).
pub fn play_arguments(mods: bool, game: Option<&Path>) -> Vec<OsString> {
    let mut a = Vec::new();
    if mods {
        a.push("--mods".into());
    }
    if let Some(game) = game {
        a.push("--game".into());
        a.push(game.as_os_str().to_owned());
    }
    a
}

/// Asks this program, in a process of its own, for the game's card (`--game-card`), within
/// [`CARD_LIMIT`]: its answer, or [`Card::NoAnswer`] when it hung or ended without one.
fn ask_game_card() -> Card {
    use std::io::Read;
    let Ok(exe) = std::env::current_exe() else {
        return Card::NoAnswer;
    };
    let Ok(mut child) = std::process::Command::new(exe)
        .arg("--game-card")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        return Card::NoAnswer;
    };
    let Some(mut out) = child.stdout.take() else {
        return Card::NoAnswer;
    };
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < CARD_LIMIT => {
                std::thread::sleep(Duration::from_millis(25))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Card::NoAnswer;
            }
        }
    };
    let text = reader.join().unwrap_or_default();
    card_answer(status.success(), &text)
}

/// `--game-card`'s answer: a line with the card's name and API, an empty line for no card;
/// anything else, or a program that did not end well (a crash in the driver), no answer.
pub fn card_answer(ended_well: bool, text: &str) -> Card {
    if !ended_well {
        return Card::NoAnswer;
    }
    match card_from_line(text) {
        Some(card) => Card::Known(Some(card)),
        None if text.trim().is_empty() => Card::Known(None),
        None => Card::NoAnswer,
    }
}

/// `--game-card`'s line: the card's name and its API.
pub fn card_from_line(text: &str) -> Option<(String, String)> {
    let (name, api) = text.lines().next()?.split_once('\t')?;
    (!name.is_empty()).then(|| (name.to_string(), api.to_string()))
}

impl App {
    fn shown(&self) -> usize {
        if self.finished {
            usize::MAX
        } else {
            (self.started.elapsed().as_secs_f32() * screen::TYPE_RATE) as usize
        }
    }

    fn cursor_on(&self) -> bool {
        (self.started.elapsed().as_millis() as u64 / screen::BLINK_MS).is_multiple_of(2)
    }

    /// What the picture shows now, to know when it must be drawn again: the characters typed so
    /// far (the intro's, or the screen's up to its whole text) and the cursor's blink.
    fn frame_key(&self) -> (usize, bool) {
        match &self.mode {
            Mode::Intro(intro) => {
                let ms = self.started.elapsed().as_millis() as u64;
                let typed: usize = intro.typed(ms).iter().map(|l| l.chars().count()).sum();
                (typed + usize::from(ms >= intro.total_ms()), false)
            }
            _ => (
                self.shown()
                    .min(screen::total_chars(&self.look, &self.screen)),
                self.cursor_on(),
            ),
        }
    }

    /// Shows `screen`, typed in from its start (`typed`) or whole at once.
    fn show(&mut self, mode: Mode, screen: Screen, typed: bool) {
        self.mode = mode;
        self.screen = screen;
        self.started = Instant::now();
        self.finished = !typed;
        // Something else shows: drawn whatever the last key was (a new screen can have the same).
        self.frame.changed();
        if self.hidden && !self.closed {
            if let Some(w) = &self.window {
                w.set_minimized(false);
                w.set_visible(true);
                w.focus_window();
                // Where the program cannot bring its window forward (Wayland, a minimised
                // window), the desktop at least shows that it asks for the player.
                w.request_user_attention(Some(winit::window::UserAttentionType::Critical));
            }
            self.hidden = false;
            self.minimise_at = None;
        }
        self.request_redraw();
    }

    fn request_redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn show_start(&mut self) {
        if let Some((why, code)) = self.place.refused.clone() {
            // Play.exe's stop code for the refusal, as it ended with it.
            self.exit_code = code;
            let s = message_screen(&why, &[EXIT.to_string()]);
            self.show(Mode::Refused, s, true);
            return;
        }
        let s = start_screen(&self.place, &self.card);
        self.show(Mode::Start, s, true);
    }

    /// Play (with mods), and with a game folder typed or dropped.
    fn play(&mut self, mods: bool, game: Option<PathBuf>) {
        let args = play_arguments(mods, game.as_deref());
        self.output.clear();
        self.back_to_waiting();
        self.working = true;
        start_work(args);
    }

    /// Whether the window can show a frame now ([`drawable`]).
    fn can_draw(&self) -> bool {
        let Some(w) = &self.window else {
            return false;
        };
        let size = w.inner_size();
        drawable(
            self.hidden,
            w.is_minimized().unwrap_or(false),
            (size.width, size.height),
        )
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        if !self.can_draw() {
            return;
        }
        // A device the driver lost is never drawn on (wgpu would fail the frame): chosen again.
        if self.gpu.as_ref().is_some_and(Gpu::is_lost) {
            return self.choose_again(event_loop, "its device was lost");
        }
        let shown = self.shown();
        let cursor = self.cursor_on();
        let key = self.frame_key();
        let (Some(surface), Some(config), Some(gpu)) = (&self.surface, &self.config, &mut self.gpu)
        else {
            return;
        };
        let canvas = if let Mode::Intro(intro) = &self.mode {
            let lines = intro.typed(self.started.elapsed().as_millis() as u64);
            self.rows = None;
            screen::draw_intro(&self.look, &lines)
        } else {
            let (canvas, rows) = screen::draw(&self.look, &self.screen, shown, cursor);
            self.rows = Some(rows);
            canvas
        };
        let picture = (screen::WIDTH, screen::HEIGHT);
        gpu.upload(&canvas.pixels, picture);
        use wgpu::CurrentSurfaceTexture as T;
        let (got, frame) = match surface.get_current_texture() {
            T::Success(f) | T::Suboptimal(f) => (Got::Texture, Some(f)),
            T::Outdated => (Got::Outdated, None),
            T::Lost => (Got::Lost, None),
            T::Timeout => (Got::Timeout, None),
            T::Occluded => (Got::Occluded, None),
            T::Validation => (Got::Validation, None),
        };
        let step = frame_step(gpu.is_lost(), got);
        let frame = match (step, frame) {
            (FrameStep::Draw, Some(f)) => f,
            (FrameStep::ChooseAgain, _) => {
                return self.choose_again(event_loop, &format!("its window surface: {got:?}"))
            }
            // Not drawn now: tried again after a short wait ([`retry_after`]), never at once in a
            // loop and never left undrawn (the intro's last picture has no later change).
            _ => {
                if step == FrameStep::Reconfigure {
                    surface.configure(&gpu.device, config);
                }
                self.frame.on_frame(step, key, Instant::now());
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        gpu.draw(
            &mut encoder,
            &view,
            config.format,
            (config.width, config.height),
            picture,
        );
        gpu.queue.submit([encoder.finish()]);
        gpu.queue.present(frame);
        self.frame.on_frame(FrameStep::Draw, key, Instant::now());
        let intro_over = match &self.mode {
            Mode::Intro(intro) => {
                Some(self.started.elapsed().as_millis() as u64 >= intro.total_ms())
            }
            _ => None,
        };
        match intro_over {
            Some(true) => self.end_intro(),
            Some(false) => {}
            None => {
                if !self.finished && shown >= screen::total_chars(&self.look, &self.screen) {
                    self.finished = true;
                }
            }
        }
    }

    /// The drawing stopped while the launcher runs (`why`, said in the log): the device and the
    /// surface dropped and chosen again for the window as at the start ([`gpu::choose`]); if
    /// nothing draws now, or the drawing was already chosen again twice within a second
    /// ([`may_choose_again`]: a driver that keeps losing it), the launcher goes on without its
    /// window ([`App::no_drawing`]).
    fn choose_again(&mut self, event_loop: &ActiveEventLoop, why: &str) {
        let now = Instant::now();
        if !may_choose_again(&self.chosen_again, now) {
            return self.no_drawing(event_loop, &format!("{why}, again and again"));
        }
        self.chosen_again
            .retain(|t| now.duration_since(*t) < CHOOSE_AGAIN_SPAN);
        self.chosen_again.push(now);
        eprintln!("the launcher's drawing stopped ({why}): choosing its device again");
        self.surface = None;
        self.config = None;
        self.gpu = None;
        self.instance = None;
        self.frame.changed();
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        match gpu::choose(Some(window), (size.width, size.height)) {
            Ok(drawing) => {
                eprintln!("drawing with {}", drawing.gpu.describe());
                let Some((surface, config)) = drawing.surface else {
                    unreachable!("a window's drawing always has its surface");
                };
                self.instance = Some(drawing.instance);
                self.surface = Some(surface);
                self.config = Some(config);
                self.gpu = Some(drawing.gpu);
                self.request_redraw();
            }
            Err(e) => self.no_drawing(event_loop, &e),
        }
    }

    /// Nothing draws the window any more. With no Play running: said as at the start, and Play
    /// goes on without the window (Windows' message boxes; play.sh in the terminal). While Play
    /// runs: said, and the window goes as a close takes it ([`App::close_while_working`]).
    fn no_drawing(&mut self, event_loop: &ActiveEventLoop, why: &str) {
        if self.working {
            cannot_draw_then(
                why,
                &self.place,
                "Play goes on without it: its questions are answered as Esc answers them, and \
                 nv-rs ends when Play ends.",
            );
            self.close_while_working(event_loop);
        } else {
            cannot_draw(why, &self.place);
            self.no_window = true;
            event_loop.exit();
        }
    }

    /// The window goes while Play runs (the game may be starting, its one-game claim held): a
    /// question it asks is answered as Esc answers it, later ones too, play.sh's input is closed,
    /// and the launcher ends when Play ends, with its code.
    fn close_while_working(&mut self, event_loop: &ActiveEventLoop) {
        if matches!(self.mode, Mode::Message(_) | Mode::Folder(_)) {
            self.back(event_loop);
        }
        self.closed = true;
        crate::play_unix::close_input();
        if let Some(w) = &self.window {
            w.set_visible(false);
            if on_wayland(event_loop) {
                w.set_minimized(true);
            }
        }
        self.hidden = true;
    }

    /// The intro is over (or skipped): noted in the launcher's own file (on Windows once Play has
    /// passed its first-write checks, so nothing is written into the package before), then the
    /// start screen is typed (`FinishIntro`).
    fn end_intro(&mut self) {
        if let Err(e) = place::intro_ended(&self.place.own, self.place.refused.is_some()) {
            eprintln!("launcher.ini: {e}");
        }
        self.show_start();
    }

    /// The row under the window position, if any.
    fn row_at(&self, pos: (f64, f64)) -> Option<usize> {
        let (config, rows) = (self.config.as_ref()?, self.rows.as_ref()?);
        let (x, y, w, h) = gpu::fit(
            (screen::WIDTH, screen::HEIGHT),
            (config.width, config.height),
        );
        let px = (pos.0 - x as f64) * screen::WIDTH as f64 / w as f64;
        let py = (pos.1 - y as f64) * screen::HEIGHT as f64 / h as f64;
        if px < rows.x as f64 || px > (rows.x + rows.width as i32) as f64 || py < rows.top as f64 {
            return None;
        }
        let k = ((py - rows.top as f64) / rows.row as f64) as usize;
        (k < rows.shown).then_some(rows.first + k)
    }

    /// Whether what is typed goes to the prompt on this screen.
    /// Does what a choice does.
    fn act(&mut self, act: Act, event_loop: &ActiveEventLoop) {
        match act {
            Act::EndIntro => self.end_intro(),
            Act::Play { mods, game } => self.play(mods, game),
            Act::Exit => event_loop.exit(),
            Act::Answer(i) => {
                if let Mode::Message(reply) = &mut self.mode {
                    if let Some(r) = reply.take() {
                        let _ = r.send(i);
                    }
                }
                self.back_to_waiting();
            }
            Act::Folder(answer) => {
                if let Mode::Folder(reply) = &mut self.mode {
                    if let Some(r) = reply.take() {
                        let _ = r.send(answer);
                    }
                }
                self.back_to_waiting();
            }
            Act::TypeLine(line) => {
                crate::play_unix::type_line(&line);
                self.add_output(&format!("{line}\n"));
            }
            Act::BackToStart => self.show_start(),
            Act::OpenSettings => self.show_settings(false, 0, None),
            Act::OpenTesting => self.show_settings(true, 0, None),
            Act::OpenChoices(index) => {
                let testing = matches!(self.mode, Mode::Settings { testing: true });
                if index < rows_of(testing).len() {
                    self.show_choices(testing, index);
                }
            }
            Act::Save(choice) => {
                if let Mode::Choices { testing, index } = self.mode {
                    self.save(testing, index, &choice);
                }
            }
            Act::BackToList => {
                if let Mode::Choices { testing, index } = self.mode {
                    self.show_settings(testing, index, None);
                }
            }
            Act::Nothing => {}
        }
    }

    /// The Settings list (or the Testing list), the row at `highlight` highlighted, with `note`
    /// (a save that failed) above it.
    fn show_settings(&mut self, testing: bool, highlight: usize, note: Option<String>) {
        let profiles = crate::profiles::Profiles::at(&self.place.own);
        let (shown, in_files) = match profiles.shown().and_then(|s| Ok((s, profiles.in_files()?))) {
            Ok(v) => v,
            Err(e) => {
                let s = message_screen(
                    &format!("nv-rs cannot read its settings: {e}"),
                    &[BACK.to_string()],
                );
                self.show(Mode::Settings { testing }, s, true);
                return;
            }
        };
        let mut welcome = Vec::new();
        if let Some(note) = note {
            welcome.push(note);
        }
        welcome.push(if in_files {
            "Saved at once, for both ways of playing (with and without mods). The game reads them at its next start.".to_string()
        } else {
            "Saved for Play to move into the game's own files at its next start.".to_string()
        });
        if testing {
            welcome.push(
                "Still being tried: the new lighting draws on Vulkan only; JIP LN for mods that need it."
                    .into(),
            );
        }
        let mut s = Screen {
            title: if testing { "-Testing-" } else { "-Settings-" }.into(),
            welcome,
            entries: settings_entries(rows_of(testing), &shown, testing),
            ..Default::default()
        };
        s.highlighted = highlight.min(s.entries.len() - 1);
        let retype = !matches!(self.mode, Mode::Settings { .. } | Mode::Choices { .. });
        self.show(Mode::Settings { testing }, s, retype);
    }

    /// The choices of the row at `index` of the list, the one it has highlighted.
    fn show_choices(&mut self, testing: bool, index: usize) {
        let row = rows_of(testing)[index];
        let profiles = crate::profiles::Profiles::at(&self.place.own);
        let shown = match profiles.shown() {
            Ok(s) => s,
            Err(e) => return self.show_settings(testing, index, Some(format!("Not read: {e}"))),
        };
        let now = row.current(&shown);
        let mut entries = row.choices(&shown, self.desktop);
        let highlighted = entries.iter().position(|c| *c == now).unwrap_or(0);
        entries.push(BACK.into());
        let s = Screen {
            title: format!("-{}-", row.label()),
            welcome: vec![format!("Now: {now}")],
            entries,
            highlighted,
            ..Default::default()
        };
        self.show(Mode::Choices { testing, index }, s, false);
    }

    /// `choice` saved for the row at `index`, then its list again (with what went wrong when it
    /// was not saved).
    fn save(&mut self, testing: bool, index: usize, choice: &str) {
        let row = rows_of(testing)[index];
        let profiles = crate::profiles::Profiles::at(&self.place.own);
        let note = profiles
            .save(row, choice)
            .err()
            .map(|e| format!("Not saved: {e}"));
        self.show_settings(testing, index, note);
    }

    /// Left or Right on a row of the Settings (or Testing) list: its next or previous choice,
    /// saved at once.
    fn step_row(&mut self, forward: bool) {
        let Mode::Settings { testing } = self.mode else {
            return;
        };
        let index = self.screen.highlighted;
        let Some(&row) = rows_of(testing).get(index) else {
            return;
        };
        let profiles = crate::profiles::Profiles::at(&self.place.own);
        let Ok(shown) = profiles.shown() else {
            return;
        };
        if let Some(next) = stepped_choice(
            &row.choices(&shown, self.desktop),
            &row.current(&shown),
            forward,
        ) {
            self.save(testing, index, &next);
        }
    }

    /// Enter: the highlighted entry chosen (or, with something typed, the typed line).
    fn choose(&mut self, event_loop: &ActiveEventLoop) {
        let typed = std::mem::take(&mut self.screen.typed);
        let chosen = self.screen.highlighted;
        let entry = self.screen.entries.get(chosen).cloned().unwrap_or_default();
        let act = on_enter(self.mode.kind(), &typed, chosen, &entry);
        self.act(act, event_loop);
    }

    /// Esc: the screen's way back, whatever is typed.
    fn back(&mut self, event_loop: &ActiveEventLoop) {
        let act = on_escape(self.mode.kind(), &self.screen.entries);
        if act != Act::Nothing {
            self.screen.typed.clear();
        }
        self.act(act, event_loop);
    }

    /// After an answer, the waiting screen until Play says more.
    fn back_to_waiting(&mut self) {
        let s = Screen {
            title: "-nv-rs-".into(),
            welcome: vec!["Getting ready...".into()],
            ..Default::default()
        };
        self.show(Mode::Waiting, s, false);
    }

    /// play.sh's output, added as it comes; the screen follows its end.
    fn add_output(&mut self, text: &str) {
        append_output(&mut self.output, text);
        let ended = match self.mode {
            Mode::Output(e) => e,
            _ => None,
        };
        let typed = std::mem::take(&mut self.screen.typed);
        let mut s = Screen {
            title: "-nv-rs-".into(),
            welcome: self.output.clone(),
            follow_end: true,
            typed,
            ..Default::default()
        };
        if ended.is_some() {
            s.entries = vec!["Back".into(), EXIT.into()];
        }
        if self.hidden && ended.is_some() {
            self.show(Mode::Output(ended), s, false);
        } else {
            self.mode = Mode::Output(ended);
            self.screen = s;
            self.finished = true;
        }
        self.request_redraw();
    }

    /// The game's window is there: the launcher's steps aside (hidden; on Wayland, where a
    /// program cannot hide its window, minimised once the game has run [`WAYLAND_MINIMISE_AFTER`],
    /// so a start that fails at once leaves it on screen).
    fn step_aside(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.set_visible(false);
        }
        if on_wayland(event_loop) {
            self.minimise_at = Some(Instant::now() + WAYLAND_MINIMISE_AFTER);
        }
        self.hidden = true;
    }
}

/// The monitor the first window opens on: the primary one, else the first listed.
fn first_monitor(event_loop: &ActiveEventLoop) -> Option<winit::monitor::MonitorHandle> {
    event_loop
        .primary_monitor()
        .or_else(|| event_loop.available_monitors().next())
}

/// Whether winit runs on Wayland (not X11, Windows or macOS).
fn on_wayland(event_loop: &ActiveEventLoop) -> bool {
    #[cfg(all(not(windows), not(target_vendor = "apple")))]
    {
        use winit::platform::wayland::ActiveEventLoopExtWayland;
        event_loop.is_wayland()
    }
    #[cfg(any(windows, target_vendor = "apple"))]
    {
        let _ = event_loop;
        false
    }
}

impl winit::application::ApplicationHandler<Event> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let monitor = first_monitor(event_loop);
        let (w, h) = first_size(monitor.as_ref().map(|m| {
            let size = m.size().to_logical::<f64>(m.scale_factor());
            (size.width, size.height)
        }));
        // The resolution row's own size: Windows' as the DPI-unaware game sees it, elsewhere the
        // monitor's.
        #[cfg(windows)]
        {
            let (dw, dh) = crate::play::desktop_size();
            self.desktop = (dw.max(0) as u32, dh.max(0) as u32);
        }
        #[cfg(not(windows))]
        if let Some(m) = &monitor {
            self.desktop = (m.size().width, m.size().height);
        }
        // Made hidden, then fitted into the monitor's work area with its frame (below), then shown.
        let mut attributes = winit::window::Window::default_attributes()
            .with_title("nv-rs")
            .with_visible(false)
            .with_inner_size(winit::dpi::LogicalSize::new(w, h))
            .with_min_inner_size(winit::dpi::LogicalSize::new(
                screen::WIDTH / 2,
                screen::HEIGHT / 2,
            ));
        // Centred on its monitor (Windows would place it at its own default spot).
        if let Some(m) = &monitor {
            let scale = m.scale_factor();
            let size = ((w * scale).round() as u32, (h * scale).round() as u32);
            let at = centred(
                (m.position().x, m.position().y),
                (m.size().width, m.size().height),
                size,
            );
            attributes = attributes.with_position(winit::dpi::PhysicalPosition::new(at.0, at.1));
        }
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                cannot_draw(&format!("the window could not be made: {e}"), &self.place);
                self.no_window = true;
                event_loop.exit();
                return;
            }
        };
        // The whole window (its title bar and borders too) inside the work area, centred there:
        // the frame is known only now that the window is made.
        if let Some(m) = &monitor {
            let (inner, outer) = (window.inner_size(), window.outer_size());
            let frame = (
                outer.width.saturating_sub(inner.width),
                outer.height.saturating_sub(inner.height),
            );
            let (fitted, at) = fit_in(work_area(m), frame, (inner.width, inner.height));
            if fitted != (inner.width, inner.height) {
                let _ =
                    window.request_inner_size(winit::dpi::PhysicalSize::new(fitted.0, fitted.1));
            }
            window.set_outer_position(winit::dpi::PhysicalPosition::new(at.0, at.1));
        }
        let size = window.inner_size();
        let drawing = match gpu::choose(Some(window.clone()), (size.width, size.height)) {
            Ok(d) => d,
            Err(e) => {
                cannot_draw(&e, &self.place);
                self.no_window = true;
                event_loop.exit();
                return;
            }
        };
        eprintln!(
            "drawing with {}; look: {}",
            drawing.gpu.describe(),
            self.look.from
        );
        let Some((surface, config)) = drawing.surface else {
            unreachable!("a window's drawing always has its surface");
        };
        self.instance = Some(drawing.instance);
        self.config = Some(config);
        self.surface = Some(surface);
        self.gpu = Some(drawing.gpu);
        window.set_visible(true);
        self.window = Some(window);
        let proxy = std::sync::Mutex::new(self.proxy.clone());
        session::connect(move |e| proxy.lock().is_ok_and(|p| p.send_event(e).is_ok()));
        // The game's card, from a process of its own.
        std::thread::spawn(|| {
            session::send(Event::Card(ask_game_card()));
        });
        if let Some(args) = self.action.take() {
            if self.place.refused.is_some() {
                self.show_start();
            } else {
                self.output.clear();
                self.back_to_waiting();
                self.working = true;
                start_work(args);
            }
        } else if self.place.refused.is_none()
            && self.look.intro.is_some()
            && !place::intro_shown(&self.place.own)
        {
            let [welcome, logon, header] = self.look.intro.clone().expect("checked");
            let odd = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .is_ok_and(|d| d.subsec_nanos() & 1 == 1);
            let intro = Intro::new(&welcome, &logon, &header, odd);
            self.show(Mode::Intro(intro), Screen::default(), true);
        } else {
            self.show_start();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        match event {
            Event::Status(line) => {
                let s = Screen {
                    title: "-nv-rs-".into(),
                    welcome: vec![line],
                    ..Default::default()
                };
                self.show(Mode::Waiting, s, true);
            }
            Event::StatusDone => {}
            Event::GameShown => self.step_aside(event_loop),
            Event::Message {
                text,
                entries,
                reply,
                ..
            } => {
                if self.closed {
                    // Closed: answered as Esc answers it.
                    let _ = reply.send(back_entry(&entries).unwrap_or(0));
                    return;
                }
                let s = message_screen(&text, &entries);
                self.show(Mode::Message(Some(reply)), s, true);
            }
            Event::Folder { text, reply } => {
                if self.closed {
                    let _ = reply.send(None);
                    return;
                }
                let mut s = message_screen(&text, &["Back".to_string()]);
                s.welcome.push(
                    "Type or paste the folder below and press Enter, or drop it onto this window."
                        .into(),
                );
                self.show(Mode::Folder(Some(reply)), s, true);
            }
            Event::Output(text) => {
                if !matches!(self.mode, Mode::Output(_)) {
                    self.mode = Mode::Output(None);
                }
                self.add_output(&text);
            }
            Event::Ended(code) => {
                self.exit_code = code;
                self.working = false;
                // Closed while Play ran: the launcher ends with it.
                if code == 0 || self.closed {
                    event_loop.exit();
                } else if matches!(self.mode, Mode::Output(_)) {
                    // play.sh's own lines say what went wrong; Back or Exit below them.
                    self.mode = Mode::Output(Some(code));
                    self.add_output("");
                } else {
                    // Play.exe's message was shown and answered: back to the start.
                    self.show_start();
                }
            }
            Event::Pasted { id, text } => {
                if accept_paste(self.asked_paste, id, Instant::now(), self.mode.kind()) {
                    if let Some(text) = text {
                        self.screen.typed.push_str(&text);
                        self.request_redraw();
                    }
                }
                if self.asked_paste.is_some_and(|(asked, _, _)| asked == id) {
                    self.asked_paste = None;
                }
            }
            Event::Card(card) => {
                self.card = card;
                if matches!(self.mode, Mode::Start) {
                    let keep = (self.screen.highlighted, self.screen.typed.clone());
                    self.screen = start_screen(&self.place, &self.card);
                    (self.screen.highlighted, self.screen.typed) = keep;
                    self.request_redraw();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
        use winit::keyboard::{Key, NamedKey};
        match event {
            // With no Play running the launcher ends at once (with a refusal's stop code, else
            // the last one). While Play runs (the game may be starting, its one-game claim held)
            // only the window goes: a question it asks is answered as Esc answers it, later ones
            // too, play.sh's input is closed, and the launcher ends when Play ends, with its code.
            WindowEvent::CloseRequested => {
                if !self.working {
                    std::process::exit(session::stop_code().unwrap_or(self.exit_code));
                }
                self.close_while_working(event_loop);
            }
            WindowEvent::Resized(size) => {
                if let (Some(surface), Some(config), Some(g)) =
                    (&self.surface, &mut self.config, &self.gpu)
                {
                    config.width = size.width.max(1);
                    config.height = size.height.max(1);
                    surface.configure(&g.device, config);
                }
                self.frame.changed();
                self.request_redraw();
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            WindowEvent::ModifiersChanged(m) => self.modifiers = m.state(),
            WindowEvent::DroppedFile(path) => {
                match &mut self.mode {
                    Mode::Start => self.play(false, Some(path)),
                    Mode::Folder(_) => self.act(Act::Folder(Some(path)), event_loop),
                    Mode::Output(None) => self.screen.typed = path.to_string_lossy().into_owned(),
                    _ => {}
                }
                self.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if matches!(self.mode, Mode::Intro(_)) {
                    // Any key skips the intro.
                    self.end_intro();
                    return;
                }
                if !self.finished {
                    // Any key finishes the typing, as a click does in the game's terminal.
                    self.finished = true;
                    self.request_redraw();
                    return;
                }
                let n = self.screen.entries.len().max(1);
                match &event.logical_key {
                    Key::Named(NamedKey::ArrowUp) => {
                        self.screen.highlighted = (self.screen.highlighted + n - 1) % n
                    }
                    Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::Tab) => {
                        self.screen.highlighted = (self.screen.highlighted + 1) % n
                    }
                    Key::Named(NamedKey::PageUp) => {
                        self.screen.scroll = self.screen.scroll.saturating_sub(5)
                    }
                    Key::Named(NamedKey::PageDown) => self.screen.scroll += 5,
                    Key::Named(NamedKey::ArrowRight) => self.step_row(true),
                    Key::Named(NamedKey::ArrowLeft) => self.step_row(false),
                    Key::Named(NamedKey::Enter) => self.choose(event_loop),
                    Key::Named(NamedKey::Backspace) => {
                        self.screen.typed.pop();
                    }
                    Key::Named(NamedKey::Escape) => self.back(event_loop),
                    _ => {
                        // Ctrl+V (Cmd+V on macOS) pastes the clipboard's line; the key types
                        // nothing itself. Some systems give Ctrl+V as its control character.
                        let key = match &event.logical_key {
                            Key::Character(s) if s.as_str() == "\u{16}" => "v",
                            Key::Character(s) => s.as_str(),
                            _ => "",
                        };
                        let physical_v = event.physical_key
                            == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyV);
                        let kind = self.mode.kind();
                        let paste = crate::paste::is_paste(
                            key,
                            physical_v,
                            self.modifiers.control_key(),
                            self.modifiers.super_key(),
                        );
                        if takes_typing(kind) && paste {
                            // Read on a thread of its own; its answer comes as an event.
                            self.pastes += 1;
                            let id = self.pastes;
                            self.asked_paste = Some((id, Instant::now(), kind));
                            crate::paste::read_later(move |text| {
                                session::send(Event::Pasted { id, text });
                            });
                        } else if takes_typing(kind) {
                            if let Some(text) = &event.text {
                                self.screen
                                    .typed
                                    .extend(text.chars().filter(|c| !c.is_control()));
                            }
                        }
                    }
                }
                self.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse = (position.x, position.y);
                if let Some(i) = self.row_at(self.mouse) {
                    if self.finished && self.screen.highlighted != i {
                        self.screen.highlighted = i;
                        self.request_redraw();
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => (p.y / 30.0) as f32,
                };
                if lines > 0.0 {
                    self.screen.scroll = self.screen.scroll.saturating_sub(lines.ceil() as usize);
                } else if lines < 0.0 {
                    self.screen.scroll += (-lines).ceil() as usize;
                }
                self.request_redraw();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if matches!(self.mode, Mode::Intro(_)) {
                    self.end_intro();
                } else if !self.finished {
                    self.finished = true;
                } else if let Some(i) = self.row_at(self.mouse) {
                    self.screen.highlighted = i;
                    self.choose(event_loop);
                }
                self.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.hidden {
            // Wayland: minimised once the game has run a moment (WAYLAND_MINIMISE_AFTER).
            match self.minimise_at {
                Some(at) if Instant::now() >= at => {
                    if let Some(w) = &self.window {
                        w.set_minimized(true);
                    }
                    self.minimise_at = None;
                    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
                }
                Some(at) => {
                    event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(at))
                }
                None => event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait),
            }
            return;
        }
        let (wake, redraw) = self.frame.wake(
            self.can_draw(),
            !self.finished || matches!(self.mode, Mode::Intro(_)),
            self.started.elapsed().as_millis() as u64,
            self.frame_key(),
            Instant::now(),
        );
        event_loop.set_control_flow(match wake {
            Some(at) => winit::event_loop::ControlFlow::WaitUntil(at),
            None => winit::event_loop::ControlFlow::Wait,
        });
        if redraw {
            self.request_redraw();
        }
    }
}

/// Nothing can draw the launcher's window: said where the player can read it (a message box of
/// Windows'; else the terminal it was started from, and without a terminal the launcher's log in
/// nv-rs's own folder), with what happens next.
fn cannot_draw(why: &str, place: &Place) {
    cannot_draw_then(
        why,
        place,
        if cfg!(windows) {
            "Play goes on without it, in Windows' own message boxes. Updating the graphics \
             driver may bring the window back."
        } else {
            "It goes on in the terminal instead: play.sh, as ./play.sh starts it."
        },
    );
}

/// [`cannot_draw`] with `next` saying what happens next.
fn cannot_draw_then(why: &str, place: &Place, next: &str) {
    let text = format!("nv-rs's launcher cannot draw its window: {why}.\n\n{next}");
    eprintln!("{text}");
    #[cfg(windows)]
    {
        let _ = place;
        #[link(name = "user32")]
        extern "system" {
            fn MessageBoxW(
                owner: *mut std::ffi::c_void,
                text: *const u16,
                title: *const u16,
                kind: u32,
            ) -> i32;
        }
        let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
        let (t, title) = (wide(&text), wide("nv-rs"));
        // SAFETY: both strings live through the call; no owner window. 0x30: MB_ICONWARNING.
        unsafe { MessageBoxW(std::ptr::null_mut(), t.as_ptr(), title.as_ptr(), 0x30) };
    }
    #[cfg(not(windows))]
    {
        use std::io::IsTerminal;
        if !std::io::stderr().is_terminal() && place.refused.is_none() {
            let logs = place.own.join("logs");
            let written = std::fs::create_dir_all(&logs).and_then(|()| {
                use std::io::Write;
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(logs.join("launcher.log"))?;
                writeln!(f, "{text}")
            });
            if let Err(e) = written {
                eprintln!("{}: {e}", logs.display());
            }
        }
    }
}

/// The keyboard library winit needs on Linux and cannot load, if one is missing (as
/// engine-platform's window layer asks): libxkbcommon always, libxkbcommon-x11 on X11.
#[cfg(all(not(windows), not(target_vendor = "apple")))]
fn missing_keyboard_library() -> Option<&'static str> {
    if xkbcommon_dl::xkbcommon_option().is_none() {
        return Some("libxkbcommon.so.0");
    }
    let named = |name| std::env::var_os(name).is_some_and(|v| !v.is_empty());
    let wayland = named("WAYLAND_DISPLAY") || named("WAYLAND_SOCKET");
    if !wayland && xkbcommon_dl::x11::xkbcommon_x11_option().is_none() {
        return Some("libxkbcommon-x11.so.0");
    }
    None
}

/// No window: Play goes on without it. Windows: Play's steps with Windows' own message boxes and
/// folder picker, as Play.exe asked; Linux and macOS: play.sh in the terminal, with the same
/// arguments.
fn without_window(raw: &[String]) -> i32 {
    #[cfg(windows)]
    {
        crate::play::answer_in_boxes();
        crate::play::run_play(raw)
    }
    #[cfg(not(windows))]
    {
        crate::without_window(raw)
    }
}

/// The window, until the player leaves or the game ends well: the exit code.
pub fn run(raw: Vec<String>) -> i32 {
    let place = place::place();
    #[cfg(all(not(windows), not(target_vendor = "apple")))]
    if let Some(library) = missing_keyboard_library() {
        cannot_draw(
            &format!("the keyboard library {library} is missing (install libxkbcommon)"),
            &place,
        );
        return without_window(&raw);
    }
    let event_loop = match EventLoop::<Event>::with_user_event().build() {
        Ok(l) => l,
        Err(e) => {
            cannot_draw(&format!("no window system: {e}"), &place);
            return without_window(&raw);
        }
    };
    let look = look_of(place.game.as_deref(), &place.fallout_ini, false);
    let mut app = App {
        look,
        place,
        proxy: event_loop.create_proxy(),
        instance: None,
        window: None,
        surface: None,
        config: None,
        gpu: None,
        mode: Mode::Start,
        screen: Screen::default(),
        started: Instant::now(),
        finished: false,
        rows: None,
        mouse: (0.0, 0.0),
        card: Card::Asking,
        action: (!raw.is_empty()).then(|| raw.iter().map(OsString::from).collect()),
        output: Vec::new(),
        hidden: false,
        minimise_at: None,
        working: false,
        closed: false,
        frame: FrameState::default(),
        chosen_again: Vec::new(),
        no_window: false,
        exit_code: 0,
        modifiers: winit::keyboard::ModifiersState::empty(),
        pastes: 0,
        asked_paste: None,
        desktop: (0, 0),
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("error: {e}");
        return 1;
    }
    if app.no_window {
        return without_window(&raw);
    }
    app.exit_code
}

/// The start screen (or the intro) drawn off screen and saved: nothing else is written.
pub fn capture(c: &crate::Capture) -> Result<(), String> {
    let fallout_ini = c.profile.clone().unwrap_or_default();
    let place = Place {
        own: PathBuf::new(),
        game: c
            .game
            .clone()
            .filter(|g| crate::game::Game::at(g).is_some()),
        fallout_ini: fallout_ini.clone(),
        adapter_card: None,
        mods: false,
        refused: None,
    };
    let look = look_of(place.game.as_deref(), &fallout_ini, c.plain);
    let mut drawing = gpu::choose(None, (0, 0))?;
    let picture = (screen::WIDTH, screen::HEIGHT);
    let canvas = match (c.intro_at, &look.intro) {
        (Some(ms), Some([welcome, logon, header])) => {
            let intro = Intro::new(welcome, logon, header, false);
            screen::draw_intro(&look, &intro.typed(ms))
        }
        (Some(_), None) => return Err("the intro needs the game's look".into()),
        (None, _) => {
            let card =
                Card::Known(gpu::game_card().map(|(n, a)| (n, gpu::api_name(a).to_string())));
            let s = start_screen(&place, &card);
            screen::draw(&look, &s, usize::MAX, true).0
        }
    };
    let g = &mut drawing.gpu;
    g.upload(&canvas.pixels, picture);
    let rgba = g.capture(picture, c.size)?;
    std::fs::write(&c.file, crate::png::encode(c.size.0, c.size.1, &rgba))
        .map_err(|e| format!("{}: {e}", c.file.display()))?;
    println!(
        "captured {}x{} to {} with {}; look: {}",
        c.size.0,
        c.size.1,
        c.file.display(),
        g.describe(),
        look.from
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(game: Option<&str>, mods: bool) -> Place {
        Place {
            own: PathBuf::new(),
            game: game.map(PathBuf::from),
            fallout_ini: PathBuf::from("no such folder/Fallout.ini"),
            adapter_card: Some("Radeon X".into()),
            mods,
            refused: None,
        }
    }

    /// The start screen offers what works today: Play, Play with mods only when there are mods,
    /// Exit; no entry for anything that does not exist yet.
    #[test]
    fn the_start_screen_offers_what_works() {
        let s = start_screen(&place(Some("/g/Fallout New Vegas"), false), &Card::Asking);
        assert_eq!(s.entries, [PLAY, SETTINGS, EXIT]);
        assert_eq!(s.welcome[0], "Game: Fallout New Vegas");
        let s = start_screen(&place(Some("/g/x"), true), &Card::Asking);
        assert_eq!(s.entries, [PLAY, PLAY_WITH_MODS, SETTINGS, EXIT]);
        let s = start_screen(&place(None, false), &Card::Asking);
        assert!(s.welcome[0].contains("not found"), "{:?}", s.welcome);
        assert_eq!(s.welcome.len(), 1);
    }

    /// The renderer line names the game's renderer and card, never the launcher's own device: the
    /// card asked for once known, the adapter table's card on the real Direct3D 9; a question that
    /// did not answer never claims Direct3D 9 (the game still starts on Vulkan).
    #[test]
    fn the_renderer_line_names_the_games_card() {
        let p = place(Some("/g/x"), false);
        let known = Card::Known(Some(("NVIDIA RTX".into(), "Vulkan".into())));
        assert_eq!(
            start_screen(&p, &Card::Asking).welcome[1],
            "Renderer: Vulkan"
        );
        assert_eq!(
            start_screen(&p, &known).welcome[1],
            "Renderer: Vulkan, NVIDIA RTX"
        );
        let none = start_screen(&p, &Card::Known(None)).welcome[1].clone();
        assert!(none.starts_with("Renderer: "), "{none}");
        let silent = start_screen(&p, &Card::NoAnswer).welcome[1].clone();
        assert!(
            silent.starts_with("Renderer: Vulkan (its driver did not answer"),
            "{silent}"
        );
        // The real Direct3D 9, chosen in the profile: the adapter table's card.
        let dir = std::env::temp_dir().join(format!("nv-launcher-renderer-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ini = dir.join("Fallout.ini");
        std::fs::write(&ini, "[nv-rs Display]\r\nsRenderer=d3d9\r\n").unwrap();
        let p = Place {
            fallout_ini: ini,
            ..place(Some("/g/x"), false)
        };
        assert_eq!(
            start_screen(&p, &known).welcome[1],
            "Renderer: Direct3D 9, Radeon X"
        );
        let p = Place {
            adapter_card: None,
            ..p
        };
        assert_eq!(start_screen(&p, &known).welcome[1], "Renderer: Direct3D 9");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// `--game-card`'s answer has three outcomes: a card, no card (an empty line), no answer (a
    /// program that crashed, or a line that is not an answer).
    #[test]
    fn the_card_question_has_three_answers() {
        assert_eq!(
            card_from_line("NVIDIA GeForce RTX 5070 Ti Laptop GPU\tVulkan\n"),
            Some((
                "NVIDIA GeForce RTX 5070 Ti Laptop GPU".into(),
                "Vulkan".into()
            ))
        );
        assert_eq!(card_from_line("\tVulkan"), None);
        assert_eq!(
            card_answer(true, "Card\tDirect3D 12\n"),
            Card::Known(Some(("Card".into(), "Direct3D 12".into())))
        );
        assert_eq!(card_answer(true, "\n"), Card::Known(None));
        assert_eq!(card_answer(true, ""), Card::Known(None));
        assert_eq!(card_answer(false, ""), Card::NoAnswer);
        assert_eq!(card_answer(false, "Card\tVulkan\n"), Card::NoAnswer);
        assert_eq!(card_answer(true, "garbage"), Card::NoAnswer);
    }

    /// play.sh's output shows as a terminal shows it: a question without its line break waits on
    /// its line, the typed answer goes after it; CR LF ends a line; only the newest lines stay.
    #[test]
    fn output_is_kept_as_a_terminal_shows_it() {
        let mut lines = Vec::new();
        append_output(&mut lines, "Found your game\r\nType yes to copy them: ");
        assert_eq!(lines, ["Found your game", "Type yes to copy them: "]);
        append_output(&mut lines, "yes\n");
        append_output(&mut lines, "Copied 3 save(s)\n");
        assert_eq!(
            lines,
            [
                "Found your game",
                "Type yes to copy them: yes",
                "Copied 3 save(s)",
                ""
            ]
        );
        for i in 0..OUTPUT_LINES + 5 {
            append_output(&mut lines, &format!("{i}\n"));
        }
        assert_eq!(lines.len(), OUTPUT_LINES);
        assert_eq!(lines[OUTPUT_LINES - 2], format!("{}", OUTPUT_LINES + 4));
    }

    /// Esc goes back: Back or No where there is one, else the last entry; nothing without entries.
    #[test]
    fn escape_takes_back_or_no_else_the_last_entry() {
        let e = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(back_entry(&e(&["Yes", "No"])), Some(1));
        // No is taken wherever it stands (not only as the last entry).
        assert_eq!(back_entry(&e(&["No", "Yes"])), Some(0));
        assert_eq!(back_entry(&e(&["Back", "Exit"])), Some(0));
        assert_eq!(back_entry(&e(&["Open the logs folder", "Back"])), Some(1));
        assert_eq!(back_entry(&e(&["Play", "Exit"])), Some(1));
        assert_eq!(back_entry(&e(&["OK"])), Some(0));
        assert_eq!(back_entry(&[]), None);
    }

    /// Enter sends what is typed (a folder from the start screen or at the folder prompt); Esc
    /// never does: on the start screen it is Exit, at the folder prompt Back (no folder), on a
    /// question No, whatever is typed.
    #[test]
    fn enter_sends_the_typed_folder_and_escape_never_does() {
        let e = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let start = e(&[PLAY, EXIT]);
        assert_eq!(
            on_enter(Kind::Start, " \"/g/Fallout New Vegas\" ", 0, PLAY),
            Act::Play {
                mods: false,
                game: Some(PathBuf::from("/g/Fallout New Vegas"))
            }
        );
        assert_eq!(
            on_enter(Kind::Start, "", 0, PLAY),
            Act::Play {
                mods: false,
                game: None
            }
        );
        assert_eq!(
            on_enter(Kind::Start, "", 1, PLAY_WITH_MODS),
            Act::Play {
                mods: true,
                game: None
            }
        );
        assert_eq!(on_escape(Kind::Start, &start), Act::Exit);
        let folder = e(&["Back"]);
        assert_eq!(
            on_enter(Kind::Folder, "/g", 0, "Back"),
            Act::Folder(Some(PathBuf::from("/g")))
        );
        assert_eq!(on_enter(Kind::Folder, "", 0, "Back"), Act::Folder(None));
        assert_eq!(on_escape(Kind::Folder, &folder), Act::Folder(None));
        assert_eq!(on_escape(Kind::Message, &e(&["Yes", "No"])), Act::Answer(1));
        assert_eq!(on_escape(Kind::Refused, &e(&[EXIT])), Act::Exit);
        assert_eq!(
            on_escape(Kind::Ended, &e(&["Back", EXIT])),
            Act::BackToStart
        );
        assert_eq!(on_escape(Kind::Waiting, &[]), Act::Nothing);
        assert_eq!(on_escape(Kind::Output, &[]), Act::Nothing);
        assert_eq!(
            on_enter(Kind::Output, "yes", 0, ""),
            Act::TypeLine("yes".into())
        );
    }

    /// The first window keeps the terminal's 4:3 shape, at most 960 by 720, and fits a small or
    /// scaled screen with room to spare.
    #[test]
    fn the_first_window_fits_the_screen() {
        assert_eq!(first_size(None), (960.0, 720.0));
        assert_eq!(first_size(Some((2560.0, 1440.0))), (960.0, 720.0));
        // A 1920x1080 laptop at 150%: 1280 by 720 logical.
        let (w, h) = first_size(Some((1280.0, 720.0)));
        assert!(
            h <= 720.0 * HEIGHT_SHARE && (w / h - 4.0 / 3.0).abs() < 0.01,
            "{w}x{h}"
        );
        // Windows leaves room for the title bar and the taskbar.
        let expect = if cfg!(windows) {
            (768.0, 576.0)
        } else {
            (844.0, 633.0)
        };
        assert_eq!(first_size(Some((1280.0, 720.0))), expect);
        // A narrow screen: the width decides.
        let (w, h) = first_size(Some((800.0, 1280.0)));
        assert!(
            w <= 800.0 * 0.9 && (w / h - 4.0 / 3.0).abs() < 0.01,
            "{w}x{h}"
        );
        // Centred on its monitor, a second monitor's origin included; never left of it.
        assert_eq!(centred((0, 0), (1920, 1080), (960, 720)), (480, 180));
        assert_eq!(centred((1920, -200), (1280, 1024), (960, 720)), (2080, -48));
        assert_eq!(centred((0, 0), (800, 600), (960, 720)), (0, 0));
    }

    /// The whole window, frame included, inside the work area: a 1080p screen at 150% (work area
    /// 1032 high above a 48-pixel taskbar, a 59-pixel title bar) keeps its bottom above the
    /// taskbar, the outer rectangle centred in the work area; a window that fits keeps its size.
    #[test]
    fn the_whole_window_fits_the_work_area() {
        let ((w, h), (x, y)) = fit_in((0, 0, 1920, 1032), (16, 59), (1440, 1080));
        assert_eq!((w, h), (1297, 973));
        assert!(y >= 0 && y as u32 + h + 59 <= 1032, "{y} {h}");
        assert_eq!(x, (1920 - (w as i32 + 16)) / 2);
        assert_eq!(
            fit_in((0, 0, 1920, 1032), (16, 39), (960, 720)),
            ((960, 720), (472, 136))
        );
        // A work area that does not start at 0 (a taskbar at the top, a second monitor).
        assert_eq!(
            fit_in((1920, 40, 1280, 984), (0, 0), (960, 720)),
            ((960, 720), (2080, 172))
        );
        // Narrow: the width limits it, the shape kept.
        assert_eq!(
            fit_in((0, 0, 800, 1200), (0, 0), (960, 720)),
            ((800, 600), (0, 300))
        );
    }

    /// A frame's next step: a lost device or surface is chosen again (whatever the surface
    /// said), an outdated one set up again, a busy or hidden one skipped.
    #[test]
    fn a_lost_device_is_chosen_again_never_drawn_on() {
        assert_eq!(frame_step(false, Got::Texture), FrameStep::Draw);
        assert_eq!(frame_step(true, Got::Texture), FrameStep::ChooseAgain);
        assert_eq!(frame_step(false, Got::Lost), FrameStep::ChooseAgain);
        assert_eq!(frame_step(false, Got::Validation), FrameStep::ChooseAgain);
        assert_eq!(frame_step(false, Got::Outdated), FrameStep::Reconfigure);
        assert_eq!(frame_step(true, Got::Outdated), FrameStep::ChooseAgain);
        assert_eq!(frame_step(false, Got::Timeout), FrameStep::Skip);
        assert_eq!(frame_step(false, Got::Occluded), FrameStep::Skip);
    }

    /// No redraw loop: a window stepped aside, minimised or of no size is not drawn; a frame set
    /// up again or skipped is tried again after a short wait, not at once; a drawing chosen again
    /// twice within a second is not chosen a third time.
    #[test]
    fn a_surface_that_stays_outdated_is_not_asked_again_nonstop() {
        assert!(drawable(false, false, (960, 720)));
        assert!(!drawable(true, false, (960, 720)));
        assert!(!drawable(false, true, (960, 720)));
        assert!(!drawable(false, false, (0, 0)));
        assert!(!drawable(false, false, (960, 0)));
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        assert_eq!(retry_after(FrameStep::Draw, t0), None);
        assert_eq!(retry_after(FrameStep::ChooseAgain, t0), None);
        for step in [FrameStep::Reconfigure, FrameStep::Skip] {
            let at = retry_after(step, t0);
            assert_eq!(at, Some(t0 + RETRY_AFTER));
            // Not at once (the 6 ms typing wake does not ask again)...
            assert!(!wants_redraw(None, (12, true), at, ms(6)));
        }
        assert!(may_choose_again(&[], t0));
        assert!(may_choose_again(&[t0], ms(100)));
        assert!(!may_choose_again(&[t0, ms(100)], ms(200)));
        // A second later the budget is there again.
        assert!(may_choose_again(&[t0, ms(100)], ms(1150)));
    }

    /// The intro's last picture (its key never changes again: no blink, nothing more typed) whose
    /// frame came back out of date or skipped is still drawn once the short wait is over, so the
    /// intro ends; so is a new screen with the same key as the last one drawn. Nothing changed
    /// and drawn: no redraw.
    #[test]
    fn a_frame_not_drawn_is_drawn_after_its_wait_even_with_no_later_change() {
        let t0 = Instant::now();
        let last = (57, false);
        for step in [FrameStep::Reconfigure, FrameStep::Skip] {
            let at = retry_after(step, t0);
            assert!(wants_redraw(None, last, at, t0 + RETRY_AFTER), "{step:?}");
            assert!(
                wants_redraw(None, last, at, t0 + Duration::from_secs(5)),
                "{step:?}"
            );
        }
        assert!(wants_redraw(None, last, None, t0));
        assert!(!wants_redraw(Some(last), last, None, t0));
    }

    /// The redraw's bookkeeping as the window uses it, over several retry periods: a surface that
    /// keeps skipping is asked once each wait (never in between, never left), the frame that is
    /// drawn at last ends the retries, a new screen with the same key is drawn, a minimised
    /// window asks nothing whatever is pending.
    #[test]
    fn the_frame_state_retries_each_wait_until_a_frame_is_drawn() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let key = (57, false);
        let mut f = FrameState::default();
        let mut asked = Vec::new();
        // Every 6 ms (the intro's typing wake) for half a second; each asked redraw is skipped
        // until 350 ms, then drawn.
        for t in (0..500).step_by(6) {
            let now = ms(t);
            let (_, redraw) = f.wake(true, true, t, key, now);
            if redraw {
                asked.push(t);
                let step = if t < 350 {
                    FrameStep::Skip
                } else {
                    FrameStep::Draw
                };
                f.on_frame(step, key, now);
            }
        }
        // At once, then once a retry period (the first wake at or after each wait), then drawn.
        assert_eq!(asked, [0, 102, 204, 306, 408], "{asked:?}");
        assert_eq!(
            f,
            FrameState {
                drawn: Some(key),
                retry_at: None
            }
        );
        assert!(!f.wake(true, false, 0, key, ms(600)).1);
        // A new screen that happens to have the same key: drawn.
        f.changed();
        assert!(f.wake(true, false, 0, key, ms(600)).1);
        // Minimised: nothing asked, no wake.
        f.on_frame(FrameStep::Skip, key, ms(600));
        assert_eq!(f.wake(false, true, 0, key, ms(900)), (None, false));
    }

    /// The whole wake decision, as the event loop takes it: the intro's last frame skipped is
    /// asked again once the wait is over (the 6 ms typing wakes in between ask nothing); a
    /// finished screen's frame skipped wakes at the retry, not at the next blink, and is
    /// asked again then, even when its key equals the last one drawn; a drawn screen waits for the
    /// blink; a minimised (or stepped aside, or 0x0) window neither wakes nor draws.
    #[test]
    fn the_event_loop_wakes_for_a_retry_and_never_for_a_hidden_window() {
        let t0 = Instant::now();
        let ms = |n| Duration::from_millis(n);
        let key = (57, false);
        // The intro over, its last frame skipped.
        let (drawn, retry_at) = frame_outcome(FrameStep::Skip, key, t0);
        assert_eq!((drawn, retry_at), (None, Some(t0 + RETRY_AFTER)));
        let at = |now, typing, shown_ms| {
            next_wake(&Waiting {
                can_draw: true,
                typing,
                shown_ms,
                drawn,
                key,
                retry_at,
                now,
            })
        };
        assert_eq!(at(t0 + ms(6), true, 2006), (Some(t0 + ms(12)), false));
        // The wait is over: the redraw is asked now (it then clears the retry).
        assert_eq!(
            at(t0 + RETRY_AFTER, true, 2100),
            (Some(t0 + RETRY_AFTER), true)
        );
        // A finished screen (the cursor blinks every 400 ms; 100 ms into it): woken at the retry.
        assert_eq!(at(t0, false, 100), (Some(t0 + RETRY_AFTER), false));
        assert!(at(t0 + RETRY_AFTER, false, 200).1);
        // Drawn and unchanged: the blink, no redraw.
        let drawn_now = next_wake(&Waiting {
            can_draw: true,
            typing: false,
            shown_ms: 100,
            drawn: Some(key),
            key,
            retry_at: None,
            now: t0,
        });
        assert_eq!(drawn_now, (Some(t0 + ms(300)), false));
        // A frame drawn after the retry: its key, no retry left.
        assert_eq!(frame_outcome(FrameStep::Draw, key, t0), (Some(key), None));
        assert_eq!(frame_outcome(FrameStep::ChooseAgain, key, t0), (None, None));
        // Minimised: no wake, no redraw, whatever is pending.
        let hidden = next_wake(&Waiting {
            can_draw: false,
            typing: true,
            shown_ms: 0,
            drawn: None,
            key,
            retry_at: None,
            now: t0,
        });
        assert_eq!(hidden, (None, false));
    }

    /// Play's work always ends with a code: its own, or 101 when it panicked (so `Ended` is sent
    /// and a closed launcher ends).
    #[test]
    fn plays_work_always_ends_with_a_code() {
        assert_eq!(work_code(|| 3), 3);
        // (The panic's own message is printed: the hook is the whole test run's.)
        assert_eq!(work_code(|| panic!("made-up failure")), 101);
    }

    /// Play's arguments: `--mods` for Play with mods, `--game FOLDER` for a typed or dropped
    /// folder, nothing for plain Play (Play.exe's and play.sh's own options).
    #[test]
    fn play_arguments_name_mods_and_the_game() {
        assert!(play_arguments(false, None).is_empty());
        assert_eq!(
            play_arguments(true, Some(Path::new("/g/Fallout New Vegas"))),
            ["--mods", "--game", "/g/Fallout New Vegas"].map(OsString::from)
        );
        assert_eq!(
            play_arguments(false, Some(Path::new("x"))),
            ["--game", "x"].map(OsString::from)
        );
    }

    /// The Settings list shows each row with its value (its note left out), then Testing and
    /// Back; the Testing list its rows and Back. Enter on a row opens its choices, on a choice
    /// saves it; Esc goes back a list (never saving); Left and Right step round a row's choices.
    #[test]
    fn the_settings_lists_open_choose_and_go_back() {
        let shown = settings::Files::default();
        let main = settings_entries(&settings::ROWS, &shown, false);
        assert_eq!(main.len(), 14);
        assert_eq!(main[0], "Renderer: Vulkan");
        assert_eq!(main[12], TESTING_ENTRY);
        assert_eq!(main[13], BACK);
        let testing = settings_entries(&settings::TESTING, &shown, true);
        assert_eq!(testing.len(), settings::TESTING.len() + 1);
        assert_eq!(testing.last().map(String::as_str), Some(BACK));
        assert_eq!(on_enter(Kind::Start, "", 1, SETTINGS), Act::OpenSettings);
        assert_eq!(
            on_enter(Kind::Settings, "", 0, &main[0]),
            Act::OpenChoices(0)
        );
        assert_eq!(
            on_enter(Kind::Settings, "", 12, TESTING_ENTRY),
            Act::OpenTesting
        );
        assert_eq!(on_escape(Kind::Settings, &main), Act::BackToStart);
        assert_eq!(on_escape(Kind::Testing, &testing), Act::OpenSettings);
        assert_eq!(
            on_enter(Kind::Testing, "", 2, &testing[2]),
            Act::OpenChoices(2)
        );
        let choices: Vec<String> = ["Off", "On", BACK].map(String::from).to_vec();
        assert_eq!(on_enter(Kind::Choices, "", 1, "On"), Act::Save("On".into()));
        assert_eq!(on_escape(Kind::Choices, &choices), Act::BackToList);
        let c: Vec<String> = ["No limit", "60", "90"].map(String::from).to_vec();
        assert_eq!(stepped_choice(&c, "60", true).as_deref(), Some("90"));
        assert_eq!(stepped_choice(&c, "90", true).as_deref(), Some("No limit"));
        assert_eq!(stepped_choice(&c, "No limit", false).as_deref(), Some("90"));
        assert_eq!(stepped_choice(&c, "75", true).as_deref(), Some("No limit"));
        // "Game's setting" is passed over, whichever way.
        let o: Vec<String> = [settings::OWN, "Off", "2x", "4x"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            stepped_choice(&o, settings::OWN, true).as_deref(),
            Some("Off")
        );
        assert_eq!(
            stepped_choice(&o, settings::OWN, false).as_deref(),
            Some("4x")
        );
        assert_eq!(stepped_choice(&o, "4x", true).as_deref(), Some("Off"));
        assert_eq!(stepped_choice(&o, "Off", false).as_deref(), Some("4x"));
        assert_eq!(
            stepped_choice(&[settings::OWN.to_string()], settings::OWN, true),
            None
        );
        assert_eq!(short("Vulkan (recommended)"), "Vulkan");
    }

    /// Typing, and so a paste, reaches the prompt on the start screen, Play's folder question and
    /// play.sh's output only: every other kind of screen is listed and refuses it.
    #[test]
    fn typing_goes_to_the_prompt_screens_only() {
        let all = [
            Kind::Intro,
            Kind::Start,
            Kind::Refused,
            Kind::Waiting,
            Kind::Message,
            Kind::Folder,
            Kind::Output,
            Kind::Ended,
            Kind::Settings,
            Kind::Testing,
            Kind::Choices,
        ];
        let takes: Vec<Kind> = all.into_iter().filter(|k| takes_typing(*k)).collect();
        assert_eq!(takes, [Kind::Start, Kind::Folder, Kind::Output]);
    }

    /// A paste's answer is added only for the paste asked last, within half a second, on the
    /// same kind of screen; a late one, an older one, or one after the screen changed is dropped.
    #[test]
    fn a_late_or_stale_paste_is_dropped() {
        let t0 = Instant::now();
        let asked = Some((3, t0, Kind::Folder));
        let at = |ms| t0 + Duration::from_millis(ms);
        assert!(accept_paste(asked, 3, at(20), Kind::Folder));
        assert!(accept_paste(asked, 3, at(500), Kind::Folder));
        assert!(!accept_paste(asked, 3, at(501), Kind::Folder));
        assert!(!accept_paste(asked, 2, at(20), Kind::Folder));
        assert!(!accept_paste(asked, 3, at(20), Kind::Start));
        assert!(!accept_paste(None, 3, at(20), Kind::Folder));
    }

    /// A message keeps its paragraphs as lines, its entries as given.
    #[test]
    fn a_message_keeps_its_paragraphs() {
        let s = message_screen("a\n\nb", &["Yes".into(), "No".into()]);
        assert_eq!(s.welcome, ["a", "", "b"]);
        assert_eq!(s.entries, ["Yes", "No"]);
    }
}
