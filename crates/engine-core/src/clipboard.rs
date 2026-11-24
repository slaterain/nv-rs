//! Portable clipboard text boundary. Only explicit paste actions call this;
//! initialization never reads the clipboard. No provider means unavailable.
use crate::Engine;
use std::sync::OnceLock;

pub type Reader = fn(&mut Engine, usize) -> Option<Vec<u8>>;
static READER: OnceLock<Reader> = OnceLock::new();

pub fn install(reader: Reader) {
    let _ = READER.set(reader);
}

/// At most `max_bytes` of CF_TEXT-equivalent bytes, stopping at the first NUL.
pub fn read_ansi_prefix(e: &mut Engine, max_bytes: usize) -> Option<Vec<u8>> {
    if max_bytes == 0 {
        return Some(Vec::new());
    }
    READER.get().and_then(|read| read(e, max_bytes))
}
