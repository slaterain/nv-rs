//! A fast hasher for the device's caches (pipelines, bind groups, samplers, objects by handle):
//! the multiply-and-rotate hash the Rust compiler uses for its own tables ("FxHash"). Its keys are
//! the backend's own state and the game's handles, never input chosen to collide, so the default
//! hasher's protection against that (SipHash, about 1 ms a frame on the street) buys nothing here.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// A `HashMap` with [`FxHasher`].
pub type Map<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

#[derive(Default, Clone, Copy)]
pub struct FxHasher {
    hash: u64,
}

impl FxHasher {
    #[inline]
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for c in &mut chunks {
            self.add(u64::from_le_bytes(c.try_into().expect("8 bytes")));
        }
        let rest = chunks.remainder();
        if !rest.is_empty() {
            let mut w = [0u8; 8];
            w[..rest.len()].copy_from_slice(rest);
            self.add(u64::from_le_bytes(w));
        }
    }
    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add(u64::from(i));
    }
    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.add(u64::from(i));
    }
    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add(u64::from(i));
    }
    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add(i);
    }
    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_keys_hash_alike_and_the_map_works() {
        let mut m: Map<(u64, Vec<u32>), u32> = Map::default();
        for k in 0..1000u64 {
            m.insert((k, vec![k as u32, 7]), k as u32);
        }
        for k in 0..1000u64 {
            assert_eq!(m.get(&(k, vec![k as u32, 7])), Some(&(k as u32)));
        }
        assert_eq!(m.get(&(5, vec![5, 8])), None);
        let h = |b: &[u8]| {
            let mut x = FxHasher::default();
            x.write(b);
            x.finish()
        };
        assert_eq!(h(b"abcdefghij"), h(b"abcdefghij"));
        assert_ne!(h(b"abcdefghij"), h(b"abcdefghik"));
    }
}
