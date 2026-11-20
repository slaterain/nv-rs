//! The repository's checks. The crate is its test `tests/repo_hygiene.rs` (`cargo test -p
//! nvinspect --test repo_hygiene`): what must never be committed. Until 2026-11-03 it was also a
//! command-line inspector of the game's data through the removed hand-written layer
//! (docs/adr/0011-old-layer-removed.md).
//!
//! [`vcall_words`] (the program `vcall-words`) compares the argument words of the crates' virtual
//! calls with the vtables of the contributor's own exe.

pub mod vcall_words;
