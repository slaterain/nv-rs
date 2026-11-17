//! UI Organizer's patching of a target file (`UIOLoad`, `ui_organizer.h` (UIO), its second half):
//! the `<include src="..."/>` lines put into the file's text where its lists say, in memory.

use crate::hash::{begins_ci, lower, str_hash, Map, Set};

/// `kTileTypeNames` (UIO): the closing tags that end a tile.
const TILE_TYPES: [&[u8]; 8] = [
    b"menu",
    b"rect",
    b"image",
    b"text",
    b"hotrect",
    b"nif",
    b"radial",
    b"template",
];

fn is_tile_type(name: &[u8]) -> bool {
    let hash = str_hash(name);
    TILE_TYPES.iter().any(|t| str_hash(t) == hash)
}

/// `XMLCache::WriteInclude` (UIO): `<include src="path"/>`.
pub fn write_include(out: &mut Vec<u8>, path: &[u8]) {
    out.extend_from_slice(b"<include src=\"");
    out.extend_from_slice(path);
    out.extend_from_slice(b"\"/>");
}

/// `FindChr` (UIO) on a C string: the first `c` at or after `from`, stopping at a 0.
fn find_chr(d: &[u8], from: usize, c: u8) -> Option<usize> {
    let mut i = from;
    while i < d.len() && d[i] != 0 {
        if d[i] == c {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// `StrLen` (UIO) of the C string at `from`.
fn str_len(d: &[u8], from: usize) -> usize {
    d[from.min(d.len())..]
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(d.len() - from.min(d.len()))
}

// Translated from UIO 2.30 (426d963), utility.cpp: SubStrCI.
/// `SubStrCI(src, sub, length)` (UIO): `sub` searched without case in the C string at `src`, at
/// the positions 0 to `StrLen(src) - StrLen(sub) - length` (none when that is negative). UIO
/// passes the tag's length less 4 for `length`, so the search runs on to near the end of the file,
/// past the tag.
fn sub_str_ci(d: &[u8], src: usize, sub: &[u8], length: usize) -> Option<usize> {
    let last = str_len(d, src) as i64 - sub.len() as i64 - length as i64;
    let mut at = 0i64;
    while at <= last {
        let p = src + at as usize;
        if sub
            .iter()
            .enumerate()
            .all(|(i, &c)| d.get(p + i).map(|&b| lower(b)) == Some(lower(c)))
        {
            return Some(p);
        }
        at += 1;
    }
    None
}

// Translated from UIO 2.30 (426d963), ui_organizer.h: UIOLoad (the patching loop).
/// The target file's text (as `Menu::CompressAndStoreXML` gives it) with the includes of `tiles`
/// (tile name -> paths; "" = the end of the file) put in, as UIO writes it:
///
/// - The text is walked from `<` to the next `>`. A tag of 4 bytes or more:
///   - a closing tag (`</name>`) whose name is a tile type ends the tile last opened: the paths
///     of that tile are written (its set's order) before the text since the last tag written and
///     this tag; the tile leaves the lists; then, with no tile left, the rest of the text is
///     copied and the walk ends;
///   - a tag of 12 bytes or more with `name=` found by [`sub_str_ci`] from its fifth byte (which
///     can find one in a later tag) and a quoted value inside the tag opens that tile (a
///     self-closing tag opens one too, which UIO never closes); without `name=` found, a tag of 20
///     bytes or more starting `include src` (no case) inside an open tile is dropped when its path
///     is already one of the tile's paths or starts `includes_`.
/// - The text after the last `>` is not copied (UIO's loop ends at the last tag).
/// - The end-of-file paths are written last.
pub fn patch(text: &[u8], tiles: Map<Set>) -> Vec<u8> {
    patch_noted(text, tiles).0
}

/// A listed tile UIO's walk never closed: its name, its count of paths, whether a tag of that
/// name opened it.
pub type Unclosed = (Vec<u8>, usize, bool);

/// [`patch`], with the named tiles never closed in the text (each with its count of paths, which
/// were not written: UIO drops them without a word, and whether a tag of that name opened it, so
/// that UIO's walk met it but never reached its end).
pub fn patch_noted(text: &[u8], mut tiles: Map<Set>) -> (Vec<u8>, Vec<Unclosed>) {
    let length = text.len();
    let mut d = text.to_vec();
    d.push(0);
    let mut out = Vec::with_capacity(length + 256);
    let mut buf = 0usize;
    let mut open: Vec<Vec<u8>> = Vec::new();
    // The listed tiles a named tag opened (for the report only).
    let mut met: Vec<Vec<u8>> = Vec::new();
    'walk: while let Some(lt) = find_chr(&d, buf, b'<') {
        let bgn = lt + 1;
        let Some(end) = find_chr(&d, bgn, b'>') else {
            break;
        };
        let len = end - bgn;
        if len >= 4 {
            if d[bgn] == b'/' {
                if !open.is_empty() && is_tile_type(&d[bgn + 1..end]) {
                    let tile = open.pop().unwrap_or_default();
                    if let Some(paths) = tiles.remove(&tile) {
                        for p in paths.iter() {
                            write_include(&mut out, p);
                        }
                    }
                }
                if tiles.is_empty() {
                    out.extend_from_slice(&d[buf..length]);
                    break 'walk;
                }
            } else if len >= 12 {
                let (kind, from) = if let Some(at) = sub_str_ci(&d, bgn + 4, b"name=", len - 4) {
                    (1, at + 5)
                } else if len >= 20 && !open.is_empty() && begins_ci(&d[bgn..], b"include src") {
                    (2, bgn + 12)
                } else {
                    (0, 0)
                };
                if kind != 0 {
                    if let Some(q) = find_chr(&d, from, b'"') {
                        let q1 = q + 1;
                        if q1 < end {
                            if let Some(q2) = find_chr(&d, q1, b'"') {
                                if q2 < end && q2 > q1 {
                                    let value = d[q1..q2].to_vec();
                                    if kind == 1 {
                                        if tiles.get(&value).is_some() {
                                            met.push(value.clone());
                                        }
                                        open.push(value);
                                    } else {
                                        let top =
                                            open.last().map(Vec::as_slice).unwrap_or_default();
                                        let listed =
                                            tiles.get(top).is_some_and(|p| p.contains(&value));
                                        if listed || begins_ci(&value, b"includes_") {
                                            buf = end + 1;
                                            continue 'walk;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let next = end + 1;
        out.extend_from_slice(&d[buf..next]);
        buf = next;
    }
    let mut missing = Vec::new();
    for (tile, paths) in tiles.iter() {
        if tile.is_empty() {
            for p in paths.iter() {
                write_include(&mut out, p);
            }
        } else {
            let opened = met.iter().any(|m| m.eq_ignore_ascii_case(tile));
            missing.push((tile.to_vec(), paths.len(), opened));
        }
    }
    (out, missing)
}

#[cfg(test)]
#[path = "patch_tests.rs"]
mod tests;
