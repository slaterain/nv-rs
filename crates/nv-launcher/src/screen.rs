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
/// Where the intro's text starts (`computers_intro_welcome`'s x and y).
pub const INTRO_AT: (i32, i32) = (100, 60);

/// How the terminal looks: from the player's game, or the port's own plain look before the game
/// is found.
pub struct Look {
    pub font: Font,
    pub colour: [f32; 3],
    pub background: Option<(u32, u32, Vec<u8>)>,
    /// The two header lines: the game's `sComputersHeader1` and `sComputersHeader2`.
    pub header: [String; 2],
    /// The intro's lines (`sHackingIntro01`, `sComputersLogon`, `sHackingHeader2`): the game's
    /// look only.
    pub intro: Option<[String; 3]>,
    /// Where the look came from, for the log.
    pub from: String,
}

impl Look {
    /// The game's terminal, read from the installed game; None for any piece missing (the plain
    /// look is used then, and the log says which piece).
    pub fn from_game(game: &Game, inis: &[std::path::PathBuf]) -> Result<Look, String> {
        let fnt = game
            .read(
                r"textures\fonts\fixedsys_comp_uniform_width.fnt",
                "Fallout - Textures2.bsa",
            )
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
            .read(
                r"textures\interface\shared\background\pipboy.dds",
                "Fallout - Textures2.bsa",
            )
            .and_then(|d| crate::canvas::dds_rgba(&d));
        let text = |name: &str| {
            game.exe_setting_text(name)
                .ok_or(format!("{name} is not in the game's FalloutNV.exe"))
        };
        let header = [text("sComputersHeader1")?, text("sComputersHeader2")?];
        let intro = [
            text("sHackingIntro01")?,
            text("sComputersLogon")?,
            text("sHackingHeader2")?,
        ];
        Ok(Look {
            font,
            colour: colour.map(|c| c as f32 / 255.0),
            background,
            header,
            intro: Some(intro),
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
            header: [
                "NV-RS LAUNCHER".into(),
                "FALLOUT: NEW VEGAS NOT FOUND YET".into(),
            ],
            intro: None,
            from: "the port's own look".into(),
        }
    }

    fn at(&self, brightness: f32, alpha: f32) -> [f32; 4] {
        let b = brightness / 255.0;
        [
            self.colour[0] * b,
            self.colour[1] * b,
            self.colour[2] * b,
            alpha / 255.0,
        ]
    }
}

/// What a screen shows.
#[derive(Clone, Debug, Default)]
pub struct Screen {
    /// The header's third line (the game shows the terminal's server name there).
    pub title: String,
    /// The welcome text's lines (each wrapped at the text's width).
    pub welcome: Vec<String>,
    /// The entries (without the prompt).
    pub entries: Vec<String>,
    pub highlighted: usize,
    /// What has been typed at the bottom prompt.
    pub typed: String,
    /// A text longer than its room shows its end (a program's output as it comes), else its
    /// start, moved by [`Screen::scroll`] lines.
    pub follow_end: bool,
    pub scroll: usize,
}

/// Where the rows are, for the mouse: the list's top, its x, width and a row's height.
pub struct Rows {
    pub top: i32,
    pub x: i32,
    pub width: u32,
    pub row: u32,
    /// The first entry shown (the list scrolls) and how many show.
    pub first: usize,
    pub shown: usize,
}

/// Where everything goes on `screen` (the welcome text's height decides where the list starts).
struct Layout {
    line_height: i32,
    welcome_y: i32,
    /// The welcome text's lines that show (a long text shows a part of itself).
    text: std::ops::Range<usize>,
    line_y: i32,
    rows: Rows,
    prompt_y: i32,
}

/// `computers_welcome`'s `wrapwidth`.
const WRAP: u32 = 600;

/// The welcome text's lines wrapped at `computers_welcome`'s `wrapwidth` (600), at spaces; a word
/// longer than the width (a folder's path) is broken where the width ends.
pub fn welcome_lines(look: &Look, screen: &Screen) -> Vec<String> {
    let mut out = Vec::new();
    for line in &screen.welcome {
        let mut now = String::new();
        for word in line.split(' ') {
            let next = if now.is_empty() {
                word.to_string()
            } else {
                format!("{now} {word}")
            };
            if look.font.width(&next) <= WRAP {
                now = next;
                continue;
            }
            if !now.is_empty() {
                out.push(std::mem::take(&mut now));
            }
            for c in word.chars() {
                now.push(c);
                if look.font.width(&now) > WRAP {
                    now.pop();
                    out.push(std::mem::take(&mut now));
                    now.push(c);
                }
            }
        }
        out.push(now);
    }
    out
}

fn layout(look: &Look, screen: &Screen) -> Layout {
    let lh = look.font.line_height() as i32;
    let welcome_y = 60 + lh * 3 + 20;
    let row = (lh + 20) as u32;
    let prompt_y = HEIGHT as i32 - lh - 30;
    // The text keeps room below it for the entries (three rows at least, or as many as there
    // are), the line and the gaps; a longer text shows the part that fits.
    let lines = welcome_lines(look, screen).len();
    let keep = screen.entries.len().min(3) as i32 * row as i32 + 2 + 20 + 10;
    let room = ((prompt_y - keep - welcome_y).max(lh) / lh) as usize;
    let visible = lines.min(room);
    let start = if screen.follow_end {
        lines - visible
    } else {
        screen.scroll.min(lines - visible)
    };
    let line_y = welcome_y + lh * visible as i32 + 10;
    let top = line_y + 2 + 10;
    // As many rows as fit above the prompt, at most the game's 12; scrolled so the highlighted
    // one shows.
    let fit = (((prompt_y - 10 - top).max(0) as u32 / row) as usize).clamp(1, 12);
    let first = screen.highlighted.saturating_sub(fit - 1);
    let shown = screen.entries.len().saturating_sub(first).min(fit);
    Layout {
        line_height: lh,
        welcome_y,
        text: start..start + visible,
        line_y,
        rows: Rows {
            top,
            x: 100,
            width: 700 - 5,
            row,
            first,
            shown,
        },
        prompt_y,
    }
}

/// How many characters the screen types before it is whole.
pub fn total_chars(look: &Look, screen: &Screen) -> usize {
    let l = layout(look, screen);
    let rows = l.rows;
    look.header.iter().map(|l| l.chars().count()).sum::<usize>()
        + screen.title.chars().count()
        + welcome_lines(look, screen)[l.text]
            .iter()
            .map(|l| l.chars().count())
            .sum::<usize>()
        + screen.entries[rows.first..rows.first + rows.shown]
            .iter()
            .map(|e| e.chars().count() + 2)
            .sum::<usize>()
}

/// The screen drawn with `look`, `shown` characters of its text typed so far (`usize::MAX`: all),
/// the cursor on or off.
pub fn draw(look: &Look, screen: &Screen, shown: usize, cursor: bool) -> (Canvas, Rows) {
    let mut c = Canvas::new(WIDTH, HEIGHT);
    if let Some((w, h, px)) = &look.background {
        // `computers_background`: x 10, the screen's size, brightness 90, alpha 255.
        c.stretch((px, *w, *h), (10, 0, WIDTH, HEIGHT), look.at(90.0, 255.0));
    }
    let l = layout(look, screen);
    let lh = l.line_height;
    let text = look.at(255.0, 255.0);
    // The highlight shows once everything is typed (the game enables the items then).
    let done = shown >= total_chars(look, screen);
    let mut budget = shown;
    let mut typed = |s: &str| -> String {
        let n = s.chars().count().min(budget);
        budget -= n;
        s.chars().take(n).collect()
    };
    // The header, centred: `FinishIntro` measures 17 a character, the game font's advance (the
    // font's own width gives the same); the plain font's by its own cell.
    let centred = |s: &str| (WIDTH as i32 - look.font.width(s) as i32) / 2;
    let lines = [
        look.header[0].as_str(),
        look.header[1].as_str(),
        screen.title.as_str(),
    ];
    for (i, line) in lines.iter().enumerate() {
        let part = typed(line);
        look.font
            .draw(&mut c, centred(line), 60 + lh * i as i32, &part, text);
    }
    // The welcome text (the part that shows), then the line under it, as wide as the text.
    let mut widest = 0;
    let welcome = welcome_lines(look, screen);
    for (i, line) in welcome[l.text.clone()].iter().enumerate() {
        let part = typed(line);
        look.font
            .draw(&mut c, 100, l.welcome_y + lh * i as i32, &part, text);
        widest = widest.max(look.font.width(line));
    }
    if !l.text.is_empty() {
        c.fill(100, l.line_y, widest, 2, look.at(200.0, 255.0));
    }
    // The list.
    let r = &l.rows;
    for (k, entry) in screen.entries[r.first..r.first + r.shown]
        .iter()
        .enumerate()
    {
        let i = r.first + k;
        let ry = r.top + r.row as i32 * k as i32;
        let part = typed(&format!("> {entry}"));
        let colour = if i == screen.highlighted && done {
            c.fill(r.x, ry, r.width, r.row - 1, look.at(128.0, 224.0));
            [0.0, 5.0 / 255.0, 0.0, 1.0]
        } else {
            text
        };
        look.font.draw(&mut c, r.x, ry + 15, &part, colour);
    }
    // The prompt and the cursor.
    let prompt = format!("> {}", screen.typed);
    look.font.draw(&mut c, 100, l.prompt_y, &prompt, text);
    if cursor {
        let x = 100 + look.font.width(&prompt) as i32;
        c.fill(x, l.prompt_y + (lh - 17) / 2, 17, 17, text);
    }
    (c, l.rows)
}

/// The intro's lines as typed so far (`crate::intro`) on the screen's background, in the menu's
/// four intro texts: `computers_intro_welcome` at (100, 60), and each of `_logon`, `_enterpw`
/// and `_pwdisplay` at the one before's y plus twice its height (one empty line between). The
/// prompt line and its cursor are hidden until `FinishIntro`.
pub fn draw_intro(look: &Look, lines: &[String]) -> Canvas {
    let mut c = Canvas::new(WIDTH, HEIGHT);
    if let Some((w, h, px)) = &look.background {
        c.stretch((px, *w, *h), (10, 0, WIDTH, HEIGHT), look.at(90.0, 255.0));
    }
    let lh = look.font.line_height() as i32;
    let text = look.at(255.0, 255.0);
    let (x, y) = INTRO_AT;
    for (i, line) in lines.iter().enumerate() {
        look.font.draw(&mut c, x, y + 2 * lh * i as i32, line, text);
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(welcome: &[&str]) -> Screen {
        Screen {
            title: "-nv-rs-".into(),
            welcome: welcome.iter().map(|s| s.to_string()).collect(),
            entries: (0..20).map(|i| format!("Entry {i}")).collect(),
            ..Default::default()
        }
    }

    /// A path with no spaces wider than the text is broken where the width ends, every letter
    /// kept.
    #[test]
    fn a_long_path_is_broken_at_the_width() {
        let look = Look::plain();
        let path = "C:\\Games\\Steam\\steamapps\\common\\Fallout New Vegas\\Data\\FalloutNV.esm";
        let lines = welcome_lines(&look, &screen(&[path]));
        assert!(lines.len() >= 3, "{lines:?}");
        assert!(lines.iter().all(|l| look.font.width(l) <= 600), "{lines:?}");
        assert_eq!(lines.concat().replace(' ', ""), path.replace(' ', ""));
    }

    /// A text longer than its room shows its start (moved by the scroll) or, following a
    /// program's output, its end; the entries keep their rows below it.
    #[test]
    fn a_long_text_shows_a_part_and_keeps_the_entries() {
        let look = Look::plain();
        let many: Vec<String> = (0..60).map(|i| format!("line {i}")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        let mut s = screen(&many);
        s.entries = vec!["Back".into(), "Exit".into()];
        let l = layout(&look, &s);
        assert_eq!(l.text.start, 0);
        assert!(l.text.len() > 5 && l.text.len() < 60, "{:?}", l.text);
        assert_eq!(l.rows.shown, 2);
        let prompt = HEIGHT as i32 - look.font.line_height() as i32 - 30;
        assert!(l.rows.top + (l.rows.row * 2) as i32 <= prompt - 10);
        s.scroll = 1000;
        assert_eq!(layout(&look, &s).text.end, 60);
        s.scroll = 3;
        assert_eq!(layout(&look, &s).text.start, 3);
        s.follow_end = true;
        assert_eq!(layout(&look, &s).text.end, 60);
    }

    /// The welcome text wraps at 600 at spaces and keeps every word; the list starts below it.
    #[test]
    fn the_welcome_text_wraps_at_600() {
        let look = Look::plain();
        let s = screen(&["Card: NVIDIA GeForce RTX 5070 Ti Laptop GPU", "Game: x"]);
        let lines = welcome_lines(&look, &s);
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(lines.iter().all(|l| look.font.width(l) <= 600), "{lines:?}");
        assert_eq!(
            lines[..2].join(" "),
            "Card: NVIDIA GeForce RTX 5070 Ti Laptop GPU"
        );
        let l = layout(&look, &s);
        assert_eq!(
            l.line_y,
            l.welcome_y + 3 * look.font.line_height() as i32 + 10
        );
    }

    /// The list shows the rows that fit above the prompt and scrolls to the highlighted one.
    #[test]
    fn the_list_fits_above_the_prompt_and_scrolls() {
        let look = Look::plain();
        let mut s = screen(&["Game: x"]);
        let r = layout(&look, &s).rows;
        let prompt = HEIGHT as i32 - look.font.line_height() as i32 - 30;
        assert!(r.shown >= 1 && r.shown < 20);
        assert!(r.top + (r.row * r.shown as u32) as i32 <= prompt - 10);
        assert_eq!(r.first, 0);
        s.highlighted = 15;
        let r = layout(&look, &s).rows;
        assert_eq!(r.first + r.shown - 1, 15);
    }
}
