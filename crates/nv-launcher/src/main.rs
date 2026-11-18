//! nv-launcher, phase 1 prototype: the start screen of nv-rs's one launcher, in the look of the
//! game's own terminal (read from the player's installed game; the port's own plain look before
//! the game is found).
//!
//!   nv-launcher --game FOLDER                      the start screen in a window
//!   nv-launcher --game FOLDER --capture FILE.png   drawn off screen and saved, then it ends
//!   --size WIDTHxHEIGHT                            the capture's size (960x720)
//!   --plain                                        the port's own look, as before the game is found
//!   --settings                                     the Settings screen instead (today's 12 rows)
//!   --profile FALLOUT.INI                          the play profile's file the screens read
//!                                                  (none: every row shows the host's default)
//!
//! Silent: it plays no sound. Nothing is written but the capture.

mod canvas;
mod font;
mod game;
mod gpu;
mod png;
mod screen;
// The Settings screen's rows (UI-free; the window does not use them yet).
#[allow(dead_code)]
mod settings;

use screen::{Look, Screen};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Options {
    game: Option<PathBuf>,
    /// The play profile's Fallout.ini (the renderer it chooses); none: the host table's default.
    profile: Option<PathBuf>,
    capture: Option<PathBuf>,
    size: (u32, u32),
    plain: bool,
    settings: bool,
}

fn options() -> Result<Options, String> {
    let mut o = Options {
        game: std::env::var_os("NV_RS_GAME").map(PathBuf::from),
        profile: None,
        capture: None,
        size: (screen::WIDTH, screen::HEIGHT),
        plain: false,
        settings: false,
    };
    let mut args = std::env::args_os().skip(1);
    while let Some(a) = args.next() {
        match a.to_str() {
            Some("--game") => o.game = Some(args.next().ok_or("--game needs a folder")?.into()),
            Some("--profile") => {
                o.profile = Some(args.next().ok_or("--profile needs a Fallout.ini")?.into())
            }
            Some("--capture") => {
                o.capture = Some(args.next().ok_or("--capture needs a file")?.into())
            }
            Some("--size") => {
                let s = args.next().ok_or("--size needs WIDTHxHEIGHT")?;
                let s = s.to_string_lossy();
                let (w, h) = s.split_once('x').ok_or("--size needs WIDTHxHEIGHT")?;
                o.size = (
                    w.parse().map_err(|_| "--size: not a number")?,
                    h.parse().map_err(|_| "--size: not a number")?,
                );
            }
            Some("--plain") => o.plain = true,
            Some("--settings") => o.settings = true,
            _ => return Err(format!("unknown argument {}", a.to_string_lossy())),
        }
    }
    Ok(o)
}

/// The look: the game's terminal when the game is there and every piece reads, else the plain one
/// (with the reason, for the log).
fn look(o: &Options) -> Look {
    if o.plain {
        return Look::plain();
    }
    let Some(folder) = &o.game else {
        eprintln!("look: no game named (--game): the port's own look");
        return Look::plain();
    };
    let Some(g) = game::Game::at(folder) else {
        eprintln!("look: no game at {}: the port's own look", folder.display());
        return Look::plain();
    };
    match Look::from_game(&g, &[]) {
        Ok(l) => l,
        Err(why) => {
            eprintln!("look: {why}: the port's own look");
            Look::plain()
        }
    }
}

/// The play profile's files as the screens read them: `--profile`'s Fallout.ini (and the
/// FalloutPrefs.ini beside it), else none (every row then shows the host table's default).
fn profile_files(o: &Options) -> settings::Files {
    let read = |p: &std::path::Path| {
        std::fs::read(p)
            .map(|b| b.iter().map(|&c| c as char).collect())
            .unwrap_or_default()
    };
    match &o.profile {
        Some(ini) => settings::Files {
            ini: read(ini),
            prefs: read(&ini.with_file_name("FalloutPrefs.ini")),
        },
        None => settings::Files::default(),
    }
}

/// A choice's name without its note: "Vulkan (recommended)" is "Vulkan" in a list.
fn short(choice: &str) -> &str {
    choice.split(" (").next().unwrap_or(choice)
}

