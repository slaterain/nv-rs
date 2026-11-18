//! The terminal screen, laid out as the game's `menus\computers_menu.xml` lays out its terminal
//! (and `ComputersMenu::FinishIntro` 00758d30 fills it), in the menu's own units: a 4:3 screen
//! 720 high (`screen()`'s 960 times 0.75, "ref: FORenderedTerminal::Initialize()"), 960 wide.
//! - the background `Interface\Shared\Background\pipboy.dds` over the whole screen, 10 to the
//!   right, at brightness 90;
//! - three header lines centred from y 60 (x = (width - 17 * length) / 2, as `FinishIntro`
//!   centres them): `sComputersHeader1`, `sComputersHeader2`, then the terminal's own line;
//! - the welcome text at x 100, 20 below the header, wrapped at 600;
//! - a line 10 below it (`line.xml`, 2 thick at this size) as wide as the text;
//! - the list 10 below the line, x 100, 700 wide, 12 rows showing; each row is its text's
//!   height plus 20 (`list_box_template.xml`'s `_VerticalSpacing`), the text 15 into the row;
//!   each entry is the prompt `"> "` and its name (`00757d10`'s `"%s%s"`);
//! - the highlighted row: a box 5 narrower than the list and one row high less one, filled with
//!   the terminal colour at brightness 128 (`globals.xml`'s `_background_fill_brightness`) and
//!   alpha 224 (`_fill_alpha`), its text in the colour (0, 5, 0) (`&uselocalcolor;`);
//! - the prompt `"> "` 30 above the bottom with the 17 by 17 cursor after it, blinking every
//!   400 ms (`_blink_interval`).
//!
//! Every colour is the terminal colour (`[Interface] iSystemColorTerminal*`) times the tile's
//! brightness over 255 (`Tile::UpdateTile`'s colour step), text at 255 on an HD screen.
//! Text appears a character at a time at `iComputersDisplayRateMenus` (150 a second), the lines
//! in order, as `AnimatingTextManager` types them; any key finishes it.

use crate::canvas::Canvas;
use crate::font::Font;
use crate::game::Game;

pub const WIDTH: u32 = 960;
pub const HEIGHT: u32 = 720;

/// Characters a second the terminal types its text at (`iComputersDisplayRateMenus`, 150).
pub const TYPE_RATE: f32 = 150.0;
/// The cursor's blink, milliseconds (`computers_cursor`'s `_blink_interval`).
pub const BLINK_MS: u64 = 400;

/// How the terminal looks: from the player's game, or the port's own plain look before the game
/// is found.
pub struct Look {
    pub font: Font,
    pub colour: [f32; 3],
    pub background: Option<(u32, u32, Vec<u8>)>,
    /// The two header lines: the game's `sComputersHeader1` and `sComputersHeader2`.
    pub header: [String; 2],
    /// Where the look came from, for the log.
    pub from: String,
}

impl Look {
    /// The game's terminal, read from the installed game; None for any piece missing (the plain
    /// look is used then, and the log says which piece).
    pub fn from_game(game: &Game, inis: &[std::path::PathBuf]) -> Result<Look, String> {
        let fnt = game
            .read(r"textures\fonts\fixedsys_comp_uniform_width.fnt", "Fallout - Textures2.bsa")
            .ok_or("the terminal font (font 5) is not in the game's files")?;
        let texture_name: String = fnt
            .get(0x0c..0x0c + 0x24)
            .ok_or("the terminal font's file is too short")?
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| b as char)
            .collect();
        let tex = game
            .read(
                &format!(r"textures\fonts\{}.tex", texture_name.to_ascii_lowercase()),
                "Fallout - Textures2.bsa",
            )
            .ok_or("the terminal font's texture is not in the game's files")?;
        let font = Font::game(&fnt, &tex).ok_or("the terminal font could not be read")?;
        let colour = game
            .terminal_colour(inis)
            .ok_or("the terminal colour is in none of the game's INI files")?;
        let background = game
            .read(r"textures\interface\shared\background\pipboy.dds", "Fallout - Textures2.bsa")
            .and_then(|d| crate::canvas::dds_rgba(&d));
        let header = [
            game.exe_setting_text("sComputersHeader1")
                .ok_or("sComputersHeader1 is not in the game's FalloutNV.exe")?,
            game.exe_setting_text("sComputersHeader2")
                .ok_or("sComputersHeader2 is not in the game's FalloutNV.exe")?,
        ];
        Ok(Look {
            font,
            colour: colour.map(|c| c as f32 / 255.0),
            background,
            header,
            from: format!("the game at {}", game.folder.display()),
        })
    }

    /// The port's own look before the game is found: its bitmap font, a green of its own, no
    /// background, its own header.
    pub fn plain() -> Look {
        Look {
            font: Font::pixel(),
            colour: [0.2, 0.95, 0.45],
            background: None,
            header: ["NV-RS LAUNCHER".into(), "FALLOUT: NEW VEGAS NOT FOUND YET".into()],
            from: "the port's own look".into(),
        }
    }

    fn at(&self, brightness: f32, alpha: f32) -> [f32; 4] {
        let b = brightness / 255.0;
        [self.colour[0] * b, self.colour[1] * b, self.colour[2] * b, alpha / 255.0]
    }
}

