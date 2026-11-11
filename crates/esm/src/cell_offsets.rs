//! The exterior cell offset table of every world space record of a plugin, built from its
//! records in one pass (Cell Offset Generator's idea, by WallSoGB, GPL-3.0; nothing of its code
//! is used: it builds its tables by asking the game's own search for every grid square).
//!
//! **What the game does.** A world space record (`WRLD`) of a master file may carry an `OFST`
//! subrecord: one 32-bit offset per exterior cell of the grid its `NAM0` / `NAM9` corners span
//! (row by row, 0 for no cell), each from the world space record's first byte to the cell
//! record's. `TESWorldSpace::FindCellInFile` (`005854f0`, FalloutNV.exe 1.4.0.525) finds a cell
//! of a master file with an offset table by index; every other file (a plugin, or a master whose
//! table was stripped, as YUP's and TTW's are) it searches record by record from the world space
//! record: it enters the world's children group (group type 1), the exterior block groups (4)
//! whose label is the cell's block key or its sub-block key and the sub-block groups (5) whose
//! label is its sub-block key, skips the cell children groups (6, 8, 9), the other blocks and
//! sub-blocks and the records flagged "ignored" (`0x1000`, `TESFile::NextForm(1)`, `00472150`;
//! not after a skipped group, `00473660`), passes `TLOD` records, and stops at anything else. The
//! first `CELL` record whose `XCLC` grid position is the cell's is the one found (a persistent
//! cell, flag `0x400`, never is; a cell with no `XCLC` is at 0, 0).
//!
//! **What this does.** [`world_offsets`] reads a plugin's world space group once and, for each
//! world space record, the table that search gives: for every grid position a cell record can be
//! found at, the offset of the record the search finds (the walk is run once per block and
//! sub-block key, in the search's own order, so duplicates, misplaced cells and flags give the
//! same answer). A table is only made when it gives the same answer as the search for every
//! position: a found cell outside the record's corners, no corners, a grid of no or more than
//! [`MAX_CELLS`] squares, or a structure the search would read differently than this one
//! ([`Decline`]) leaves that world space without one.

use crate::inflate::zlib_decompress;
use std::collections::{BTreeSet, HashMap};
use std::io::{self, Read, Seek, SeekFrom};

/// The header of a record and of a group in a Fallout 3 / New Vegas plugin.
const HEADER_LEN: u64 = 24;
const GRUP: [u8; 4] = *b"GRUP";
const WRLD: [u8; 4] = *b"WRLD";
const CELL: [u8; 4] = *b"CELL";
/// Passed over by the search (form type 68's tag, `01187338`).
const TLOD: [u8; 4] = *b"TLOD";
/// A header-only record (`011873c8`): the reader does not skip its size.
const TOFT: [u8; 4] = *b"TOFT";
const XXXX: [u8; 4] = *b"XXXX";
const XCLC: [u8; 4] = *b"XCLC";
const NAM0: [u8; 4] = *b"NAM0";
const NAM9: [u8; 4] = *b"NAM9";
const OFST: [u8; 4] = *b"OFST";
/// Record flags: persistent (a cell that is never found), ignored (passed over by
/// `NextForm(1)`), compressed.
const PERSISTENT: u32 = 0x400;
const IGNORED: u32 = 0x1000;
const COMPRESSED: u32 = 0x4_0000;

/// The most grid squares a built table may have (16 MiB of offsets).
pub const MAX_CELLS: u64 = 1 << 22;

/// Why a world space record has no built table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decline {
    /// The record has no `NAM0` or no `NAM9` of 8 bytes: the game's grid would be the
    /// `OFFSET_DATA` defaults, which find nothing.
    NoCorners,
    /// The corners span no square, or more than [`MAX_CELLS`].
    GridSize,
    /// The search finds a cell outside the corners, which a table cannot give.
    CellOutsideGrid,
    /// The file has two world space records with this form ID.
    DuplicateRecord,
    /// A record the search would read (a cell or the world space) is compressed and does not
    /// inflate, or a header-only record (`TOFT`) is inside the world space group.
    Unreadable,
    /// Not built: a master's record with its own `OFST` (the game reads that one), and not all
    /// tables were asked for.
    NotAsked,
}

/// One world space record of a plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldOffsets {
    /// The offset of the record's header in the file (what `TESWorldSpace::Load` stores in
    /// `OFFSET_DATA::iFileOffset`).
    pub record_offset: u32,
    /// The form ID as the file stores it (its own master indices).
    pub form_id: u32,
    /// The record's flags.
    pub flags: u32,
    /// `NAM0` and `NAM9`: the south-west and north-east corners as the record stores them (the
    /// bits of two `f32`, x then y), the last of each.
    pub min_corner: Option<[u32; 2]>,
    pub max_corner: Option<[u32; 2]>,
    /// The record's own `OFST` table (the last one), if it has one.
    pub stored: Option<Vec<u32>>,
    /// The table built from the records, or why there is none.
    pub built: Result<Vec<u32>, Decline>,
}

