//! The game's menus, one module each, each following its menu class's code
//! in FalloutNV.exe (filling its tiles, answering clicks and keys). The
//! game state they show comes from the caller; what the player chooses goes
//! back the same way. Findings with addresses:
//! `%USERPROFILE%\nv-re\findings\menus.md`.

pub mod barter;
pub mod chargen;
pub mod container;
pub mod dialog;
pub mod levelup;
pub mod message;
pub mod quantity;
pub mod sleepwait;
pub mod textedit;
pub mod traits;
pub mod vigor;

#[cfg(test)]
pub(crate) mod test_support;
