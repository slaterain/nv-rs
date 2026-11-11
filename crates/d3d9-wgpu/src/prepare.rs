//! The pipelines a run will need, made before play (the first-start preparation): a list of every
//! pipeline the backend has made, under a key that is the same from run to run, shipped with the
//! port and grown by each run on the player's machine. At start-up the backend makes every listed
//! pipeline whose two shaders the game has created, on worker threads, before the game goes on;
//! what is not on the list (a mod's new shader, a state nobody recorded) is made on first use, as
//! before, and added to the local list for the next start. A multisampled entry is made at each
//! multisample count the run's surfaces have, whatever count it was recorded at, so a list
//! recorded at 4x serves a player at 2x or 8x (the same draws on that player's targets); a
//! single-sampled entry only at one sample.
//!
//! The list only predicts what to make: the pipelines themselves are kept and found under the
//! backend's exact key of the live state, so an entry that matches nothing, or two shaders whose
//! hashes collide, can cost a pipeline made for nothing but never a wrong draw. A collision is
//! still detected (two different bytecodes with one hash) and those entries are not prepared.
//!
//! One entry a line, all numbers, in an order that does not change:
//! `p2 VS PS DECL STRIDES TOPOLOGY INDEX32 COLOURS DEPTH STATES SAMPLES FILTERABLE ATOC COUNT CLIP`
//! (hashes and the declaration in hexadecimal, lists comma-separated, `-` for none). `p1` lines
//! (the first version, whose shader hashes came from the map hasher) are not read: a list a run
//! wrote before keeps only its `p2` lines when it is written again.

/// A shader's bytecode hash, fixed by its definition so the shipped list matches in every build:
/// 64-bit FNV-1a over the tokens as little-endian bytes ([`crate::cache::hash`]), not the map
/// hasher, whose output (and the way the standard library feeds a `[u32]` to it) may change.
pub(crate) fn bytecode_hash(tokens: &[u32]) -> u64 {
    let bytes: Vec<u8> = tokens.iter().flat_map(|t| t.to_le_bytes()).collect();
    crate::cache::hash(&bytes)
}

/// One pipeline as the list keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Entry {
    pub vs: u64,
    pub ps: u64,
    /// The declaration's elements: stream, offset, type, usage, index.
    pub decl: Vec<(u16, u16, u8, u8, u8)>,
    pub strides: [u32; 8],
    pub topology: u8,
    pub index32: bool,
    /// The colour targets' and the depth surface's `D3DFORMAT`s.
    pub colours: [Option<u32>; 4],
    pub depth: Option<u32>,
    pub states: [u32; 30],
    pub samples: u32,
    pub filterable: u32,
    pub atoc: bool,
    pub count: bool,
    /// The user clip planes: 0 off; 1 the clip distances (every draw on an adapter that has
    /// them, planes on or off; before that, planes on); 2 (discard) or 3 (discard per sample)
    /// with a plane on, as the adapter that recorded it cut them: another adapter cuts them its
    /// own way.
    pub clip: u8,
}

