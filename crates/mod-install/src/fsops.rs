//! Every change this crate makes on disk goes through [`act`], and only there: a test reads the
//! crate's source and fails if any other code removes, renames, copies, links, makes a folder or
//! opens a file for writing (`only_fsops_changes_the_disk`). Three rules, always on, for every
//! change:
//!
//! - **Names** ([`valid_relative`]): a path the build acts on inside its folder is made of plain
//!   parts only: no empty part, no `.` or `..`, no `\`, `:` or control character, nothing that is
//!   not a normal path component. A record or listing with anything else is not trusted.
//! - **Real location** ([`Guard`]): just before each change, the real location (links resolved) of
//!   the folder it happens in must be inside the guard's own folder's real location, and outside
//!   the game's folder and every folder the guard forbids. Otherwise nothing is changed and the
//!   build stops in words. The few files outside the folder this may write (the record beside it,
//!   the plugin list) are named in the guard exactly, and their folder too must be outside every
//!   forbidden folder. A guard whose folder and a forbidden folder are inside one another, either
//!   way round, is refused when it is made.
//! - **Fresh names only**: no existing name is ever opened for writing. A file is written or copied
//!   under a name just removed (as a name, never followed) and made new (`create_new`, which also
//!   refuses a link left at that name), so a link at that name never leads a write anywhere.
//!
//! What remains is a race with another program changing folders between the check and the change
//! (a hostile local process); that is accepted and not guarded against.

use std::path::{Component, Path, PathBuf};

