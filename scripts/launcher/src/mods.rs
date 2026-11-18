//! "Play with mods": the game reads a whole game folder made in `userdata\modgame`, never the
//! player's own folder: the game's files linked there, the installed mods
//! (`userdata\mod installs\<NN Name>`, in their order) and then the package's loose `mods` folder
//! linked over them, only what changed since the last start redone. The work is
//! `crates/mod-install` (`overlay::build`); this only names Play.exe's folders.
use std::path::Path;

pub use mod_install::overlay::{Built, ModError};

/// Makes `install` (removed first) from the game's folder, the mods installed in `installs` and the
/// loose `mods` folder, and writes the plugin list to `plugins_txt`: `mods\plugins.txt` as it is
/// when there is one, else the list made from the mods' plugins.
pub fn build_install(
    game: &Path,
    installs: &Path,
    mods: &Path,
    install: &Path,
    plugins_txt: &Path,
) -> Result<Built, ModError> {
    let layers = mod_install::installs::layers(installs, mods).map_err(|e| ModError {
        kind: mod_install::overlay::classify(e.raw_os_error(), false),
        step: format!("reading {}: {e}", installs.display()),
    })?;
    mod_install::overlay::build(game, &layers, install, plugins_txt)
}
