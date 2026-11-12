//! JIP LN NVSE's hash map (`UnorderedMap`, `internal/containers.h` (JIP LN NVSE)), kept for the
//! order it lists its entries in: JIP writes its co-save records by walking these maps, so a save
//! made after a load lists them as JIP's would. The map is an array of buckets (a power of two,
//! allocated at the first insertion), each a chain with the newest entry first; it doubles when
//! an insertion finds more entries than buckets; a walk goes bucket by bucket, each chain from
//! its head. `Clear` empties the chains and keeps the buckets.
//!
//! The hash and the sameness of a key are JIP's, by key type ([`Key`]): an `unsigned long`
//! (`UInt32`) or a pointer mixes its bits, an `unsigned int` (`UINT`) is its own hash, and a
//! string is hashed without case and compared by its hash alone.

/// JIP's smallest and largest bucket counts and the default (`MAP_MIN_BUCKET_COUNT`,
/// `MAP_MAX_BUCKET_COUNT`, `MAP_DEFAULT_BUCKET_COUNT`).
pub const MIN_BUCKETS: u32 = 4;
pub const MAX_BUCKETS: u32 = 0x8000;
pub const DEFAULT_BUCKETS: u32 = 8;

/// `STR_HASH_SEED` (`internal/containers.cpp`).
const STR_HASH_SEED: u32 = 0xd39c_a509;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/containers.cpp: AlignBucketCount.
/// A bucket count: `count` held between [`MIN_BUCKETS`] and [`MAX_BUCKETS`], up to a power of two.
pub fn align_bucket_count(count: u32) -> u32 {
    let count = count.clamp(MIN_BUCKETS, MAX_BUCKETS);
    let top = 31 - count.leading_zeros();
    let low = count.trailing_zeros();
    (1 + u32::from(low != top)) << top
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/containers.cpp: StrHashCI.
/// JIP's hash of a text without case (`A`-`Z` read as `a`-`z`, `kLwrCaseConverter`): from the
/// seed, each byte up to the terminator, shifted by 4, 12, 20 and 0 bits in turn, is subtracted
/// and the sum multiplied by 31.
pub fn str_hash_ci(text: &[u8]) -> u32 {
    let mut h = STR_HASH_SEED;
    for (i, &b) in text.iter().take_while(|&&b| b != 0).enumerate() {
        let c = u32::from(b.to_ascii_lowercase());
        h = h.wrapping_sub(c << [4, 12, 20, 0][i % 4]).wrapping_mul(31);
    }
    h
}

/// `HashKey` for a key of four bytes that is not `UINT` (`UInt32`, which JIP declares
/// `unsigned long`, and pointers): the bits mixed.
pub fn mixed_hash(key: u32) -> u32 {
    key.wrapping_mul(0xd) ^ (key >> 0xf)
}

/// A key type of JIP's map: its hash and whether two keys are the same.
pub trait Key: Clone {
    fn hash(&self) -> u32;
    /// `HashedKey::Equal`: the stored key against one being looked up, with that one's hash.
    fn same(&self, other: &Self, other_hash: u32) -> bool;
}

/// A `UInt32` key (an `unsigned long`: a form id, a mod index): mixed hash.
impl Key for u32 {
    fn hash(&self) -> u32 {
        mixed_hash(*self)
    }
    fn same(&self, other: &u32, _: u32) -> bool {
        self == other
    }
}

/// A `UINT` key (`unsigned int`: ExtraJIP's keys): its own hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Uint(pub u32);

impl Key for Uint {
    fn hash(&self) -> u32 {
        self.0
    }
    fn same(&self, other: &Uint, _: u32) -> bool {
        self == other
    }
}

/// A `char*` key (a variable's name): hashed without case, the same as any key of the same hash
/// (`HashedKey<char*>` keeps the first spelling).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name(pub Vec<u8>);

impl Key for Name {
    fn hash(&self) -> u32 {
        str_hash_ci(&self.0)
    }
    fn same(&self, _: &Name, other_hash: u32) -> bool {
        self.hash() == other_hash
    }
}

/// JIP's `UnorderedMap` with resizing on (every map the co-save uses).
#[derive(Clone, Debug)]
pub struct UnorderedMap<K, V> {
    /// The buckets once made: each chain newest first, with each entry's hash.
    buckets: Option<Vec<Vec<(u32, K, V)>>>,
    num_buckets: u32,
    len: u32,
}

impl<K: Key, V> Default for UnorderedMap<K, V> {
    fn default() -> Self {
        UnorderedMap::new(DEFAULT_BUCKETS)
    }
}

impl<K: Key, V> UnorderedMap<K, V> {
    /// A map that will make `buckets` buckets at its first insertion (`UnorderedMap(numBuckets)`).
    pub fn new(buckets: u32) -> Self {
        UnorderedMap {
            buckets: None,
            num_buckets: buckets,
            len: 0,
        }
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn bucket_count(&self) -> u32 {
        self.num_buckets
    }

    // `ResizeTable`: every chain moved, from the first bucket and each chain's head, to the head
    // of its new bucket.
    fn resize(&mut self, count: u32) {
        let old = self.buckets.take().unwrap_or_default();
        let mut new: Vec<Vec<(u32, K, V)>> = (0..count).map(|_| Vec::new()).collect();
        for chain in old {
            for entry in chain {
                new[(entry.0 & (count - 1)) as usize].insert(0, entry);
            }
        }
        self.buckets = Some(new);
        self.num_buckets = count;
    }

    // Translated from JIP LN NVSE 57.30 (5a30ac4), internal/containers.h: UnorderedMap::SetBucketCount.
    /// Sets the bucket count to `count` aligned: an empty map takes it for its next buckets; a
    /// map with entries is rehashed only when they fit in the new count.
    pub fn set_bucket_count(&mut self, count: u32) {
        let count = align_bucket_count(count);
        if self.num_buckets == count {
            return;
        }
        if self.len == 0 {
            self.buckets = None;
            self.num_buckets = count;
        } else if self.len <= count {
            self.resize(count);
        }
    }

    // Translated from JIP LN NVSE 57.30 (5a30ac4), internal/containers.h: UnorderedMap::InsertKey.
    /// The value of `key`, made with `make` when it is new (`Emplace`, `operator[]`), and whether
    /// it was new. The buckets are made at the first insertion; before each insertion with more
    /// entries than buckets (below the largest count) the map doubles, the key there or not.
    pub fn emplace_with(&mut self, key: K, make: impl FnOnce() -> V) -> (&mut V, bool) {
        if self.buckets.is_none() {
            self.buckets = Some((0..self.num_buckets).map(|_| Vec::new()).collect());
        } else if self.len > self.num_buckets && self.num_buckets < MAX_BUCKETS {
            self.resize(self.num_buckets << 1);
        }
        let hash = key.hash();
        let mask = self.num_buckets - 1;
        let chain = &mut self.buckets.as_mut().unwrap()[(hash & mask) as usize];
        if let Some(at) = chain.iter().position(|(_, k, _)| k.same(&key, hash)) {
            return (&mut chain[at].2, false);
        }
        self.len += 1;
        chain.insert(0, (hash, key, make()));
        (&mut chain[0].2, true)
    }

    /// [`Self::emplace_with`] with the value's default.
    pub fn emplace(&mut self, key: K) -> &mut V
    where
        V: Default,
    {
        self.emplace_with(key, V::default).0
    }

    /// The value of `key` (`GetPtr`).
    pub fn get(&self, key: &K) -> Option<&V> {
        if self.len == 0 {
            return None;
        }
        let hash = key.hash();
        let chain = &self.buckets.as_ref()?[(hash & (self.num_buckets - 1)) as usize];
        chain
            .iter()
            .find(|(_, k, _)| k.same(key, hash))
            .map(|(_, _, v)| v)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if self.len == 0 {
            return None;
        }
        let hash = key.hash();
        let mask = self.num_buckets - 1;
        let chain = &mut self.buckets.as_mut()?[(hash & mask) as usize];
        chain
            .iter_mut()
            .find(|(_, k, _)| k.same(key, hash))
            .map(|(_, _, v)| v)
    }

    /// `Erase`: whether the key was there.
    pub fn erase(&mut self, key: &K) -> bool {
        if self.len == 0 {
            return false;
        }
        let hash = key.hash();
        let mask = self.num_buckets - 1;
        let Some(buckets) = self.buckets.as_mut() else {
            return false;
        };
        let chain = &mut buckets[(hash & mask) as usize];
        match chain.iter().position(|(_, k, _)| k.same(key, hash)) {
            Some(at) => {
                chain.remove(at);
                self.len -= 1;
                true
            }
            None => false,
        }
    }

    /// `Clear`: the entries go, the buckets stay.
    pub fn clear(&mut self) {
        if let Some(buckets) = self.buckets.as_mut() {
            buckets.iter_mut().for_each(Vec::clear);
        }
        self.len = 0;
    }

    /// Keeps the entries `keep` says yes to, in their places (`Iterator::Remove` during a walk).
    pub fn retain(&mut self, mut keep: impl FnMut(&K, &mut V) -> bool) {
        if let Some(buckets) = self.buckets.as_mut() {
            for chain in buckets {
                let before = chain.len();
                chain.retain_mut(|(_, k, v)| keep(k, v));
                self.len -= (before - chain.len()) as u32;
            }
        }
    }

    /// The entries in JIP's walking order (`Begin`, `++`).
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.buckets
            .iter()
            .flatten()
            .flatten()
            .map(|(_, k, v)| (k, v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.buckets
            .iter_mut()
            .flatten()
            .flatten()
            .map(|(_, k, v)| (&*k, v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_counts_align_up_to_powers_of_two() {
        let a = align_bucket_count;
        assert_eq!(
            [a(0), a(1), a(4), a(5), a(8), a(9), a(0x20), a(0x21)],
            [4, 4, 4, 8, 8, 16, 0x20, 0x40]
        );
        assert_eq!(
            (a(0x8000), a(0x8001), a(u32::MAX)),
            (0x8000, 0x8000, 0x8000)
        );
    }

    /// The hash of a text: blind to case (A to Z only), stopping at a 0, as JIP's assembly runs
    /// it four bytes at a time. Values worked out from the routine by hand for short texts.
    #[test]
    fn texts_hash_without_case() {
        let manual = |t: &[u8]| {
            let mut h: u32 = STR_HASH_SEED;
            let shifts = [4u32, 12, 20, 0];
            for (i, &b) in t.iter().enumerate() {
                let c = u32::from(b.to_ascii_lowercase());
                h = h.wrapping_sub(c << shifts[i % 4]);
                h = (h << 5).wrapping_sub(h);
            }
            h
        };
        for t in [&b"a"[..], b"ab", b"abc", b"abcd", b"abcde", b"MyVar_1", b""] {
            assert_eq!(str_hash_ci(t), manual(t), "{t:?}");
        }
        assert_eq!(str_hash_ci(b"MyVar"), str_hash_ci(b"myvar"));
        assert_ne!(
            str_hash_ci(b"\xc1"),
            str_hash_ci(b"\xe1"),
            "only A to Z fold"
        );
        assert_eq!(str_hash_ci(b"ab\0cd"), str_hash_ci(b"ab"));
        assert_eq!(str_hash_ci(b""), STR_HASH_SEED);
    }

    #[test]
    fn keys_hash_by_type() {
        assert_eq!(0x14u32.hash(), 0x14 * 13);
        assert_eq!(0x0001_8000u32.hash(), (0x0001_8000u32 * 13) ^ 3);
        assert_eq!(
            0xffff_ffffu32.hash(),
            0xffff_ffffu32.wrapping_mul(13) ^ 0x1_ffff
        );
        assert_eq!(Uint(0x1234).hash(), 0x1234);
        assert!(Name(b"Abc".to_vec()).same(&Name(b"aBC".to_vec()), str_hash_ci(b"aBC")));
    }

    /// Entries walk bucket by bucket, each chain newest first; the buckets double before an
    /// insertion that finds more entries than buckets, moving each chain's entries to the heads
    /// of their new chains.
    #[test]
    fn the_walking_order() {
        let mut m: UnorderedMap<Uint, u32> = UnorderedMap::new(4);
        for k in [1, 5, 2, 9] {
            *m.emplace(Uint(k)) = k * 10;
        }
        // Buckets of 4: 1, 5, 9 in bucket 1 (newest first), 2 in bucket 2.
        let keys = |m: &UnorderedMap<Uint, u32>| m.iter().map(|(k, _)| k.0).collect::<Vec<_>>();
        assert_eq!(keys(&m), [9, 5, 1, 2]);
        // A fifth entry: 4 entries do not exceed 4 buckets, so no resize yet.
        *m.emplace(Uint(4)) = 40;
        assert_eq!((m.bucket_count(), keys(&m)), (4, vec![4, 9, 5, 1, 2]));
        // The sixth finds 5 > 4: the table doubles first. Bucket 0 (4) goes to 4; bucket 1
        // (9, 5, 1) goes, in that order, to the heads of 1, 5 and 1: bucket 1 is then 1, 9.
        *m.emplace(Uint(3)) = 30;
        assert_eq!(m.bucket_count(), 8);
        assert_eq!(keys(&m), [1, 9, 2, 3, 4, 5]);
        assert_eq!(m.get(&Uint(5)), Some(&50));
        assert!(m.erase(&Uint(9)));
        assert_eq!(keys(&m), [1, 2, 3, 4, 5]);
        m.clear();
        assert_eq!((m.len(), m.bucket_count()), (0, 8), "the buckets stay");
    }

    /// An existing key is found, not added; a key of the same text hash is the same name.
    #[test]
    fn names_are_one_by_their_hash() {
        let mut m: UnorderedMap<Name, u8> = UnorderedMap::new(4);
        let (v, new) = m.emplace_with(Name(b"Var".to_vec()), || 1);
        assert!(new && *v == 1);
        let (v, new) = m.emplace_with(Name(b"vAR".to_vec()), || 2);
        assert!(!new && *v == 1);
        assert_eq!(
            m.iter().next().unwrap().0 .0,
            b"Var",
            "the first spelling kept"
        );
    }

    #[test]
    fn bucket_counts_set_before_and_after_entries() {
        let mut m: UnorderedMap<u32, u8> = UnorderedMap::default();
        m.set_bucket_count(3);
        assert_eq!(m.bucket_count(), 4);
        for k in 0..6 {
            m.emplace(k);
        }
        assert_eq!(m.bucket_count(), 8, "doubled at the sixth");
        m.set_bucket_count(4);
        assert_eq!(m.bucket_count(), 8, "6 entries do not fit in 4");
        m.set_bucket_count(16);
        assert_eq!(m.bucket_count(), 16);
        let mut keys: Vec<u32> = m.iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        assert_eq!(keys, [0, 1, 2, 3, 4, 5]);
    }
}
