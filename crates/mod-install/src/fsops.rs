//! Every change this crate makes on disk goes through [`act`], and only there: a test reads the
//! crate's source and fails if any other code removes, renames, copies or links a file
//! (`only_fsops_changes_the_disk`). Two guards, both always on, for every change:
//!
//! - **Names** ([`valid_relative`]): a path the build acts on inside its folder is made of plain
//!   parts only: no empty part, no `.` or `..`, no `\`, `:` or control character, nothing that is
//!   not a normal path component. A record or listing with anything else is not trusted.
//! - **Real location** ([`Guard`]): just before each change, the real location (links resolved) of
//!   the folder it happens in must be inside the guard's own folder's real location, and outside
//!   the game's folder and every folder the guard forbids. Otherwise nothing is changed and the
//!   build stops in words. The few files outside the folder this may write (the record beside it,
//!   the plugin list) are named in the guard exactly.
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
    /// A hard link `to` naming the file `from`.
    HardLink { from: &'a Path, to: &'a Path },
    /// A symbolic link `to` leading to `from` (Linux and macOS).
    Symlink { from: &'a Path, to: &'a Path },
    /// A copy of `from` as `to`.
    Copy { from: &'a Path, to: &'a Path },
    /// A file written whole: under `temporary`, synced, renamed over `to`.
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
    /// Files outside the root that may be written whole (exact paths).
    extra: Vec<PathBuf>,
}

fn real(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

impl Guard {
    /// A guard for changes inside `root` (which must be a plain folder by the first change made
    /// in it), never inside any of `forbidden`; `extra` files may also be written whole.
    pub fn new(root: &Path, forbidden: &[PathBuf], extra: &[PathBuf]) -> Self {
        Self {
            root: real(root),
            root_given: root.to_path_buf(),
            forbidden: forbidden.iter().map(|p| real(p)).collect(),
            extra: extra.to_vec(),
        }
    }

    /// Learns the root's real location again (after it was made).
    pub fn settle(&mut self) {
        self.root = real(&self.root_given);
    }

    /// Whether `path`'s folder is, really, inside the root and outside every forbidden folder.
    fn allows(&self, path: &Path) -> Result<(), String> {
        let Some(parent) = path.parent() else {
            return Err(format!("{} has no folder", path.display()));
        };
        let place = real(parent);
        if !place.starts_with(&self.root) {
            return Err(format!(
                "{} is not inside {} (a link?)",
                place.display(),
                self.root.display()
            ));
        }
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
            match std::fs::remove_file(path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                other => other,
            }
        }
        Op::MakeDir(path) => {
            if path == guard.root_given {
                return std::fs::create_dir_all(path);
            }
            guard.allows(path).map_err(refused)?;
            std::fs::create_dir(path)
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
            std::fs::copy(from, to).map(|_| ())
        }
        Op::WriteWhole {
            to,
            temporary,
            bytes,
        } => {
            if !guard.extra.iter().any(|e| e == to) || temporary.parent() != to.parent() {
                return Err(refused(format!("{} may not be written", to.display())));
            }
            use std::io::Write;
            let mut file = std::fs::File::create(temporary)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(temporary, to)
        }
    }
}

/// Removes an installed mod's whole folder: `folder` must be directly inside `installs` and a
/// plain folder (a link is refused).
pub fn remove_mod_folder(installs: &Path, folder: &Path) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(folder)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(refused(format!(
            "{} is not a plain folder",
            folder.display()
        )));
    }
    if real(folder).parent() != Some(real(installs).as_path()) {
        return Err(refused(format!(
            "{} is not inside {}",
            folder.display(),
            installs.display()
        )));
    }
    std::fs::remove_dir_all(folder)
}
