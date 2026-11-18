//! FOMOD mod installers: the `fomod` folder of a downloaded mod (`ModuleConfig.xml`, the
//! installer; `info.xml`, its name and author), read and run as the mod managers run it, so the
//! port can install such a mod into its own folder. Pure functions: nothing here opens an
//! archive, shows a window or touches the game. A front end (the launcher today, the terminal
//! launcher later) gives the archive's file listing and the player's answers and gets back the
//! file copies to make.
//!
//! - [`text::decode`]: the bytes of an XML file as text, whatever encoding it really has (UTF-16
//!   with or without a byte order mark, UTF-8, Windows-1252); the encoding a file declares is
//!   ignored, as it is often wrong.
//! - [`xml::parse`]: a small reader that takes what the managers take, and a little more (a bare
//!   `&` in a description is kept as written instead of refusing the file).
//! - [`config::parse_config`] and [`config::parse_info`]: the installer as a [`config::Config`]:
//!   steps, groups, options, conditions and files; every schema version from 1.0 (FOMM's
//!   "dependancy" spellings, one step) to 5.x, in one tolerant pass.
//! - [`layout::layout`]: where in an archive's listing the installer and the files are (the
//!   `fomod` folder is often one folder down), or that there is no installer.
//! - [`install::Session`]: the steps a player sees, the default choices, checking a choice, and
//!   [`install::Session::plan`]: the files to copy, conflicts already decided.
//!
//! # Limits
//!
//! A download may be hostile. Every count that comes from it has a cap far above any real mod; a
//! file past one is refused with an error that names it, never cut short (a mod is never
//! installed by half). Every per-item lookup goes through an index, so the work stays in
//! proportion to the size.
//!
//! | Cap | Number | Why that number |
//! |---|---|---|
//! | `config::MAX_FILE_BYTES` | 8 MB | real `ModuleConfig.xml` files are a few hundred KB |
//! | `xml::MAX_DEPTH` | 256 levels | real installers nest under 15; a deeper tree could overflow the stack |
//! | `xml::MAX_ATTRIBUTES` | 64 a tag | real tags have at most 6 |
//! | `config::MAX_STEPS` | 1,024 | real installers have tens |
//! | `config::MAX_GROUPS` | 10,000 | real installers have tens |
//! | `config::MAX_OPTIONS_PER_GROUP` | 2,000 | real groups have a few, at most hundreds |
//! | `config::MAX_OPTIONS` | 20,000 | real installers have tens to hundreds |
//! | `config::MAX_FILE_ITEMS` | 200,000 | real installers have hundreds |
//! | `config::MAX_CONDITIONS` | 50,000 flags and conditions | real installers have hundreds |
//! | `config::MAX_CONDITION_DEPTH` | 32 | real conditions nest two or three deep |
//! | `layout::MAX_ENTRIES` | 500,000 files | a big texture mod has tens of thousands |
//! | `layout::MAX_PATH_PARTS` | 64 | the game's deepest Data paths have under 12 |
//! | `layout::MAX_PATH_BYTES` | 1,024 | Windows' own limit for most programs is 260 characters |
//! | `install::MAX_COPIES` | 500,000 | as many as the listing may hold files |
//!
//! Indexed: the player's files ([`install::Known`], one lookup per file condition), each group's
//! choices (one flag per option), the archive's listing ([`layout::Listing`]: one lookup per file,
//! one range per folder, one per wrapper level when finding the installer), repeated identical
//! file items (resolved once), and the flags (one map while walking the steps).
//!
//! Behaviour follows the schema (fomod-schema, MIT) and what Mod Organizer 2's and Vortex's
//! installers do, read for behaviour only (both GPL-3.0; nothing copied): where they differ, the
//! choice made is said at the place it is made. The census and design are in the measurement
//! folder's `fomod\CENSUS.md`.

pub mod config;
pub mod install;
pub mod layout;
pub mod text;
pub mod xml;

#[cfg(test)]
mod tests;