impl WorldOffsets {
    /// The cell grid of the corners as the game works it out (`_ftol2` of each float, shifted
    /// right by 12): minimum x, minimum y, maximum x, maximum y.
    pub fn grid(&self) -> Option<(i32, i32, i32, i32)> {
        let (min, max) = (self.min_corner?, self.max_corner?);
        Some((
            cell_of(min[0]),
            cell_of(min[1]),
            cell_of(max[0]),
            cell_of(max[1]),
        ))
    }
}

/// `_ftol2_sse` of a float's bits (truncation; out of range or NaN gives `0x80000000`), shifted
/// right by 12 as the game does.
pub fn cell_of(bits: u32) -> i32 {
    let value = f32::from_bits(bits);
    let whole = if value.is_nan() || value >= 2_147_483_648.0 || value < -2_147_483_648.0 {
        i32::MIN
    } else {
        value as i32
    };
    whole >> 12
}

/// The index of the cell (`x`, `y`) in a table over `grid` (`00584530`), if inside.
pub fn grid_index(grid: (i32, i32, i32, i32), x: i32, y: i32) -> Option<usize> {
    let (min_x, min_y, max_x, max_y) = grid;
    if x > max_x || x < min_x || y > max_y || y < min_y {
        return None;
    }
    let width = i64::from(max_x) - i64::from(min_x) + 1;
    usize::try_from(width * (i64::from(y) - i64::from(min_y)) + i64::from(x) - i64::from(min_x)).ok()
}

/// The block and sub-block keys of a cell (`00544280`, `005442c0`: the coordinates shifted right
/// by 5 and by 3, packed by `00587410` as x high, y low, 16 bits each).
fn keys(x: i32, y: i32) -> (u32, u32) {
    let pack = |x: i32, y: i32| (u32::from(x as u16) << 16) | u32::from(y as u16);
    (pack(x >> 5, y >> 5), pack(x >> 3, y >> 3))
}

/// Every world space record of the plugin `file`, in file order, with the table built for each
/// the game would use (every record of a file that is not a master, and a master's records
/// with no `OFST` of their own: a master reads its own), or for all of them with `build_all`. An error is the file's: it could not be read, or it is not laid out as
/// a plugin (a group or record running past its group or the file).
pub fn world_offsets<R: Read + Seek>(file: R, build_all: bool) -> io::Result<Vec<WorldOffsets>> {
    let mut source = Source::new(file)?;
    let mut worlds = Vec::new();
    // The file header record, then the top groups.
    let mut at = 0;
    if source.len < HEADER_LEN {
        return Err(malformed("no file header"));
    }
    let header = source.header(at)?;
    // The file header record's flags: bit 0, a master (`TESFile::GetMaster`, `00471c20`).
    let master = header.flags() & 1 != 0;
    at += HEADER_LEN + u64::from(header.size);
    while at + HEADER_LEN <= source.len {
        let header = source.header(at)?;
        if header.tag != GRUP {
            // A record outside any group (a trailing `TOFT`): the loader's business, not ours.
            at += HEADER_LEN + u64::from(header.size);
            continue;
        }
        let end = at + u64::from(header.size);
        if u64::from(header.size) < HEADER_LEN || end > source.len {
            return Err(malformed("a top group runs past the file"));
        }
        if header.label == u32::from_le_bytes(WRLD) && header.form_id == 0 {
            let wanted = Wanted { master, build_all };
            let group = parse_group(&mut source, at + HEADER_LEN, end, wanted)?;
            worlds.extend(group.worlds());
        }
        at = end;
    }
    Ok(worlds)
}

fn malformed(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("plugin layout: {what}"))
}

/// A record or group header: the tag, the size field (a record's data size, a group's whole
/// size), the flags or group label, the form ID or group type.
struct Header {
    tag: [u8; 4],
    size: u32,
    label: u32,
    form_id: u32,
}

impl Header {
    fn flags(&self) -> u32 {
        self.label
    }
}

/// The plugin's bytes, read through a window (the search's walk skips most of the file).
struct Source<R> {
    file: R,
    len: u64,
    start: u64,
    window: Vec<u8>,
}

const WINDOW: usize = 1 << 16;

impl<R: Read + Seek> Source<R> {
    fn new(mut file: R) -> io::Result<Self> {
        let len = file.seek(SeekFrom::End(0))?;
        Ok(Source {
            file,
            len,
            start: 0,
            window: Vec::new(),
        })
    }