/// The start screen. The status names what the GAME will use: the renderer its profile chooses
/// and, on Vulkan, the card and API the game's own rule picks ([`gpu::game_card`]), not the
/// launcher's own drawing device.
fn start_screen(o: &Options, card: Option<(String, wgpu::Backend)>) -> Screen {
    let game = match &o.game {
        Some(f) if !o.plain => f
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        _ => String::new(),
    };
    let mut welcome = Vec::new();
    if game.is_empty() {
        welcome.push(
            "Your game was not found yet. Type its folder below, or drop the folder onto this \
             window."
                .to_string(),
        );
    } else {
        welcome.push(format!("Game: {game}"));
        let renderer = settings::Row::Renderer.current(&profile_files(o));
        if renderer.starts_with("Vulkan") {
            // One line, wrapped at 600, so the list keeps its six rows.
            match card {
                Some((name, wgpu::Backend::Dx12)) => {
                    welcome.push(format!("Renderer: Direct3D 12, {name}"))
                }
                Some((name, _)) => welcome.push(format!("Renderer: Vulkan, {name}")),
                // No card for wgpu: on Windows the game takes the real Direct3D 9 by itself.
                None if cfg!(windows) => welcome.push("Renderer: Direct3D 9 (no Vulkan)".into()),
                None => welcome.push("Renderer: no graphics card found".into()),
            }
        } else {
            // The real Direct3D 9's card is the one Windows' adapter table names (Play.exe's
            // tables); the prototype does not read it.
            welcome.push("Renderer: Direct3D 9".into());
        }
    }
    let entries = if game.is_empty() {
        vec!["Look for the game again", "Help", "Exit"]
    } else {
        vec!["Play", "Mods", "Settings", "Import saves", "Help", "Exit"]
    };
    Screen {
        title: "-nv-rs-".into(),
        welcome,
        entries: entries.into_iter().map(String::from).collect(),
        highlighted: 0,
        typed: String::new(),
    }
}

/// The screen the prototype opens on: the start screen, or (`--settings`) the Settings screen:
/// today's 12 rows with the profile's values ([`settings::ROWS`]), the Testing list, Back.
fn first_screen(o: &Options, card: Option<(String, wgpu::Backend)>) -> Screen {
    if !o.settings {
        return start_screen(o, card);
    }
    let files = profile_files(o);
    let mut entries: Vec<String> = settings::ROWS
        .iter()
        .map(|r| format!("{}: {}", r.label(), short(&r.current(&files))))
        .collect();
    entries.push("Testing".into());
    entries.push("Back".into());
    Screen {
        title: "-Settings-".into(),
        welcome: vec!["Saved at once, for both ways of playing. Enter: choices.".into()],
        entries,
        highlighted: 0,
        typed: String::new(),
    }
}

fn capture(o: &Options, file: &std::path::Path) -> Result<(), String> {
    let look = look(o);
    let instance = gpu::instance();
    let picture = (screen::WIDTH, screen::HEIGHT);
    let mut g = gpu::Gpu::new(&instance, None, picture)?;
    let s = first_screen(o, gpu::game_card());
    let (canvas, _) = screen::draw(&look, &s, usize::MAX, true);
    g.upload(&canvas.pixels, picture);
    let rgba = g.capture(picture, o.size)?;
    std::fs::write(file, png::encode(o.size.0, o.size.1, &rgba))
        .map_err(|e| format!("{}: {e}", file.display()))?;
    println!(
        "captured {}x{} to {} with {}; look: {}",
        o.size.0,
        o.size.1,
        file.display(),
        g.describe(),
        look.from
    );
    Ok(())
}

/// The window: the start screen typed in, the highlight moved with the arrow keys or the mouse,
/// Enter says the entry chosen (the prototype does nothing else), Esc or Exit ends it.
struct App {
    look: Look,
    options: Options,
    instance: wgpu::Instance,
    window: Option<Arc<winit::window::Window>>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    gpu: Option<gpu::Gpu>,
    screen: Option<Screen>,
    started: Instant,
    finished: bool,
    rows: Option<screen::Rows>,
    mouse: (f64, f64),
}

impl App {
    fn shown(&self) -> usize {
        if self.finished {
            usize::MAX
        } else {
            (self.started.elapsed().as_secs_f32() * screen::TYPE_RATE) as usize
        }
    }

