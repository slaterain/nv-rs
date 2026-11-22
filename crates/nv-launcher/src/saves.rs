//! Importing saves: the player's saves COPIED into nv-rs's own folder, never moved, never changed
//! where they are. One rule on every system, play.sh's (`copy_saves` there): each save (`.fos`)
//! goes with its script extender co-save (`.nvse`, same name) when there is one; the same save
//! already there (and the same co-save, or none on either side) is left alone; a different save
//! (or co-save) of the same name is replaced, both together, only when the player says yes; a
//! co-save left over from another save goes when its save is replaced by one without. A pair
//! lands whole or not at all ([`put_pair`]): both copied beside their places first, the old pair
//! moved aside, the save moved in last, the old pair put back on any failure.
//! Windows' Play.exe (`--import-saves`) uses it; play.sh keeps its own copy of the rule in shell.

use std::path::{Path, PathBuf};

/// What an import did.
#[derive(Debug, Default, PartialEq)]
pub struct Outcome {
    /// Saves copied (each with its co-save when it has one).
    pub copied: usize,
    /// Different saves already there, kept on the player's no.
    pub kept: usize,
    /// Saves that could not be copied, with why.
    pub failed: Vec<String>,
}

/// The saves (`.fos`, any case) in `dir`, by name.
pub fn saves_in(dir: &Path) -> Vec<PathBuf> {
    let mut saves: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("fos"))
                })
                .collect()
        })
        .unwrap_or_default();
    saves.sort();
    saves
}

/// The file in `dir` named `name` in any case (as play.sh's `child` finds it).
fn child(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.is_file() {
        return Some(exact);
    }
    std::fs::read_dir(dir).ok()?.flatten().find_map(|e| {
        let p = e.path();
        (p.is_file() && e.file_name().to_string_lossy().eq_ignore_ascii_case(name)).then_some(p)
    })
}