    /// `n` bytes at `at`, which must be inside the file.
    fn bytes(&mut self, at: u64, n: usize) -> io::Result<Vec<u8>> {
        match at.checked_add(n as u64) {
            Some(end) if end <= self.len => {}
            _ => return Err(malformed("a read past the end of the file")),
        }
        let inside = at >= self.start && at + n as u64 <= self.start + self.window.len() as u64;
        if !inside {
            if n > WINDOW {
                let mut out = vec![0; n];
                self.file.seek(SeekFrom::Start(at))?;
                self.file.read_exact(&mut out)?;
                return Ok(out);
            }
            let take = (self.len - at).min(WINDOW as u64) as usize;
            self.window.resize(take, 0);
            self.file.seek(SeekFrom::Start(at))?;
            self.file.read_exact(&mut self.window)?;
            self.start = at;
        }
        let from = (at - self.start) as usize;
        Ok(self.window[from..from + n].to_vec())
    }

    fn header(&mut self, at: u64) -> io::Result<Header> {
        let b = self.bytes(at, HEADER_LEN as usize)?;
        let word = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        Ok(Header {
            tag: [b[0], b[1], b[2], b[3]],
            size: word(4),
            label: word(8),
            form_id: word(12),
        })
    }

    /// A record's data as the game's reader gives its subrecords (inflated when compressed);
    /// `None` when it does not inflate.
    fn record_data(&mut self, at: u64, header: &Header) -> io::Result<Option<Vec<u8>>> {
        let raw = self.bytes(at + HEADER_LEN, header.size as usize)?;
        if header.flags() & COMPRESSED == 0 {
            return Ok(Some(raw));
        }
        if raw.len() < 4 {
            return Ok(None);
        }
        let size = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize;
        Ok(zlib_decompress(&raw[4..], Some(size))
            .ok()
            .filter(|out| out.len() == size))
    }
}

/// The subrecords of a record's data, in order: tag and data (`XXXX` gives the next one's size).
fn subrecords(data: &[u8]) -> Option<Vec<([u8; 4], &[u8])>> {
    let mut out = Vec::new();
    let mut at = 0;
    let mut long = None;
    while at < data.len() {
        let head = data.get(at..at + 6)?;
        let tag = [head[0], head[1], head[2], head[3]];
        let mut size = usize::from(u16::from_le_bytes([head[4], head[5]]));
        at += 6;
        if let Some(n) = long.take() {
            size = n;
        }
        let body = data.get(at..at + size)?;
        at += size;
        if tag == XXXX && size == 4 {
            long = Some(u32::from_le_bytes([body[0], body[1], body[2], body[3]]) as usize);
            continue;
        }
        out.push((tag, body));
    }
    Some(out)
}

/// One header of the world space group, in file order (a group's content follows it).
#[derive(Debug, Clone)]
enum Node {
    Group {
        kind: u32,
        label: u32,
        /// The index after the group's content.
        end: usize,
    },
    Record {
        tag: [u8; 4],
        flags: u32,
        offset: u32,
        /// A cell's grid position as the search reads it (`None`: persistent, never found).
        cell: Option<(i32, i32)>,
        depth: u32,
    },
    /// The end of the world space group: the next header is another top group's or the file's
    /// end, where the search stops.
    Stop,
}

struct WorldRecord {
    node: usize,
    offsets: WorldOffsets,
}

/// Which tables are built: all of them, or the ones the game needs (every world space record of
/// a file that is not a master, and those of a master with no `OFST`).
#[derive(Clone, Copy)]
struct Wanted {
    master: bool,
    build_all: bool,
}

impl Wanted {
    fn table(self, world: &WorldOffsets) -> bool {
        self.build_all || !self.master || world.stored.is_none()
    }
}

struct Group {
    nodes: Vec<Node>,
    worlds: Vec<WorldRecord>,
    unreadable: bool,
    wanted: Wanted,
}

/// Reads the world space group's content (`start..end`): every header the search can meet,
/// with the content of world children, block and sub-block groups (the only ones it enters).
fn parse_group<R: Read + Seek>(
    source: &mut Source<R>,
    start: u64,
    end: u64,
    wanted: Wanted,
) -> io::Result<Group> {
    let mut group = Group {
        nodes: Vec::new(),
        worlds: Vec::new(),
        unreadable: false,
        wanted,
    };
    parse_content(source, start, end, 0, &mut group)?;
    group.nodes.push(Node::Stop);
    Ok(group)
}

