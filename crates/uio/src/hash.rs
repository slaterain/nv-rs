//! UI Organizer's hash containers (`containers.h`, `utility.cpp` (UIO)), as far as their order
//! and equality decide what UIO writes: the order of the `<include>` lines it injects into a tile
//! is the walking order of a set of paths, and two names are one key when their hashes are equal
//! (UIO's string keys keep the hash only, or the hash and the first spelling).

/// `kCaseConverter` (UIO): only `A` to `Z` change, to `a` to `z`.
#[inline]
pub fn lower(c: u8) -> u8 {
    c.to_ascii_lowercase()
}

// Translated from UIO 2.30 (426d963), utility.cpp: StrHash.
/// UIO's string hash: 0x1505, then for each byte `h * 33 + lower(byte)`, wrapping.
pub fn str_hash(s: &[u8]) -> u32 {
    s.iter().fold(0x1505u32, |h, &c| {
        h.wrapping_mul(33).wrapping_add(u32::from(lower(c)))
    })
}

// Translated from UIO 2.30 (426d963), utility.cpp: StrBeginsCI.
/// Whether `s` begins with `prefix`, without case (`kCaseConverter`); a shorter `s` does not.
pub fn begins_ci(s: &[u8], prefix: &[u8]) -> bool {
    s.len() >= prefix.len() && s.iter().zip(prefix).all(|(a, b)| lower(*a) == lower(*b))
}

/// `MAP_MIN_BUCKET_COUNT`, `MAP_MAX_BUCKET_COUNT`, `MAP_DEFAULT_BUCKET_COUNT` (UIO).
const MIN_BUCKETS: u32 = 4;
const MAX_BUCKETS: u32 = 0x4_0000;
pub const DEFAULT_BUCKETS: u32 = 8;

// Translated from UIO 2.30 (426d963), containers.cpp: AlignBucketCount.
/// A bucket count made a power of two: 4 at least, 0x40000 at most, else the next power of two
/// up (a power of two stays).
pub fn align_bucket_count(count: u32) -> u32 {
    if count <= MIN_BUCKETS {
        return MIN_BUCKETS;
    }
    if count >= MAX_BUCKETS {
        return MAX_BUCKETS;
    }
    if count.is_power_of_two() {
        count
    } else {
        2 << (31 - count.leading_zeros())
    }
}

/// One entry: the key's hash, the key as first inserted, the value.
#[derive(Clone, Debug)]
struct Entry<V> {
    hash: u32,
    key: Vec<u8>,
    value: V,
}

// Translated from UIO 2.30 (426d963), containers.h: UnorderedMap<char*, T> (and UnorderedSet<char*>,
// the same table without values).
/// UIO's `UnorderedMap` with string keys: buckets made at the first insert (the count aligned),
/// doubled before an insert while it holds more entries than buckets, a new entry put first in
/// its bucket (and a resize walks each old bucket from its first entry, putting each first in its
/// new bucket); keys equal when their hashes are. It is walked bucket by bucket, each from its
/// first entry.
#[derive(Clone, Debug)]
pub struct Map<V> {
    /// Each bucket's entries, its first entry first; empty until the first insert.
    buckets: Vec<Vec<Entry<V>>>,
    /// The bucket count asked for, until the first insert aligns it.
    count: u32,
    entries: usize,
}

