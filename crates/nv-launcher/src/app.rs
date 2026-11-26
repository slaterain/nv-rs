//! The launcher's window: the terminal's screens, typed in a character at a time as the game types
//! them, the highlight moved with the arrow keys or the mouse, Enter or a click to choose; what is
//! typed goes to the prompt. Play runs on a thread of its own and asks the window through
//! [`crate::session`]; the window steps aside while the game plays and comes back when it ends
//! badly (a good end closes the launcher). The picture is drawn again only when something on it
//! changes: a typed character, the cursor's blink, an input.

use crate::crash::consent::{self, Consent};
use crate::gpu::{self, Gpu};
use crate::intro::Intro;
use crate::pacing;
use crate::place::{self, Place};
use crate::screen::{self, Look, Screen};
use crate::session::{self, Event};
use crate::settings;
use crate::sound;
use crate::sound_files::Cue;
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

/// The start screen's entries: Play, Play with mods (when there are mods), Mods, Settings, Exit.
pub const PLAY: &str = "Play";
pub const PLAY_WITH_MODS: &str = "Play with mods";
/// The mod installer (`crates/mod-chooser`'s text chooser) in the terminal.
pub const MODS: &str = "Mods";
/// Play.exe's and play.sh's argument for the mod installer.
pub const MOD_INSTALLER: &str = "--install-mods";
/// The saves' import, on the start screen only when saves to import were found ([`Place::saves`]),
/// and Play.exe's and play.sh's argument for it.
pub const IMPORT_SAVES: &str = "Import saves";
pub const SAVES_IMPORT: &str = "--import-saves";

/// What the window does when the work ends with `code`: the mod installer's good end (`mods`)
/// goes back to the start screen, Play's ends the launcher; a window closed meanwhile ends it;
/// a bad end shows the work's own lines with Back and Exit when it wrote any (`lines`), else the
/// start screen (its message was shown and answered).
#[derive(Debug, PartialEq)]
pub enum EndStep {
    Exit,
    Start,
    Lines,
}

pub fn end_step(mods: bool, code: i32, closed: bool, lines: bool) -> EndStep {
    if closed || (code == 0 && !mods) {
        EndStep::Exit
    } else if code == 0 || !lines {
        EndStep::Start
    } else {
        EndStep::Lines
    }
}
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
    entries.push(MODS.to_string());
    if place.saves {
        entries.push(IMPORT_SAVES.to_string());
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

/// The screen of a program's `output` (the mod installer's, play.sh's): its lines as the welcome
/// text, which shows its end (new output brings it back there; Page Up and the wheel show the
/// earlier lines, the installer's list above its question), what the player has `typed` at the
/// prompt, and once the program has `ended` Back and Exit.
pub fn output_screen(output: &[String], ended: bool, typed: String) -> Screen {
    Screen {
        title: "-nv-rs-".into(),
        welcome: output.to_vec(),
        follow_end: true,
        typed,
        entries: if ended {
            vec!["Back".into(), EXIT.into()]
        } else {
            Vec::new()
        },
        ..Default::default()
    }
}

/// Esc's entry: the screen's Back, No or Don't send, else its last (Exit, OK); none without
/// entries.
pub fn back_entry(entries: &[String]) -> Option<usize> {
    let last = entries.len().checked_sub(1)?;
    Some(
        entries
            .iter()
            .position(|x| x == "Back" || x == "No" || x == "Don't send")
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

/// The smallest the window may be made, in the monitor's logical units: half the terminal's
/// size, but never more than the first size (`first`, [`first_size`]), so a monitor that says it
/// is very small in logical units (a scale factor larger than its height allows, as a monitor
/// reporting a wrong size in millimetres can give on X11) never gets a window taller or wider
/// than itself, its bottom rows off the screen.
pub fn least_size(first: (f64, f64)) -> (f64, f64) {
    (
        (f64::from(screen::WIDTH) / 2.0).min(first.0),
        (f64::from(screen::HEIGHT) / 2.0).min(first.1),
    )
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

/// The launcher window's frame: none (no title bar or border, the maintainer's wish), titled
/// "nv-rs" for the task bar.
pub fn frameless(a: winit::window::WindowAttributes) -> winit::window::WindowAttributes {
    a.with_title("nv-rs").with_decorations(false)
}

/// The terminal's screen's own size, in its units.
pub const SCREEN: (u32, u32) = (screen::WIDTH, screen::HEIGHT);

/// The entry under the window position `pos` (pixels), the screen standing in the window as
/// `framing` puts it ([`gpu::framing`]) and its rows laid out as `rows`: the index of a shown
/// entry, none above, below, left or right of them.
pub fn row_hit(framing: &gpu::Framing, rows: &screen::Rows, pos: (f64, f64)) -> Option<usize> {
    let (px, py) = framing.in_screen(pos)?;
    if rows.row == 0
        || px < rows.x as f64
        || px > (rows.x + rows.width as i32) as f64
        || py < rows.top as f64
    {
        return None;
    }
    let k = ((py - rows.top as f64) / rows.row as f64) as usize;
    (k < rows.shown).then_some(rows.first + k)
}

/// The window's own button ([`screen::buttons`], in the picture's top-right corner, so the
/// window's) under the window position `pos` (pixels), the picture standing in the window as
/// `framing` puts it, as for [`row_hit`].
pub fn button_hit(framing: &gpu::Framing, look: &Look, pos: (f64, f64)) -> Option<screen::Button> {
    let p = framing.in_picture(pos)?;
    screen::button_at(look, framing.picture.0, p)
}

/// A window the minimise button minimises: winit's (on every system), or a test's own.
pub trait Minimise {
    fn minimise(&self);
}

impl Minimise for winit::window::Window {
    fn minimise(&self) {
        self.set_minimized(true);
    }
}

/// A press of the window's `button`: Minimise minimises `window` and does nothing more; Exit
/// does what the menu's Exit does ([`on_enter`]'s), or while Play's work runs (`working`: no
/// screen offers Exit then) what closing the window does then ([`Act::Close`]).
pub fn press_button(button: screen::Button, working: bool, window: Option<&impl Minimise>) -> Act {
    match button {
        screen::Button::Minimise => {
            if let Some(w) = window {
                w.minimise();
            }
            Act::Nothing
        }
        screen::Button::Exit if working => Act::Close,
        screen::Button::Exit => Act::Exit,
    }
}

/// Whether a left press moves the window (it has no title bar): anywhere but on an entry
/// (which the press chooses), and not while the window has stepped aside for the game.
pub fn press_drags(on_entry: bool, hidden: bool) -> bool {
    !on_entry && !hidden
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
    /// The Settings list (its rows, the lists it opens, Back).
    Settings,
    /// A list Settings opens ([`SUBLISTS`]: its rows, Back).
    Sublist,
    /// One row's choices (and Back).
    Choices,
    /// What a crash report holds, the first of the crash reports screens (at the first start,
    /// until answered, and from Settings).
    Consent,
    /// The crash reports question: the three answers.
    Question,
    /// A crash report made from this PC as an example, from the first screen.
    Sample,
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
    /// The mod installer, its lines in the terminal.
    Mods,
    /// The saves' import, its questions in the terminal.
    ImportSaves,
    Answer(usize),
    Folder(Option<PathBuf>),
    TypeLine(String),
    BackToStart,
    OpenSettings,
    /// A list Settings opens.
    OpenList(settings::List),
    /// The row at this place of the list on screen.
    OpenChoices(usize),
    /// The choice of the row whose choices show.
    Save(String),
    /// From a row's choices back to its list.
    BackToList,
    /// The Settings list's "Launcher sounds" row: on to off, or off to on.
    ToggleSounds,
    /// Crash reports answered at the question.
    Consent(Consent),
    /// The example report, from the first crash reports screen.
    ShowSample,
    /// The first crash reports screen (back from the question or the example).
    OpenConsent,
    /// The question, from the first screen.
    OpenQuestion,
    /// The first screen opened from Settings' crash reports row (Enter or a click: one press
    /// never answers).
    OpenConsentFromSettings,
    /// The crash reports screens left unanswered: to where they were opened from.
    CloseConsent,
    /// The exit button while Play's work runs: only the window goes, as when it is closed then
    /// ([`App::close_while_working`]).
    Close,
    Nothing,
}

/// Whether the prompt is still the one a paste was asked at after `act`: only when nothing was
/// done. Anything else took it (Enter typed its line, Esc or a choice left it), so a paste still
/// being read is dropped when it comes.
pub fn prompt_kept(act: &Act) -> bool {
    *act == Act::Nothing
}

/// The sounds of a choice: an entry chosen (Enter or a click, nothing typed at the prompt) plays
/// its tile's click sound and the terminal's "enter" (`Tile::PlayTileSound` 00a0b110 with the
/// list template's `clicksound`, and `DoClick` 00757f70); a typed line, the launcher's own use of
/// the prompt, plays none.
pub fn choice_sounds(typed: &str, entry: Option<&str>) -> &'static [crate::sound_files::Cue] {
    use crate::sound_files::Cue;
    if typed.trim().is_empty() && entry.is_some() {
        &[Cue::Ok, Cue::Enter]
    } else {
        &[]
    }
}

/// The Settings list's row for the terminal's sounds (the launcher's own setting).
pub const SOUNDS_ROW: &str = "Launcher sounds";

/// The Settings list's row for crash reports (`crate::crash::consent`), after the sounds row.
pub const CRASH_ROW: &str = "Crash reports";
/// Its place in the Settings list.
pub const CRASH_ROW_INDEX: usize = settings::ROWS.len() + 1;

/// The crash reports screens' entries. The first screen says what a report holds (Choose, the
/// example, and Not now from the start screen or Back from Settings); the second asks (the
/// three answers, Back). Two screens, so that in the launcher's own plain look every line and
/// every choice shows at once.
pub const CHOOSE_REPORTS: &str = "Choose what happens to them";
pub const SHOW_REPORT: &str = "Show me what a report contains";
pub const NOT_NOW: &str = "Not now";
pub const SEND_REPORTS: &str = "Send them automatically";
pub const ASK_REPORTS: &str = "Ask me each time";
pub const NEVER_REPORTS: &str = "Never send them";

/// The first crash reports screen's entries (`from_settings`: Back instead of Not now).
pub fn about_entries(from_settings: bool) -> Vec<String> {
    let last = if from_settings { BACK } else { NOT_NOW };
    [CHOOSE_REPORTS, SHOW_REPORT, last]
        .map(String::from)
        .to_vec()
}

/// The question's entries.
pub fn question_entries() -> Vec<String> {
    [SEND_REPORTS, ASK_REPORTS, NEVER_REPORTS, BACK]
        .map(String::from)
        .to_vec()
}

/// The first crash reports screen's text: what a report holds and what is taken out of it; at the
/// first start (`in_force`: none) that nothing is sent before an answer and what Not now does,
/// from Settings the answer in force.
pub fn about_text(in_force: Option<Consent>) -> Vec<String> {
    let mut text: Vec<String> = vec![
        "When the game stops or crashes, nv-rs writes a report: your system and card, settings, mods, where the game stopped, its log's end, and an F12 report's note and place from that run.".into(),
        "Your user name and folders are taken out; no saves or pictures.".into(),
    ];
    text.push(match in_force {
        None => "Nothing is sent until you choose. Not now asks at the next start.".into(),
        Some(answer) => format!("Now: {}.", answer.label()),
    });
    text
}

/// The question: where reports go, that a report sent includes where the player was in the game
/// and an F12 report's note, that nothing is sent before an answer, and, where this copy of
/// nv-rs cannot send (`can_send`), that it cannot.
pub fn question_text(can_send: bool) -> Vec<String> {
    let mut text = vec![
        "Send crash reports to Sentry (sentry.io), the crash-report service nv-rs's developers use? The report includes where you were and your F12 note. Settings can change this.".into(),
        "Nothing is sent until you choose.".into(),
    ];
    if !can_send {
        text.push("This copy cannot send yet.".into());
    }
    text
}

/// The question's screen, opened on "Never send them": quick presses of Enter through the
/// screens never choose sending.
pub fn question_screen(can_send: bool) -> Screen {
    let entries = question_entries();
    let highlighted = entries.iter().position(|e| e == NEVER_REPORTS).unwrap_or(0);
    Screen {
        title: format!("-{CRASH_ROW}-"),
        welcome: question_text(can_send),
        entries,
        highlighted,
        ..Default::default()
    }
}

/// Where the crash reports screens stand in the window: shown at this start yet, and opened from
/// Settings (Back returns there, and an answer there is written at once).
#[derive(Debug, Default)]
pub struct ConsentFlow {
    asked: bool,
    from_settings: bool,
}

impl ConsentFlow {
    /// What the start screen shows ([`start_kind`]); the screens count as shown at this start
    /// when they open there.
    pub fn at_start(&mut self, refused: bool, answer: Consent) -> Kind {
        let kind = start_kind(refused, answer, self.asked);
        if kind == Kind::Consent {
            self.asked = true;
            self.from_settings = false;
        }
        kind
    }

    /// The screens opened from Settings' row.
    pub fn open_in_settings(&mut self) {
        self.from_settings = true;
    }

    /// Whether the screens were opened from Settings.
    pub fn in_settings(&self) -> bool {
        self.from_settings
    }

    /// Whether an answer given now is written at once ([`keeps_at_once`]).
    pub fn answer_now(&self) -> bool {
        keeps_at_once(self.from_settings)
    }

    /// The screens left: whether to Settings (else the start screen).
    pub fn close(&mut self) -> bool {
        std::mem::take(&mut self.from_settings)
    }
}

/// The launcher's own folder (the package's: `BUILD.txt` and `crash-reports.txt` are beside it).
fn program_folder() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_default()
}

/// Whether the crash reports screens open at the start screen: not answered yet, and not asked
/// already since the launcher started ("Not now" asks again at the next start).
pub fn asks_consent(answer: Consent, asked: bool) -> bool {
    answer == Consent::Unanswered && !asked
}

/// What the start screen shows: the refusal where nv-rs may not write in its own folder, else
/// the crash reports screens while unanswered and not yet asked in this start, else itself.
pub fn start_kind(refused: bool, answer: Consent, asked: bool) -> Kind {
    if refused {
        Kind::Refused
    } else if asks_consent(answer, asked) {
        Kind::Consent
    } else {
        Kind::Start
    }
}

/// What Left or Right does on a Settings list's row at `index` (none: the row's own choices):
/// the sounds row flips; the crash reports row opens its screens as Enter does, so no press
/// there ever answers (sending is never switched on without the question).
pub fn row_step_act(list: settings::List, index: usize) -> Option<Act> {
    if list != settings::List::Main {
        None
    } else if index == settings::ROWS.len() {
        Some(Act::ToggleSounds)
    } else if index == CRASH_ROW_INDEX {
        Some(Act::OpenConsentFromSettings)
    } else {
        None
    }
}

/// Whether an answer is written into the launcher's own file at once: on Windows the first
/// start's answer waits for Play's first-write checks (the package gets nothing before, as the
/// intro's note waits); an answer from Settings is written at once, as Settings' other rows are.
pub fn keeps_at_once(from_settings: bool) -> bool {
    from_settings || cfg!(not(windows))
}

/// `answer` for crash reports: this launcher's own from now on, so a crash in this run follows
/// it too (never sending more than the saved answer: [`consent::in_force`]), and kept in nv-rs's
/// own folder `own` (none where nv-rs may not write there, `refused`): at once (`now`), or, for
/// the first start's answer on Windows, once Play's first-write checks pass ([`keeps_at_once`];
/// a first answer never kept so is asked again at the next start). What went wrong, if it could
/// not be kept.
pub fn keep_consent(own: &Path, refused: bool, answer: Consent, now: bool) -> Option<String> {
    consent::set_session(own, answer);
    if refused {
        return Some("Not saved: nv-rs may not write in its own folder".into());
    }
    consent::keep(own, answer, now).err().map(|e| {
        format!("Your crash reports answer was not saved ({e}): until nv-rs closes, a report goes only if both it and the saved answer let it.")
    })
}

/// The lists the Settings list opens, in its order after the crash reports row: the table's keys
/// outside the fixes, and every fix row ([`settings::List`]).
pub const SUBLISTS: [settings::List; 2] = [settings::List::More, settings::List::Fixes];

/// The Settings list's entry that opens `list` (a [`SUBLISTS`] one), and its place there.
pub fn sublist_entry(list: settings::List) -> usize {
    CRASH_ROW_INDEX + 1 + SUBLISTS.iter().position(|l| *l == list).unwrap_or(0)
}

/// The start screen's entry for Settings.
pub const SETTINGS: &str = "Settings";
pub const BACK: &str = "Back";

/// Whether typing (and so a paste) goes to the prompt on a screen of `kind`: the start screen's
/// folder, Play's folder question and play.sh's output only; never a menu, a message or the
/// intro.
pub fn takes_typing(kind: Kind) -> bool {
    matches!(kind, Kind::Start | Kind::Folder | Kind::Output)
}

/// Which of a screen's lines show at once instead of typing, as the game's terminal does it:
/// the two header lines once the first screen has typed them (`FinishIntro` types them once);
/// the title when it is the one already shown; the welcome text when it is the same text as the
/// last screen's (the terminal display function 007586e0 adds it only when it differs). The entries always type again.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AtOnce {
    pub header: bool,
    pub title: bool,
    pub welcome: bool,
}

/// [`AtOnce`] for a screen with `title` and `welcome` after the screen `last` (its title and
/// welcome text; None: no screen yet, or the intro).
pub fn shown_at_once(
    last: Option<&(String, Vec<String>)>,
    title: &str,
    welcome: &[String],
) -> AtOnce {
    match last {
        None => AtOnce::default(),
        Some((t, w)) => AtOnce {
            header: true,
            title: t == title,
            welcome: w.as_slice() == welcome,
        },
    }
}

/// Which of a screen's lines (`texts`, the entries from `first_entry`) show whole at once
/// ([`AtOnce`]).
fn whole_lines(texts: &[String], first_entry: usize, at_once: AtOnce) -> Vec<bool> {
    // The prompt, the last line, types once with the header (`FinishIntro`).
    let prompt = texts.len().saturating_sub(1).max(3);
    (0..texts.len())
        .map(|i| match i {
            0 | 1 => at_once.header,
            2 => at_once.title,
            i if i == prompt => at_once.header,
            i if i < first_entry => at_once.welcome,
            _ => false,
        })
        .collect()
}

/// The text manager's queue for a screen: its lines that type ([`whole_lines`] the others), in
/// order, 6 ms a character.
pub fn screen_queue(texts: &[String], first_entry: usize, at_once: AtOnce) -> Vec<pacing::Line> {
    let whole = whole_lines(texts, first_entry, at_once);
    texts
        .iter()
        .zip(&whole)
        .filter(|(_, &w)| !w)
        .map(|(t, _)| pacing::Line {
            chars: t.chars().count(),
            ms: screen::MS_PER_CHAR,
            intro: false,
            keyed: false,
        })
        .collect()
}

/// The characters typed of each of a screen's lines (`texts`, the entries from `first_entry`)
/// `t` milliseconds after it showed, frames `frame` milliseconds apart: the lines [`AtOnce`]
/// whole, the rest in order on the game's clock ([`pacing::typed`], 6 ms a character).
pub fn typed_lines(
    texts: &[String],
    first_entry: usize,
    at_once: AtOnce,
    frame: f64,
    t: f64,
) -> Vec<usize> {
    let whole = whole_lines(texts, first_entry, at_once);
    let queue = screen_queue(texts, first_entry, at_once);
    let (mut typed, _) = pacing::typed(&queue, frame, t);
    typed.reverse();
    texts
        .iter()
        .zip(&whole)
        .map(|(text, &w)| {
            if w {
                text.chars().count()
            } else {
                typed.pop().unwrap_or(0)
            }
        })
        .collect()
}

/// The focus moved up or down among the entries whose lines have started (`started` of them,
/// from the first): it wraps among them; with none started it stays.
pub fn step_focus(highlighted: usize, started: usize, up: bool) -> usize {
    if started == 0 {
        return highlighted;
    }
    let at = highlighted.min(started - 1);
    if up {
        (at + started - 1) % started
    } else {
        (at + 1) % started
    }
}

/// Whether Enter does anything: with something typed at the prompt it is always that line's;
/// otherwise, on a screen with entries, only once the highlighted entry's line has started (the
/// game's focus can only be on a tile that shows).
pub fn may_choose(typed: &str, has_entries: bool, highlighted_started: bool) -> bool {
    !typed.trim().is_empty() || !has_entries || highlighted_started
}

/// The player's "Frame rate limit in menus" in frames a second (0: no limit), which the game's
/// terminal types on ([`pacing`]), as the Settings screen shows it (a change saved for Play to
/// move into the game's files counts already); the play profile's own file when that cannot be
/// read; the row's default when neither has it.
pub fn menu_frame_limit(own: &Path, fallout_ini: &Path) -> u32 {
    let files = crate::profiles::Profiles::at(own)
        .shown()
        .unwrap_or_else(|_| profile_files(fallout_ini));
    frame_limit_of(&settings::Row::MenuFrameLimit.current(&files))
}

/// The most frames a second the typing counts: a larger limit set by hand counts as this (the
/// model steps through every frame of a screen, so a limit of billions would hold the window).
pub const MOST_FRAMES: u32 = 1000;

/// The menu frame limit row's value as frames a second: "No limit" (or 0) 0, else its number, at
/// most [`MOST_FRAMES`] (120 when it reads as nothing else, the row's default).
pub fn frame_limit_of(value: &str) -> u32 {
    if value == "No limit" {
        0
    } else {
        value.trim().parse::<u32>().unwrap_or(120).min(MOST_FRAMES)
    }
}

/// A key (or a click) as the window's handler tells them apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Press {
    Up,
    Down,
    PageUp,
    PageDown,
    Left,
    Right,
    Enter,
    Escape,
    Backspace,
    /// The left mouse button on the window.
    Click,
    /// Shift, Ctrl, Alt or the system key alone.
    Modifier,
    /// Any other key: a character for the prompt, the paste key.
    Other,
}

