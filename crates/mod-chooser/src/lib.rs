//! The mod installer's text front end: the same on Windows, Linux and macOS, in plain words a
//! player who has never used a terminal can follow (the maintainer's and lead's rules, 2026-11-18):
//! every question says what to type, Enter takes the marked (recommended) choice, and a typo is
//! answered with what was wrong and the same question again: nothing chosen so far is lost.
//!
//! It lists the mods installed with nv-rs and the archives waiting in the mods folder. Picking an
//! archive installs it: a FOMOD installer's steps are asked one group at a time (`crates/fomod`),
//! the chosen files are unpacked (`crates/mod-archive`) into a fresh folder of the port's own that
//! is deleted if anything fails, the record is written, the folder takes its place in
//! `mod installs` (`crates/mod-install`), and the archive moves to `installed archives` in the mods
//! folder. An archive with no installer installs its Data layout as it is; a `script.cs`
//! installer is a program nv-rs does not run, and its plain layout is offered when it has one.
//! Picking an installed mod shows its record; `remove` and its number removes it.
//!
//! Nothing here touches the game's folder: it is only read, to answer the installers' questions
//! about which plugins and files the player has.

use fomod::config::{parse_config, parse_info, FileState};
use fomod::install::{Facts, PlanError, Session};
use fomod::layout::{layout, Layout, Listing};
use mod_install::installs::{self, Installed, Record};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// Where things are, and what the port provides.
#[derive(Debug, Clone)]
pub struct Places {
    /// The game's folder (the one holding Data): only read.
    pub game: PathBuf,
    /// The port's `mod installs` folder.
    pub installs: PathBuf,
    /// The loose `mods` folder the player drops archives into.
    pub loose: PathBuf,
    /// Script extenders the port provides, by name and version (`nvse`, `6.4.9`).
    pub extenders: Vec<(String, String)>,
}

/// The game's version, for the installers' game version conditions.
const GAME_VERSION: &str = "1.4.0.525";

/// The most files of the game's Data folder read for the installers' questions (as
/// `mod_install::overlay::MAX_FILES`).
const MAX_FACT_FILES: usize = 500_000;

type Out<'a> = &'a mut dyn Write;

/// One line typed, trimmed; `None` when the input ended (taken as stopping).
fn ask(input: &mut dyn BufRead, out: Out, question: &str) -> std::io::Result<Option<String>> {
    writeln!(out, "{question}")?;
    write!(out, "> ")?;
    out.flush()?;
    let mut line = String::new();
    if input.read_line(&mut line)? == 0 {
        writeln!(out)?;
        return Ok(None);
    }
    Ok(Some(line.trim().to_owned()))
}

/// Numbers typed with spaces or commas between them, each from 1 to `count`, as indices from 0;
/// `Err` with what to type when one is not.
fn numbers(text: &str, count: usize) -> Result<Vec<usize>, String> {
    let mut out = Vec::new();
    for word in text.split([' ', ',']).filter(|w| !w.is_empty()) {
        match word.parse::<usize>() {
            Ok(n) if (1..=count).contains(&n) => out.push(n - 1),
            _ => {
                return Err(format!(
                    "\"{word}\" is not one of the numbers shown. Type numbers from 1 to {count}."
                ))
            }
        }
    }
    Ok(out)
}