    fn redraw(&mut self) {
        let shown = self.shown();
        let cursor =
            (self.started.elapsed().as_millis() as u64 / screen::BLINK_MS).is_multiple_of(2);
        let (Some(surface), Some(config), Some(gpu), Some(s)) =
            (&self.surface, &self.config, &mut self.gpu, &self.screen)
        else {
            return;
        };
        let (canvas, rows) = screen::draw(&self.look, s, shown, cursor);
        self.rows = Some(rows);
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
        if !self.finished && shown >= screen::total_chars(&self.look, s) {
            self.finished = true;
        }
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

    fn choose(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let Some(s) = &self.screen else { return };
        let entry = &s.entries[s.highlighted];
        println!("chosen: {entry}");
        if entry == "Exit" {
            event_loop.exit();
        }
    }
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
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
                eprintln!("the window could not be made: {e}");
                event_loop.exit();
                return;
            }
        };
        let surface = match self.instance.create_surface(window.clone()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("the window cannot be drawn in: {e}");
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        let g = match gpu::Gpu::new(
            &self.instance,
            Some(&surface),
            (screen::WIDTH, screen::HEIGHT),
        ) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("{e}");
                event_loop.exit();
                return;
            }
        };
        let Some(mut config) =
            surface.get_default_config(&g.adapter, size.width.max(1), size.height.max(1))
        else {
            eprintln!(
                "the window's drawing surface does not work with {}",
                g.describe()
            );
            event_loop.exit();
            return;
        };
        // The game's menus blend in the stored values: a format without the sRGB step keeps them.
        let caps = surface.get_capabilities(&g.adapter);
        if let Some(f) = caps.formats.iter().find(|f| !f.is_srgb()) {
            config.format = *f;
        }
        surface.configure(&g.device, &config);
        println!("drawing with {}; look: {}", g.describe(), self.look.from);
        self.screen = Some(first_screen(&self.options, gpu::game_card()));
        self.started = Instant::now();
        self.config = Some(config);
        self.surface = Some(surface);
        self.gpu = Some(g);
        self.window = Some(window.clone());
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::{ElementState, MouseButton, WindowEvent};
        use winit::keyboard::{Key, NamedKey};
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
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
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if !self.finished {
                    // Any key finishes the typing, as a click does in the game's terminal.
                    self.finished = true;
                } else if let Some(s) = &mut self.screen {
                    let n = s.entries.len();
                    match event.logical_key {
                        Key::Named(NamedKey::ArrowUp) => {
                            s.highlighted = (s.highlighted + n - 1) % n
                        }
                        Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::Tab) => {
                            s.highlighted = (s.highlighted + 1) % n
                        }
                        Key::Named(NamedKey::Enter) => self.choose(event_loop),
                        Key::Named(NamedKey::Escape) => event_loop.exit(),
                        _ => {}
                    }
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse = (position.x, position.y);
                if let Some(i) = self.row_at(self.mouse) {
                    if self.finished {
                        if let Some(s) = &mut self.screen {
                            if s.highlighted != i {
                                s.highlighted = i;
                                if let Some(w) = &self.window {
                                    w.request_redraw();
                                }
                            }
                        }
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if !self.finished {
                    self.finished = true;
                } else if let Some(i) = self.row_at(self.mouse) {
                    if let Some(s) = &mut self.screen {
                        s.highlighted = i;
                    }
                    self.choose(event_loop);
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Typing: a frame every 1/150 s until the text is in; then only the cursor's blink.
        let next = if self.finished {
            let ms = self.started.elapsed().as_millis() as u64;
            let wait = screen::BLINK_MS - ms % screen::BLINK_MS;
            Instant::now() + Duration::from_millis(wait)
        } else {
            Instant::now() + Duration::from_secs_f32(1.0 / screen::TYPE_RATE)
        };
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(next));
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

fn main() {
    let o = match options() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    if let Some(file) = o.capture.clone() {
        if let Err(e) = capture(&o, &file) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }
    let event_loop = match winit::event_loop::EventLoop::new() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("error: no window system: {e}");
            std::process::exit(1);
        }
    };
    let mut app = App {
        look: look(&o),
        options: o,
        instance: gpu::instance(),
        window: None,
        surface: None,
        config: None,
        gpu: None,
        screen: None,
        started: Instant::now(),
        finished: false,
        rows: None,
        mouse: (0.0, 0.0),
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