/// The [`Press`] of a key as winit names it.
pub fn press_of(key: &winit::keyboard::Key) -> Press {
    use winit::keyboard::{Key, NamedKey as N};
    match key {
        Key::Named(N::ArrowUp) => Press::Up,
        Key::Named(N::ArrowDown) | Key::Named(N::Tab) => Press::Down,
        Key::Named(N::PageUp) => Press::PageUp,
        Key::Named(N::PageDown) => Press::PageDown,
        Key::Named(N::ArrowLeft) => Press::Left,
        Key::Named(N::ArrowRight) => Press::Right,
        Key::Named(N::Enter) => Press::Enter,
        Key::Named(N::Escape) => Press::Escape,
        Key::Named(N::Backspace) => Press::Backspace,
        Key::Named(N::Shift | N::Control | N::Alt | N::AltGraph | N::Super | N::Meta) => {
            Press::Modifier
        }
        _ => Press::Other,
    }
}

/// What a press does on a screen that is not the intro.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyAct {
    /// The screen's text shows whole at once (the launcher's shortcut, [`press_finishes_typing`]).
    FinishTyping,
    Act(Press),
    Nothing,
}

/// The launcher's own shortcut, not the game's (its terminal has no key for it): while a screen
/// types, the first press of a key (not a modifier alone, not a held key's repeat) or a click
/// shows the whole text at once and does nothing else. Without it a player would wait about 3 s
/// on Settings each time. Removing it is this one rule: [`key_act`] then gives [`gated`]'s answer.
pub fn press_finishes_typing(press: Press, repeat: bool, typing: bool) -> bool {
    typing && !repeat && press != Press::Modifier
}

/// The game's rule for a press (`repeat`: a held key's auto-repeat) while the screen types or
/// not: the entries are disabled until every line is in, so moving the focus, choosing, Esc's way
/// back and Left/Right on a row wait; a held key's repeat never chooses or goes back (Esc held to
/// skip the intro would else go on to Exit), while a held Left or Right steps a row on, once the
/// screen is whole; typing at the prompt always goes.
pub fn gated(press: Press, repeat: bool, typing: bool) -> KeyAct {
    use Press::*;
    let acts = matches!(press, Enter | Escape | Click);
    let needs_live = acts || matches!(press, Up | Down | Left | Right);
    if (repeat && acts) || (typing && needs_live) || press == Modifier {
        KeyAct::Nothing
    } else {
        KeyAct::Act(press)
    }
}

/// The key handler's decision (the window's only one): nothing for the repeats of the key whose
/// press finished the typing, until it is let go (`finisher`: this press is such a repeat), so a
/// held key that finished the typing does not go on to act; else [`press_finishes_typing`], else
/// [`gated`].
pub fn key_act(press: Press, repeat: bool, typing: bool, finisher: bool) -> KeyAct {
    if repeat && finisher {
        KeyAct::Nothing
    } else if press_finishes_typing(press, repeat, typing) {
        KeyAct::FinishTyping
    } else {
        gated(press, repeat, typing)
    }
}

/// How long a paste's answer may take: one that comes later is dropped (the player has moved on).
pub const PASTE_WAIT: Duration = Duration::from_millis(500);

/// The paste being read: its id, when it was asked, on what kind of screen, and what was typed
/// at the prompt then.
#[derive(Clone, Debug, PartialEq)]
pub struct Asked {
    pub id: u64,
    pub at: Instant,
    pub kind: Kind,
    pub typed: String,
}

/// The pastes asked so far and the one being read ([`Asked`]).
#[derive(Default)]
pub struct Pastes {
    count: u64,
    asked: Option<Asked>,
}

impl Pastes {
    /// A paste asked `now` on a screen of `kind` with `typed` at the prompt: its id.
    pub fn ask(&mut self, now: Instant, kind: Kind, typed: &str) -> u64 {
        self.count += 1;
        self.asked = Some(Asked {
            id: self.count,
            at: now,
            kind,
            typed: typed.to_string(),
        });
        self.count
    }

    /// The prompt the paste was asked for is gone (Enter took it, Esc left it, another screen
    /// shows): its answer, when it comes, is dropped.
    pub fn forget(&mut self) {
        self.asked = None;
    }

    /// The clipboard's answer for paste `id`, arriving `now` on a screen of `kind` with `typed`
    /// at the prompt: whether it is added there ([`accept_paste`]). The paste being read is done
    /// with once its own answer has come, added or not.
    pub fn answer(&mut self, id: u64, now: Instant, kind: Kind, typed: &str) -> bool {
        let accept = accept_paste(self.asked.as_ref(), id, now, kind, typed);
        if self.asked.as_ref().is_some_and(|a| a.id == id) {
            self.asked = None;
        }
        accept
    }
}

