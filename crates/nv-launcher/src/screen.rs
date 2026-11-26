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
//! The launcher's own addition, as the window has no title bar: minimise and exit buttons in the
//! top-right corner above the header ([`buttons`]), in the text's colour and the list's highlight.
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

/// The milliseconds a character a screen types at: 1000 / `iComputersDisplayRateMenus` (150), in
/// whole milliseconds as the terminal display function 007586e0 works it out (6), on the game's frame clock
/// (`crate::pacing`).
pub const MS_PER_CHAR: u64 = 6;
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// Whether [`MORE_BELOW`] shows under them, in the place of the text's last line: the text
    /// goes on below (read with Page Down or the wheel).
    more: bool,
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

/// Where `screen`'s rows go, as [`draw`] lays them out (the tests' mouse hits).
#[cfg(test)]
pub fn rows_of(look: &Look, screen: &Screen) -> Rows {
    layout(look, screen).rows
}

/// Whether every line of `screen`'s text and every entry show at once, nothing to scroll (the
/// tests of screens that must be read whole).
#[cfg(test)]
pub fn shows_whole(look: &Look, screen: &Screen) -> bool {
    let l = layout(look, screen);
    l.text.len() == welcome_lines(look, screen).len()
        && l.rows.first == 0
        && l.rows.shown == screen.entries.len()
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
    // A text cut before its end (not a program's output, which shows its end) says so in its
    // last line's place.
    let more = !screen.follow_end && start + visible < lines;
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
        text: start..start + visible - usize::from(more),
        more,
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

/// A box on the screen: x, y, width, height.
pub type Area = (i32, i32, u32, u32);

/// The header's three lines and where each goes (x, y, width, height): centred from y 60, a line
/// apart. `FinishIntro` measures 17 a character, the game font's advance (the font's own width
/// gives the same); the plain font's by its own cell.
fn header_lines<'a>(look: &'a Look, screen: &'a Screen) -> [(&'a str, Area); 3] {
    let lh = look.font.line_height();
    let line = |i: i32, s: &'a str| {
        let w = look.font.width(s);
        (
            s,
            ((WIDTH as i32 - w as i32) / 2, 60 + lh as i32 * i, w, lh),
        )
    };
    [
        line(0, &look.header[0]),
        line(1, &look.header[1]),
        line(2, &screen.title),
    ]
}

/// The header's lines' boxes (x, y, width, height) as [`draw`] puts them (the tests' fit of the
/// window's buttons).
#[cfg(test)]
pub fn header_boxes(look: &Look, screen: &Screen) -> [Area; 3] {
    header_lines(look, screen).map(|(_, b)| b)
}

/// The window's own buttons (it has no title bar), in the screen's top-right corner on every
/// screen, the intro's too: minimise, then exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Minimise,
    Exit,
}

impl Button {
    /// The sign it shows in the terminal's font: the usual ones.
    pub fn sign(self) -> &'static str {
        match self {
            Button::Minimise => "_",
            Button::Exit => "X",
        }
    }
}

/// The buttons' boxes (x, y, width, height): squares a line high plus 10 (at most 45), 5 apart,
/// 10 from the screen's top and right edges, so above the header (its first line starts at 60).
pub fn buttons(look: &Look) -> [(Button, Area); 2] {
    let side = (look.font.line_height() + 10).min(45);
    let exit = WIDTH as i32 - 10 - side as i32;
    [
        (Button::Minimise, (exit - 5 - side as i32, 10, side, side)),
        (Button::Exit, (exit, 10, side, side)),
    ]
}

/// The button whose box holds the point `p` of the screen (in its own units), if any.
pub fn button_at(look: &Look, p: (f64, f64)) -> Option<Button> {
    buttons(look)
        .into_iter()
        .find(|&(_, (x, y, w, h))| {
            p.0 >= x as f64
                && p.0 < (x + w as i32) as f64
                && p.1 >= y as f64
                && p.1 < (y + h as i32) as f64
        })
        .map(|(b, _)| b)
}

