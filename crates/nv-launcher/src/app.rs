//! The launcher's window: the terminal's screens, typed in a character at a time as the game types
//! them, the highlight moved with the arrow keys or the mouse, Enter or a click to choose; what is
//! typed goes to the prompt. Play runs on a thread of its own and asks the window through
//! [`crate::session`]; the window steps aside while the game plays and comes back when it ends
//! badly (a good end closes the launcher).

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

/// The game's card as far as it is known.
#[derive(Clone, Debug, PartialEq)]
pub enum Card {
    Asking,
    Known(Option<(String, String)>),
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
            "Your game was not found yet. Type its folder below, or drop the folder onto this \
             window, or choose Play to look for it."
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
                }
            });
        }
    }
    entries.push(PLAY.to_string());
    if place.mods {
        entries.push(PLAY_WITH_MODS.to_string());
    }
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

/// What the window shows now.
enum Mode {
    /// The game's intro (the first start only), started at the instant.
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
    /// The window could not be made or drawn in ([`cannot_draw`] said why).
    no_window: bool,
    exit_code: i32,
}

/// Starts Play's work on a thread of its own with `args` (Play.exe's, or play.sh's on Linux and
/// macOS); it sends `Ended` with its exit code.
fn start_work(args: Vec<OsString>) {
    std::thread::spawn(move || {
        #[cfg(windows)]
        let code = {
            let args: Vec<String> = args
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            crate::play::run_play(&args)
        };
        #[cfg(not(windows))]
        let code = match crate::play_unix::script() {
            Some(script) => crate::play_unix::run(&script, &args),
            None => 127,
        };
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
/// [`CARD_LIMIT`].
fn ask_game_card() -> Option<(String, String)> {
    use std::io::Read;
    let exe = std::env::current_exe().ok()?;
    let mut child = std::process::Command::new(exe)
        .arg("--game-card")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let mut out = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < CARD_LIMIT => {
                std::thread::sleep(Duration::from_millis(25))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    card_from_line(&reader.join().ok()?)
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

    /// Shows `screen`, typed in from its start (`typed`) or whole at once.
    fn show(&mut self, mode: Mode, screen: Screen, typed: bool) {
        self.mode = mode;
        self.screen = screen;
        self.started = Instant::now();
        self.finished = !typed;
        if self.hidden {
            if let Some(w) = &self.window {
                w.set_visible(true);
                w.focus_window();
            }
            self.hidden = false;
        }
        self.request_redraw();
    }

    fn request_redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn show_start(&mut self) {
        if let Some(why) = self.place.refused.clone() {
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
        let waiting = Screen {
            title: "-nv-rs-".into(),
            welcome: vec!["Getting ready...".into()],
            ..Default::default()
        };
        self.show(Mode::Waiting, waiting, false);
        start_work(args);
    }

    fn redraw(&mut self) {
        if self.hidden {
            return;
        }
        let shown = self.shown();
        let cursor =
            (self.started.elapsed().as_millis() as u64 / screen::BLINK_MS).is_multiple_of(2);
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
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                surface.configure(&gpu.device, config);
                return;
            }
            _ => return,
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

    /// The intro is over (or skipped): it is noted in the launcher's own file, then the start
    /// screen is typed (`FinishIntro`).
    fn end_intro(&mut self) {
        if self.place.refused.is_none() {
            if let Err(e) = place::note_intro_shown(&self.place.own) {
                eprintln!("launcher.ini: {e}");
            }
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
    fn takes_typing(&self) -> bool {
        matches!(
            self.mode,
            Mode::Start | Mode::Folder(_) | Mode::Output(None)
        )
    }

    /// The highlighted entry chosen (or, with something typed, the typed line).
    fn choose(&mut self, event_loop: &ActiveEventLoop) {
        let typed = std::mem::take(&mut self.screen.typed);
        let chosen = self.screen.highlighted;
        let entry = self.screen.entries.get(chosen).cloned().unwrap_or_default();
        match &mut self.mode {
            Mode::Intro(_) => self.end_intro(),
            Mode::Start if !typed.trim().is_empty() => {
                self.play(false, Some(session::typed_folder(&typed)))
            }
            Mode::Start if entry == PLAY => self.play(false, None),
            Mode::Start if entry == PLAY_WITH_MODS => self.play(true, None),
            Mode::Start | Mode::Refused if entry == EXIT => event_loop.exit(),
            Mode::Start | Mode::Refused | Mode::Waiting => {}
            Mode::Message(reply) => {
                if let Some(r) = reply.take() {
                    let _ = r.send(chosen);
                }
                self.back_to_waiting();
            }
            Mode::Folder(reply) => {
                let answer = (!typed.trim().is_empty()).then(|| session::typed_folder(&typed));
                if let Some(r) = reply.take() {
                    let _ = r.send(answer);
                }
                self.back_to_waiting();
            }
            Mode::Output(None) => {
                crate::play_unix::type_line(&typed);
                self.add_output(&format!("{typed}\n"));
            }
            Mode::Output(Some(_)) if entry == EXIT => event_loop.exit(),
            Mode::Output(Some(_)) => self.show_start(),
        }
    }

    /// Esc: the screen's way back (its Back or No entry, else its last one).
    fn back(&mut self, event_loop: &ActiveEventLoop) {
        if self.screen.entries.is_empty() || matches!(self.mode, Mode::Waiting) {
            return;
        }
        if let Some(i) = back_entry(&self.screen.entries) {
            self.screen.highlighted = i;
            self.choose(event_loop);
        }
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
        self.mode = Mode::Output(ended);
        self.screen = s;
        self.finished = true;
        if self.hidden && ended.is_some() {
            self.show(Mode::Output(ended), self.screen.clone(), false);
        }
        self.request_redraw();
    }
}

impl winit::application::ApplicationHandler<Event> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = winit::window::Window::default_attributes()
            .with_title("nv-rs")
            .with_inner_size(winit::dpi::LogicalSize::new(screen::WIDTH, screen::HEIGHT))
            .with_min_inner_size(winit::dpi::LogicalSize::new(
                screen::WIDTH / 2,
                screen::HEIGHT / 2,
            ));
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                cannot_draw(&format!("the window could not be made: {e}"));
                self.no_window = true;
                event_loop.exit();
                return;
            }
        };
        let (adapter, surface) = match gpu::choose(Some(window.clone())) {
            Ok((instance, adapter, Some(surface))) => {
                self.instance = Some(instance);
                (adapter, surface)
            }
            Ok(_) => unreachable!("a window always gets a surface"),
            Err(e) => {
                cannot_draw(&e);
                self.no_window = true;
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        let g = match Gpu::new(adapter, (screen::WIDTH, screen::HEIGHT)) {
            Ok(g) => g,
            Err(e) => {
                cannot_draw(&e);
                self.no_window = true;
                event_loop.exit();
                return;
            }
        };
        let Some(mut config) =
            surface.get_default_config(&g.adapter, size.width.max(1), size.height.max(1))
        else {
            cannot_draw(&format!(
                "the window's drawing surface does not work with {}",
                g.describe()
            ));
            self.no_window = true;
            event_loop.exit();
            return;
        };
        // The game's menus blend in the stored values: a format without the sRGB step keeps them.
        let caps = surface.get_capabilities(&g.adapter);
        if let Some(f) = caps.formats.iter().find(|f| !f.is_srgb()) {
            config.format = *f;
        }
        surface.configure(&g.device, &config);
        eprintln!("drawing with {}; look: {}", g.describe(), self.look.from);
        self.config = Some(config);
        self.surface = Some(surface);
        self.gpu = Some(g);
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
            Event::GameShown => {
                if let Some(w) = &self.window {
                    w.set_visible(false);
                }
                self.hidden = true;
            }
            Event::Message {
                text,
                entries,
                reply,
            } => {
                let s = message_screen(&text, &entries);
                self.show(Mode::Message(Some(reply)), s, true);
            }
            Event::Folder { text, reply } => {
                let mut s = message_screen(&text, &["Back".to_string()]);
                s.welcome.push(
                    "Type the folder below and press Enter, or drop it onto this window.".into(),
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
                if code == 0 {
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
            Event::Card(card) => {
                self.card = Card::Known(card);
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
            // The work behind the window (Play's steps) ends with it; a game already started
            // plays on.
            WindowEvent::CloseRequested => std::process::exit(self.exit_code),
            WindowEvent::Resized(size) => {
                if let (Some(surface), Some(config), Some(g)) =
                    (&self.surface, &mut self.config, &self.gpu)
                {
                    config.width = size.width.max(1);
                    config.height = size.height.max(1);
                    surface.configure(&g.device, config);
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::DroppedFile(path) => match &mut self.mode {
                Mode::Start => self.play(false, Some(path)),
                Mode::Folder(reply) => {
                    if let Some(r) = reply.take() {
                        let _ = r.send(Some(path));
                    }
                    self.back_to_waiting();
                }
                Mode::Output(None) => {
                    self.screen.typed = path.to_string_lossy().into_owned();
                    self.request_redraw();
                }
                _ => {}
            },
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
                    Key::Named(NamedKey::Enter) => self.choose(event_loop),
                    Key::Named(NamedKey::Backspace) => {
                        self.screen.typed.pop();
                    }
                    Key::Named(NamedKey::Escape) => self.back(event_loop),
                    _ => {
                        if self.takes_typing() {
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
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
            return;
        }
        // Typing: a frame every character (the menus' 6 ms) until the text is in; then only the
        // cursor's blink.
        let typing = !self.finished || matches!(self.mode, Mode::Intro(_));
        let next = if typing {
            Instant::now()
                + Duration::from_millis(crate::intro::ms_per_char(crate::intro::DISPLAY_RATE_MENUS))
        } else {
            let ms = self.started.elapsed().as_millis() as u64;
            Instant::now() + Duration::from_millis(screen::BLINK_MS - ms % screen::BLINK_MS)
        };
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(next));
        self.request_redraw();
    }
}

/// Nothing can draw the launcher's window: said where the player can read it (a message box of
/// Windows', else the terminal it was started from), with the way around it.
fn cannot_draw(why: &str) {
    let text = format!(
        "nv-rs's launcher cannot draw its window: {why}.\n\n{}",
        if cfg!(windows) {
            "Update the graphics driver and start Play.exe again; Play.exe --check names what it finds."
        } else {
            "It goes on in the terminal instead: play.sh, as ./play.sh starts it."
        }
    );
    eprintln!("{text}");
    #[cfg(windows)]
    {
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
        // SAFETY: both strings live through the call; no owner window. 0x10: MB_ICONERROR.
        unsafe { MessageBoxW(std::ptr::null_mut(), t.as_ptr(), title.as_ptr(), 0x10) };
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

/// No window: Windows has said so in its message box; Linux and macOS go on in the terminal
/// (play.sh with the same arguments).
fn without_window(raw: &[String]) -> i32 {
    if cfg!(windows) {
        1
    } else {
        crate::without_window(raw)
    }
}

/// The window, until the player leaves or the game ends well: the exit code.
pub fn run(raw: Vec<String>) -> i32 {
    #[cfg(all(not(windows), not(target_vendor = "apple")))]
    if let Some(library) = missing_keyboard_library() {
        cannot_draw(&format!(
            "the keyboard library {library} is missing (install libxkbcommon)"
        ));
        return without_window(&raw);
    }
    let event_loop = match EventLoop::<Event>::with_user_event().build() {
        Ok(l) => l,
        Err(e) => {
            cannot_draw(&format!("no window system: {e}"));
            return without_window(&raw);
        }
    };
    let place = place::place();
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
        no_window: false,
        exit_code: 0,
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
    let (_, adapter, _) = gpu::choose(None)?;
    let picture = (screen::WIDTH, screen::HEIGHT);
    let mut g = Gpu::new(adapter, picture)?;
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
        assert_eq!(s.entries, [PLAY, EXIT]);
        assert_eq!(s.welcome[0], "Game: Fallout New Vegas");
        let s = start_screen(&place(Some("/g/x"), true), &Card::Asking);
        assert_eq!(s.entries, [PLAY, PLAY_WITH_MODS, EXIT]);
        let s = start_screen(&place(None, false), &Card::Asking);
        assert!(s.welcome[0].contains("not found"), "{:?}", s.welcome);
        assert_eq!(s.welcome.len(), 1);
    }

    /// The renderer line names the game's renderer and card, never the launcher's own device: the
    /// card asked for once known, the adapter table's card on the real Direct3D 9.
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

    /// `--game-card`'s line is read back; an empty or broken one is no card.
    #[test]
    fn the_card_line_is_read_back() {
        assert_eq!(
            card_from_line("NVIDIA GeForce RTX 5070 Ti Laptop GPU\tVulkan\n"),
            Some((
                "NVIDIA GeForce RTX 5070 Ti Laptop GPU".into(),
                "Vulkan".into()
            ))
        );
        assert_eq!(card_from_line("\n"), None);
        assert_eq!(card_from_line(""), None);
        assert_eq!(card_from_line("\tVulkan"), None);
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
        assert_eq!(back_entry(&e(&["Back", "Exit"])), Some(0));
        assert_eq!(back_entry(&e(&["Open the logs folder", "Back"])), Some(1));
        assert_eq!(back_entry(&e(&["Play", "Exit"])), Some(1));
        assert_eq!(back_entry(&e(&["OK"])), Some(0));
        assert_eq!(back_entry(&[]), None);
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

    /// A message keeps its paragraphs as lines, its entries as given.
    #[test]
    fn a_message_keeps_its_paragraphs() {
        let s = message_screen("a\n\nb", &["Yes".into(), "No".into()]);
        assert_eq!(s.welcome, ["a", "", "b"]);
        assert_eq!(s.entries, ["Yes", "No"]);
    }
}