/// Whether the clipboard's answer for paste `id`, arriving `now` on a screen of `kind` with
/// `typed` at the prompt, is added there: it is the paste asked last (`asked`) and still being
/// read, within [`PASTE_WAIT`], the screen is still of the kind it was asked on, and nothing was
/// typed or taken at the prompt since (the player has gone on without it).
pub fn accept_paste(asked: Option<&Asked>, id: u64, now: Instant, kind: Kind, typed: &str) -> bool {
    asked.is_some_and(|a| {
        a.id == id
            && now.saturating_duration_since(a.at) <= PASTE_WAIT
            && a.kind == kind
            && a.typed == typed
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
        Kind::Start if entry == MODS => Act::Mods,
        Kind::Start if entry == IMPORT_SAVES => Act::ImportSaves,
        Kind::Start if entry == SETTINGS => Act::OpenSettings,
        Kind::Start | Kind::Refused | Kind::Ended if entry == EXIT => Act::Exit,
        Kind::Ended => Act::BackToStart,
        Kind::Start | Kind::Refused | Kind::Waiting => Act::Nothing,
        Kind::Message => Act::Answer(chosen),
        Kind::Folder => Act::Folder(typed_folder),
        Kind::Output => Act::TypeLine(typed.to_string()),
        Kind::Settings if SUBLISTS.iter().any(|l| l.name() == entry) => SUBLISTS
            .into_iter()
            .find(|l| l.name() == entry)
            .map_or(Act::Nothing, Act::OpenList),
        Kind::Settings if entry.starts_with(SOUNDS_ROW) => Act::ToggleSounds,
        Kind::Settings if entry.starts_with(CRASH_ROW) => Act::OpenConsentFromSettings,
        Kind::Settings if entry == BACK => Act::BackToStart,
        Kind::Sublist if entry == BACK => Act::OpenSettings,
        Kind::Settings | Kind::Sublist => Act::OpenChoices(chosen),
        Kind::Choices if entry == BACK => Act::BackToList,
        Kind::Choices => Act::Save(entry.to_string()),
        Kind::Consent if entry == CHOOSE_REPORTS => Act::OpenQuestion,
        Kind::Consent if entry == SHOW_REPORT => Act::ShowSample,
        // Not now or Back (and Esc): unanswered, asked again at the next start, nothing sent.
        Kind::Consent => Act::CloseConsent,
        Kind::Question if entry == SEND_REPORTS => Act::Consent(Consent::Send),
        Kind::Question if entry == ASK_REPORTS => Act::Consent(Consent::Ask),
        Kind::Question if entry == NEVER_REPORTS => Act::Consent(Consent::Never),
        Kind::Question | Kind::Sample => Act::OpenConsent,
    }
}

/// A choice's name without its note: "Vulkan (recommended)" is "Vulkan" in a list.
pub fn short(choice: &str) -> &str {
    choice.split(" (").next().unwrap_or(choice)
}

/// A row as a list shows it: its label and its value (the value's note left out).
pub fn row_entry(row: settings::Row, shown: &settings::Files) -> String {
    format!("{}: {}", row.label(), short(&row.current(shown)))
}

/// A Settings list's entries for what `shown` says: each of `list`'s rows with its value, then
/// (the main list) the launcher's sounds (`sounds`: on or off), crash reports (`crash`: the
/// player's answer) and the lists it opens, then Back.
pub fn settings_entries(
    list: settings::List,
    shown: &settings::Files,
    sounds: bool,
    crash: Consent,
) -> Vec<String> {
    let mut e: Vec<String> = list.rows().iter().map(|&r| row_entry(r, shown)).collect();
    if list == settings::List::Main {
        e.push(sounds_entry(sounds));
        e.push(crash_entry(crash));
        e.extend(SUBLISTS.iter().map(|l| l.name().to_string()));
    }
    e.push(BACK.into());
    e
}

/// A Settings list's screen for what `shown` says: `note` (a save that failed), where choices
/// are saved (`in_files`: in the game's files at once, else for Play to move in), the list's own
/// lines, then its entries ([`settings_entries`]), the first highlighted.
pub fn list_screen(
    list: settings::List,
    shown: &settings::Files,
    in_files: bool,
    note: Option<String>,
    sounds: bool,
    crash: Consent,
) -> Screen {
    let mut welcome = Vec::new();
    if let Some(note) = note {
        welcome.push(note);
    }
    welcome.push(if in_files {
        "Saved at once, for both ways of playing (with and without mods). The game reads them at its next start.".to_string()
    } else {
        "Saved for Play to move into the game's own files at its next start.".to_string()
    });
    welcome.extend(list.notes());
    Screen {
        title: format!("-{}-", list.name()),
        welcome,
        entries: settings_entries(list, shown, sounds, crash),
        ..Default::default()
    }
}

/// A row's choices screen for what `shown` says: what it is now (and, for a fix row, whether
/// that comes from the preset), what it does (a table row's plain text; for the preset, how many
/// fixes are chosen one by one), its choices with the one it has highlighted, then Back.
/// `desktop` is the main monitor's size, for the resolution row.
pub fn choices_screen(row: settings::Row, shown: &settings::Files, desktop: (u32, u32)) -> Screen {
    let now = row.current(shown);
    let mut entries = row.choices(shown, desktop);
    let highlighted = entries.iter().position(|c| *c == now).unwrap_or(0);
    entries.push(BACK.into());
    let mut welcome = vec![row.now_line(shown)];
    welcome.extend(row.about());
    if row == settings::Row::GameFixes {
        let chosen = settings::List::Fixes
            .rows()
            .iter()
            .filter(|r| r.chosen_here(shown).is_some())
            .count();
        if chosen > 0 {
            welcome.push(format!(
                "Kept with either: the {chosen} chosen in {}.",
                settings::List::Fixes.name()
            ));
        }
    }
    Screen {
        title: format!("-{}-", row.label()),
        welcome,
        entries,
        highlighted,
        ..Default::default()
    }
}

/// Where the Settings list opens from the screen `mode` shows: on the entry of the list it came
/// back from, else at its top.
fn settings_entry_from(mode: &Mode) -> usize {
    match mode {
        Mode::Settings { list } if *list != settings::List::Main => sublist_entry(*list),
        _ => 0,
    }
}

/// The crash reports row as the list shows it.
pub fn crash_entry(answer: Consent) -> String {
    format!("{CRASH_ROW}: {}", answer.label())
}

/// The sounds row as the list shows it.
pub fn sounds_entry(on: bool) -> String {
    format!("{SOUNDS_ROW}: {}", if on { "On" } else { "Off" })
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
    /// A Settings list: the main one, or one it opens.
    Settings {
        list: settings::List,
    },
    /// The choices of the row at `index` of a Settings list.
    Choices {
        list: settings::List,
        index: usize,
    },
    /// One of the crash reports screens (`screen`: `Kind::Consent`, `Question` or `Sample`).
    Consent {
        screen: Kind,
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
            Mode::Settings {
                list: settings::List::Main,
            } => Kind::Settings,
            Mode::Settings { .. } => Kind::Sublist,
            Mode::Choices { .. } => Kind::Choices,
            Mode::Consent { screen } => *screen,
        }
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
    /// Which of the screen's lines showed at once ([`AtOnce`]), the last screen's title and welcome
    /// text (None before the first, and for the intro), and the frame clock's period (ms).
    shown_at_once: AtOnce,
    last_shown: Option<(String, Vec<String>)>,
    frame_ms: f64,
    rows: Option<screen::Rows>,
    mouse: (f64, f64),
    /// The window's button under the mouse (drawn highlighted).
    hover: Option<screen::Button>,
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
    /// The work running is the mod installer (Mods) or the saves' import, not Play: its good end
    /// goes back to the start screen.
    to_start: bool,
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
    /// Pastes asked so far, and the one being read.
    pastes: Pastes,
    /// The main monitor's size for the resolution row, as the game sees it.
    desktop: (u32, u32),
    /// The terminal's sounds (`crate::sound`), whether they are on (Settings), whether the
    /// window has the focus, and the time (ms after the screen showed) up to which the text
    /// manager's beats have been played.
    sounds: crate::sound::Sounds,
    sounds_on: bool,
    focused: bool,
    beats_to: f64,
    /// The key whose press finished the typing (or the intro), while it is held.
    finisher: Option<winit::keyboard::PhysicalKey>,
    /// Whether crash reports may be sent (`crate::crash::consent`), where their screens stand
    /// ([`ConsentFlow`]), and a line for the start screen (an answer that could not be kept).
    consent: Consent,
    consent_flow: ConsentFlow,
    start_note: Option<String>,
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
    /// The characters typed so far of each of the screen's lines (`screen::line_texts`); None
    /// once all of it is in.
    fn shown(&self) -> Option<Vec<usize>> {
        if self.finished {
            return None;
        }
        let texts = screen::line_texts(&self.look, &self.screen);
        let first_entry = screen::first_entry_line(&self.look, &self.screen);
        Some(typed_lines(
            &texts,
            first_entry,
            self.shown_at_once,
            self.frame_ms,
            self.started.elapsed().as_secs_f64() * 1000.0,
        ))
    }

    /// Whether the entries can be highlighted and chosen now ([`screen::entries_live`]).
    fn entries_live(&self) -> bool {
        screen::entries_live(&self.look, &self.screen, self.shown().as_deref())
    }

    /// Whether the screen is still typing (the intro aside).
    fn typing(&self) -> bool {
        !self.entries_live()
    }

    /// The launcher's shortcut ([`press_finishes_typing`]): the screen's text whole at once.
    fn finish_typing(&mut self) {
        self.finished = true;
        self.request_redraw();
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
                self.shown().map_or_else(
                    || screen::total_chars(&self.look, &self.screen),
                    |v| v.iter().sum(),
                ),
                self.cursor_on(),
            ),
        }
    }

    /// Shows `screen`, typed in from its start (`typed`) or whole at once.
    fn show(&mut self, mode: Mode, screen: Screen, typed: bool) {
        // What the last screen showed (whole: the game completes it first, `ForceFinish`) and
        // so what the new one does not type again.
        let intro = matches!(mode, Mode::Intro(_));
        self.shown_at_once =
            shown_at_once(self.last_shown.as_ref(), &screen.title, &screen.welcome);
        self.last_shown = (!intro).then(|| (screen.title.clone(), screen.welcome.clone()));
        self.frame_ms =
            pacing::frame_ms(menu_frame_limit(&self.place.own, &self.place.fallout_ini));
        self.mode = mode;
        self.screen = screen;
        // Another prompt: a paste asked at the last one is dropped when it comes.
        self.pastes.forget();
        self.started = Instant::now();
        self.beats_to = -1.0;
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
        let kind = self
            .consent_flow
            .at_start(self.place.refused.is_some(), self.consent);
        if let (Kind::Refused, Some((why, code))) = (kind, self.place.refused.clone()) {
            // Play.exe's stop code for the refusal, as it ended with it.
            self.exit_code = code;
            let s = message_screen(&why, &[EXIT.to_string()]);
            self.show(Mode::Refused, s, true);
            return;
        }
        if kind == Kind::Consent {
            return self.show_consent();
        }
        let mut s = start_screen(&self.place, &self.card);
        if let Some(note) = self.start_note.take() {
            s.welcome.push(note);
        }
        self.show(Mode::Start, s, true);
    }

    /// Play (with mods), and with a game folder typed or dropped.
    fn play(&mut self, mods: bool, game: Option<PathBuf>) {
        crate::piped::reopen();
        let args = play_arguments(mods, game.as_deref());
        self.output.clear();
        self.to_start = false;
        self.back_to_waiting();
        self.start_play(args);
    }

    /// Mods: the mod installer (Play.exe's `--install-mods` on Windows, play.sh's on Linux and
    /// macOS, both the game program's text chooser) with its lines in the terminal and its
    /// questions answered at the prompt (`crate::piped`). No game starts, so the sounds go on.
    fn mods(&mut self) {
        self.work_to_start(MOD_INSTALLER);
    }

    /// Import saves: Play.exe's `--import-saves` on Windows, play.sh's on Linux and macOS (the
    /// saves found COPIED into nv-rs's own folder, each with its script extender co-save, a
    /// different save already there replaced only on a yes), its questions in the terminal.
    fn import_saves(&mut self) {
        self.work_to_start(SAVES_IMPORT);
    }

    /// Work that is not Play (`arg`: the installer or the import) on its thread, its good end
    /// back to the start screen ([`end_step`]). No game starts, so the sounds go on.
    fn work_to_start(&mut self, arg: &str) {
        crate::piped::reopen();
        self.output.clear();
        self.to_start = true;
        self.back_to_waiting();
        start_work(vec![arg.into()]);
        self.working = true;
    }

    /// Play's work started in [`sound::PLAY_ORDER`]: the sounds stopped and the device let go
    /// before the work (which starts the game) begins.
    fn start_play(&mut self, args: Vec<OsString>) {
        let mut args = Some(args);
        let sounds = self.sounds.clone();
        sound::play_in_order(
            || sounds.stop(),
            || sounds.release(),
            || {
                if let Some(a) = args.take() {
                    start_work(a);
                }
            },
        );
        self.working = true;
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
            let (canvas, rows) = screen::draw(&self.look, &self.screen, shown.as_deref(), cursor);
            self.rows = Some(rows);
            canvas
        };
        // The 4:3 screen in the window's shape, the buttons in the window's corner.
        let framing = gpu::framing(SCREEN, (config.width, config.height), gpu.most_side());
        let mut canvas = canvas.placed(framing.picture, framing.screen_at);
        screen::draw_buttons(&mut canvas, &self.look, self.hover);
        gpu.upload(&canvas.pixels, framing.picture);
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
        gpu.draw(&mut encoder, &view, config.format, &framing);
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
                let texts = screen::line_texts(&self.look, &self.screen);
                let all_in = shown
                    .as_ref()
                    .is_none_or(|v| texts.iter().zip(v).all(|(t, &n)| n >= t.chars().count()));
                if !self.finished && all_in {
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
        crate::piped::close_input();
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

    /// How the screen stands in the window now, as [`App::redraw`] draws it.
    fn framing(&self) -> Option<gpu::Framing> {
        let (config, gpu) = (self.config.as_ref()?, self.gpu.as_ref()?);
        Some(gpu::framing(
            SCREEN,
            (config.width, config.height),
            gpu.most_side(),
        ))
    }

    /// The row under the window position, if any.
    fn row_at(&self, pos: (f64, f64)) -> Option<usize> {
        row_hit(&self.framing()?, self.rows.as_ref()?, pos)
    }

    /// The window's button under the window position, if any.
    fn button_at(&self, pos: (f64, f64)) -> Option<screen::Button> {
        button_hit(&self.framing()?, &self.look, pos)
    }

    /// The button under the mouse is `over` now: drawn again when that changes.
    fn set_hover(&mut self, over: Option<screen::Button>) {
        if self.hover != over {
            self.hover = over;
            self.frame.changed();
            self.request_redraw();
        }
    }

    /// Whether what is typed goes to the prompt on this screen.
    /// Does what a choice does.
    fn act(&mut self, act: Act, event_loop: &ActiveEventLoop) {
        match act {
            Act::EndIntro => self.end_intro(),
            Act::Play { mods, game } => self.play(mods, game),
            Act::Exit => event_loop.exit(),
            Act::Mods => self.mods(),
            Act::ImportSaves => self.import_saves(),
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
                crate::piped::type_line(&line);
                self.add_output(&format!("{line}\n"));
            }
            Act::BackToStart => self.show_start(),
            Act::OpenSettings => {
                // Back from a list it opened: on that list's entry.
                let at = settings_entry_from(&self.mode);
                self.show_settings(settings::List::Main, at, None)
            }
            Act::OpenList(list) => self.show_settings(list, 0, None),
            Act::OpenChoices(index) => {
                if let Mode::Settings { list } = self.mode {
                    if index < list.rows().len() {
                        self.show_choices(list, index);
                    }
                }
            }
            Act::Save(choice) => {
                if let Mode::Choices { list, index } = self.mode {
                    self.save(list, index, &choice);
                }
            }
            Act::BackToList => {
                if let Mode::Choices { list, index } = self.mode {
                    self.show_settings(list, index, None);
                }
            }
            Act::ToggleSounds => self.toggle_sounds(),
            Act::Consent(answer) => self.answer_consent(answer),
            Act::ShowSample => self.show_sample(),
            Act::OpenConsent => self.show_consent(),
            Act::OpenQuestion => self.show_question(),
            Act::OpenConsentFromSettings => self.open_consent_from_settings(),
            Act::CloseConsent => self.close_consent(None),
            Act::Close => self.close_while_working(event_loop),
            Act::Nothing => {}
        }
    }

    /// The terminal's sounds turned off (all stops, the device let go) or on, kept in the
    /// launcher's own file; the Settings list again on the row (with why it was not kept).
    fn toggle_sounds(&mut self) {
        let on = !self.sounds_on;
        let note = if self.place.refused.is_some() {
            Some("Not saved: nv-rs may not write in its own folder".to_string())
        } else {
            crate::place::set_sounds(&self.place.own, on)
                .err()
                .map(|e| format!("Not saved: {e}"))
        };
        self.sounds_on = on;
        self.update_sound();
        self.show_settings(settings::List::Main, settings::ROWS.len(), note);
    }

    /// The crash reports screens opened from Settings' row (Enter, a click, Left or Right).
    fn open_consent_from_settings(&mut self) {
        self.consent_flow.open_in_settings();
        self.show_consent();
    }

    /// The first crash reports screen: what a report holds; Choose, the example, Not now (from
    /// the start screen) or Back (from Settings).
    fn show_consent(&mut self) {
        let s = Screen {
            title: format!("-{CRASH_ROW}-"),
            welcome: about_text(self.consent_flow.in_settings().then_some(self.consent)),
            entries: about_entries(self.consent_flow.in_settings()),
            ..Default::default()
        };
        let retype = !matches!(self.mode, Mode::Consent { .. });
        self.show(
            Mode::Consent {
                screen: Kind::Consent,
            },
            s,
            retype,
        );
    }

    /// The question: where reports go, the three answers, Back.
    fn show_question(&mut self) {
        // This copy can send when its package has the crash service's file.
        let s = question_screen(crate::crash::send::package_dsn(&program_folder()).is_some());
        // Typed in like the first screen: a first press or click only finishes its text, so a
        // double click on Choose never answers.
        self.show(
            Mode::Consent {
                screen: Kind::Question,
            },
            s,
            true,
        );
    }

    /// The example report ([`crate::crash::sample`]: made from this PC now, nothing written),
    /// long enough to scroll; Back to the first screen.
    fn show_sample(&mut self) {
        let text = crate::crash::sample(
            &self.place.own,
            &program_folder(),
            self.place.game.as_deref(),
            self.place.mods,
            self.place.adapter_card.clone(),
        );
        let s = Screen {
            title: "-A crash report-".into(),
            welcome: text.lines().map(str::to_string).collect(),
            entries: vec![BACK.into()],
            ..Default::default()
        };
        self.show(
            Mode::Consent {
                screen: Kind::Sample,
            },
            s,
            true,
        );
    }

    /// `answer` kept for crash reports ([`keep_consent`]).
    fn set_consent(&mut self, answer: Consent, now: bool) -> Option<String> {
        self.consent = answer;
        keep_consent(&self.place.own, self.place.refused.is_some(), answer, now)
    }

    /// The question answered: kept, then back to where the screens were opened from.
    fn answer_consent(&mut self, answer: Consent) {
        self.log(&format!("crash reports: {}", answer.label()));
        let note = self.set_consent(answer, self.consent_flow.answer_now());
        if let Some(note) = &note {
            self.log(&format!("crash reports: {note}"));
        }
        self.close_consent(note);
    }

    /// The crash reports screens left (answered or not), with what went wrong when the answer
    /// was not kept: Settings on its row, or the start screen.
    fn close_consent(&mut self, note: Option<String>) {
        if self.consent_flow.close() {
            self.show_settings(settings::List::Main, CRASH_ROW_INDEX, note);
        } else {
            self.start_note = note;
            self.show_start();
        }
    }

    /// The device open (and the hum playing) while the terminal is heard ([`sound::heard`]),
    /// else let go; a line for the log when it would not open.
    fn update_sound(&mut self) {
        let minimised = self
            .window
            .as_ref()
            .and_then(|w| w.is_minimized())
            .unwrap_or(false);
        let shown = self.window.is_some() && !self.hidden && !minimised;
        if sound::heard(self.sounds_on, shown, self.focused, self.working) {
            self.sounds.open();
        } else {
            self.sounds.release_later();
        }
        for line in self.sounds.take_lines() {
            self.log(&line);
        }
    }

    /// A line for the launcher's log ([`place::log_line`]).
    fn log(&self, text: &str) {
        let now = cfg!(not(windows)) && self.place.refused.is_none();
        place::log_line(&self.place.own, now, text);
    }

    /// The text manager's sound beats since the last call ([`pacing::beats`]), played: the
    /// intro's, or the screen's while it types. A screen finished at once (the launcher's press)
    /// has no more: what plays ends as it does when the queue empties, and the characters it
    /// skipped make no sound.
    fn play_beats(&mut self) {
        let t = self.started.elapsed().as_secs_f64() * 1000.0;
        let from = std::mem::replace(&mut self.beats_to, t);
        if !self.sounds.is_open() {
            return;
        }
        let (queue, frame) = match &self.mode {
            Mode::Intro(intro) => (intro.queue(), intro.frame),
            _ if self.finished => return,
            _ => {
                let texts = screen::line_texts(&self.look, &self.screen);
                let first = screen::first_entry_line(&self.look, &self.screen);
                (
                    screen_queue(&texts, first, self.shown_at_once),
                    self.frame_ms,
                )
            }
        };
        // Each at its own time; after a stall the older ones are dropped ([`pacing::scheduled`]).
        let beats = pacing::beats(&queue, frame, from, t);
        for (delay, beat) in pacing::scheduled(&beats, t) {
            match beat {
                // The kept sounds play only when not playing: at once.
                pacing::Beat::Scroll => self.sounds.play(Cue::Scroll),
                pacing::Beat::Enter => self.sounds.play(Cue::Enter),
                pacing::Beat::Singles { n, ms } => {
                    for i in 0..n {
                        let at = delay + ms * i as u64 / n as u64;
                        self.sounds.play_after(Cue::Single, at);
                    }
                }
            }
        }
    }

    /// The focus moved to entry `i` (from the mouse or the arrows): its sound when it is another.
    fn focus(&mut self, i: usize) {
        if self.screen.highlighted != i {
            self.screen.highlighted = i;
            self.sounds.play(Cue::Focus);
        }
    }

    /// A Settings list, the row at `highlight` highlighted, with `note` (a save that failed)
    /// above it.
    fn show_settings(&mut self, list: settings::List, highlight: usize, note: Option<String>) {
        let profiles = crate::profiles::Profiles::at(&self.place.own);
        let (shown, in_files) = match profiles.shown().and_then(|s| Ok((s, profiles.in_files()?))) {
            Ok(v) => v,
            Err(e) => {
                let s = message_screen(
                    &format!("nv-rs cannot read its settings: {e}"),
                    &[BACK.to_string()],
                );
                self.show(Mode::Settings { list }, s, true);
                return;
            }
        };
        let mut s = list_screen(list, &shown, in_files, note, self.sounds_on, self.consent);
        s.highlighted = highlight.min(s.entries.len() - 1);
        let retype = !matches!(self.mode, Mode::Settings { .. } | Mode::Choices { .. });
        self.show(Mode::Settings { list }, s, retype);
    }

    /// The choices of the row at `index` of the list, the one it has highlighted.
    fn show_choices(&mut self, list: settings::List, index: usize) {
        let row = list.rows()[index];
        let profiles = crate::profiles::Profiles::at(&self.place.own);
        let shown = match profiles.shown() {
            Ok(s) => s,
            Err(e) => return self.show_settings(list, index, Some(format!("Not read: {e}"))),
        };
        let s = choices_screen(row, &shown, self.desktop);
        self.show(Mode::Choices { list, index }, s, false);
    }

    /// `choice` saved for the row at `index`, then its list again (with what went wrong when it
    /// was not saved).
    fn save(&mut self, list: settings::List, index: usize, choice: &str) {
        let row = list.rows()[index];
        let profiles = crate::profiles::Profiles::at(&self.place.own);
        let note = profiles
            .save(row, choice)
            .err()
            .map(|e| format!("Not saved: {e}"));
        self.show_settings(list, index, note);
    }

    /// Left or Right on a row of a Settings list: its next or previous choice, saved at once.
    fn step_row(&mut self, forward: bool) {
        let Mode::Settings { list } = self.mode else {
            return;
        };
        let index = self.screen.highlighted;
        // The sounds row has two choices: either way flips it. The crash reports row opens its
        // screens ([`row_step_act`]).
        match row_step_act(list, index) {
            Some(Act::ToggleSounds) => return self.toggle_sounds(),
            Some(Act::OpenConsentFromSettings) => return self.open_consent_from_settings(),
            _ => {}
        }
        let Some(&row) = list.rows().get(index) else {
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
            self.save(list, index, &next);
        }
    }

    /// Enter: the highlighted entry chosen (or, with something typed, the typed line).
    fn choose(&mut self, event_loop: &ActiveEventLoop) {
        let chosen = self.screen.highlighted;
        if !may_choose(
            &self.screen.typed,
            !self.screen.entries.is_empty(),
            self.entries_live() && chosen < self.screen.entries.len(),
        ) {
            return;
        }
        let typed = std::mem::take(&mut self.screen.typed);
        let entry = self.screen.entries.get(chosen).cloned();
        for &cue in choice_sounds(&typed, entry.as_deref()) {
            self.sounds.play(cue);
        }
        let act = on_enter(
            self.mode.kind(),
            &typed,
            chosen,
            entry.as_deref().unwrap_or(""),
        );
        // The prompt is taken (Enter typed its line, or a choice left it): a paste still being
        // read is not for what comes next.
        if !prompt_kept(&act) {
            self.pastes.forget();
        }
        self.act(act, event_loop);
    }

    /// Esc: the screen's way back, whatever is typed.
    fn back(&mut self, event_loop: &ActiveEventLoop) {
        let act = on_escape(self.mode.kind(), &self.screen.entries);
        // Every way back shows another screen, which drops a paste still being read (`show`).
        if !prompt_kept(&act) {
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
        let s = output_screen(&self.output, ended.is_some(), typed);
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
        // No title bar or border, as a terminal's screen (the maintainer's wish): the window is
        // moved by dragging its background ([`press_drags`]) and closed by Exit, Esc's way back
        // to it, or the system's own close keys. The title still names it in the task bar.
        let least = least_size((w, h));
        let mut attributes = frameless(winit::window::Window::default_attributes())
            .with_visible(false)
            .with_inner_size(winit::dpi::LogicalSize::new(w, h))
            .with_min_inner_size(winit::dpi::LogicalSize::new(least.0, least.1));
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
        // From here on a program the work runs takes the lines typed at the prompt.
        crate::piped::window_takes_lines(true);
        // The game's card, from a process of its own.
        std::thread::spawn(|| {
            session::send(Event::Card(ask_game_card()));
        });
        // The terminal's sounds, read from the player's game (when it is there).
        if let Some(game) = self.place.game.clone() {
            let files = profile_files(&self.place.fallout_ini);
            std::thread::spawn(move || {
                let v = crate::sound_files::Volumes::from_files(&files.ini, &files.prefs);
                let (banks, said) = sound::load(&game, v);
                session::send(Event::Sounds { banks, said });
            });
        } else {
            self.log("sounds: the game was not found yet: the launcher is silent");
        }
        if let Some(args) = self.action.take() {
            if self.place.refused.is_some() {
                self.show_start();
            } else {
                self.output.clear();
                self.back_to_waiting();
                self.start_play(args);
            }
        } else if self.place.refused.is_none()
            && self.look.intro.is_some()
            && !place::intro_shown(&self.place.own)
        {
            let [welcome, logon, header] = self.look.intro.clone().expect("checked");
            let odd = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .is_ok_and(|d| d.subsec_nanos() & 1 == 1);
            let intro = Intro::new(
                &welcome,
                &logon,
                &header,
                odd,
                pacing::frame_ms(menu_frame_limit(&self.place.own, &self.place.fallout_ini)),
            );
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
                self.working = false;
                let mods = std::mem::take(&mut self.to_start);
                if mods {
                    // The installer may have added or removed mods (Play with mods follows), the
                    // import its saves.
                    let now = place::place();
                    self.place.mods = now.mods;
                    self.place.saves = now.saves;
                } else {
                    self.exit_code = code;
                }
                match end_step(
                    mods,
                    code,
                    self.closed,
                    matches!(self.mode, Mode::Output(_)),
                ) {
                    // Closed while the work ran, or Play's good end: the launcher ends.
                    EndStep::Exit => event_loop.exit(),
                    // The installer's good end, or Play.exe's message shown and answered.
                    EndStep::Start => self.show_start(),
                    // The work's own lines say what went wrong; Back or Exit below them.
                    EndStep::Lines => {
                        self.mode = Mode::Output(Some(code));
                        self.add_output("");
                    }
                }
            }
            Event::Pasted { id, text } => {
                let (kind, now) = (self.mode.kind(), Instant::now());
                if self.pastes.answer(id, now, kind, &self.screen.typed) {
                    if let Some(text) = text {
                        self.screen.typed.push_str(&text);
                        self.request_redraw();
                    }
                }
            }
            Event::Sounds { banks, said } => {
                for line in said {
                    self.log(&line);
                }
                self.sounds.release_later();
                self.sounds = sound::Sounds::new(banks);
                self.update_sound();
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
        use winit::keyboard::Key;
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
            // The game stops running, and sounding, without the focus: so does the terminal's sound.
            WindowEvent::Focused(f) => {
                self.focused = f;
                self.update_sound();
            }
            WindowEvent::DroppedFile(path) => {
                match &mut self.mode {
                    Mode::Start => self.play(false, Some(path)),
                    Mode::Folder(_) => self.act(Act::Folder(Some(path)), event_loop),
                    Mode::Output(None) => self.screen.typed = path.to_string_lossy().into_owned(),
                    _ => {}
                }
                self.request_redraw();
            }
            // The key whose press finished the typing is let go: its repeats are over.
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Released
                    && self.finisher == Some(event.physical_key) =>
            {
                self.finisher = None;
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let press = press_of(&event.logical_key);
                if matches!(self.mode, Mode::Intro(_)) {
                    // Esc or Enter finishes the intro (the launcher's extra; the game takes a
                    // click on the background); other keys, and a held key's repeats, do nothing.
                    if crate::intro::key_finishes(press, event.repeat) {
                        self.finisher = Some(event.physical_key);
                        self.end_intro();
                    }
                    return;
                }
                let finisher = self.finisher == Some(event.physical_key);
                let act = key_act(press, event.repeat, self.typing(), finisher);
                let press = match act {
                    KeyAct::FinishTyping => {
                        self.finisher = Some(event.physical_key);
                        return self.finish_typing();
                    }
                    KeyAct::Nothing => return,
                    KeyAct::Act(p) => p,
                };
                let count = self.screen.entries.len();
                match press {
                    Press::Up => self.focus(step_focus(self.screen.highlighted, count, true)),
                    Press::Down => self.focus(step_focus(self.screen.highlighted, count, false)),
                    Press::PageUp => {
                        self.screen.scroll = screen::scrolled(&self.look, &self.screen, true, 5)
                    }
                    Press::PageDown => {
                        self.screen.scroll = screen::scrolled(&self.look, &self.screen, false, 5)
                    }
                    Press::Right => self.step_row(true),
                    Press::Left => self.step_row(false),
                    Press::Enter => self.choose(event_loop),
                    Press::Backspace => {
                        self.screen.typed.pop();
                    }
                    Press::Escape => self.back(event_loop),
                    Press::Click | Press::Modifier => {}
                    Press::Other => {
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
                            let id = self.pastes.ask(Instant::now(), kind, &self.screen.typed);
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
                self.set_hover(self.button_at(self.mouse));
                if let Some(i) = self.row_at(self.mouse) {
                    // The focus moves only onto live entries (the game's are disabled while
                    // it types).
                    if self.entries_live() && self.screen.highlighted != i {
                        self.focus(i);
                        self.request_redraw();
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => self.set_hover(None),
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => (p.y / 30.0) as f32,
                };
                // The wheel up shows what is above (a program's output's earlier lines too).
                if lines != 0.0 {
                    let by = lines.abs().ceil() as usize;
                    self.screen.scroll =
                        screen::scrolled(&self.look, &self.screen, lines > 0.0, by);
                }
                self.request_redraw();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                // The window's own buttons first, on every screen, typing or not; a press on
                // one never moves the window.
                if let Some(b) = self.button_at(self.mouse) {
                    if b == screen::Button::Minimise {
                        self.set_hover(None);
                    }
                    let act = press_button(b, self.working, self.window.as_deref());
                    return self.act(act, event_loop);
                }
                // Off the entries the press also moves the window (it has no title bar); the
                // intro's own click (which ends it) moves it the same way.
                let intro = matches!(self.mode, Mode::Intro(_));
                let row = if intro { None } else { self.row_at(self.mouse) };
                if intro {
                    // A click on the terminal's background finishes the intro (`DoClick` 00757f70,
                    // tile 13); the background is the whole window.
                    self.end_intro();
                } else {
                    match key_act(Press::Click, false, self.typing(), false) {
                        KeyAct::FinishTyping => self.finish_typing(),
                        KeyAct::Act(_) => {
                            // A live entry under the mouse is chosen.
                            if let Some(i) = row {
                                self.screen.highlighted = i;
                                self.choose(event_loop);
                            }
                        }
                        KeyAct::Nothing => {}
                    }
                }
                if press_drags(row.is_some(), self.hidden) {
                    if let Some(w) = &self.window {
                        let _ = w.drag_window();
                    }
                }
                self.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.update_sound();
        self.play_beats();
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
        // While the sounds are open (or a try is due), a look at the device once a second.
        let wake = sound::check_wake(wake, self.sounds.watching(), Instant::now());
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
    let app_own = place.own.clone();
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
        shown_at_once: AtOnce::default(),
        last_shown: None,
        frame_ms: pacing::frame_ms(0),
        rows: None,
        mouse: (0.0, 0.0),
        hover: None,
        card: Card::Asking,
        action: (!raw.is_empty()).then(|| raw.iter().map(OsString::from).collect()),
        output: Vec::new(),
        hidden: false,
        minimise_at: None,
        working: false,
        to_start: false,
        closed: false,
        frame: FrameState::default(),
        chosen_again: Vec::new(),
        no_window: false,
        exit_code: 0,
        modifiers: winit::keyboard::ModifiersState::empty(),
        pastes: Pastes::default(),
        desktop: (0, 0),
        sounds: sound::Sounds::new(Vec::new()),
        sounds_on: place::sounds_on(&app_own),
        focused: true,
        beats_to: -1.0,
        finisher: None,
        consent: consent::read(&app_own),
        consent_flow: ConsentFlow::default(),
        start_note: None,
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
        saves: false,
        refused: None,
    };
    let look = look_of(place.game.as_deref(), &fallout_ini, c.plain);
    let mut drawing = gpu::choose(None, (0, 0))?;
    let canvas = match (c.intro_at, &look.intro, &c.output) {
        (_, _, Some(file)) => {
            // A program's output as the terminal shows it while the program runs (the mod
            // installer's lines), scrolled up `scroll` lines from its end.
            let text =
                std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
            let mut lines = Vec::new();
            append_output(&mut lines, &text);
            let mut s = output_screen(&lines, false, String::new());
            s.scroll = c.scroll;
            screen::draw(&look, &s, None, true).0
        }
        (Some(ms), Some([welcome, logon, header]), None) => {
            let intro = Intro::new(welcome, logon, header, false, pacing::frame_ms(0));
            screen::draw_intro(&look, &intro.typed(ms))
        }
        (Some(_), None, None) => return Err("the intro needs the game's look".into()),
        (None, _, None) => {
            let card =
                Card::Known(gpu::game_card().map(|(n, a)| (n, gpu::api_name(a).to_string())));
            let s = start_screen(&place, &card);
            screen::draw(&look, &s, None, true).0
        }
    };
    let g = &mut drawing.gpu;
    let framing = gpu::framing(SCREEN, c.size, g.most_side());
    let mut canvas = canvas.placed(framing.picture, framing.screen_at);
    screen::draw_buttons(&mut canvas, &look, None);
    g.upload(&canvas.pixels, framing.picture);
    let rgba = g.capture(&framing, c.size)?;
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
            saves: false,
            refused: None,
        }
    }

    /// When the work ends: the mod installer's good end back to the start screen, Play's good
    /// end the launcher's end; a closed window ends it; a bad end shows the lines, or the start
    /// screen when the work wrote none (its message was answered). Mods runs Play.exe's and
    /// play.sh's own installer argument.
    #[test]
    fn the_installer_comes_back_to_the_start_and_play_ends_the_launcher() {
        assert_eq!(end_step(true, 0, false, true), EndStep::Start);
        assert_eq!(end_step(false, 0, false, true), EndStep::Exit);
        assert_eq!(end_step(true, 0, true, true), EndStep::Exit);
        assert_eq!(end_step(false, 3, true, true), EndStep::Exit);
        assert_eq!(end_step(true, 2, false, true), EndStep::Lines);
        assert_eq!(end_step(false, 7, false, true), EndStep::Lines);
        assert_eq!(end_step(false, 7, false, false), EndStep::Start);
        let script = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../distribution/unix/play.sh"),
        )
        .unwrap();
        assert!(script.contains(&format!("({MOD_INSTALLER}) opt_install=1")));
        assert!(script.contains(&format!("({SAVES_IMPORT}) opt_import=1")));
    }

    /// The window has no title bar or border, keeps its task bar name, and a press off the
    /// entries moves it (on an entry the press chooses it; never while the window steps aside).
    #[test]
    fn the_window_has_no_frame_and_moves_by_its_background() {
        let a = frameless(winit::window::Window::default_attributes());
        assert!(!a.decorations);
        assert_eq!(a.title, "nv-rs");
        assert!(press_drags(false, false));
        assert!(!press_drags(true, false));
        assert!(!press_drags(false, true));
    }

    /// A window that counts its minimises.
    #[derive(Default)]
    struct Counted(std::cell::Cell<u32>);

    impl Minimise for Counted {
        fn minimise(&self) {
            self.0.set(self.0.get() + 1);
        }
    }

    /// The window's two buttons are hit inside their boxes, in the window's top-right corner
    /// whatever its shape (scaled, or with bars beside the screen: the buttons are on the bar
    /// then), and nowhere else: not between them, beyond them, on the header or on an entry; no
    /// entry is hit on them.
    #[test]
    fn the_window_buttons_are_hit_in_their_boxes() {
        use screen::Button;
        let look = Look::plain();
        let s = start_screen(&place(Some("/g/x"), false), &Card::Asking);
        let rows = screen::rows_of(&look, &s);
        for (window, scale, left, picture_width) in [
            ((960, 720), 1.0, 0.0, 960),
            ((1920, 1440), 2.0, 0.0, 960),
            ((1600, 720), 1.0, 320.0, 1600),
        ] {
            let framing = gpu::framing(SCREEN, window, 8192);
            assert_eq!(framing.picture.0, picture_width);
            let boxes = screen::buttons(&look, picture_width);
            assert_eq!(boxes.map(|(b, _)| b), [Button::Minimise, Button::Exit]);
            // A point of the picture (the buttons'), and one of the screen (the rest).
            let at = |x: f64, y: f64| (x * scale, y * scale);
            let on_screen = |x: f64, y: f64| (left + x * scale, y * scale);
            let window_right = window.0 as f64 - 10.0 * scale;
            assert_eq!(
                at((boxes[1].1 .0 + boxes[1].1 .2 as i32) as f64, 0.0).0,
                window_right
            );
            for (b, (x, y, w, h)) in boxes {
                let (x, y, w, h) = (x as f64, y as f64, w as f64, h as f64);
                for (px, py) in [
                    (x + 0.5, y + 0.5),
                    (x + w / 2.0, y + h / 2.0),
                    (x + w - 0.5, y + h - 0.5),
                ] {
                    let p = at(px, py);
                    assert_eq!(button_hit(&framing, &look, p), Some(b), "{window:?} {p:?}");
                    assert_eq!(row_hit(&framing, &rows, p), None, "{window:?} {p:?}");
                }
                for (px, py) in [
                    (x - 1.0, y + h / 2.0),
                    (x + w + 1.0, y + h / 2.0),
                    (x + w / 2.0, y - 1.0),
                    (x + w / 2.0, y + h + 1.0),
                ] {
                    let p = at(px, py);
                    assert_ne!(button_hit(&framing, &look, p), Some(b), "{window:?} {p:?}");
                }
            }
            let (_, (mx, my, mw, mh)) = boxes[0];
            let mid = (my + mh as i32 / 2) as f64;
            let gap = (mx + mw as i32) as f64 + 2.5;
            assert_eq!(button_hit(&framing, &look, at(gap, mid)), None);
            let right = picture_width as f64 - 5.0;
            assert_eq!(button_hit(&framing, &look, at(right, mid)), None);
            assert_eq!(button_hit(&framing, &look, on_screen(480.0, 70.0)), None);
            let entry = on_screen(rows.x as f64 + 10.0, rows.top as f64 + 10.0);
            assert_eq!(button_hit(&framing, &look, entry), None);
            assert_eq!(row_hit(&framing, &rows, entry), Some(0));
        }
        let none = gpu::framing(SCREEN, (0, 0), 8192);
        assert_eq!(button_hit(&none, &look, (0.0, 0.0)), None);
        assert_eq!(row_hit(&none, &rows, (0.0, 0.0)), None);
    }

    /// The exit button does what the menu's Exit does (the same act, so the same way out of the
    /// event loop and the same end) on every screen that has Exit, and on the others while
    /// nothing runs; while Play's work runs (no screen offers Exit then) it closes the window as
    /// the window's own close does then. It never minimises.
    #[test]
    fn the_exit_button_goes_the_way_of_the_menus_exit() {
        let w = Counted::default();
        let s = start_screen(&place(Some("/g/x"), false), &Card::Asking);
        let i = s.entries.iter().position(|e| e == EXIT).unwrap();
        let menu = on_enter(Kind::Start, "", i, &s.entries[i]);
        assert_eq!(menu, Act::Exit);
        assert_eq!(press_button(screen::Button::Exit, false, Some(&w)), menu);
        for kind in [Kind::Refused, Kind::Ended] {
            assert_eq!(
                press_button(screen::Button::Exit, false, Some(&w)),
                on_enter(kind, "", 0, EXIT)
            );
        }
        assert_eq!(
            press_button(screen::Button::Exit, true, Some(&w)),
            Act::Close
        );
        assert_eq!(w.0.get(), 0);
    }

    /// The minimise button minimises the window (winit's, on every system) and does nothing
    /// else, Play's work running or not; before there is a window it does nothing.
    #[test]
    fn the_minimise_button_minimises_the_window() {
        for working in [false, true] {
            let w = Counted::default();
            let act = press_button(screen::Button::Minimise, working, Some(&w));
            assert_eq!(act, Act::Nothing);
            assert_eq!(w.0.get(), 1);
        }
        let none: Option<&Counted> = None;
        assert_eq!(
            press_button(screen::Button::Minimise, false, none),
            Act::Nothing
        );
    }

    /// The start screen offers what works today: Play, Play with mods only when there are mods,
    /// Mods, Settings, Exit; no entry for anything that does not exist yet.
    #[test]
    fn the_start_screen_offers_what_works() {
        let s = start_screen(&place(Some("/g/Fallout New Vegas"), false), &Card::Asking);
        assert_eq!(s.entries, [PLAY, MODS, SETTINGS, EXIT]);
        assert_eq!(s.welcome[0], "Game: Fallout New Vegas");
        let s = start_screen(&place(Some("/g/x"), true), &Card::Asking);
        assert_eq!(s.entries, [PLAY, PLAY_WITH_MODS, MODS, SETTINGS, EXIT]);
        let mut with_saves = place(Some("/g/x"), false);
        with_saves.saves = true;
        let s = start_screen(&with_saves, &Card::Asking);
        assert_eq!(s.entries, [PLAY, MODS, IMPORT_SAVES, SETTINGS, EXIT]);
        let mut with_saves = place(Some("/g/x"), false);
        with_saves.saves = true;
        let s = start_screen(&with_saves, &Card::Asking);
        assert_eq!(s.entries, [PLAY, MODS, IMPORT_SAVES, SETTINGS, EXIT]);
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
        // The crash question's: Esc and a closed window never send.
        assert_eq!(back_entry(&e(&crate::crash::ASK_ENTRIES)), Some(0));
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
        // Ultrawide screens (21:9, 32:9): the height decides, never the width; the window is
        // the terminal's 4:3, never the monitor's shape.
        for monitor in [(3440.0, 1440.0), (5120.0, 1440.0), (2560.0, 1080.0)] {
            assert_eq!(first_size(Some(monitor)), (960.0, 720.0), "{monitor:?}");
        }
        // The smallest window: half the terminal, never more than the first window, even on a
        // monitor whose logical size is far too small (a wrong scale factor).
        assert_eq!(least_size((960.0, 720.0)), (480.0, 360.0));
        for monitor in [(242.0, 101.0), (640.0, 360.0), (1280.0, 720.0)] {
            let first = first_size(Some(monitor));
            let least = least_size(first);
            assert!(least.0 <= first.0 && least.1 <= first.1, "{monitor:?}");
            assert!(first.1 <= monitor.1 && first.0 <= monitor.0, "{monitor:?}");
        }
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

    /// A press on each shown entry hits that entry and does not move the window; a press above
    /// the rows, left or right of their column, or below the last shown row hits none and moves
    /// it. The start screen, the full Settings list (17 entries: it scrolls) and its fixes list
    /// (every fix row), at the top and scrolled to the end, in the 960x720 window, a smaller one
    /// and a wider one.
    #[test]
    fn a_press_hits_the_entry_under_it_and_drags_elsewhere() {
        let look = Look::plain();
        let settings = settings_entries(
            settings::List::Main,
            &settings::Files::default(),
            true,
            Consent::Unanswered,
        );
        let fixes = settings_entries(
            settings::List::Fixes,
            &settings::Files::default(),
            true,
            Consent::Unanswered,
        );
        let start = vec![PLAY.to_string(), MODS.into(), SETTINGS.into(), EXIT.into()];
        for entries in [start, settings, fixes] {
            for highlighted in [0, entries.len() - 1] {
                let s = Screen {
                    entries: entries.clone(),
                    highlighted,
                    ..Default::default()
                };
                let rows = screen::rows_of(&look, &s);
                assert!(rows.shown > 0);
                for window in [(960, 720), (640, 480), (1280, 700)] {
                    // The screen where `fit` puts it, to check the framing's mapping against.
                    let framing = gpu::framing(SCREEN, window, 8192);
                    let (x, y, w, h) = gpu::fit((screen::WIDTH, screen::HEIGHT), window);
                    let at = |px: f64, py: f64| {
                        (
                            x as f64 + px * w as f64 / screen::WIDTH as f64,
                            y as f64 + py * h as f64 / screen::HEIGHT as f64,
                        )
                    };
                    let mid = rows.x as f64 + rows.width as f64 / 2.0;
                    let row_y = |k: f64| rows.top as f64 + (k + 0.5) * rows.row as f64;
                    for k in 0..rows.shown {
                        let hit = row_hit(&framing, &rows, at(mid, row_y(k as f64)));
                        assert_eq!(hit, Some(rows.first + k), "{window:?} row {k}");
                        assert!(!press_drags(hit.is_some(), false));
                    }
                    for (px, py) in [
                        (mid, rows.top as f64 - 5.0),
                        (rows.x as f64 - 5.0, row_y(0.0)),
                        (rows.x as f64 + rows.width as f64 + 5.0, row_y(0.0)),
                        (mid, row_y(rows.shown as f64)),
                    ] {
                        let hit = row_hit(&framing, &rows, at(px, py));
                        assert_eq!(hit, None, "{window:?} at {px},{py}");
                        assert!(press_drags(hit.is_some(), false));
                    }
                }
            }
        }
    }

    /// The Settings list shows each row with its value (its note left out), then the two lists it
    /// opens and Back; each of those its rows and Back. Enter on a row opens its choices, on a
    /// choice saves it; Esc goes back a list (never saving); Left and Right step round a row's
    /// choices.
    #[test]
    fn the_settings_lists_open_choose_and_go_back() {
        let shown = settings::Files::default();
        let main = settings_entries(settings::List::Main, &shown, true, Consent::Ask);
        assert_eq!(main.len(), 17);
        assert_eq!(main[0], "Renderer: Vulkan");
        assert_eq!(main[12], "Launcher sounds: On");
        assert_eq!(main[CRASH_ROW_INDEX], "Crash reports: Ask each time");
        assert_eq!(main[14], "New lighting and more");
        assert_eq!(main[15], "Fixes one by one");
        assert_eq!(main[16], BACK);
        for list in SUBLISTS {
            assert_eq!(main[sublist_entry(list)], list.name());
            assert_eq!(
                on_enter(Kind::Settings, "", sublist_entry(list), list.name()),
                Act::OpenList(list)
            );
            let e = settings_entries(list, &shown, true, Consent::Send);
            assert!(!e.iter().any(|x| x.starts_with(CRASH_ROW)));
            assert_eq!(e.len(), list.rows().len() + 1);
            assert_eq!(e.last().map(String::as_str), Some(BACK));
            assert_eq!(on_escape(Kind::Sublist, &e), Act::OpenSettings);
            assert_eq!(on_enter(Kind::Sublist, "", 0, &e[0]), Act::OpenChoices(0));
        }
        // The fixes list: every fix row with the value the game plays it with.
        let fixes = settings_entries(settings::List::Fixes, &shown, true, Consent::Ask);
        assert_eq!(fixes[0], "Uncapped frame rate: On");
        assert!(fixes.contains(&"JIP LN NVSE built in: On".to_string()));
        assert!(fixes.contains(&"UI Organizer built in: On".to_string()));
        let more = settings_entries(settings::List::More, &shown, true, Consent::Ask);
        assert_eq!(more[0], "Ambient occlusion: Off");
        assert!(more.contains(&"Ambient occlusion strength: 0.7".to_string()));
        assert!(more.contains(&"Sun shadows (experimental): Off".to_string()));
        // Back from a list Settings opened lands on its entry; anything else at the top.
        for list in SUBLISTS {
            assert_eq!(
                settings_entry_from(&Mode::Settings { list }),
                sublist_entry(list)
            );
        }
        let main_list = Mode::Settings {
            list: settings::List::Main,
        };
        assert_eq!(settings_entry_from(&main_list), 0);
        assert_eq!(settings_entry_from(&Mode::Refused), 0);
        assert_eq!(
            settings_entries(settings::List::Main, &shown, false, Consent::Never)[12],
            "Launcher sounds: Off"
        );
        // Enter on the crash reports row opens its screens (never answers on one press).
        assert_eq!(
            on_enter(Kind::Settings, "", CRASH_ROW_INDEX, &main[CRASH_ROW_INDEX]),
            Act::OpenConsentFromSettings
        );
        // Enter on the sounds row turns them on or off at once.
        assert_eq!(
            on_enter(Kind::Settings, "", 12, &main[12]),
            Act::ToggleSounds
        );
        assert_eq!(on_enter(Kind::Start, "", 1, SETTINGS), Act::OpenSettings);
        assert_eq!(on_enter(Kind::Start, "", 1, MODS), Act::Mods);
        assert_eq!(on_enter(Kind::Start, "", 1, IMPORT_SAVES), Act::ImportSaves);
        assert_eq!(
            on_enter(Kind::Settings, "", 0, &main[0]),
            Act::OpenChoices(0)
        );
        assert_eq!(on_escape(Kind::Settings, &main), Act::BackToStart);
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

    /// The crash reports screens: the first says what a report holds and leads to the question,
    /// the example or back (Not now from the start screen, Back from Settings; Esc the same);
    /// the question opens on Never, its answers are kept, its Back returns to the first screen;
    /// the screens open at the start screen only while unanswered and not yet shown in this
    /// start; from Settings, Enter, a click, Left and Right on the row all open them (no press
    /// there answers), and an answer there is written at once ([`ConsentFlow`]).
    #[test]
    fn the_crash_reports_screens_answer_show_and_wait() {
        for from_settings in [false, true] {
            let e = about_entries(from_settings);
            let pick = |entry: &str| {
                let i = e.iter().position(|x| x == entry).unwrap();
                on_enter(Kind::Consent, "", i, entry)
            };
            assert_eq!(pick(CHOOSE_REPORTS), Act::OpenQuestion);
            assert_eq!(pick(SHOW_REPORT), Act::ShowSample);
            assert_eq!(
                pick(if from_settings { BACK } else { NOT_NOW }),
                Act::CloseConsent
            );
            assert_eq!(on_escape(Kind::Consent, &e), Act::CloseConsent);
        }
        let q = question_entries();
        let pick = |entry: &str| {
            let i = q.iter().position(|x| x == entry).unwrap();
            on_enter(Kind::Question, "", i, entry)
        };
        assert_eq!(pick(SEND_REPORTS), Act::Consent(Consent::Send));
        assert_eq!(pick(ASK_REPORTS), Act::Consent(Consent::Ask));
        assert_eq!(pick(NEVER_REPORTS), Act::Consent(Consent::Never));
        assert_eq!(pick(BACK), Act::OpenConsent);
        assert_eq!(on_escape(Kind::Question, &q), Act::OpenConsent);
        let back = vec![BACK.to_string()];
        assert_eq!(on_enter(Kind::Sample, "", 0, BACK), Act::OpenConsent);
        assert_eq!(on_escape(Kind::Sample, &back), Act::OpenConsent);
        // The start-up gate: the refusal first, then the screens while unanswered and not yet
        // asked in this start, else the start screen.
        assert!(asks_consent(Consent::Unanswered, false));
        assert!(!asks_consent(Consent::Unanswered, true));
        for answer in consent::ANSWERS {
            assert!(!asks_consent(answer, false));
            assert_eq!(start_kind(false, answer, false), Kind::Start);
        }
        assert_eq!(start_kind(false, Consent::Unanswered, false), Kind::Consent);
        assert_eq!(start_kind(false, Consent::Unanswered, true), Kind::Start);
        assert_eq!(start_kind(true, Consent::Unanswered, false), Kind::Refused);
        // Settings' row: Enter, Left and Right all open the screens; no press there answers.
        let main = settings_entries(
            settings::List::Main,
            &settings::Files::default(),
            true,
            Consent::Unanswered,
        );
        assert_eq!(main[CRASH_ROW_INDEX], "Crash reports: Not answered");
        assert_eq!(
            on_enter(Kind::Settings, "", CRASH_ROW_INDEX, &main[CRASH_ROW_INDEX]),
            Act::OpenConsentFromSettings
        );
        assert_eq!(
            row_step_act(settings::List::Main, CRASH_ROW_INDEX),
            Some(Act::OpenConsentFromSettings)
        );
        assert_eq!(
            row_step_act(settings::List::Main, settings::ROWS.len()),
            Some(Act::ToggleSounds)
        );
        assert_eq!(row_step_act(settings::List::Main, 0), None);
        for list in SUBLISTS {
            assert_eq!(row_step_act(list, CRASH_ROW_INDEX), None);
        }
        // The question opens on Never: quick Enters never choose sending.
        for can_send in [false, true] {
            let q = question_screen(can_send);
            assert_eq!(q.entries[q.highlighted], NEVER_REPORTS);
        }
        // Settings' answers are written at once; only the first start's waits on Windows.
        assert!(keeps_at_once(true));
        assert_eq!(keeps_at_once(false), cfg!(not(windows)));
        // What a report holds (the F12 report's note and place too), where reports go, that a
        // report sent includes where the player was and the F12 note, that nothing goes before
        // an answer, and where this copy cannot send, that it cannot.
        let about = about_text(None).join(" ");
        assert!(about.contains("an F12 report's note and place from that run"));
        assert!(about.contains("no saves or pictures"));
        assert!(about.contains("Nothing is sent until you choose. Not now asks at the next start."));
        // From Settings: the answer in force, not what Not now does.
        let in_settings = about_text(Some(Consent::Send)).join(" ");
        assert!(in_settings.contains("Now: Send automatically."));
        assert!(!in_settings.contains("Not now"));
        let question = question_text(false).join(" ");
        assert!(question.contains("Sentry (sentry.io)"));
        assert!(question.contains("The report includes where you were and your F12 note."));
        assert!(!question.contains("not where you were"), "{question}");
        assert!(question.contains("Nothing is sent until you choose."));
        assert!(question.contains("cannot send yet"));
        assert!(!question_text(true).join(" ").contains("cannot send"));
    }

    /// The window's state for the screens: at the start screen they open once a start and are
    /// not from Settings; from Settings an answer is written at once and Back returns there;
    /// leaving clears it.
    #[test]
    fn the_crash_reports_flow_opens_once_and_knows_where_from() {
        let mut flow = ConsentFlow::default();
        assert_eq!(flow.at_start(false, Consent::Unanswered), Kind::Consent);
        assert!(!flow.in_settings());
        assert_eq!(flow.answer_now(), cfg!(not(windows)));
        assert!(!flow.close());
        // Not twice in one start.
        assert_eq!(flow.at_start(false, Consent::Unanswered), Kind::Start);
        assert_eq!(flow.at_start(true, Consent::Unanswered), Kind::Refused);
        // From Settings.
        flow.open_in_settings();
        assert!(flow.in_settings());
        assert!(
            flow.answer_now(),
            "an answer from Settings is written at once"
        );
        assert!(flow.close(), "back to Settings");
        assert!(!flow.in_settings());
        assert!(!flow.close());
        // A new start, answered: the start screen.
        let mut fresh = ConsentFlow::default();
        assert_eq!(fresh.at_start(false, Consent::Never), Kind::Start);
        assert_eq!(fresh.at_start(false, Consent::Unanswered), Kind::Consent);
    }

    /// In the launcher's own plain look the window's two buttons fit every screen: inside the
    /// picture, apart, and above the header's lines (the longest titles' too), so above all a
    /// screen shows below them: its text, its list and its prompt.
    #[test]
    fn the_window_buttons_fit_every_screen_of_the_plain_look() {
        let apart = |a: screen::Area, b: screen::Area| {
            a.0 + a.2 as i32 <= b.0
                || b.0 + b.2 as i32 <= a.0
                || a.1 + a.3 as i32 <= b.1
                || b.1 + b.3 as i32 <= a.1
        };
        let look = Look::plain();
        let boxes = screen::buttons(&look, screen::WIDTH).map(|(_, b)| b);
        assert!(apart(boxes[0], boxes[1]));
        for b in boxes {
            assert!(b.0 >= 0 && b.1 >= 0, "{b:?}");
            assert!(b.0 + b.2 as i32 <= screen::WIDTH as i32, "{b:?}");
            assert!(b.2 >= 30 && b.3 >= 30, "big enough to hit: {b:?}");
        }
        let fixes = generated("fixes");
        let mut screens = vec![
            start_screen(&place(Some("/g/x"), true), &Card::Asking),
            start_screen(&place(None, false), &Card::Asking),
            message_screen("A question?", &["Yes".into(), "No".into()]),
            question_screen(false),
            question_screen(true),
        ];
        for consent in [None, Some(Consent::Ask)] {
            screens.push(Screen {
                title: format!("-{CRASH_ROW}-"),
                welcome: about_text(consent),
                entries: about_entries(consent.is_some()),
                ..Default::default()
            });
        }
        for list in [settings::List::Main].into_iter().chain(SUBLISTS) {
            screens.push(list_screen(list, &fixes, true, None, true, Consent::Ask));
            for &row in list.rows() {
                screens.push(choices_screen(row, &fixes, (3840, 2160)));
            }
        }
        for s in &screens {
            let header = screen::header_boxes(&look, s);
            for b in boxes {
                assert!(b.1 + b.3 as i32 <= header[0].1, "{}: {b:?}", s.title);
                for h in header {
                    assert!(apart(b, h), "{}: {b:?} {h:?}", s.title);
                }
            }
        }
    }

    /// What the mod installer (`crates/mod-chooser`) writes for its list, as it wraps it:
    /// `installed` mods installed with nv-rs and `waiting` archives, its question, its prompt.
    fn installer_list(installed: usize, waiting: usize) -> String {
        let mut out = String::from("nv-rs mods\n==========\n\n");
        if installed > 0 {
            out.push_str("Installed with nv-rs:\n");
            for i in 1..=installed {
                out.push_str(&format!("  {i}) Installed mod number {i}\n"));
            }
        }
        if waiting > 0 {
            out.push_str("Waiting in the mods folder, not installed yet:\n");
            for i in 1..=waiting {
                out.push_str(&format!("  {}) Waiting archive {i}.7z\n", installed + i));
            }
        }
        out.push_str(
            "\nType a number and press Enter. A waiting archive's number installs it; an\n    \
             installed mod's number shows what it installed.\nTo remove an installed mod, type \
             remove and its number (like remove 1).\nJust press Enter to finish.\n> ",
        );
        out
    }

    /// Every screen in every window shape players have (21:9 and 32:9 ultrawides, a wide and
    /// short 16:9, a tall portrait screen, the first window's 4:3): the screen whole inside the
    /// window, its 4:3 shape kept and as large as fits, black beside it; every entry that shows
    /// inside the window and hit where it is drawn, the highlighted one among them wherever it is
    /// in a long list; the window's buttons in the window's top-right corner, drawn there. The
    /// Mods screen with more mods than it holds (the mod installer's list, wrapped to the
    /// screen's text width) shows its question at its end, and Page Up reaches every line of it.
    #[test]
    fn every_screen_shows_its_options_in_every_window_shape() {
        use crate::canvas::Canvas;
        const WINDOWS: [(u32, u32); 6] = [
            (3440, 1440),
            (2560, 1080),
            (5120, 1440),
            (1280, 720),
            (1080, 1920),
            (960, 720),
        ];
        let look = Look::plain();
        let fixes = generated("fixes");
        let mut installer = Vec::new();
        append_output(&mut installer, &installer_list(25, 15));
        let mods_asking = output_screen(&installer, false, String::new());
        let mods_ended = output_screen(&installer, true, String::new());
        let mut screens = vec![
            start_screen(&place(Some("/g/x"), true), &Card::Asking),
            start_screen(&place(None, false), &Card::Asking),
            mods_asking.clone(),
            mods_ended.clone(),
            message_screen("A question?", &["Yes".into(), "No".into()]),
            question_screen(false),
            question_screen(true),
        ];
        for consent in [None, Some(Consent::Ask)] {
            screens.push(Screen {
                title: format!("-{CRASH_ROW}-"),
                welcome: about_text(consent),
                entries: about_entries(consent.is_some()),
                ..Default::default()
            });
        }
        let note = Some("The choice could not be saved: the file is read-only.".to_string());
        for list in [settings::List::Main].into_iter().chain(SUBLISTS) {
            screens.push(list_screen(list, &fixes, true, None, true, Consent::Ask));
            screens.push(list_screen(
                list,
                &fixes,
                false,
                note.clone(),
                true,
                Consent::Ask,
            ));
            for &row in list.rows() {
                screens.push(choices_screen(row, &fixes, (5120, 1440)));
            }
        }
        // Each screen with each of its entries highlighted, laid out once (the layout is the
        // screen's own, the same in every window).
        let mut laid = Vec::new();
        for s in &screens {
            for highlighted in 0..s.entries.len().max(1) {
                let s = Screen {
                    highlighted,
                    ..s.clone()
                };
                let rows = screen::rows_of(&look, &s);
                if !s.entries.is_empty() {
                    assert!(rows.shown > 0, "{}", s.title);
                    assert!(
                        (rows.first..rows.first + rows.shown).contains(&highlighted),
                        "{}: entry {highlighted} of {} not shown ({rows:?})",
                        s.title,
                        s.entries.len()
                    );
                }
                let bottom = rows.top + (rows.row * rows.shown as u32) as i32;
                assert!(bottom <= screen::HEIGHT as i32, "{}: {rows:?}", s.title);
                laid.push((s, rows));
            }
        }
        let inside = |window: (u32, u32), (x, y, w, h): (f64, f64, f64, f64)| {
            x >= -0.01
                && y >= -0.01
                && x + w <= window.0 as f64 + 0.01
                && y + h <= window.1 as f64 + 0.01
        };
        for window in WINDOWS {
            let f = gpu::framing(SCREEN, window, 8192);
            let whole = f.screen_area_in_window((0, 0, screen::WIDTH, screen::HEIGHT));
            assert!(inside(window, whole), "{window:?}: {whole:?}");
            assert!(
                (whole.2 / whole.3 - 4.0 / 3.0).abs() < 0.01,
                "{window:?}: {whole:?}"
            );
            assert!(
                whole.2 >= window.0 as f64 - 2.0 || whole.3 >= window.1 as f64 - 2.0,
                "{window:?}: as large as fits, {whole:?}"
            );
            // The buttons: in the window's top-right corner, hit there.
            let scale = f.rect.2 as f64 / f.picture.0 as f64;
            for (b, area) in screen::buttons(&look, f.picture.0) {
                let (x, y, w, h) = f.area_in_window(area);
                assert!(inside(window, (x, y, w, h)), "{window:?} {b:?}");
                assert!(y <= 10.0 * scale + 1.0, "{window:?} {b:?} at {y}");
                assert!(
                    x >= window.0 as f64 - 120.0 * scale,
                    "{window:?} {b:?} at {x}"
                );
                let centre = (x + w / 2.0, y + h / 2.0);
                assert_eq!(button_hit(&f, &look, centre), Some(b), "{window:?}");
            }
            let exit = f.area_in_window(screen::buttons(&look, f.picture.0)[1].1);
            let right = window.0 as f64 - (exit.0 + exit.2);
            assert!(
                right <= 10.0 * scale + 2.0,
                "{window:?}: {right} from the right"
            );
            // Every entry that shows: inside the window, hit at its centre.
            for (s, rows) in &laid {
                for k in 0..rows.shown {
                    let area = (
                        rows.x,
                        rows.top + (rows.row * k as u32) as i32,
                        rows.width,
                        rows.row,
                    );
                    let (x, y, w, h) = f.screen_area_in_window(area);
                    assert!(
                        inside(window, (x, y, w, h)),
                        "{window:?} {}: row {k}",
                        s.title
                    );
                    let hit = row_hit(&f, rows, (x + w / 2.0, y + h / 2.0));
                    assert_eq!(hit, Some(rows.first + k), "{window:?} {}: row {k}", s.title);
                    assert_eq!(button_hit(&f, &look, (x + w / 2.0, y + h / 2.0)), None);
                }
            }
            // Drawn: the screen whole in its place on the window's picture, black around it but
            // for the buttons, which are drawn in the corner.
            for s in [&screens[0], &mods_asking, &laid.last().unwrap().0] {
                let (drawn, _) = screen::draw(&look, s, None, true);
                let mut c = Canvas {
                    width: drawn.width,
                    height: drawn.height,
                    pixels: drawn.pixels.clone(),
                }
                .placed(f.picture, f.screen_at);
                screen::draw_buttons(&mut c, &look, None);
                let px = |c: &Canvas, x: u32, y: u32| {
                    let i = ((y * c.width + x) * 4) as usize;
                    [c.pixels[i], c.pixels[i + 1], c.pixels[i + 2]]
                };
                let boxes = screen::buttons(&look, f.picture.0).map(|(_, b)| b);
                let in_box = |x: u32, y: u32| {
                    boxes.iter().any(|&(bx, by, bw, bh)| {
                        (bx..bx + bw as i32).contains(&(x as i32))
                            && (by..by + bh as i32).contains(&(y as i32))
                    })
                };
                let (ax, ay) = (f.screen_at.0 as u32, f.screen_at.1 as u32);
                for y in (0..c.height).step_by(3) {
                    for x in (0..c.width).step_by(3) {
                        let on_screen = (ax..ax + screen::WIDTH).contains(&x)
                            && (ay..ay + screen::HEIGHT).contains(&y);
                        if on_screen && !in_box(x, y) {
                            assert_eq!(px(&c, x, y), px(&drawn, x - ax, y - ay), "{window:?}");
                        } else if !on_screen && !in_box(x, y) {
                            assert_eq!(px(&c, x, y), [0, 0, 0], "{window:?} at {x},{y}");
                        }
                    }
                }
                for (bx, by, bw, bh) in boxes {
                    let lit = (by..by + bh as i32)
                        .flat_map(|y| (bx..bx + bw as i32).map(move |x| (x, y)))
                        .filter(|&(x, y)| px(&c, x as u32, y as u32) != [0, 0, 0])
                        .count();
                    assert!(lit > 20, "{window:?}: a button drawn in the corner");
                }
            }
        }
        // The Mods screen: more lines than it holds, its end (the question and the installer's
        // prompt) at rest with the line saying more is above; Page Up shows every line.
        let wrapped = screen::welcome_lines(&look, &mods_asking);
        assert!(
            screen::most_scroll(&look, &mods_asking) > 20,
            "the list is long"
        );
        let rest = screen::line_texts(&look, &mods_asking);
        assert_eq!(rest[3], screen::MORE_ABOVE);
        assert!(rest.iter().any(|l| l == "Just press Enter to finish."));
        assert_eq!(
            rest[rest.len() - 2],
            "> ",
            "the installer's own prompt last"
        );
        for s in [mods_asking, mods_ended] {
            let mut s = s;
            let mut seen = std::collections::HashSet::new();
            loop {
                seen.extend(screen::line_texts(&look, &s));
                let next = screen::scrolled(&look, &s, true, 5);
                if next == s.scroll {
                    break;
                }
                s.scroll = next;
            }
            for line in &wrapped {
                assert!(seen.contains(line), "never shown: {line:?}");
            }
            for first_and_last in ["Installed mod number 1", "Waiting archive 15.7z"] {
                assert!(seen.iter().any(|l| l.ends_with(first_and_last)));
            }
            if !s.entries.is_empty() {
                assert_eq!(screen::rows_of(&look, &s).shown, 2, "Back and Exit show");
            }
            // Page Down brings it back to its end.
            while s.scroll > 0 {
                s.scroll = screen::scrolled(&look, &s, false, 5);
            }
            assert_eq!(screen::line_texts(&look, &s)[3], screen::MORE_ABOVE);
        }
    }

    /// In the launcher's own plain look (the game not found yet), every line and every choice of
    /// the crash reports screens shows at once, and the Settings row fits its highlight.
    #[test]
    fn the_crash_reports_screens_fit_the_plain_look() {
        let look = Look::plain();
        let screens = [
            (about_text(None), about_entries(false)),
            (about_text(Some(Consent::Unanswered)), about_entries(true)),
            (about_text(Some(Consent::Send)), about_entries(true)),
            (about_text(Some(Consent::Ask)), about_entries(true)),
            (about_text(Some(Consent::Never)), about_entries(true)),
            (question_text(false), question_entries()),
            (question_text(true), question_entries()),
        ];
        for (welcome, entries) in screens {
            let s = Screen {
                title: format!("-{CRASH_ROW}-"),
                welcome,
                entries,
                ..Default::default()
            };
            assert!(screen::shows_whole(&look, &s), "{:?}", s.welcome);
        }
        for answer in [
            Consent::Unanswered,
            Consent::Send,
            Consent::Ask,
            Consent::Never,
        ] {
            let row = format!("> {}", crash_entry(answer));
            assert!(look.font.width(&row) <= 700 - 5, "{row}");
        }
    }

    /// A fresh profile's files under `preset`: the generated sections (the fix lines notes).
    fn generated(preset: &str) -> settings::Files {
        let text = game_profile::game_ini::with_sections("").unwrap().text;
        settings::Files {
            ini: game_profile::game_ini::set_key(&text, "nv-rs General", "sPreset", preset),
            prefs: String::new(),
        }
    }

    /// In the launcher's own plain look, every entry of the lists Settings opens fits its row's
    /// highlight with each of its choices, as do the entries that open them; each of their rows'
    /// choices screens shows whole (what it is now, why it does nothing when it does nothing,
    /// what it does, every choice), with the rows it needs on or off, its title within the
    /// header's line; each list's own lines show whole, with rows below them.
    #[test]
    fn the_settings_lists_fit_the_plain_look() {
        let look = Look::plain();
        let fits = |entry: &str| look.font.width(&format!("> {entry}")) <= 700 - 5;
        for list in SUBLISTS {
            assert!(fits(list.name()), "{}", list.name());
        }
        let mut checked = 0;
        let parents_off = |f: &settings::Files| {
            ["bUncap", "bJIPInstalled", "bMenuCap"]
                .into_iter()
                .fold(f.clone(), |f, key| {
                    settings::Row::of(key).unwrap().apply(&f, "Off")
                })
        };
        let fixes = generated("fixes");
        let vanilla = generated("vanilla");
        let starts = [parents_off(&fixes), fixes, vanilla];
        for shown in starts {
            for list in SUBLISTS {
                for &row in list.rows() {
                    for choice in row.choices(&shown, (3840, 2160)) {
                        let after = row.apply(&shown, &choice);
                        let entry = row_entry(row, &after);
                        assert!(fits(&entry), "{entry}");
                        let s = choices_screen(row, &after, (3840, 2160));
                        let rows = screen::rows_of(&look, &s);
                        assert_eq!(
                            (rows.first, rows.shown),
                            (0, s.entries.len()),
                            "{}: every choice shows",
                            s.title
                        );
                        assert!(
                            screen::shows_whole(&look, &s),
                            "{}: {:?}",
                            s.title,
                            s.welcome
                        );
                        assert!(look.font.width(&s.title) <= screen::WIDTH, "{}", s.title);
                        let lines = screen::line_texts(&look, &s);
                        assert!(lines[3].starts_with("Now: "), "{}: {lines:?}", s.title);
                        checked += 1;
                    }
                }
                for (in_files, note) in [(true, None), (false, Some("Not saved: x".to_string()))] {
                    let s = list_screen(list, &shown, in_files, note, true, Consent::Ask);
                    let text = screen::welcome_lines(&look, &s).len();
                    assert_eq!(
                        screen::first_entry_line(&look, &s),
                        3 + text,
                        "{:?}",
                        s.title
                    );
                    assert!(screen::rows_of(&look, &s).shown >= 4, "{:?}", s.title);
                }
            }
        }
        assert!(checked > 120, "{checked}");
        // The preset's screen counts the fixes chosen one by one, and still shows whole.
        let uncap = settings::Row::of("bUncap").unwrap();
        let grab = settings::Row::of("bGrabPush").unwrap();
        let two = grab.apply(&uncap.apply(&generated("fixes"), "Off"), "Off");
        let s = choices_screen(settings::Row::GameFixes, &two, (1, 1));
        assert_eq!(
            s.welcome.last().map(String::as_str),
            Some("Kept with either: the 2 chosen in Fixes one by one.")
        );
        assert!(screen::shows_whole(&look, &s), "{:?}", s.welcome);
        let s = choices_screen(settings::Row::GameFixes, &generated("fixes"), (1, 1));
        assert_eq!(s.welcome, ["Now: Fixes (recommended)"]);
        // A fix row's screen: what it is, where that comes from, and what it does.
        let s = choices_screen(uncap, &generated("vanilla"), (1, 1));
        assert_eq!(s.title, "-Uncapped frame rate-");
        assert_eq!(s.welcome[0], "Now: Off, from Game fixes");
        assert_eq!(s.welcome[1], uncap.about().unwrap());
        assert!(!s
            .welcome
            .iter()
            .any(|l| l.contains("bUncap") || l.contains("iFrameLimit")));
        assert_eq!(s.entries, ["Off", "On", BACK]);
        assert_eq!(s.highlighted, 0);
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
            Kind::Sublist,
            Kind::Choices,
            Kind::Consent,
            Kind::Question,
            Kind::Sample,
        ];
        let takes: Vec<Kind> = all.into_iter().filter(|k| takes_typing(*k)).collect();
        assert_eq!(takes, [Kind::Start, Kind::Folder, Kind::Output]);
    }

    /// A paste's answer is added only for the paste asked last, within half a second, on the
    /// same kind of screen; a late one, an older one, or one after the screen changed is dropped.
    #[test]
    fn a_late_or_stale_paste_is_dropped() {
        let t0 = Instant::now();
        let asked = Asked {
            id: 3,
            at: t0,
            kind: Kind::Folder,
            typed: "C:/made".into(),
        };
        let a = Some(&asked);
        let at = |ms| t0 + Duration::from_millis(ms);
        assert!(accept_paste(a, 3, at(20), Kind::Folder, "C:/made"));
        assert!(accept_paste(a, 3, at(500), Kind::Folder, "C:/made"));
        assert!(!accept_paste(a, 3, at(501), Kind::Folder, "C:/made"));
        assert!(!accept_paste(a, 2, at(20), Kind::Folder, "C:/made"));
        assert!(!accept_paste(a, 3, at(20), Kind::Start, "C:/made"));
        // Typed on, or the prompt taken, since the paste was asked.
        assert!(!accept_paste(a, 3, at(20), Kind::Folder, "C:/made/up"));
        assert!(!accept_paste(a, 3, at(20), Kind::Folder, ""));
        assert!(!accept_paste(None, 3, at(20), Kind::Folder, "C:/made"));
    }

    fn pacing_test_screen() -> Screen {
        Screen {
            title: "-made-".into(),
            welcome: vec!["Some made up text".into()],
            entries: vec!["First".into(), "Second".into()],
            ..Default::default()
        }
    }

    /// The first screen types its header; later ones show it at once, and the title and the
    /// welcome text too when they are the ones already shown. The intro counts as no screen.
    #[test]
    fn the_header_types_once_and_an_unchanged_text_is_not_typed_again() {
        let s = pacing_test_screen();
        assert_eq!(shown_at_once(None, &s.title, &s.welcome), AtOnce::default());
        let last = (s.title.clone(), s.welcome.clone());
        let all = AtOnce {
            header: true,
            title: true,
            welcome: true,
        };
        assert_eq!(shown_at_once(Some(&last), &s.title, &s.welcome), all);
        let other = vec!["Other made up text".to_string()];
        assert_eq!(
            shown_at_once(Some(&last), "-other-", &other),
            AtOnce {
                header: true,
                title: false,
                welcome: false
            }
        );
    }

    /// Lines shown at once are whole from the start; the others type in order on the frame
    /// clock, the entries always among them, the prompt last on the first screen only.
    #[test]
    fn typed_lines_skip_what_shows_at_once() {
        let texts: Vec<String> = ["AB", "CD", "-t-", "wel", "> x", "> yz", "> "]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let f = pacing::frame_ms(120);
        let at_once = AtOnce {
            header: true,
            title: true,
            welcome: true,
        };
        // Only the entries type: one character a frame at 120 frames a second; the prompt whole.
        assert_eq!(
            typed_lines(&texts, 4, at_once, f, 0.0),
            [2, 2, 3, 3, 1, 0, 2]
        );
        assert_eq!(
            typed_lines(&texts, 4, at_once, f, 2.0 * f),
            [2, 2, 3, 3, 3, 0, 2]
        );
        // The next line starts the frame after.
        assert_eq!(
            typed_lines(&texts, 4, at_once, f, 3.0 * f),
            [2, 2, 3, 3, 3, 1, 2]
        );
        // Nothing at once (the first screen): the header types first, the prompt last.
        assert_eq!(
            typed_lines(&texts, 4, AtOnce::default(), f, 0.0),
            [1, 0, 0, 0, 0, 0, 0]
        );
        let first = AtOnce::default();
        // 2 + 2 + 3 + 3 + 3 + 4 characters, one a frame, and a frame between lines (6 starts).
        let before_prompt = typed_lines(&texts, 4, first, f, 16.0 * f);
        assert_eq!(before_prompt, [2, 2, 3, 3, 3, 4, 0]);
        assert_eq!(
            typed_lines(&texts, 4, first, f, 17.0 * f),
            [2, 2, 3, 3, 3, 4, 1]
        );
        // A title that changed types although the header does not.
        let title_only = AtOnce {
            header: true,
            title: false,
            welcome: true,
        };
        assert_eq!(
            typed_lines(&texts, 4, title_only, f, 0.0),
            [2, 2, 1, 3, 0, 0, 2]
        );
    }

    /// The focus wraps among the entries; Enter chooses an entry only once they are live (a typed
    /// line always goes).
    #[test]
    fn the_focus_wraps_and_enter_waits_for_live_entries() {
        assert_eq!(step_focus(0, 0, false), 0);
        assert_eq!(step_focus(0, 0, true), 0);
        assert_eq!(step_focus(0, 1, false), 0);
        assert_eq!(step_focus(0, 2, false), 1);
        assert_eq!(step_focus(1, 2, false), 0);
        assert_eq!(step_focus(0, 3, true), 2);
        assert_eq!(step_focus(5, 2, false), 0);
        assert_eq!(step_focus(4, 3, true), 1);
        assert!(!may_choose("", true, false));
        assert!(may_choose("", true, true));
        assert!(may_choose("C:/made", true, false));
        assert!(may_choose("", false, false));
    }

    /// The entries show as their lines type but are live (highlighted, chosen) only once every
    /// line of the screen is in, the prompt too, as the game enables its tiles when its text
    /// queue is empty.
    #[test]
    fn entries_are_live_only_once_the_whole_screen_is_in() {
        let look = Look::plain();
        let s = pacing_test_screen();
        let first = screen::first_entry_line(&look, &s);
        let texts = screen::line_texts(&look, &s);
        assert_eq!(texts.last().map(String::as_str), Some(screen::PROMPT));
        let whole: Vec<usize> = texts.iter().map(|t| t.chars().count()).collect();
        assert!(screen::entries_live(&look, &s, Some(&whole)));
        assert!(screen::entries_live(&look, &s, None));
        let mut shown = whole.clone();
        shown[first] = 1;
        assert!(!screen::entries_live(&look, &s, Some(&shown)));
        // The entries all in, the prompt not yet.
        let mut shown = whole.clone();
        *shown.last_mut().unwrap() = 1;
        assert!(!screen::entries_live(&look, &s, Some(&shown)));
        assert!(!screen::entries_live(
            &look,
            &s,
            Some(&whole[..whole.len() - 1])
        ));
    }

    /// The menu frame limit row's values as frames a second, a hand-set huge one held at
    /// [`MOST_FRAMES`] (the model steps through every frame).
    #[test]
    fn the_menu_frame_limit_reads_as_frames_a_second() {
        assert_eq!(frame_limit_of("No limit"), 0);
        assert_eq!(frame_limit_of("0"), 0);
        assert_eq!(frame_limit_of("144"), 144);
        assert_eq!(frame_limit_of("120"), 120);
        assert_eq!(frame_limit_of("odd"), 120);
        assert_eq!(frame_limit_of("4294967295"), MOST_FRAMES);
        assert_eq!(frame_limit_of("1001"), MOST_FRAMES);
        assert_eq!(frame_limit_of("1000"), 1000);
        // The typing model with the largest limit still answers at once for a long screen.
        let lines = [pacing::Line {
            chars: 400,
            ms: 6,
            intro: false,
            keyed: false,
        }];
        let f = pacing::frame_ms(frame_limit_of("4294967295"));
        assert_eq!(pacing::typed(&lines, f, 60_000.0).0, [400]);
    }

    /// The key handler's decision: while a screen types, the first press of a key (not a
    /// modifier) only finishes the typing (the launcher's shortcut), and no key that acts does
    /// anything else; held keys' repeats never choose or go back (Enter, Esc), while a held Left or
    /// Right steps on once the screen is whole; the key that finished the typing does nothing more
    /// while it is held; a whole screen takes every key.
    #[test]
    fn while_typing_a_press_only_finishes_it_and_repeats_never_act() {
        use Press::*;
        for p in [
            Escape, Enter, Left, Right, Up, Down, Click, Other, Backspace,
        ] {
            assert_eq!(
                key_act(p, false, true, false),
                KeyAct::FinishTyping,
                "{p:?}"
            );
        }
        assert_eq!(key_act(Modifier, false, true, false), KeyAct::Nothing);
        // A held key's repeat during typing: Esc, Enter, Left/Right and the focus do nothing.
        for p in [Escape, Enter, Left, Right, Up, Down, Click] {
            assert_eq!(key_act(p, true, true, false), KeyAct::Nothing, "{p:?}");
        }
        // Esc held past the intro: its repeats on the start screen never give Exit, typed or not.
        assert_eq!(key_act(Escape, true, false, false), KeyAct::Nothing);
        assert_eq!(key_act(Enter, true, false, false), KeyAct::Nothing);
        assert_eq!(key_act(Left, true, false, false), KeyAct::Act(Left));
        assert_eq!(key_act(Right, true, false, false), KeyAct::Act(Right));
        // Repeats that only move or type still do once the screen is whole.
        assert_eq!(key_act(Down, true, false, false), KeyAct::Act(Down));
        assert_eq!(
            key_act(Backspace, true, false, false),
            KeyAct::Act(Backspace)
        );
        assert_eq!(key_act(Other, true, false, false), KeyAct::Act(Other));
        for p in [Escape, Enter, Left, Right, Up, Down, Click] {
            assert_eq!(key_act(p, false, false, false), KeyAct::Act(p), "{p:?}");
        }
        // The held key that finished the typing: its repeats do nothing until it is let go, even
        // those that would act on the whole screen; a new press of it acts.
        for p in [Left, Right, Down, Other, Backspace] {
            assert_eq!(key_act(p, true, false, true), KeyAct::Nothing, "{p:?}");
        }
        assert_eq!(key_act(Right, false, false, true), KeyAct::Act(Right));
    }

    /// winit's keys as the handler tells them apart: modifiers alone never finish the typing.
    #[test]
    fn keys_are_told_apart_as_the_handler_needs() {
        use winit::keyboard::{Key, NamedKey as N};
        assert_eq!(press_of(&Key::Named(N::Escape)), Press::Escape);
        assert_eq!(press_of(&Key::Named(N::Enter)), Press::Enter);
        assert_eq!(press_of(&Key::Named(N::Tab)), Press::Down);
        assert_eq!(press_of(&Key::Named(N::ArrowLeft)), Press::Left);
        assert_eq!(press_of(&Key::Named(N::Control)), Press::Modifier);
        assert_eq!(press_of(&Key::Named(N::Shift)), Press::Modifier);
        assert_eq!(press_of(&Key::Character("v".into())), Press::Other);
    }

    /// Without the launcher's shortcut (the game's rule alone) the keys that act wait while a
    /// screen types, and typing at the prompt goes on.
    #[test]
    fn the_games_rule_alone_holds_the_keys_that_act_while_typing() {
        use Press::*;
        for p in [Escape, Enter, Left, Right, Up, Down, Click] {
            assert_eq!(gated(p, false, true), KeyAct::Nothing, "{p:?}");
        }
        assert_eq!(gated(Other, false, true), KeyAct::Act(Other));
        assert_eq!(gated(Backspace, true, true), KeyAct::Act(Backspace));
        assert!(press_finishes_typing(Enter, false, true));
        assert!(!press_finishes_typing(Enter, true, true));
        assert!(!press_finishes_typing(Enter, false, false));
        assert!(!press_finishes_typing(Modifier, false, true));
    }

    /// A paste asked at one prompt never lands in the next prompt of the same kind: once Enter,
    /// Esc or another screen has taken the prompt ([`Pastes::forget`]) its answer is dropped, even
    /// when the new prompt is as empty as the old one was; a paste's own answer ends it, and
    /// only the paste asked last is added.
    #[test]
    fn a_paste_is_only_for_the_prompt_it_was_asked_at() {
        let t0 = Instant::now();
        let soon = t0 + Duration::from_millis(30);
        let mut p = Pastes::default();
        let id = p.ask(t0, Kind::Output, "");
        assert!(p.answer(id, soon, Kind::Output, ""));
        // Its answer came: a second copy of it is not added again.
        assert!(!p.answer(id, soon, Kind::Output, ""));

        let id = p.ask(t0, Kind::Output, "");
        p.forget();
        assert!(!p.answer(id, soon, Kind::Output, ""));

        // Two pastes: the older one's answer is dropped and leaves the newer one waiting.
        let old = p.ask(t0, Kind::Folder, "x");
        let new = p.ask(t0, Kind::Folder, "x");
        assert_ne!(old, new);
        assert!(!p.answer(old, soon, Kind::Folder, "x"));
        assert!(p.answer(new, soon, Kind::Folder, "x"));

        // Typed on before the answer came: dropped.
        let id = p.ask(t0, Kind::Start, "ab");
        assert!(!p.answer(id, soon, Kind::Start, "abc"));
    }

    /// A message keeps its paragraphs as lines, its entries as given.
    #[test]
    fn a_message_keeps_its_paragraphs() {
        let s = message_screen("a\n\nb", &["Yes".into(), "No".into()]);
        assert_eq!(s.welcome, ["a", "", "b"]);
        assert_eq!(s.entries, ["Yes", "No"]);
    }

    /// Enter at play.sh's empty output prompt types its (empty) line: the prompt is taken, so a
    /// paste asked there just before is dropped when it comes (REVIEW-822); only doing nothing
    /// keeps the prompt.
    #[test]
    fn a_paste_asked_before_enter_at_an_empty_prompt_is_dropped() {
        let t0 = Instant::now();
        let mut p = Pastes::default();
        let id = p.ask(t0, Kind::Output, "");
        let act = on_enter(Kind::Output, "", 0, "");
        assert_eq!(act, Act::TypeLine(String::new()));
        assert!(!prompt_kept(&act));
        // What `choose` does with it.
        if !prompt_kept(&act) {
            p.forget();
        }
        assert!(!p.answer(id, t0 + Duration::from_millis(5), Kind::Output, ""));
        assert!(prompt_kept(&Act::Nothing));
        assert!(!prompt_kept(&Act::BackToStart));
    }

    /// Choosing an entry plays its tile's click and the terminal's "enter"; a typed line, or no
    /// entry, plays nothing.
    #[test]
    fn choosing_an_entry_clicks_and_enters() {
        use crate::sound_files::Cue;
        assert_eq!(choice_sounds("", Some("Play")), [Cue::Ok, Cue::Enter]);
        assert_eq!(choice_sounds("  ", Some("Play")), [Cue::Ok, Cue::Enter]);
        assert!(choice_sounds("C:/made/up", Some("Play")).is_empty());
        assert!(choice_sounds("", None).is_empty());
    }

    /// The screen's sound queue is the lines that type: on a later screen the header, title,
    /// welcome text shown before and the prompt are not in it, the entries are.
    #[test]
    fn the_sound_queue_is_the_lines_that_type() {
        let texts: Vec<String> = ["AB", "CD", "-t-", "wel", "> x", "> "]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let later = AtOnce {
            header: true,
            title: true,
            welcome: true,
        };
        let q = screen_queue(&texts, 4, later);
        assert_eq!(q.iter().map(|l| l.chars).collect::<Vec<_>>(), [3]);
        assert!(q.iter().all(|l| !l.intro && !l.keyed));
        assert_eq!(screen_queue(&texts, 4, AtOnce::default()).len(), 6);
    }
}
