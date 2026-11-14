//! `std::unordered_map<std::string, T>` as MSVC's standard library keeps it, for its order of
//! iteration: xNVSE's new compiler writes one `PluginVersion` call per entry of such a map at the
//! start of every script it compiles (`m_mpPluginRequirements`), and fills it from another (the
//! `#version` statements, `g_currentCompilerPluginVersions`), so the compiled bytes follow the
//! order MSVC's map walks its entries in.
//!
//! The model follows MSVC 14.44's `<xhash>` (`_Hash::_Find_last`, `_Insert_new_node_before`,
//! `_Check_rehash_required_1`, `_Desired_grow_bucket_count`, `_Forced_rehash`): one list of
//! entries, and per bucket the first and last entry of its run in the list. A key into an empty
//! bucket goes to the end of the list; a key into a bucket that has entries goes before the
//! bucket's first. Buckets start at 8 with a maximum load factor of 1; past it they grow eight
//! times (below 512) and the list is walked again, each entry going before its bucket's first
//! when the bucket already has one. Keys hash with `std::hash<std::string>`, 32-bit FNV-1a.

/// The map.
#[derive(Clone, Debug)]
pub struct MsvcMap<V> {
    /// The entries in list order.
    entries: Vec<(Vec<u8>, V)>,
    buckets: usize,
}

impl<V> Default for MsvcMap<V> {
    fn default() -> Self {
        MsvcMap {
            entries: Vec::new(),
            buckets: 8,
        }
    }
}

/// `std::hash<std::string>` on 32-bit MSVC (`_Fnv1a_bytes`).
pub fn fnv1a(bytes: &[u8]) -> u32 {
    let mut h: u32 = 2_166_136_261;
    for &b in bytes {
        h ^= u32::from(b);
        h = h.wrapping_mul(16_777_619);
    }
    h
}

impl<V> MsvcMap<V> {
    fn bucket(&self, key: &[u8]) -> usize {
        fnv1a(key) as usize & (self.buckets - 1)
    }

    /// The entry places of `bucket`'s run in the list (first, last), if it has entries.
    fn run(&self, bucket: usize) -> Option<(usize, usize)> {
        let mut first = None;
        let mut last = None;
        for (i, (k, _)) in self.entries.iter().enumerate() {
            if self.bucket(k) == bucket {
                first.get_or_insert(i);
                last = Some(i);
            }
        }
        first.zip(last)
    }

    pub fn contains(&self, key: &[u8]) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }

    pub fn get(&self, key: &[u8]) -> Option<&V> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entries in the order MSVC's map walks them.
    pub fn iter(&self) -> impl Iterator<Item = (&[u8], &V)> {
        self.entries.iter().map(|(k, v)| (k.as_slice(), v))
    }

    /// `map[key] = value`: an existing entry's value replaced, else a new entry where MSVC puts
    /// it.
    pub fn set(&mut self, key: &[u8], value: V) {
        if let Some(e) = self.entries.iter_mut().find(|(k, _)| k == key) {
            e.1 = value;
            return;
        }
        let new_size = self.entries.len() + 1;
        if new_size as f32 / self.buckets as f32 > 1.0 {
            self.rehash(new_size);
        }
        let at = match self.run(self.bucket(key)) {
            None => self.entries.len(),
            Some((first, _)) => first,
        };
        self.entries.insert(at, (key.to_vec(), value));
    }

    /// `_Desired_grow_bucket_count` and `_Forced_rehash`.
    fn rehash(&mut self, for_size: usize) {
        let required = (for_size as f32).ceil() as usize;
        let required = required.max(8);
        let mut buckets = self.buckets;
        if buckets < required {
            buckets = if buckets < 512 && buckets * 8 >= required {
                buckets * 8
            } else {
                required
            };
        }
        self.buckets = buckets.next_power_of_two();
        let old = std::mem::take(&mut self.entries);
        for (k, v) in old {
            let at = match self.run(self.bucket(&k)) {
                None => self.entries.len(),
                Some((first, _)) => first,
            };
            self.entries.insert(at, (k, v));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FNV-1a's published value for "a" (32 bits).
    #[test]
    fn hashes_like_msvc() {
        assert_eq!(fnv1a(b""), 0x811c_9dc5);
        assert_eq!(fnv1a(b"a"), 0xe40c_292c);
    }

    fn keys(m: &MsvcMap<u32>) -> Vec<String> {
        m.iter()
            .map(|(k, _)| String::from_utf8_lossy(k).into_owned())
            .collect()
    }

    /// Keys in different buckets keep their order; a key into a bucket with an entry goes
    /// before that entry; a replaced value keeps its place.
    #[test]
    fn colliding_keys_go_before_their_buckets_first() {
        let mut m = MsvcMap::default();
        let names: Vec<Vec<u8>> = (0..200).map(|i| format!("p{i}").into_bytes()).collect();
        let b = |k: &[u8]| fnv1a(k) as usize & 7;
        let first = names[0].clone();
        let other = names.iter().find(|k| b(k) != b(&first)).unwrap().clone();
        let same = names
            .iter()
            .skip(1)
            .find(|k| b(k) == b(&first))
            .unwrap()
            .clone();
        m.set(&first, 1);
        m.set(&other, 2);
        m.set(&same, 3);
        let s = |k: &[u8]| String::from_utf8_lossy(k).into_owned();
        assert_eq!(keys(&m), [s(&same), s(&first), s(&other)]);
        m.set(&first, 9);
        assert_eq!(keys(&m), [s(&same), s(&first), s(&other)]);
        assert_eq!(m.get(&first), Some(&9));
    }

    /// The ninth entry makes 64 buckets: the list is walked again in its order, each entry going
    /// before the first of its new bucket.
    #[test]
    fn the_ninth_entry_rehashes() {
        let mut m = MsvcMap::default();
        let names: Vec<Vec<u8>> = (0..9).map(|i| format!("plugin{i}").into_bytes()).collect();
        for (i, k) in names.iter().enumerate() {
            m.set(k, i as u32);
        }
        assert_eq!(m.buckets, 64);
        // The same walk done by hand on the list as it was before the ninth entry.
        let mut before: MsvcMap<u32> = MsvcMap::default();
        for (i, k) in names.iter().take(8).enumerate() {
            before.set(k, i as u32);
        }
        let mut expect: Vec<Vec<u8>> = Vec::new();
        for (k, _) in before.iter() {
            let bucket = fnv1a(k) as usize & 63;
            let at = expect
                .iter()
                .position(|e| fnv1a(e) as usize & 63 == bucket)
                .unwrap_or(expect.len());
            expect.insert(at, k.to_vec());
        }
        let ninth = &names[8];
        let at = expect
            .iter()
            .position(|e| fnv1a(e) as usize & 63 == fnv1a(ninth) as usize & 63)
            .unwrap_or(expect.len());
        expect.insert(at, ninth.clone());
        let got: Vec<Vec<u8>> = m.iter().map(|(k, _)| k.to_vec()).collect();
        assert_eq!(got, expect);
    }
}