impl<V> Map<V> {
    /// An empty table that will have `count` buckets (aligned) at its first insert.
    pub fn with_buckets(count: u32) -> Map<V> {
        Map {
            buckets: Vec::new(),
            count,
            entries: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries == 0
    }

    fn mask(&self) -> usize {
        self.buckets.len() - 1
    }

    fn resize(&mut self, count: u32) {
        let mut new: Vec<Vec<Entry<V>>> = (0..count).map(|_| Vec::new()).collect();
        let mask = count as usize - 1;
        for bucket in std::mem::take(&mut self.buckets) {
            for entry in bucket {
                new[entry.hash as usize & mask].insert(0, entry);
            }
        }
        self.buckets = new;
        self.count = count;
    }

    /// `InsertKey`: the entry of `key`'s hash, made with `make` when there is none (first in its
    /// bucket); true when it was made.
    pub fn insert_with(&mut self, key: &[u8], make: impl FnOnce() -> V) -> (&mut V, bool) {
        if self.buckets.is_empty() {
            self.count = align_bucket_count(self.count);
            self.buckets = (0..self.count).map(|_| Vec::new()).collect();
        } else if self.entries > self.buckets.len() && (self.buckets.len() as u32) < MAX_BUCKETS {
            self.resize((self.buckets.len() as u32) << 1);
        }
        let hash = str_hash(key);
        let b = hash as usize & self.mask();
        if let Some(i) = self.buckets[b].iter().position(|e| e.hash == hash) {
            return (&mut self.buckets[b][i].value, false);
        }
        self.entries += 1;
        self.buckets[b].insert(
            0,
            Entry {
                hash,
                key: key.to_vec(),
                value: make(),
            },
        );
        (&mut self.buckets[b][0].value, true)
    }

    fn find(&self, key: &[u8]) -> Option<(usize, usize)> {
        if self.entries == 0 {
            return None;
        }
        let hash = str_hash(key);
        let b = hash as usize & self.mask();
        self.buckets[b]
            .iter()
            .position(|e| e.hash == hash)
            .map(|i| (b, i))
    }

    pub fn get(&self, key: &[u8]) -> Option<&V> {
        self.find(key).map(|(b, i)| &self.buckets[b][i].value)
    }

    pub fn contains(&self, key: &[u8]) -> bool {
        self.find(key).is_some()
    }

    /// The iterator's `Remove`: the entry of `key`'s hash taken out, its value answered.
    pub fn remove(&mut self, key: &[u8]) -> Option<V> {
        let (b, i) = self.find(key)?;
        self.entries -= 1;
        Some(self.buckets[b].remove(i).value)
    }

    /// The entries in UIO's walking order: the key as first inserted, the value.
    pub fn iter(&self) -> impl Iterator<Item = (&[u8], &V)> {
        self.buckets
            .iter()
            .flat_map(|b| b.iter().map(|e| (e.key.as_slice(), &e.value)))
    }
}

impl<V: Default> Map<V> {
    /// `operator[]`: the entry of `key`, made empty when there is none.
    pub fn entry(&mut self, key: &[u8]) -> &mut V {
        self.insert_with(key, V::default).0
    }
}

impl<V> Default for Map<V> {
    fn default() -> Map<V> {
        Map::with_buckets(DEFAULT_BUCKETS)
    }
}

/// UIO's `UnorderedSet<char*>`: the same table without values.
#[derive(Clone, Debug, Default)]
pub struct Set(Map<()>);

impl Set {
    /// An empty set that will have `count` buckets (aligned) at its first insert.
    pub fn with_buckets(count: u32) -> Set {
        Set(Map::with_buckets(count))
    }

    /// `Insert`: true when `key` (by its hash) was not there.
    pub fn insert(&mut self, key: &[u8]) -> bool {
        self.0.insert_with(key, || ()).1
    }