fn parse_content<R: Read + Seek>(
    source: &mut Source<R>,
    start: u64,
    end: u64,
    depth: u32,
    group: &mut Group,
) -> io::Result<()> {
    let mut at = start;
    // At the top: whether the last header was a world space record (not flagged ignored) with no
    // table wanted. The children group after it is not read: no other record's walk gets past it.
    let mut after_unwanted_world = false;
    while at < end {
        if at + HEADER_LEN > end {
            return Err(malformed("a header runs past its group"));
        }
        let header = source.header(at)?;
        let skip_children = std::mem::take(&mut after_unwanted_world);
        if header.tag == GRUP {
            let size = u64::from(header.size);
            if size < HEADER_LEN || at + size > end {
                return Err(malformed("a group runs past its group"));
            }
            let index = group.nodes.len();
            group.nodes.push(Node::Group {
                kind: header.form_id,
                label: header.label,
                end: 0,
            });
            let skipped = depth == 0 && header.form_id == 1 && skip_children;
            if matches!(header.form_id, 1 | 4 | 5) && !skipped {
                parse_content(source, at + HEADER_LEN, at + size, depth + 1, group)?;
            }
            let after = group.nodes.len();
            if let Node::Group { end, .. } = &mut group.nodes[index] {
                *end = after;
            }
            at += size;
            continue;
        }
        let next = at + HEADER_LEN + u64::from(header.size);
        if next > end {
            return Err(malformed("a record runs past its group"));
        }
        if header.tag == TOFT {
            group.unreadable = true;
        }
        let offset = u32::try_from(at).map_err(|_| malformed("an offset past 4 GiB"))?;
        let mut cell = None;
        if header.tag == CELL && header.flags() & PERSISTENT == 0 {
            match source.record_data(at, &header)? {
                Some(data) => cell = Some(grid_position(&data, &mut group.unreadable)),
                None => group.unreadable = true,
            }
        }
        if header.tag == WRLD {
            let offsets = match source.record_data(at, &header)? {
                Some(data) => world_record(offset, &header, &data),
                None => {
                    group.unreadable = true;
                    world_record(offset, &header, &[])
                }
            };
            after_unwanted_world = depth == 0
                && header.flags() & IGNORED == 0
                && !group.wanted.table(&offsets);
            group.worlds.push(WorldRecord {
                node: group.nodes.len(),
                offsets,
            });
        }
        group.nodes.push(Node::Record {
            tag: header.tag,
            flags: header.flags(),
            offset,
            cell,
            depth,
        });
        at = next;
    }
    Ok(())
}

/// The grid position the search reads from a non-persistent cell's data: its first `XCLC`'s
/// first two words, read into a zeroed 12-byte buffer (0, 0 without one).
fn grid_position(data: &[u8], unreadable: &mut bool) -> (i32, i32) {
    let Some(subs) = subrecords(data) else {
        *unreadable = true;
        return (0, 0);
    };
    let mut grid = [0u8; 12];
    if let Some((_, body)) = subs.iter().find(|(tag, _)| *tag == XCLC) {
        let n = body.len().min(12);
        grid[..n].copy_from_slice(&body[..n]);
    }
    let word = |i: usize| i32::from_le_bytes([grid[i], grid[i + 1], grid[i + 2], grid[i + 3]]);
    (word(0), word(4))
}

fn world_record(offset: u32, header: &Header, data: &[u8]) -> WorldOffsets {
    let mut world = WorldOffsets {
        record_offset: offset,
        form_id: header.form_id,
        flags: header.flags(),
        min_corner: None,
        max_corner: None,
        stored: None,
        built: Err(Decline::NotAsked),
    };
    let corner = |body: &[u8]| {
        (body.len() == 8).then(|| {
            [
                u32::from_le_bytes([body[0], body[1], body[2], body[3]]),
                u32::from_le_bytes([body[4], body[5], body[6], body[7]]),
            ]
        })
    };
    for (tag, body) in subrecords(data).unwrap_or_default() {
        match tag {
            NAM0 => world.min_corner = corner(body),
            NAM9 => world.max_corner = corner(body),
            OFST if !body.is_empty() => {
                world.stored = Some(
                    body.chunks_exact(4)
                        .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
                        .collect(),
                );
            }
            _ => {}
        }
    }
    world
}

impl Group {
    fn worlds(mut self) -> Vec<WorldOffsets> {
        let mut ids: HashMap<u32, usize> = HashMap::new();
        for world in &self.worlds {
            *ids.entry(world.offsets.form_id).or_default() += 1;
        }
        let mut out = Vec::with_capacity(self.worlds.len());
        for mut world in std::mem::take(&mut self.worlds) {
            if self.wanted.table(&world.offsets) {
                world.offsets.built = if self.unreadable {
                    Err(Decline::Unreadable)
                } else if ids[&world.offsets.form_id] > 1 {
                    Err(Decline::DuplicateRecord)
                } else {
                    self.build(&world)
                };
            }
            out.push(world.offsets);
        }
        out
    }

    /// The node after `at` the reader's `NextForm(1)` stops on: the next one, past records
    /// flagged ignored (not groups or `TOFT`).
    fn next_form(&self, at: usize) -> usize {
        let mut next = at + 1;
        while let Some(Node::Record { tag, flags, .. }) = self.nodes.get(next) {
            if *tag != TOFT && flags & IGNORED != 0 {
                next += 1;
            } else {
                break;
            }
        }
        next
    }