/// The buttons drawn over a screen: each sign in the text's colour, the ink of "X" centred in its
/// box (so "_" sits low, as a minimise sign does); the one under the mouse (`hover`) as the list's
/// highlighted row is, its box filled at brightness 128 and alpha 224, its sign in (0, 5, 0).
pub fn draw_buttons(c: &mut Canvas, look: &Look, hover: Option<Button>) {
    let (dx, dy) = sign_offset(look);
    for (b, (x, y, w, h)) in buttons(look) {
        let colour = if hover == Some(b) {
            c.fill(x, y, w, h, look.at(128.0, 224.0));
            [0.0, 5.0 / 255.0, 0.0, 1.0]
        } else {
            look.at(255.0, 255.0)
        };
        look.font.draw(c, x + dx, y + dy, b.sign(), colour);
    }
}

/// Where a sign's pen goes in its box: the ink of "X" (drawn once on its own) centred in it.
fn sign_offset(look: &Look) -> (i32, i32) {
    let side = buttons(look)[0].1 .2 as i32;
    let (w, h) = (look.font.width("X").max(1), look.font.line_height().max(1));
    let mut c = Canvas::new(w, h);
    look.font.draw(&mut c, 0, 0, "X", [1.0; 4]);
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if c.pixels[((y * w + x) * 4) as usize] > 0 {
                (x0, y0) = (x0.min(x), y0.min(y));
                (x1, y1) = (x1.max(x + 1), y1.max(y + 1));
            }
        }
    }
    if x0 >= x1 {
        return ((side - w as i32) / 2, (side - h as i32) / 2);
    }
    ((side - (x0 + x1) as i32) / 2, (side - (y0 + y1) as i32) / 2)
}

/// The lines the screen types, in the order the game's text manager types them: the two header
/// lines, the title (the header's third line), the welcome text's lines that show, each entry
/// that shows as `"> "` and its text, and last the prompt `"> "` (`FinishIntro` 00758d30 puts the
/// header at the queue's front, after the screen's own items, and the prompt, 01072118, at its
/// end).
pub fn line_texts(look: &Look, screen: &Screen) -> Vec<String> {
    let l = layout(look, screen);
    let rows = l.rows;
    let mut out = vec![
        look.header[0].clone(),
        look.header[1].clone(),
        screen.title.clone(),
    ];
    out.extend(welcome_lines(look, screen)[l.text].iter().cloned());
    if l.more {
        out.push(MORE_BELOW.to_string());
    }
    out.extend(
        screen.entries[rows.first..rows.first + rows.shown]
            .iter()
            .map(|e| format!("> {e}")),
    );
    out.push(PROMPT.to_string());
    out
}

/// The prompt at the bottom, before what is typed there.
pub const PROMPT: &str = "> ";

/// The line under a text that goes on below what shows.
pub const MORE_BELOW: &str = "(more below: Page Down)";

/// Where the first entry's line is in [`line_texts`].
pub fn first_entry_line(look: &Look, screen: &Screen) -> usize {
    let l = layout(look, screen);
    3 + l.text.len() + usize::from(l.more)
}

/// How many characters the screen types before it is whole.
pub fn total_chars(look: &Look, screen: &Screen) -> usize {
    line_texts(look, screen)
        .iter()
        .map(|l| l.chars().count())
        .sum()
}

/// Whether the screen's entries can be highlighted and chosen with `shown` characters of each
/// line ([`line_texts`]) typed (None: all): only once every line is in. In the game each entry's
/// tile is made disabled (007586e0: `_enabled` 0 right after the tile is made) and shows as its
/// line types; `DoIdle` 00758470 enables them all (007596a0) only when the text queue is empty.
pub fn entries_live(look: &Look, screen: &Screen, shown: Option<&[usize]>) -> bool {
    let Some(shown) = shown else {
        return true;
    };
    all_in(&line_texts(look, screen), shown)
}