/// The archives waiting in the loose folder (its top level, told by their first bytes; RAR too,
/// to say it can't be read), by name.
fn waiting(loose: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(loose) else {
        return vec![];
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .filter(|p| {
            let mut head = [0u8; 8];
            let read = std::fs::File::open(p)
                .and_then(|mut f| std::io::Read::read(&mut f, &mut head))
                .unwrap_or(0);
            !matches!(
                mod_archive::format_of(&head[..read]),
                mod_archive::Format::Unknown
            )
        })
        .collect();
    out.sort();
    out
}

/// What the player has, for the installers' questions: every file of the game's Data and of the
/// installed mods counts as there and switched on (the port switches on every plugin it plays
/// with).
fn facts(places: &Places) -> Facts {
    let mut plugins = Vec::new();
    let mut walk = |root: &Path| {
        let mut stack = vec![(root.to_path_buf(), String::new(), 0usize)];
        while let Some((dir, relative, depth)) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.filter_map(Result::ok) {
                if plugins.len() >= MAX_FACT_FILES {
                    return;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                let path = if relative.is_empty() {
                    name.clone()
                } else {
                    format!("{relative}/{name}")
                };
                match entry.file_type() {
                    Ok(t) if t.is_dir() && depth < 16 => {
                        stack.push((entry.path(), path, depth + 1))
                    }
                    Ok(t) if t.is_file() => plugins.push((path, FileState::Active)),
                    _ => {}
                }
            }
        }
    };
    if let Some(data) = mod_install::overlay::game_data(&places.game) {
        walk(&data);
    }
    for mod_folder in installs::installed(&places.installs).unwrap_or_default() {
        walk(&mod_folder.folder);
    }
    Facts {
        plugins,
        game_version: Some(GAME_VERSION.to_owned()),
        extenders: places.extenders.clone(),
    }
}

/// The front end: the list, then what the player picks, until they press Enter on the list.
pub fn run(input: &mut dyn BufRead, out: Out, places: &Places) -> std::io::Result<()> {
    writeln!(out, "nv-rs mods")?;
    writeln!(out, "==========")?;
    loop {
        let installed = installs::installed(&places.installs).unwrap_or_default();
        let archives = waiting(&places.loose);
        writeln!(out)?;
        if installed.is_empty() && archives.is_empty() {
            writeln!(
                out,
                "No mods yet. Put a mod's archive (.7z, .zip or .fomod) in the mods folder:\n  {}\nthen start this again.",
                places.loose.display()
            )?;
            return Ok(());
        }
        if !installed.is_empty() {
            writeln!(out, "Installed with nv-rs:")?;
            for (i, m) in installed.iter().enumerate() {
                writeln!(out, "  {}) {}", i + 1, m.name)?;
            }
        }
        if !archives.is_empty() {
            writeln!(out, "Waiting in the mods folder, not installed yet:")?;
            for (i, a) in archives.iter().enumerate() {
                writeln!(
                    out,
                    "  {}) {}",
                    installed.len() + i + 1,
                    a.file_name().unwrap_or_default().to_string_lossy()
                )?;
            }
        }
        let total = installed.len() + archives.len();
        let Some(answer) = ask(
            input,
            out,
            "\nType a number and press Enter to install that mod (or to see an installed one).\nType \"remove\" and a number to remove an installed mod. Just press Enter to finish.",
        )?
        else {
            return Ok(());
        };
        if answer.is_empty() {
            writeln!(out, "Done. The mods are used the next time you choose Play with mods.")?;
            return Ok(());
        }
        let (remove, rest) = match answer.strip_prefix("remove") {
            Some(rest) => (true, rest.trim()),
            None => (false, answer.as_str()),
        };
        let picked = match numbers(rest, total) {
            Ok(p) if p.len() == 1 => p[0],
            Ok(_) => {
                writeln!(out, "Type one number from 1 to {total}.")?;
                continue;
            }
            Err(why) => {
                writeln!(out, "{why}")?;
                continue;
            }
        };
        if picked < installed.len() {
            let chosen = &installed[picked];
            if remove {
                remove_flow(input, out, places, chosen)?;
            } else {
                show_record(out, chosen)?;
            }
        } else if remove {
            writeln!(
                out,
                "That one is not installed: only mods from the \"Installed with nv-rs\" list can be removed."
            )?;
        } else {
            install_flow(input, out, places, &archives[picked - installed.len()])?;
        }
    }
}

fn show_record(out: Out, chosen: &Installed) -> std::io::Result<()> {
    let text = std::fs::read_to_string(chosen.folder.join(mod_install::RECORD)).unwrap_or_default();
    let record = Record::parse(&text);
    writeln!(out, "\n{} {}", chosen.name, record.version)?;
    if !record.author.is_empty() {
        writeln!(out, "  by {}", record.author)?;
    }
    if !record.archive.is_empty() {
        writeln!(out, "  from {}", record.archive)?;
    }
    for choice in &record.choices {
        writeln!(out, "  chosen: {choice}")?;
    }
    writeln!(out, "  {} files", record.files.len())?;
    Ok(())
}

fn remove_flow(
    input: &mut dyn BufRead,
    out: Out,
    places: &Places,
    chosen: &Installed,
) -> std::io::Result<()> {
    let answer = ask(
        input,
        out,
        &format!(
            "Remove {}? Its files are deleted from nv-rs's folder (your game folder is not touched).\nType yes and press Enter to remove it, or just press Enter to keep it.",
            chosen.name
        ),
    )?;
    if answer.as_deref().is_some_and(|a| a.eq_ignore_ascii_case("yes")) {
        match installs::remove(&places.installs, &chosen.folder) {
            Ok(()) => writeln!(out, "Removed {}.", chosen.name)?,
            Err(e) => writeln!(out, "{} could not be removed: {e}", chosen.name)?,
        }
    } else {
        writeln!(out, "Kept {}.", chosen.name)?;
    }
    Ok(())
}

/// What an install puts where: (path in the archive, path in the mod's folder).
type Wanted = Vec<(String, String)>;

/// Installs `archive`: the right flow for what it holds.
fn install_flow(
    input: &mut dyn BufRead,
    out: Out,
    places: &Places,
    archive: &Path,
) -> std::io::Result<()> {
    let file_name = archive
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    writeln!(out, "\n{file_name}")?;
    let entries = match mod_archive::list(archive) {
        Ok(e) => e,
        Err(e) => return writeln!(out, "{}", e.player_text()),
    };
    let listing: Vec<String> = entries.iter().map(|e| e.path.clone()).collect();
    let found = match layout(&listing) {
        Ok(l) => l,
        Err(e) => return writeln!(out, "{e}"),
    };
    let stem = Path::new(&file_name)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let read_info = |info: &Option<String>| {
        info.as_deref()
            .and_then(|path| mod_archive::read(archive, path).ok().flatten())
            .map(|bytes| parse_info(&bytes))
            .unwrap_or_default()
    };
    match found {
        Layout::Installer {
            prefix,
            config,
            info,
        } => {
            let info = read_info(&info);
            let bytes = match mod_archive::read(archive, &config) {
                Ok(Some(b)) => b,
                Ok(None) => return writeln!(out, "The installer file could not be found in it."),
                Err(e) => return writeln!(out, "{}", e.player_text()),
            };
            let config = match parse_config(&bytes) {
                Ok(c) => c,
                Err(e) => return writeln!(out, "This mod's installer can't be read: {e}."),
            };
            let name = first_of(&[&info.name, &config.name, &stem]);
            let facts = facts(places);
            let mut session = match Session::new(&config, &facts) {
                Ok(s) => s,
                Err(why) => return writeln!(out, "This mod can't be installed: {why}."),
            };
            writeln!(out, "{name} {}", info.version)?;
            if !info.author.is_empty() {
                writeln!(out, "by {}", info.author)?;
            }
            let Some((wanted, choices)) =
                ask_steps(input, out, &mut session, &listing, &prefix)?
            else {
                return writeln!(out, "Stopped: nothing was installed.");
            };
            let record = Record {
                name: name.clone(),
                version: info.version.clone(),
                author: info.author.clone(),
                choices,
                ..Record::default()
            };
            install(out, places, archive, &name, wanted, record)
        }
        Layout::Script { data, .. } => {
            writeln!(
                out,
                "This mod's installer is a small program (script.cs). nv-rs does not run programs that come with a download."
            )?;
            match data {
                Some(data) => plain_flow(input, out, places, archive, &listing, &data, &stem),
                None => writeln!(
                    out,
                    "It has no files laid out as in the game's Data folder either. Install it with a mod manager and copy the result into the mods folder instead."
                ),
            }
        }
        Layout::Plain { data, info } => {
            let info = read_info(&info);
            let name = first_of(&[&info.name, &stem]);
            plain_flow(input, out, places, archive, &listing, &data, &name)
        }
        Layout::Unknown => writeln!(
            out,
            "nv-rs can't tell how to install this: it has no installer and no files laid out as in the game's Data folder (.esp, .esm, .bsa, Meshes, Textures...)."
        ),
    }
}

fn first_of(names: &[&str]) -> String {
    names
        .iter()
        .find(|n| !n.trim().is_empty())
        .map_or_else(|| "Mod".to_owned(), |n| n.trim().to_owned())
}

/// An archive without an installer: its files from `data` down, as they are, after a yes.
fn plain_flow(
    input: &mut dyn BufRead,
    out: Out,
    places: &Places,
    archive: &Path,
    listing: &[String],
    data: &str,
    name: &str,
) -> std::io::Result<()> {
    let index = match Listing::new(listing) {
        Ok(l) => l,
        Err(e) => return writeln!(out, "{e}"),
    };
    let wanted: Wanted = index
        .under(data)
        .into_iter()
        .map(|(rest, given)| (given.to_owned(), rest))
        .collect();
    let answer = ask(
        input,
        out,
        &format!(
            "Install {name} as it is laid out ({} files)? Type yes and press Enter to install it, or just press Enter to leave it.",
            wanted.len()
        ),
    )?;
    if !answer.as_deref().is_some_and(|a| a.eq_ignore_ascii_case("yes")) {
        return writeln!(out, "Left as it is: nothing was installed.");
    }
    let record = Record {
        name: name.to_owned(),
        ..Record::default()
    };
    install(out, places, archive, name, wanted, record)
}

/// The steps, one group at a time, until the player says yes to the plan (`Some`: what to unpack
/// and the choices made) or stops (`None`).
fn ask_steps(
    input: &mut dyn BufRead,
    out: Out,
    session: &mut Session,
    listing: &[String],
    prefix: &str,
) -> std::io::Result<Option<(Wanted, Vec<String>)>> {
    let mut history: Vec<usize> = Vec::new();
    let mut step = session.next_step(None);
    loop {
        while let Some(index) = step {
            match ask_step(input, out, session, index, history.len() + 1)? {
                StepAnswer::Next => {
                    history.push(index);
                    step = session.next_step(Some(index));
                }
                StepAnswer::Back => match history.pop() {
                    Some(previous) => step = Some(previous),
                    None => writeln!(out, "This is the first step.")?,
                },
                StepAnswer::Stop => return Ok(None),
            }
        }
        let plan = match session.plan(listing, prefix) {
            Ok(plan) => plan,
            Err(PlanError::Choices(problems)) => {
                for (_, why) in &problems {
                    writeln!(out, "Not yet: {why}.")?;
                }
                step = problems.first().map(|(s, _)| *s);
                continue;
            }
            Err(e) => {
                writeln!(out, "This mod can't be installed: {e}.")?;
                return Ok(None);
            }
        };
        writeln!(out, "\n{} files will be installed.", plan.copies.len())?;
        if !plan.missing.is_empty() {
            writeln!(
                out,
                "{} files the installer names are not in the archive; the mod may still work.",
                plan.missing.len()
            )?;
        }
        if !plan.refused.is_empty() {
            writeln!(
                out,
                "{} files would go outside the game's Data folder and are left out.",
                plan.refused.len()
            )?;
        }
        let answer = ask(
            input,
            out,
            "Type yes and press Enter to install it. Press Enter to go back to the choices, or type q to stop.",
        )?;
        match answer.as_deref() {
            Some(a) if a.eq_ignore_ascii_case("yes") => {
                let wanted = plan
                    .copies
                    .into_iter()
                    .map(|c| (c.source, c.destination))
                    .collect();
                return Ok(Some((wanted, choices(session))));
            }
            None => return Ok(None),
            Some(a) if a.eq_ignore_ascii_case("q") => return Ok(None),
            Some(_) => step = history.pop(),
        }
    }
}

enum StepAnswer {
    Next,
    Back,
    Stop,
}

/// One step, group by group.
fn ask_step(
    input: &mut dyn BufRead,
    out: Out,
    session: &mut Session,
    index: usize,
    number: usize,
) -> std::io::Result<StepAnswer> {
    let view = session.view(index);
    let title = if view.name.is_empty() {
        "Choices".to_owned()
    } else {
        view.name.clone()
    };
    writeln!(out, "\nStep {number}: {title}")?;
    let mut group = 0;
    while group < view.groups.len() {
        let current = session.view(index).groups[group].clone();
        writeln!(out, "\n  {}", current.name)?;
        for (i, option) in current.options.iter().enumerate() {
            let mark = if option.selected { "[x]" } else { "[ ]" };
            let tag = match option.kind {
                fomod::config::PluginType::Required => "  (needed)",
                fomod::config::PluginType::Recommended => "  (recommended)",
                fomod::config::PluginType::NotUsable => "  (can't be chosen here)",
                _ => "",
            };
            writeln!(out, "    {}) {mark} {}{tag}", i + 1, option.name)?;
        }
        use fomod::config::GroupType as G;
        let rule = match current.kind {
            G::SelectExactlyOne => "Type the number of the one you want",
            G::SelectAtMostOne => "Type the number of the one you want, or 0 for none",
            G::SelectAtLeastOne => "Type the numbers you want, with spaces between them (at least one)",
            G::SelectAny => "Type the numbers you want, with spaces between them, or 0 for none",
            G::SelectAll => "All of these are installed",
        };
        let question = format!(
            "{rule}. Press Enter to keep the ones marked [x]. Type ? and a number to read about an option, b to go back a step, q to stop."
        );
        let Some(answer) = ask(input, out, &question)? else {
            return Ok(StepAnswer::Stop);
        };
        let lower = answer.to_ascii_lowercase();
        if lower == "q" {
            return Ok(StepAnswer::Stop);
        }
        if lower == "b" {
            return Ok(StepAnswer::Back);
        }
        if let Some(n) = answer.strip_prefix('?') {
            match numbers(n.trim(), current.options.len()) {
                Ok(p) if p.len() == 1 => {
                    let option = &current.options[p[0]];
                    let text = if option.description.is_empty() {
                        "(no description)"
                    } else {
                        option.description.as_str()
                    };
                    writeln!(out, "\n  {}: {text}", option.name)?;
                }
                _ => writeln!(
                    out,
                    "Type ? and one number from 1 to {}.",
                    current.options.len()
                )?,
            }
            continue;
        }
        let chosen = if answer.is_empty() {
            Ok(session.chosen(index, group))
        } else if answer == "0" {
            Ok(vec![])
        } else {
            numbers(&answer, current.options.len())
        };
        match chosen {
            Err(why) => writeln!(out, "{why}")?,
            Ok(chosen) => match session.choose(index, group, &chosen) {
                Ok(()) => group += 1,
                Err(why) => writeln!(out, "Not that: {why}.")?,
            },
        }
    }
    Ok(StepAnswer::Next)
}

/// The choices made, `Step / Group / Option` each, for the record.
fn choices(session: &Session) -> Vec<String> {
    let mut out = Vec::new();
    let mut step = session.next_step(None);
    while let Some(i) = step {
        let view = session.view(i);
        for group in &view.groups {
            for option in group.options.iter().filter(|o| o.selected) {
                out.push(format!("{} / {} / {}", view.name, group.name, option.name));
            }
        }
        step = session.next_step(Some(i));
    }
    out
}

/// Unpacks into a fresh folder beside the installed ones (deleted on a failure), writes the
/// record, gives the folder its place, and moves the archive to `installed archives`.
fn install(
    out: Out,
    places: &Places,
    archive: &Path,
    name: &str,
    wanted: Wanted,
    mut record: Record,
) -> std::io::Result<()> {
    let folder = match installs::next_folder(&places.installs, name) {
        Ok(f) => f,
        Err(e) => return writeln!(out, "The mods folder could not be read: {e}"),
    };
    let mut staging_name = folder.file_name().unwrap_or_default().to_os_string();
    staging_name.push(".part");
    let staging = places.installs.join(staging_name);
    if staging.symlink_metadata().is_ok() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    let file_name = archive
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    writeln!(out, "Installing {name}...")?;
    let done = match mod_archive::extract(archive, &wanted, &staging, mod_archive::Limits::default())
    {
        Ok(done) => done,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&staging);
            return writeln!(out, "Nothing was installed.\n{}", e.player_text());
        }
    };
    record.archive = file_name.clone();
    record.archive_size = std::fs::metadata(archive).map_or(0, |m| m.len());
    record.files = wanted.iter().map(|(_, d)| d.replace('\\', "/")).collect();
    let finish = std::fs::write(staging.join(mod_install::RECORD), record.text())
        .and_then(|()| std::fs::rename(&staging, &folder));
    if let Err(e) = finish {
        let _ = std::fs::remove_dir_all(&staging);
        return writeln!(out, "Nothing was installed: {e}");
    }
    writeln!(
        out,
        "Installed {name} ({} files). It is used the next time you choose Play with mods.",
        done.written.len()
    )?;
    let kept = places.loose.join("installed archives");
    let moved = std::fs::create_dir_all(&kept)
        .and_then(|()| std::fs::rename(archive, kept.join(&file_name)));
    if moved.is_err() {
        writeln!(
            out,
            "(The archive could not be moved to {}; you can move or delete it yourself.)",
            kept.display()
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