    /// The search's walk from the world space record `start` for the block key `block` and the
    /// sub-block key `sub_block` (`005854f0`), without stopping at a match: each cell position
    /// with the offset of the first cell record the walk meets there.
    fn walk(&self, start: usize, block: u32, sub_block: u32) -> HashMap<(i32, i32), u32> {
        let mut first = HashMap::new();
        let mut at = self.next_form(start);
        loop {
            match &self.nodes[at] {
                Node::Group { kind, label, end } => match kind {
                    1 => at = self.next_form(at),
                    4 if *label == block || *label == sub_block => at = self.next_form(at),
                    5 if *label == sub_block => at = self.next_form(at),
                    4 | 5 | 6 | 8 | 9 => at = *end,
                    _ => break,
                },
                Node::Record {
                    tag, cell, offset, ..
                } if *tag == CELL => {
                    if let Some(position) = cell {
                        first.entry(*position).or_insert(*offset);
                    }
                    at = self.next_form(at);
                }
                Node::Record { tag, .. } if *tag == TLOD => at = self.next_form(at),
                _ => break,
            }
        }
        first
    }

    fn build(&self, world: &WorldRecord) -> Result<Vec<u32>, Decline> {
        let grid = world.offsets.grid().ok_or(Decline::NoCorners)?;
        let (min_x, min_y, max_x, max_y) = grid;
        let width = i64::from(max_x) - i64::from(min_x) + 1;
        let height = i64::from(max_y) - i64::from(min_y) + 1;
        if width <= 0 || height <= 0 || (width * height) as u64 > MAX_CELLS {
            return Err(Decline::GridSize);
        }
        // Every cell the walk can meet lies before the first header at the world space record's
        // own level that stops every walk.
        let level = match &self.nodes[world.node] {
            Node::Record { depth, .. } => *depth,
            _ => return Err(Decline::Unreadable),
        };
        let mut candidates = BTreeSet::new();
        for node in &self.nodes[world.node + 1..] {
            match node {
                Node::Record { tag, cell, .. } if *tag == CELL => {
                    if let Some(position) = cell {
                        candidates.insert(*position);
                    }
                }
                Node::Record {
                    tag, flags, depth, ..
                } if *depth == level && *tag != TLOD && flags & IGNORED == 0 => break,
                Node::Stop => break,
                _ => {}
            }
        }
        let mut walks: HashMap<(u32, u32), HashMap<(i32, i32), u32>> = HashMap::new();
        let mut table = vec![0u32; (width * height) as usize];
        for (x, y) in candidates {
            let (block, sub_block) = keys(x, y);
            let found = walks
                .entry((block, sub_block))
                .or_insert_with(|| self.walk(world.node, block, sub_block))
                .get(&(x, y))
                .copied();
            let Some(offset) = found else { continue };
            let index = grid_index(grid, x, y).ok_or(Decline::CellOutsideGrid)?;
            table[index] = offset - world.offsets.record_offset;
        }
        Ok(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn sub(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = tag.to_vec();
        out.extend_from_slice(&(data.len() as u16).to_le_bytes());
        out.extend_from_slice(data);
        out
    }

    fn record(tag: &[u8; 4], flags: u32, form_id: u32, data: &[u8]) -> Vec<u8> {
        let mut out = tag.to_vec();
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&form_id.to_le_bytes());
        out.extend_from_slice(&[0; 8]);
        out.extend_from_slice(data);
        out
    }

    fn group(label: u32, kind: u32, content: &[Vec<u8>]) -> Vec<u8> {
        let body: Vec<u8> = content.concat();
        let mut out = GRUP.to_vec();
        out.extend_from_slice(&(24 + body.len() as u32).to_le_bytes());
        out.extend_from_slice(&label.to_le_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&[0; 8]);
        out.extend_from_slice(&body);
        out
    }

    fn cell_with(flags: u32, form_id: u32, x: i32, y: i32) -> Vec<u8> {
        let mut grid = x.to_le_bytes().to_vec();
        grid.extend_from_slice(&y.to_le_bytes());
        grid.extend_from_slice(&0u32.to_le_bytes());
        let data = [sub(b"EDID", b"c\0"), sub(b"XCLC", &grid)].concat();
        record(b"CELL", flags, form_id, &data)
    }

    fn cell(form_id: u32, x: i32, y: i32) -> Vec<u8> {
        cell_with(0, form_id, x, y)
    }

    /// The grid position a test cell record holds (after its header and its `EDID`).
    fn position_of(cell: &[u8]) -> (i32, i32) {
        let at = 24 + 8 + 6;
        let word = |i: usize| i32::from_le_bytes(cell[i..i + 4].try_into().unwrap());
        (word(at), word(at + 4))
    }

    fn corner(x: f32, y: f32) -> Vec<u8> {
        [x.to_le_bytes(), y.to_le_bytes()].concat()
    }

    /// A world space record with the corners of cells (-2, -2) and (1, 1): a 4 x 4 grid.
    fn world(form_id: u32) -> Vec<u8> {
        let data = [
            sub(b"EDID", b"w\0"),
            sub(b"NAM0", &corner(-8192.0, -8192.0)),
            sub(b"NAM9", &corner(8191.0, 8191.0)),
        ]
        .concat();
        record(b"WRLD", 0, form_id, &data)
    }

    const GRID: (i32, i32, i32, i32) = (-2, -2, 1, 1);

    fn slot(x: i32, y: i32) -> usize {
        grid_index(GRID, x, y).unwrap()
    }

    fn block(x: i32, y: i32) -> u32 {
        keys(x, y).0
    }

    fn sub_block(x: i32, y: i32) -> u32 {
        keys(x, y).1
    }

    /// The file: a header record, a weapon group, then the world space group with `content`.
    fn plugin(content: &[Vec<u8>]) -> Vec<u8> {
        let header = record(b"TES4", 1, 0, &sub(b"HEDR", &[0; 12]));
        let weapons = group(u32::from_le_bytes(*b"WEAP"), 0, &[record(b"WEAP", 0, 7, &[])]);
        [header, weapons, group(u32::from_le_bytes(WRLD), 0, content)].concat()
    }

    fn offsets(bytes: &[u8], build_all: bool) -> Vec<WorldOffsets> {
        world_offsets(Cursor::new(bytes), build_all).unwrap()
    }

    /// Where `needle` (a record) starts in `bytes`.
    fn at(bytes: &[u8], needle: &[u8]) -> u32 {
        bytes.windows(needle.len()).position(|w| w == needle).unwrap() as u32
    }

    /// A cell's children group (temporary references).
    fn cell_children() -> Vec<u8> {
        group(0x200, 6, &[group(0x200, 9, &[record(b"REFR", 0, 0x300, &[])])])
    }

    /// One world's children as the game's tools lay them out: the persistent cell and its
    /// children, then each cell (with its children) in its block and sub-block groups.
    fn children(form_id: u32, cells: &[Vec<u8>]) -> Vec<u8> {
        let persistent = cell_with(PERSISTENT, 0x100, 0, 0);
        let mut sub_blocks: Vec<(u32, u32, Vec<Vec<u8>>)> = Vec::new();
        for c in cells {
            let (x, y) = position_of(c);
            let (b, s) = (block(x, y), sub_block(x, y));
            let list = match sub_blocks.iter_mut().position(|(bb, ss, _)| *bb == b && *ss == s) {
                Some(i) => &mut sub_blocks[i].2,
                None => {
                    sub_blocks.push((b, s, Vec::new()));
                    &mut sub_blocks.last_mut().unwrap().2
                }
            };
            list.push(c.clone());
            list.push(cell_children());
        }
        let mut blocks: Vec<(u32, Vec<Vec<u8>>)> = Vec::new();
        for (b, s, list) in sub_blocks {
            let g = group(s, 5, &list);
            match blocks.iter_mut().position(|(bb, _)| *bb == b) {
                Some(i) => blocks[i].1.push(g),
                None => blocks.push((b, vec![g])),
            }
        }
        let mut content = vec![persistent, group(0x100, 6, &[group(0x100, 8, &[])])];
        content.extend(blocks.into_iter().map(|(b, list)| group(b, 4, &list)));
        group(form_id, 1, &content)
    }

    #[test]
    fn each_cell_is_found_where_the_search_finds_it() {
        let cells = [
            cell(0x10, -2, -2),
            cell(0x11, 1, 1),
            cell(0x12, 0, -1),
            cell(0x13, -1, 0),
        ];
        let bytes = plugin(&[world(0x50), children(0x50, &cells)]);
        let worlds = offsets(&bytes, false);
        assert_eq!(worlds.len(), 1);
        let w = &worlds[0];
        assert_eq!(w.record_offset, at(&bytes, &world(0x50)));
        assert_eq!(w.grid(), Some(GRID));
        let table = w.built.as_ref().unwrap();
        assert_eq!(table.len(), 16);
        for c in &cells {
            let (x, y) = position_of(c);
            assert_eq!(table[slot(x, y)], at(&bytes, c) - w.record_offset, "cell {x}, {y}");
        }
        assert_eq!(table.iter().filter(|&&o| o != 0).count(), 4);
    }

    #[test]
    fn a_cell_in_the_wrong_block_is_not_found() {
        // The cell (1, 1) inside the sub-block group of (-2, -2): the search for (1, 1) never
        // enters it.
        let content = group(
            0x50,
            1,
            &[group(
                block(-2, -2),
                4,
                &[group(
                    sub_block(-2, -2),
                    5,
                    &[cell(0x10, -2, -2), cell(0x11, 1, 1)],
                )],
            )],
        );
        let bytes = plugin(&[world(0x50), content]);
        let table = offsets(&bytes, false)[0].built.clone().unwrap();
        assert_ne!(table[slot(-2, -2)], 0);
        assert_eq!(table[slot(1, 1)], 0);
    }

    #[test]
    fn the_first_of_two_cells_at_one_position_is_found() {
        let first = cell(0x10, 0, 0);
        let second = cell(0x11, 0, 0);
        let bytes = plugin(&[world(0x50), children(0x50, &[first.clone(), second])]);
        let w = &offsets(&bytes, false)[0];
        let table = w.built.as_ref().unwrap();
        assert_eq!(table[slot(0, 0)], at(&bytes, &first) - w.record_offset);
    }

    #[test]
    fn an_ignored_cell_is_passed_over_unless_it_follows_a_skipped_group() {
        // Inside a sub-block entered with NextForm(1) the ignored cell is never read; right
        // after a block group the search skips, it is (the skip reads the next header as is).
        let ignored = cell_with(IGNORED, 0x10, 0, 0);
        let inside = group(
            0x50,
            1,
            &[group(
                block(0, 0),
                4,
                &[group(sub_block(0, 0), 5, &[ignored.clone()])],
            )],
        );
        let bytes = plugin(&[world(0x50), inside]);
        let table = offsets(&bytes, false)[0].built.clone().unwrap();
        assert_eq!(table[slot(0, 0)], 0);

        let after_skip = group(0x50, 1, &[group(block(-2, -2), 4, &[]), ignored.clone()]);
        let bytes = plugin(&[world(0x50), after_skip]);
        let w = &offsets(&bytes, false)[0];
        let table = w.built.as_ref().unwrap();
        assert_eq!(table[slot(0, 0)], at(&bytes, &ignored) - w.record_offset);
    }

    #[test]
    fn tlod_is_passed_and_another_record_ends_the_search() {
        let c = cell(0x10, 0, 0);
        let past_tlod = group(0x50, 1, &[record(b"TLOD", 0, 0x60, &[]), c.clone()]);
        let bytes = plugin(&[world(0x50), past_tlod]);
        let w = &offsets(&bytes, false)[0];
        assert_ne!(w.built.as_ref().unwrap()[slot(0, 0)], 0);

        let past_refr = group(0x50, 1, &[record(b"REFR", 0, 0x60, &[]), c]);
        let bytes = plugin(&[world(0x50), past_refr]);
        let w = &offsets(&bytes, false)[0];
        assert!(w.built.as_ref().unwrap().iter().all(|&o| o == 0));
    }

    #[test]
    fn a_cell_without_a_grid_position_is_at_the_origin() {
        let bare = record(b"CELL", 0, 0x10, &sub(b"EDID", b"c\0"));
        let bytes = plugin(&[world(0x50), group(0x50, 1, &[bare.clone()])]);
        let w = &offsets(&bytes, false)[0];
        assert_eq!(
            w.built.as_ref().unwrap()[slot(0, 0)],
            at(&bytes, &bare) - w.record_offset
        );
    }

    #[test]
    fn a_found_cell_outside_the_corners_declines() {
        let cells = [cell(0x10, 0, 0), cell(0x11, 5, 0)];
        let bytes = plugin(&[world(0x50), children(0x50, &cells)]);
        assert_eq!(offsets(&bytes, false)[0].built, Err(Decline::CellOutsideGrid));
    }

    #[test]
    fn no_corners_declines() {
        let w = record(b"WRLD", 0, 0x50, &sub(b"NAM0", &corner(0.0, 0.0)));
        let bytes = plugin(&[w, children(0x50, &[cell(0x10, 0, 0)])]);
        assert_eq!(offsets(&bytes, false)[0].built, Err(Decline::NoCorners));
    }

    #[test]
    fn a_stored_table_is_read_and_built_only_when_asked() {
        let ofst: Vec<u8> = (1..=16u32).flat_map(|n| n.to_le_bytes()).collect();
        let data = [
            sub(b"NAM0", &corner(-8192.0, -8192.0)),
            sub(b"NAM9", &corner(8191.0, 8191.0)),
            sub(b"OFST", &ofst),
        ]
        .concat();
        let bytes = plugin(&[
            record(b"WRLD", 0, 0x50, &data),
            children(0x50, &[cell(0x10, 0, 0)]),
        ]);
        let w = &offsets(&bytes, false)[0];
        assert_eq!(w.stored, Some((1..=16).collect()));
        assert_eq!(w.built, Err(Decline::NotAsked));
        let w = &offsets(&bytes, true)[0];
        let built = w.built.as_ref().unwrap();
        assert_eq!(built.iter().filter(|&&o| o != 0).count(), 1);
        // A plugin that is not a master: the game never reads its `OFST`, so its table is built.
        let mut plugin = bytes.clone();
        plugin[8] = 0;
        let w = &offsets(&plugin, false)[0];
        assert_eq!(w.built.as_ref().unwrap().iter().filter(|&&o| o != 0).count(), 1);
    }

    /// A zlib stream of one stored block.
    fn zlib_stored(data: &[u8]) -> Vec<u8> {
        let mut out = vec![0x78, 0x01, 0x01];
        out.extend_from_slice(&(data.len() as u16).to_le_bytes());
        out.extend_from_slice(&(!(data.len() as u16)).to_le_bytes());
        out.extend_from_slice(data);
        out.extend_from_slice(&crate::inflate::adler32(data).to_be_bytes());
        out
    }

    #[test]
    fn a_compressed_cell_is_read_inflated() {
        let plain = cell(0x10, 1, 0);
        let data = &plain[24..];
        let mut packed = (data.len() as u32).to_le_bytes().to_vec();
        packed.extend_from_slice(&zlib_stored(data));
        let compressed = record(b"CELL", COMPRESSED, 0x10, &packed);
        let content = group(
            0x50,
            1,
            &[group(
                block(1, 0),
                4,
                &[group(sub_block(1, 0), 5, &[compressed.clone()])],
            )],
        );
        let bytes = plugin(&[world(0x50), content]);
        let w = &offsets(&bytes, false)[0];
        assert_eq!(
            w.built.as_ref().unwrap()[slot(1, 0)],
            at(&bytes, &compressed) - w.record_offset
        );
    }

    #[test]
    fn two_records_of_one_world_space_decline() {
        let bytes = plugin(&[world(0x50), children(0x50, &[cell(0x10, 0, 0)]), world(0x50)]);
        let worlds = offsets(&bytes, false);
        assert_eq!(worlds.len(), 2);
        assert!(worlds.iter().all(|w| w.built == Err(Decline::DuplicateRecord)));
    }

    #[test]
    fn past_an_ignored_world_space_record_the_walk_goes_on_into_its_children() {
        // A master: world 0x50 without a table, then 0x51 with its own, flagged ignored. The
        // search for (1, 1) in 0x50 ends on the cell (0, 0) with nothing after it in 0x50's
        // groups; NextForm(1) then passes over 0x51's record into 0x51's children and finds its
        // cell. So 0x51's children are read even though its own table is not built; unflagged,
        // the record ends the walk and they are not.
        let own: Vec<u8> = (1..=16u32).flat_map(|n| n.to_le_bytes()).collect();
        let second = |flags: u32| {
            let data = [
                sub(b"NAM0", &corner(-8192.0, -8192.0)),
                sub(b"NAM9", &corner(8191.0, 8191.0)),
                sub(b"OFST", &own),
            ]
            .concat();
            record(b"WRLD", flags, 0x51, &data)
        };
        let first_children = group(
            0x50,
            1,
            &[group(
                block(0, 0),
                4,
                &[group(sub_block(0, 0), 5, &[cell(0x10, 0, 0)])],
            )],
        );
        let far = cell(0x11, 1, 1);
        for (flags, found) in [(IGNORED, true), (0, false)] {
            let bytes = plugin(&[
                world(0x50),
                first_children.clone(),
                second(flags),
                children(0x51, &[far.clone()]),
            ]);
            let worlds = offsets(&bytes, false);
            assert_eq!(worlds[1].built, Err(Decline::NotAsked));
            let w = &worlds[0];
            let entry = w.built.as_ref().unwrap()[slot(1, 1)];
            if found {
                assert_eq!(entry, at(&bytes, &far) - w.record_offset);
            } else {
                assert_eq!(entry, 0);
            }
        }
    }

    #[test]
    fn the_next_world_space_ends_the_search() {
        // The second world's cell is not the first world's.
        let bytes = plugin(&[world(0x50), world(0x51), children(0x51, &[cell(0x10, 0, 0)])]);
        let worlds = offsets(&bytes, false);
        assert!(worlds[0].built.as_ref().unwrap().iter().all(|&o| o == 0));
        let second = worlds[1].built.as_ref().unwrap();
        assert_eq!(second.iter().filter(|&&o| o != 0).count(), 1);
    }

    #[test]
    fn cell_of_follows_ftol() {
        assert_eq!(cell_of(f32::MAX.to_bits()), -524_288);
        assert_eq!(cell_of((-f32::MAX).to_bits()), -524_288);
        assert_eq!(cell_of(f32::NAN.to_bits()), -524_288);
        assert_eq!(cell_of((-131_072.0f32).to_bits()), -32);
        assert_eq!(cell_of(4095.9f32.to_bits()), 0);
        assert_eq!(cell_of((-0.5f32).to_bits()), 0);
        assert_eq!(cell_of((-1.0f32).to_bits()), -1);
    }
}