/// Whether every line of `texts` has typed all its characters (`shown` of each).
pub fn all_in(texts: &[String], shown: &[usize]) -> bool {
    texts.len() <= shown.len()
        && texts
            .iter()
            .zip(shown)
            .all(|(t, &n)| n >= t.chars().count())
}

/// The screen drawn with `look`, `shown` characters of each of its lines typed so far
/// ([`line_texts`]; None: all), the cursor on or off.
pub fn draw(look: &Look, screen: &Screen, shown: Option<&[usize]>, cursor: bool) -> (Canvas, Rows) {
    let mut c = Canvas::new(WIDTH, HEIGHT);
    if let Some((w, h, px)) = &look.background {
        // `computers_background`: x 10, the screen's size, brightness 90, alpha 255.
        c.stretch((px, *w, *h), (10, 0, WIDTH, HEIGHT), look.at(90.0, 255.0));
    }
    // The focus (the highlight box) only on live entries: the game's are disabled while it types.
    let live = entries_live(look, screen, shown);
    let l = layout(look, screen);
    let lh = l.line_height;
    let text = look.at(255.0, 255.0);
    // Each line as typed so far, in [`line_texts`]' order.
    let mut index = 0;
    let mut typed = |s: &str| -> String {
        let n = shown.map_or(usize::MAX, |v| v.get(index).copied().unwrap_or(0));
        index += 1;
        s.chars().take(n).collect()
    };
    // The header, centred ([`header_lines`]).
    for (line, (x, y, _, _)) in header_lines(look, screen) {
        let part = typed(line);
        look.font.draw(&mut c, x, y, &part, text);
    }
    // The welcome text (the part that shows), then the line under it, as wide as the text.
    let mut widest = 0;
    let welcome = welcome_lines(look, screen);
    let more = l.more.then_some(MORE_BELOW);
    for (i, line) in welcome[l.text.clone()]
        .iter()
        .map(String::as_str)
        .chain(more)
        .enumerate()
    {
        let part = typed(line);
        look.font
            .draw(&mut c, 100, l.welcome_y + lh * i as i32, &part, text);
        widest = widest.max(look.font.width(line));
    }
    if !l.text.is_empty() || l.more {
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
        // The highlight box follows the focus, once the entries are live.
        let colour = if i == screen.highlighted && live {
            c.fill(r.x, ry, r.width, r.row - 1, look.at(128.0, 224.0));
            [0.0, 5.0 / 255.0, 0.0, 1.0]
        } else {
            text
        };
        look.font.draw(&mut c, r.x, ry + 15, &part, colour);
    }
    // The prompt (typed last on the first screen), what is typed after it, and the cursor once
    // the prompt is in.
    let part = typed(PROMPT);
    let prompt_in = part.len() == PROMPT.len();
    let prompt = if prompt_in {
        format!("{PROMPT}{}", screen.typed)
    } else {
        part
    };
    look.font.draw(&mut c, 100, l.prompt_y, &prompt, text);
    if cursor && prompt_in {
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

    /// A text longer than its room shows its start (moved by the scroll), with a line saying more
    /// is below in its last line's place, or, scrolled to its end or following a program's
    /// output, its end with no such line; the entries keep their rows below it.
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
        assert!(l.more);
        let lines = line_texts(&look, &s);
        assert_eq!(lines[3 + l.text.len()], MORE_BELOW);
        assert_eq!(first_entry_line(&look, &s), 4 + l.text.len());
        assert_eq!(lines[first_entry_line(&look, &s)], "> Back");
        assert_eq!(l.rows.shown, 2);
        let prompt = HEIGHT as i32 - look.font.line_height() as i32 - 30;
        assert!(l.rows.top + (l.rows.row * 2) as i32 <= prompt - 10);
        s.scroll = 1000;
        let end = layout(&look, &s);
        assert_eq!((end.text.end, end.more), (60, false));
        assert_eq!(
            end.text.len(),
            l.text.len() + 1,
            "the hint's line is the text's again"
        );
        s.scroll = 3;
        assert_eq!(layout(&look, &s).text.start, 3);
        s.follow_end = true;
        let follow = layout(&look, &s);
        assert_eq!((follow.text.end, follow.more), (60, false));
        // A text that fits says nothing more.
        let short = screen(&["Game: x"]);
        assert!(!layout(&look, &short).more);
        assert!(!line_texts(&look, &short).iter().any(|l| l == MORE_BELOW));
        assert!(look.font.width(MORE_BELOW) <= WRAP);
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

    /// The window's buttons draw their signs inside their boxes in the text's colour, and only
    /// the one under the mouse gets the list's highlight box (its sign then dark).
    #[test]
    fn the_buttons_show_the_highlight_under_the_mouse() {
        let look = Look::plain();
        let at = |c: &Canvas, x: i32, y: i32| {
            let i = ((y as u32 * c.width + x as u32) * 4) as usize;
            [c.pixels[i], c.pixels[i + 1], c.pixels[i + 2]]
        };
        let lit = |c: &Canvas, (x, y, w, h): Area| {
            (y..y + h as i32)
                .flat_map(|yy| (x..x + w as i32).map(move |xx| (xx, yy)))
                .filter(|&(xx, yy)| at(c, xx, yy) != [0, 0, 0])
                .count()
        };
        let mut plain = Canvas::new(WIDTH, HEIGHT);
        draw_buttons(&mut plain, &look, None);
        let boxes = buttons(&look);
        let inside: usize = boxes.iter().map(|&(_, b)| lit(&plain, b)).sum();
        assert!(
            boxes.iter().all(|&(_, b)| lit(&plain, b) > 20),
            "each sign drawn"
        );
        assert_eq!(
            lit(&plain, (0, 0, WIDTH, HEIGHT)),
            inside,
            "nothing outside the boxes"
        );
        let text = look.at(255.0, 255.0).map(|v| (v * 255.0).round() as u8);
        for (_, (x, y, w, h)) in boxes {
            let sign: Vec<_> = (y..y + h as i32)
                .flat_map(|yy| (x..x + w as i32).map(move |xx| (xx, yy)))
                .map(|(xx, yy)| at(&plain, xx, yy))
                .filter(|p| *p != [0, 0, 0])
                .collect();
            assert!(sign.contains(&[text[0], text[1], text[2]]), "{sign:?}");
        }
        let mut hovered = Canvas::new(WIDTH, HEIGHT);
        draw_buttons(&mut hovered, &look, Some(Button::Exit));
        let fill = {
            let mut c = Canvas::new(1, 1);
            c.fill(0, 0, 1, 1, look.at(128.0, 224.0));
            at(&c, 0, 0)
        };
        let (_, (ex, ey, ew, eh)) = boxes[1];
        let (_, (mx, my, _, _)) = boxes[0];
        assert_eq!(boxes[1].0, Button::Exit);
        assert_eq!(at(&hovered, ex + 1, ey + 1), fill);
        assert_eq!(at(&hovered, ex + ew as i32 - 2, ey + eh as i32 - 2), fill);
        assert_eq!(at(&hovered, mx + 1, my + 1), [0, 0, 0]);
        // The highlighted sign is dark on the box: no text-coloured pixel left in it.
        let exit_pixels: Vec<_> = (ey..ey + eh as i32)
            .flat_map(|yy| (ex..ex + ew as i32).map(move |xx| (xx, yy)))
            .map(|(xx, yy)| at(&hovered, xx, yy))
            .collect();
        assert!(!exit_pixels.contains(&[text[0], text[1], text[2]]));
        assert!(
            exit_pixels.iter().any(|p| *p != fill),
            "the sign shows on the box"
        );
    }
}