    pub fn contains(&self, key: &[u8]) -> bool {
        self.0.contains(key)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The keys in UIO's walking order, each as first inserted.
    pub fn iter(&self) -> impl Iterator<Item = &[u8]> {
        self.0.iter().map(|(k, _)| k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_is_uios_without_case() {
        assert_eq!(str_hash(b""), 0x1505);
        // 0x1505 * 33 + 'a'
        assert_eq!(str_hash(b"a"), 0x1505 * 33 + 0x61);
        assert_eq!(str_hash(b"A"), str_hash(b"a"));
        assert_eq!(str_hash(b"HUDMainMenu"), str_hash(b"hudmainmenu"));
        // Only A-Z change: a backslash and a slash differ, as do accented letters' cases.
        assert_ne!(str_hash(b"a\\b"), str_hash(b"a/b"));
        assert_ne!(str_hash(&[0xc9]), str_hash(&[0xe9]));
        // It wraps.
        let long = vec![b'z'; 64];
        let mut h = 0x1505u32;
        for _ in 0..64 {
            h = h.wrapping_mul(33).wrapping_add(0x7a);
        }
        assert_eq!(str_hash(&long), h);
    }

    #[test]
    fn begins_ci_compares_the_prefix_only() {
        assert!(begins_ci(b"Prefabs\\x.xml", b"prefabs\\"));
        assert!(!begins_ci(b"prefab", b"prefabs\\"));
        assert!(begins_ci(b"anything", b""));
        assert!(!begins_ci(b"prefabs/x", b"prefabs\\"));
    }

    #[test]
    fn bucket_counts_align_as_uios() {
        assert_eq!(align_bucket_count(0), 4);
        assert_eq!(align_bucket_count(4), 4);
        assert_eq!(align_bucket_count(5), 8);
        assert_eq!(align_bucket_count(8), 8);
        assert_eq!(align_bucket_count(33), 64);
        assert_eq!(align_bucket_count(0x100), 0x100);
        assert_eq!(align_bucket_count(0x4_0000), 0x4_0000);
        assert_eq!(align_bucket_count(u32::MAX), 0x4_0000);
    }

    /// Keys whose hashes fall in chosen buckets of an 8-bucket table.
    fn key_in_bucket(bucket: u32, n: usize) -> Vec<u8> {
        let mut found = 0;
        for i in 0u32.. {
            let k = format!("k{i}").into_bytes();
            if str_hash(&k) & 7 == bucket {
                if found == n {
                    return k;
                }
                found += 1;
            }
        }
        unreachable!()
    }

    #[test]
    fn the_walk_is_by_bucket_newest_first() {
        let (a, b, c) = (
            key_in_bucket(3, 0),
            key_in_bucket(3, 1),
            key_in_bucket(1, 0),
        );
        let mut s = Set::default();
        assert!(s.insert(&a));
        assert!(s.insert(&b));
        assert!(s.insert(&c));
        assert!(!s.insert(&a), "already there");
        // Bucket 1 first; in bucket 3 the newer first.
        let order: Vec<&[u8]> = s.iter().collect();
        assert_eq!(order, vec![c.as_slice(), b.as_slice(), a.as_slice()]);
    }

    #[test]
    fn a_resize_comes_before_the_insert_that_finds_more_entries_than_buckets() {
        // Ten keys of bucket 0 in an 8-bucket table: the ninth insert finds 8 entries (not more
        // than 8: no resize), the tenth finds 9 and doubles the table first.
        let keys: Vec<Vec<u8>> = (0..10).map(|n| key_in_bucket(0, n)).collect();
        let mut m: Map<u32> = Map::default();
        for (i, k) in keys.iter().enumerate().take(9) {
            *m.entry(k) = i as u32;
        }
        assert_eq!(m.buckets.len(), 8);
        // A key already there also resizes first (UIO resizes before it looks).
        let (_, made) = m.insert_with(&keys[0], || 99);
        assert!(!made);
        assert_eq!(m.buckets.len(), 16);
        *m.entry(&keys[9]) = 9;
        // The resize walked bucket 0 from its first entry (key 8), putting each first in its new
        // bucket: within a new bucket the old order is reversed; key 9 came after, first.
        let mut expected: Vec<(u32, u32)> = Vec::new();
        for bucket in 0..16u32 {
            let mut here: Vec<u32> = (0..9u32)
                .filter(|&i| str_hash(&keys[i as usize]) & 15 == bucket)
                .collect();
            // Before the resize the bucket's order was 8, 7, ..., 0; reversed: 0, 1, ..., 8.
            here.sort_unstable();
            if str_hash(&keys[9]) & 15 == bucket {
                here.insert(0, 9);
            }
            expected.extend(here.into_iter().map(|i| (bucket, i)));
        }
        let walked: Vec<u32> = m.iter().map(|(_, v)| *v).collect();
        let expected: Vec<u32> = expected.into_iter().map(|(_, i)| i).collect();
        assert_eq!(walked, expected);
    }

    #[test]
    fn keys_of_one_hash_are_one_entry_with_the_first_spelling() {
        let mut m: Map<u32> = Map::default();
        *m.entry(b"Glow_Branch") = 1;
        *m.entry(b"glow_branch") += 1;
        assert_eq!(m.len(), 1);
        assert_eq!(m.iter().next(), Some((b"Glow_Branch".as_slice(), &2)));
        assert_eq!(m.remove(b"GLOW_BRANCH"), Some(2));
        assert!(m.is_empty());
        assert_eq!(m.get(b"glow_branch"), None);
        assert_eq!(m.remove(b"x"), None);
    }
}