fn list<T: std::fmt::Display>(v: impl IntoIterator<Item = T>) -> String {
    v.into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn opt(v: Option<u32>) -> String {
    v.map_or("-".into(), |x| x.to_string())
}

impl Entry {
    pub(crate) fn line(&self) -> String {
        let decl = if self.decl.is_empty() {
            "-".to_string()
        } else {
            self.decl
                .iter()
                .map(|&(s, o, t, u, i)| format!("{s:x}.{o:x}.{t:x}.{u:x}.{i:x}"))
                .collect::<Vec<_>>()
                .join(",")
        };
        format!(
            "p2 {:016x} {:016x} {decl} {} {} {} {} {} {} {} {} {} {} {}",
            self.vs,
            self.ps,
            list(self.strides),
            self.topology,
            u8::from(self.index32),
            list(self.colours.map(opt)),
            opt(self.depth),
            list(self.states),
            self.samples,
            self.filterable,
            u8::from(self.atoc),
            u8::from(self.count),
            self.clip
        )
    }

    /// An entry from a line of the list; None for anything else (another version, a damaged
    /// line), which is skipped.
    pub(crate) fn parse(line: &str) -> Option<Entry> {
        let f: Vec<&str> = line.split_ascii_whitespace().collect();
        if f.len() != 15 || f[0] != "p2" {
            return None;
        }
        let hex = |s: &str| u64::from_str_radix(s, 16).ok();
        let num = |s: &str| s.parse::<u32>().ok();
        let opt = |s: &str| {
            if s == "-" {
                Some(None)
            } else {
                num(s).map(Some)
            }
        };
        let bit = |s: &str| match s {
            "0" => Some(false),
            "1" => Some(true),
            _ => None,
        };
        let nums = |s: &str| s.split(',').map(num).collect::<Option<Vec<u32>>>();
        let decl = if f[3] == "-" {
            Vec::new()
        } else {
            f[3].split(',')
                .map(|e| {
                    let p: Vec<&str> = e.split('.').collect();
                    if p.len() != 5 {
                        return None;
                    }
                    let h = |s: &str| u32::from_str_radix(s, 16).ok();
                    Some((
                        u16::try_from(h(p[0])?).ok()?,
                        u16::try_from(h(p[1])?).ok()?,
                        u8::try_from(h(p[2])?).ok()?,
                        u8::try_from(h(p[3])?).ok()?,
                        u8::try_from(h(p[4])?).ok()?,
                    ))
                })
                .collect::<Option<Vec<_>>>()?
        };
        let colours: Vec<Option<u32>> = f[7].split(',').map(opt).collect::<Option<Vec<_>>>()?;
        Some(Entry {
            vs: hex(f[1])?,
            ps: hex(f[2])?,
            decl,
            strides: nums(f[4])?.try_into().ok()?,
            topology: u8::try_from(num(f[5])?).ok()?,
            index32: bit(f[6])?,
            colours: colours.try_into().ok()?,
            depth: opt(f[8])?,
            states: nums(f[9])?.try_into().ok()?,
            samples: num(f[10])?,
            filterable: num(f[11])?,
            atoc: bit(f[12])?,
            count: bit(f[13])?,
            clip: u8::try_from(num(f[14])?).ok().filter(|&c| c <= 3)?,
        })
    }
}

/// The list: entries in the order they were first needed, each once.
#[derive(Default)]
pub(crate) struct List {
    pub entries: Vec<Entry>,
    seen: std::collections::HashSet<Entry>,
}

impl List {
    /// Adds the entries of `text` not yet in the list (lines it cannot read are skipped); how
    /// many were added and how many lines skipped.
    pub(crate) fn merge(&mut self, text: &str) -> (usize, usize) {
        let (mut added, mut skipped) = (0, 0);
        for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
            match Entry::parse(line) {
                Some(e) => {
                    if self.add(e) {
                        added += 1;
                    }
                }
                None => skipped += 1,
            }
        }
        (added, skipped)
    }

    /// Adds `e` unless it is listed; whether it was new.
    pub(crate) fn add(&mut self, e: Entry) -> bool {
        if self.seen.contains(&e) {
            return false;
        }
        self.seen.insert(e.clone());
        self.entries.push(e);
        true
    }

    #[cfg(test)]
    pub(crate) fn text(&self) -> String {
        let mut s = String::new();
        for e in &self.entries {
            s.push_str(&e.line());
            s.push('\n');
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> Entry {
        Entry {
            vs: 0x0123_4567_89ab_cdef,
            ps: 0xfedc_ba98_7654_3210,
            decl: vec![(0, 0, 2, 0, 0), (0, 12, 4, 10, 0), (1, 0, 1, 5, 1)],
            strides: [20, 8, 0, 0, 0, 0, 0, 0],
            topology: 4,
            index32: false,
            colours: [Some(113), None, Some(21), None],
            depth: Some(75),
            states: std::array::from_fn(|i| i as u32 * 7 + u32::from(i == 21) * 0xFFFF_FF00),
            samples: 8,
            filterable: 0b1011,
            atoc: true,
            count: false,
            clip: 3,
        }
    }

    #[test]
    fn an_entry_reads_back_as_it_was_written() {
        let e = entry();
        assert_eq!(Entry::parse(&e.line()), Some(e.clone()));
        let none = Entry {
            decl: Vec::new(),
            colours: [None; 4],
            depth: None,
            ..e
        };
        assert_eq!(Entry::parse(&none.line()), Some(none));
    }

    #[test]
    fn damaged_or_foreign_lines_are_skipped() {
        let line = entry().line();
        for bad in [
            "",
            "p2 0 0",
            // The first version, whose hashes came from the map hasher.
            &line.replacen("p2", "p1", 1),
            &line[..line.len() - 2],
            &line.replace(" 8 ", " eight "),
            &line.replacen("p2", "p0", 1),
            "junk junk junk junk junk junk junk junk junk junk junk junk junk junk junk",
        ] {
            assert_eq!(Entry::parse(bad), None, "{bad}");
        }
        let mut l = List::default();
        let text = format!("{line}\nnot a line\n{line}\n");
        assert_eq!(l.merge(&text), (1, 1));
        assert_eq!(l.entries.len(), 1);
        assert_eq!(l.text(), format!("{line}\n"));
    }

    #[test]
    fn the_bytecode_hash_is_the_same_each_time_and_differs_for_other_tokens() {
        let a = [
            0xFFFE_0200u32,
            0x0200_001F,
            0x8000_0000,
            0x900F_0000,
            0xFFFF,
        ];
        let mut b = a;
        b[3] ^= 1;
        assert_eq!(bytecode_hash(&a), bytecode_hash(&a));
        assert_ne!(bytecode_hash(&a), bytecode_hash(&b));
        assert_ne!(bytecode_hash(&a), bytecode_hash(&a[..4]));
        // Pinned: 64-bit FNV-1a of these five tokens' 20 little-endian bytes. A change here would make
        // the shipped list (`data/pipelines.txt`) match nothing.
        assert_eq!(bytecode_hash(&a), 0x19e4_37ae_3a91_6934);
    }
}
