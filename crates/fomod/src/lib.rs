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
