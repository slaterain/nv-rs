//! The game's profile files as nv-rs prepares them: its own sections in `Fallout.ini`
//! ([`game_ini`]) and the play settings' lines in `FalloutPrefs.ini` ([`settings`]). Moved
//! unchanged from Play.exe's source (`crates/nv-launcher`, `play/`), which uses them as before, so that the
//! host's first start on Linux and macOS (no Play.exe there) prepares the same files the same way.
//! No dependencies; nothing here writes to the disk (only [`settings::migrated_name`] looks at it).
pub mod game_ini;
pub mod settings;