/// What a screen shows.
pub struct Screen {
    /// The header's third line (the game shows the terminal's server name there).
    pub title: String,
    /// The welcome text's lines.
    pub welcome: Vec<String>,
    /// The entries (without the prompt).
    pub entries: Vec<String>,
    pub highlighted: usize,
    /// What has been typed at the bottom prompt.
    pub typed: String,
}

/// Where the rows are, for the mouse: the list's top, its x, width and a row's height.
pub struct Rows {
    pub top: i32,
    pub x: i32,
    pub width: u32,
    pub row: u32,
}

/// How many characters the screen types before it is whole.
pub fn total_chars(look: &Look, screen: &Screen) -> usize {
    look.header.iter().map(|l| l.chars().count()).sum::<usize>()
        + screen.title.chars().count()
        + screen.welcome.iter().map(|l| l.chars().count()).sum::<usize>()
        + screen.entries.iter().take(12).map(|e| e.chars().count() + 2).sum::<usize>()
}

/// The screen drawn with `look`, `shown` characters of its text typed so far (`usize::MAX`: all),
/// the cursor on or off.
pub fn draw(look: &Look, screen: &Screen, shown: usize, cursor: bool) -> (Canvas, Rows) {
    let mut c = Canvas::new(WIDTH, HEIGHT);
    if let Some((w, h, px)) = &look.background {
        // `computers_background`: x 10, the screen's size, brightness 90, alpha 255.
        c.stretch((px, *w, *h), (10, 0, WIDTH, HEIGHT), look.at(90.0, 255.0));
    }
    let lh = look.font.line_height() as i32;
    let text = look.at(255.0, 255.0);
    // The highlight shows once everything is typed (the game enables the items then).
    let done = shown >= total_chars(look, screen);
    let mut budget = shown;
    let mut typed = |s: &str| -> String {
        let n = s.chars().count().min(budget);
        budget -= n;
        s.chars().take(n).collect()
    };
    // The header, centred by 17 a character (the game's own measure in `FinishIntro`).
    let centred = |s: &str| (WIDTH as i32 - 17 * s.chars().count() as i32) / 2;
    let lines = [look.header[0].as_str(), look.header[1].as_str(), screen.title.as_str()];
    for (i, line) in lines.iter().enumerate() {
        let part = typed(line);
        look.font.draw(&mut c, centred(line), 60 + lh * i as i32, &part, text);
    }
    // The welcome text, then the line under it.
    let mut y = 60 + lh * 3 + 20;
    let mut widest = 0;
    for line in &screen.welcome {
        let part = typed(line);
        look.font.draw(&mut c, 100, y, &part, text);
        widest = widest.max(look.font.width(line));
        y += lh;
    }
    let line_y = y + 10;
    if !screen.welcome.is_empty() {
        c.fill(100, line_y, widest, 2, look.at(200.0, 255.0));
    }
    // The list.
    let top = line_y + 2 + 10;
    let row = (lh + 20) as u32;
    for (i, entry) in screen.entries.iter().take(12).enumerate() {
        let ry = top + (row as i32) * i as i32;
        let label = format!("> {entry}");
        let part = typed(&label);
        let colour = if i == screen.highlighted && done {
            c.fill(100, ry, 700 - 5, row - 1, look.at(128.0, 224.0));
            [0.0, 5.0 / 255.0, 0.0, 1.0]
        } else {
            text
        };
        look.font.draw(&mut c, 100, ry + 15, &part, colour);
    }
    // The prompt and the cursor.
    let py = HEIGHT as i32 - lh - 30;
    let prompt = format!("> {}", screen.typed);
    look.font.draw(&mut c, 100, py, &prompt, text);
    if cursor {
        let x = 100 + look.font.width(&prompt) as i32;
        c.fill(x, py + (lh - 17) / 2, 17, 17, text);
    }
    (
        c,
        Rows {
            top,
            x: 100,
            width: 695,
            row,
        },
    )
}