/// Whether two files hold the same bytes.
fn same(a: &Path, b: &Path) -> bool {
    match (std::fs::read(a), std::fs::read(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// `path` with `tail` added to its name (`Save 1.fos` -> `Save 1.fos.part`).
fn with_tail(path: &Path, tail: &str) -> PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(tail);
    PathBuf::from(p)
}

/// One save (and its co-save, `co`, or none) put at `target` (and `target_co`) as a whole pair or
/// not at all: both copied beside their places first (`.part`); the old pair moved aside
/// (`.old`); the co-save moved in, then the save LAST (the file the game lists); on any failure
/// what was moved in is removed and the old pair put back. `Err` names what failed.
fn put_pair(save: &Path, co: Option<&Path>, target: &Path, target_co: &Path) -> Result<(), String> {
    let name = |p: &Path| {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let (part, part_co) = (with_tail(target, ".part"), with_tail(target_co, ".part"));
    let clear_parts = || {
        let _ = std::fs::remove_file(&part);
        let _ = std::fs::remove_file(&part_co);
    };
    clear_parts();
    let copied = std::fs::copy(save, &part)
        .map_err(|e| format!("{}: {e}", name(save)))
        .and_then(|_| match co {
            Some(c) => std::fs::copy(c, &part_co)
                .map(|_| ())
                .map_err(|e| format!("{}: {e}", name(c))),
            None => Ok(()),
        });
    if let Err(why) = copied {
        clear_parts();
        return Err(why);
    }
    // The old pair aside, the co-save first.
    let mut aside: Vec<(PathBuf, PathBuf)> = Vec::new();
    // The old pair put back; each one that cannot be is named with where it is now.
    let put_back = |aside: &[(PathBuf, PathBuf)]| -> String {
        let mut said = String::new();
        for (old, place) in aside.iter().rev() {
            if let Err(e) = std::fs::rename(old, place) {
                said.push_str(&format!(
                    "; could not put back {} (it is at {}): {e}",
                    name(place),
                    name(old)
                ));
            }
        }
        said
    };
    for place in [target_co, target] {
        if std::fs::symlink_metadata(place).is_ok() {
            let old = with_tail(place, ".old");
            let _ = std::fs::remove_file(&old);
            if let Err(e) = std::fs::rename(place, &old) {
                let back = put_back(&aside);
                clear_parts();
                return Err(format!("{}: {e}{back}", name(place)));
            }
            aside.push((old, place.to_path_buf()));
        }
    }
    // The new pair in: the co-save, then the save.
    let mut placed: Vec<&Path> = Vec::new();
    let moves: Vec<(&Path, &Path)> = match co {
        Some(_) => vec![(&part_co, target_co), (&part, target)],
        None => vec![(&part, target)],
    };
    for (from, to) in moves {
        if let Err(e) = std::fs::rename(from, to) {
            for p in &placed {
                let _ = std::fs::remove_file(p);
            }
            let back = put_back(&aside);
            clear_parts();
            return Err(format!("{}: {e}{back}", name(to)));
        }
        placed.push(to);
    }
    for (old, _) in &aside {
        let _ = std::fs::remove_file(old);
    }
    Ok(())
}

/// Copies the saves of `from` into `to` (made when needed) by the rule above; `replace` is asked,
/// with the save's name, before a different save already there is replaced.
pub fn copy_saves(
    from: &Path,
    to: &Path,
    replace: &mut dyn FnMut(&str) -> bool,
) -> std::io::Result<Outcome> {
    let saves = saves_in(from);
    let mut out = Outcome::default();
    if saves.is_empty() {
        return Ok(out);
    }
    std::fs::create_dir_all(to)?;
    for save in &saves {
        let (Some(name), Some(base)) = (save.file_name(), save.file_stem()) else {
            continue;
        };
        let name = name.to_string_lossy().into_owned();
        let co_name = format!("{}.nvse", base.to_string_lossy());
        let co = child(from, &co_name);
        let (target, target_co) = (to.join(&name), to.join(&co_name));
        let co_same = match &co {
            Some(c) => same(c, &target_co),
            None => !target_co.exists(),
        };
        if target.exists() && same(save, &target) && co_same {
            continue;
        }
        if (target.exists() || target_co.exists()) && !replace(&name) {
            out.kept += 1;
            continue;
        }
        // Counted as copied only when the whole pair landed; else the old pair is as it was.
        match put_pair(save, co.as_deref(), &target, &target_co) {
            Ok(()) => out.copied += 1,
            Err(why) => out.failed.push(why),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nv-saves-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn put(dir: &Path, name: &str, text: &str) {
        std::fs::write(dir.join(name), text).unwrap();
    }

    fn read(dir: &Path, name: &str) -> Option<String> {
        std::fs::read_to_string(dir.join(name)).ok()
    }

    /// play.sh's rule, as test-play.sh checks it: saves and co-saves copied, other files not;
    /// a different pair asked about once and kept on no, replaced together on yes; a stray
    /// co-save removed with its replaced save; the same save left alone; the source unchanged.
    #[test]
    fn saves_are_copied_by_play_shs_rule() {
        let (from, to) = (folder("from"), folder("to"));
        put(&from, "Save 1.fos", "one");
        put(&from, "Save 1.NVSE", "co");
        put(&from, "Save 2.fos", "two");
        put(&from, "notes.txt", "x");
        let mut asked = Vec::new();
        let out = copy_saves(&from, &to.join("Saves"), &mut |n| {
            asked.push(n.to_string());
            true
        })
        .unwrap();
        let to = to.join("Saves");
        assert_eq!(out.copied, 2);
        assert!(asked.is_empty(), "nothing was there to replace");
        assert_eq!(read(&to, "Save 1.fos").as_deref(), Some("one"));
        assert_eq!(read(&to, "Save 1.nvse").as_deref(), Some("co"));
        assert_eq!(read(&to, "notes.txt"), None);
        assert!(!std::fs::read_dir(&to)
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().ends_with(".part")));
        // Again: all the same, nothing asked or copied.
        let out = copy_saves(&from, &to, &mut |_| panic!("not asked")).unwrap();
        assert_eq!(out, Outcome::default());
        // A different pair: asked once, kept on no.
        put(&to, "Save 1.fos", "mine");
        put(&to, "Save 1.nvse", "mine-co");
        let mut n = 0;
        let out = copy_saves(&from, &to, &mut |name| {
            n += 1;
            assert_eq!(name, "Save 1.fos");
            false
        })
        .unwrap();
        assert_eq!((n, out.kept, out.copied), (1, 1, 0));
        assert_eq!(read(&to, "Save 1.fos").as_deref(), Some("mine"));
        assert_eq!(read(&to, "Save 1.nvse").as_deref(), Some("mine-co"));
        // Replaced together on yes.
        copy_saves(&from, &to, &mut |_| true).unwrap();
        assert_eq!(read(&to, "Save 1.fos").as_deref(), Some("one"));
        assert_eq!(read(&to, "Save 1.nvse").as_deref(), Some("co"));
        // A stray co-save goes when its save is replaced by one without.
        put(&to, "Save 2.fos", "mine");
        put(&to, "Save 2.nvse", "stray");
        copy_saves(&from, &to, &mut |_| true).unwrap();
        assert_eq!(read(&to, "Save 2.fos").as_deref(), Some("two"));
        assert_eq!(read(&to, "Save 2.nvse"), None);
        // The source as it was.
        assert_eq!(read(&from, "Save 1.fos").as_deref(), Some("one"));
        assert_eq!(read(&from, "notes.txt").as_deref(), Some("x"));
        let _ = std::fs::remove_dir_all(&from);
        let _ = std::fs::remove_dir_all(to.parent().unwrap());
    }

    /// A failure half-way leaves the old pair as it was and counts nothing as copied: the
    /// co-save's copy failing (its `.part` name taken by a folder), and the stray co-save that
    /// cannot be moved aside (its `.old` name taken by a folder with something in it).
    #[test]
    fn a_failure_half_way_leaves_the_old_pair() {
        let (from, to) = (folder("ffrom"), folder("fto"));
        put(&from, "Save 1.fos", "one");
        put(&from, "Save 1.nvse", "co");
        put(&from, "Save 2.fos", "two");
        put(&to, "Save 1.fos", "mine");
        put(&to, "Save 1.nvse", "mine-co");
        put(&to, "Save 2.fos", "mine2");
        put(&to, "Save 2.nvse", "stray");
        std::fs::create_dir_all(to.join("Save 1.nvse.part").join("x")).unwrap();
        std::fs::create_dir_all(to.join("Save 2.nvse.old").join("x")).unwrap();
        let out = copy_saves(&from, &to, &mut |_| true).unwrap();
        assert_eq!(out.copied, 0, "{out:?}");
        assert_eq!(out.failed.len(), 2, "{out:?}");
        assert_eq!(read(&to, "Save 1.fos").as_deref(), Some("mine"));
        assert_eq!(read(&to, "Save 1.nvse").as_deref(), Some("mine-co"));
        assert_eq!(read(&to, "Save 2.fos").as_deref(), Some("mine2"));
        assert_eq!(read(&to, "Save 2.nvse").as_deref(), Some("stray"));
        assert!(!to.join("Save 1.fos.part").exists() && !to.join("Save 2.fos.part").exists());
        assert!(!to.join("Save 1.fos.old").exists() && !to.join("Save 2.fos.old").exists());
        let _ = std::fs::remove_dir_all(&from);
        let _ = std::fs::remove_dir_all(&to);
    }

    /// No saves: nothing made, nothing asked.
    #[test]
    fn no_saves_make_nothing() {
        let from = folder("empty");
        let to = from.join("never");
        let out = copy_saves(&from, &to, &mut |_| panic!("not asked")).unwrap();
        assert_eq!(out, Outcome::default());
        assert!(!to.exists());
        assert!(!crate::place::has_saves(&from));
        put(&from, "a.FOS", "x");
        assert!(crate::place::has_saves(&from));
        let _ = std::fs::remove_dir_all(&from);
    }
}