/// One change on disk.
#[derive(Debug)]
pub enum Op<'a> {
    /// Remove one name: a file, or a link (never what it leads to). Gone already is fine.
    RemoveName(&'a Path),
    /// Remove a whole folder the guard owns, or a link standing in its place (never followed).
    RemoveTree(&'a Path),
    /// Make one folder (its parent must exist).
    MakeDir(&'a Path),
    /// Make a folder and the folders missing on its way, outside every forbidden folder (a
    /// forbidden folder itself may be made when missing: it is then empty).
    MakeDirAll(&'a Path),
    /// A hard link `to` naming the file `from`.
    HardLink { from: &'a Path, to: &'a Path },
    /// A symbolic link `to` leading to `from` (Linux and macOS).
    Symlink { from: &'a Path, to: &'a Path },
    /// A copy of `from` as the new name `to` (its time of change kept: the game orders plugins by
    /// it).
    Copy { from: &'a Path, to: &'a Path },
    /// A file written whole: under `temporary` (made new), synced, renamed over `to`.
    WriteWhole {
        to: &'a Path,
        temporary: &'a Path,
        bytes: &'a [u8],
    },
}

/// Where changes may happen.
#[derive(Debug, Clone)]
pub struct Guard {
    /// The folder every change happens inside (its real location).
    root: PathBuf,
    /// The folder as given (for removing it, or a link at its place, whole).
    root_given: PathBuf,
    /// Real locations no change may happen inside (the game's folder, the layers).
    forbidden: Vec<PathBuf>,
    /// The same, as given (learnt again by [`Guard::settle`]).
    forbidden_given: Vec<PathBuf>,
    /// Files outside the root that may be written whole (exact paths).
    extra: Vec<PathBuf>,
}

/// The real location of `path`: its nearest existing folder's, links resolved, with the rest of
/// the path (not there yet) after it. Only a part that is not there is walked past; any other
/// failure (no access, a link leading nowhere...) is an error, and the change is refused.
pub fn real(path: &Path) -> std::io::Result<PathBuf> {
    let mut rest = Vec::new();
    let mut at = path;
    loop {
        match std::fs::canonicalize(at) {
            Ok(found) => {
                let mut out = found;
                for part in rest.iter().rev() {
                    out.push(part);
                }
                return Ok(out);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        // A link leading nowhere is NotFound too: never walked past.
        if std::fs::symlink_metadata(at).is_ok() {
            return Err(std::io::Error::other(format!(
                "{} leads nowhere",
                at.display()
            )));
        }
        match (at.parent(), at.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_os_string());
                at = if parent.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    parent
                };
            }
            _ => {
                return Err(std::io::Error::other(format!(
                    "{} has no real location",
                    path.display()
                )))
            }
        }
    }
}

/// [`real`] with its failure in words.
fn real_or(path: &Path) -> Result<PathBuf, String> {
    real(path).map_err(|e| format!("{}: {e}", path.display()))
}

impl Guard {
    /// A guard for changes inside `root` (which must be a plain folder by the first change made
    /// in it), never inside any of `forbidden`; `extra` files may also be written whole. Refused
    /// when `root` and a forbidden folder are inside one another, either way round.
    pub fn new(root: &Path, forbidden: &[PathBuf], extra: &[PathBuf]) -> Result<Self, String> {
        let mut guard = Self {
            root: PathBuf::new(),
            root_given: root.to_path_buf(),
            forbidden: Vec::new(),
            forbidden_given: forbidden.to_vec(),
            extra: extra.to_vec(),
        };
        guard.settle()?;
        Ok(guard)
    }

    /// Learns the real locations again (after the root or a forbidden folder was made), with the
    /// same refusal as [`Guard::new`].
    pub fn settle(&mut self) -> Result<(), String> {
        // A link at the root's place is the root's name, not where it leads: its real location is
        // its folder's, with its name (the build removes the link itself, never followed).
        let link_at_root =
            std::fs::symlink_metadata(&self.root_given).is_ok_and(|m| m.file_type().is_symlink());
        self.root = match (
            link_at_root,
            self.root_given.parent(),
            self.root_given.file_name(),
        ) {
            (true, Some(parent), Some(name)) => real_or(parent)?.join(name),
            _ => real_or(&self.root_given)?,
        };
        self.forbidden = self
            .forbidden_given
            .iter()
            .map(|p| real_or(p))
            .collect::<Result<_, _>>()?;
        for no in &self.forbidden {
            if self.root.starts_with(no) || no.starts_with(&self.root) {
                return Err(format!(
                    "{} and {} are inside one another",
                    self.root.display(),
                    no.display()
                ));
            }
        }
        Ok(())
    }

    /// Whether the real location `place` is outside every forbidden folder.
    fn outside_forbidden(&self, place: &Path) -> Result<(), String> {
        for no in &self.forbidden {
            if place.starts_with(no) {
                return Err(format!(
                    "{} is inside the game's or a mod's own folder",
                    place.display()
                ));
            }
        }
        Ok(())
    }

    /// Whether `path`'s folder is, really, inside the root and outside every forbidden folder.
    fn allows(&self, path: &Path) -> Result<(), String> {
        let Some(parent) = path.parent() else {
            return Err(format!("{} has no folder", path.display()));
        };
        let place = real_or(parent)?;
        if !place.starts_with(&self.root) {
            return Err(format!(
                "{} is not inside {} (a link?)",
                place.display(),
                self.root.display()
            ));
        }
        self.outside_forbidden(&place)
    }
}

/// Whether `relative` (parts split by `/`) is made of plain parts only (see the module's text).
pub fn valid_relative(relative: &str) -> bool {
    !relative.is_empty() && relative.split('/').all(valid_part)
}

/// One plain part: not empty, not `.` or `..`, no `\`, `:`, `/` or control character, and one
/// normal path component on this system.
pub fn valid_part(part: &str) -> bool {
    !part.is_empty()
        && part != "."
        && part != ".."
        && !part
            .chars()
            .any(|c| c.is_control() || matches!(c, '\\' | ':' | '/'))
        && {
            let mut components = Path::new(part).components();
            matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
        }
}

fn refused(why: String) -> std::io::Error {
    std::io::Error::other(format!("refused: {why}"))
}

/// Removes the name `path` (a file or a link, never followed); gone already is fine.
fn remove_name(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Opens `path` as a new file: an existing name (a link too) is refused, never opened.
fn create_new(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
}

/// Makes the change `op` after the guards (see the module's text).
pub fn act(guard: &Guard, op: Op) -> std::io::Result<()> {
    match op {
        Op::RemoveTree(path) => {
            if path != guard.root_given {
                return Err(refused(format!(
                    "{} is not the folder's own",
                    path.display()
                )));
            }
            match std::fs::symlink_metadata(path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e),
                // A link at the folder's place: removed itself, never followed.
                Ok(meta) if meta.file_type().is_symlink() => {
                    std::fs::remove_file(path).or_else(|_| std::fs::remove_dir(path))
                }
                Ok(meta) if !meta.is_dir() => std::fs::remove_file(path),
                Ok(_) => {
                    guard.allows(&path.join("x")).map_err(refused)?;
                    std::fs::remove_dir_all(path)
                }
            }
        }
        Op::RemoveName(path) => {
            guard.allows(path).map_err(refused)?;
            remove_name(path)
        }
        Op::MakeDir(path) => {
            if path == guard.root_given {
                // Checked against every forbidden folder, either way round, by the guard.
                return std::fs::create_dir_all(path);
            }
            guard.allows(path).map_err(refused)?;
            std::fs::create_dir(path)
        }
        Op::MakeDirAll(path) => {
            let place = real_or(path).map_err(refused)?;
            if !guard.forbidden.contains(&place) {
                guard.outside_forbidden(&place).map_err(refused)?;
            }
            std::fs::create_dir_all(path)
        }
        Op::HardLink { from, to } => {
            guard.allows(to).map_err(refused)?;
            std::fs::hard_link(from, to)
        }
        Op::Symlink { from, to } => {
            guard.allows(to).map_err(refused)?;
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(from, to)
            }
            #[cfg(not(unix))]
            {
                let _ = from;
                Err(std::io::Error::other(
                    "symbolic links are not made on this system",
                ))
            }
        }
        Op::Copy { from, to } => {
            guard.allows(to).map_err(refused)?;
            let mut source = std::fs::File::open(from)?;
            let changed = source.metadata()?.modified();
            let mut file = create_new(to)?;
            std::io::copy(&mut source, &mut file)?;
            if let Ok(changed) = changed {
                file.set_modified(changed)?;
            }
            Ok(())
        }
        Op::WriteWhole {
            to,
            temporary,
            bytes,
        } => {
            let folder = to.parent();
            if !guard.extra.iter().any(|e| e == to) || temporary.parent() != folder {
                return Err(refused(format!("{} may not be written", to.display())));
            }
            let Some(folder) = folder else {
                return Err(refused(format!("{} has no folder", to.display())));
            };
            guard
                .outside_forbidden(&real_or(folder).map_err(refused)?)
                .map_err(refused)?;
            use std::io::Write;
            remove_name(temporary)?;
            let mut file = create_new(temporary)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(temporary, to)
        }
    }
}

/// Removes an installed mod's whole folder: `folder` must be directly inside `installs` and a
/// plain folder, and `installs` a plain folder too (a link at either is refused).
pub fn remove_mod_folder(installs: &Path, folder: &Path) -> std::io::Result<()> {
    for place in [installs, folder] {
        let meta = std::fs::symlink_metadata(place)?;
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(refused(format!(
                "{} is not a plain folder",
                place.display()
            )));
        }
    }
    if real(folder)?.parent() != Some(real(installs)?.as_path()) {
        return Err(refused(format!(
            "{} is not inside {}",
            folder.display(),
            installs.display()
        )));
    }
    std::fs::remove_dir_all(folder)
}
