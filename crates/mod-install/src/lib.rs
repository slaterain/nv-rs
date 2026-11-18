//! The port's own mods folders, the same on Windows, Linux and macOS (the lead's decisions,
//! 2026-11-17, measurement folder `fomod\CENSUS.md`, "Design"):
//!
//! - **Installed mods** ([`installs`]): `mod installs/<NN Name>/` in nv-rs's own folder, one
//!   folder per mod holding exactly the files it installs, laid out as in Data, and its record
//!   ([`RECORD`]: what was installed from which archive, with which choices, which files). The
//!   number is the order: a later mod wins a file both have. Removing a mod is removing its folder.
//! - **The game folder "Play with mods" plays from** ([`overlay::build`]): every file of the game's
//!   folder linked into a folder of nv-rs's own (never copied, only ever read), then each layer's
//!   files copied over it, in order: the installed mods, then the loose `mods` folder the player
//!   fills by hand (so a hand-placed file wins). The link is removed before a copy goes over it,
//!   so nothing is ever written through to the game's own file. It also writes the plugin list.
//!
//! Play.exe (Windows, `scripts/launcher`) and `play.sh --mods` (Linux and macOS) both make the
//! game folder through [`overlay::build`]; the mod chooser installs and removes through
//! [`installs`]. Nothing here reads an archive or shows anything: `crates/mod-archive` unpacks,
//! `crates/fomod` decides what goes where.
//!
//! # Limits
//!
//! Every count has a cap far above any real set-up; past one the build stops with words that
//! name it, never half done. Every lookup is indexed: each folder's names are read once while
//! files are placed in it, and plugins are kept by name.
//!
//! | Cap | Number | Why that number |
//! |---|---|---|
//! | `overlay::MAX_DEPTH` | 64 folders deep | the game's deepest Data paths have under 12; a link that loops back stops here |
//! | `overlay::MAX_FILES` | 500,000 files a layer (and in the game's Data) | a big texture mod has tens of thousands |
//! | `overlay::MAX_HEADER_BYTES` | 1 MB read of a plugin | the header record with its masters comes first and is small |
//! | `installs::MAX_INSTALLS` | 10,000 mods | the largest mod lists players keep have a few thousand |

pub mod installs;
pub mod overlay;

/// The record in each installed mod's folder (never copied into the game's Data).
pub const RECORD: &str = "nv-rs install.txt";

#[cfg(test)]
mod tests;
