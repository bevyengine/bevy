use bevy_platform::hash::FixedHasher;
use core::hash::{BuildHasher, Hash, Hasher};

/// A Bloom filter, parameterized by number of u64 segments `N` and number of hash functions `K`.
///
/// `N` should be based on how much you plan to insert into the filter.
/// `N * 64` should be at least the number of items you plan to insert.
///
/// `K` if how little you can tolerate false positives.
/// 2, the default, should work for most uses. Increase `K` to reduce false positives,
/// at a pretty large compute cost.
///
/// # Examples
///
/// ```
/// use bevy_utils::BloomFilter;
///
/// let mut filter = BloomFilter::<1>::new();
/// filter.insert(&"hello");
/// assert!(filter.contains(&"hello"));
/// assert!(!filter.contains(&"world"));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct BloomFilter<const N: usize, const K: usize = 2> {
    bits: [u64; N],
}

impl<const N: usize, const K: usize> Default for BloomFilter<N, K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize, const K: usize> BloomFilter<N, K> {
    /// Creates a new, empty filter.
    pub const fn new() -> Self {
        assert!(N > 0, "size must be at least 1");
        Self { bits: [0; N] }
    }

    /// Inserts a value into the filter.
    pub fn insert(&mut self, item: &impl Hash) {
        let (h1, h2) = self.hash(item);
        let m = (N * 64) as u64;
        for i in 0..K {
            let idx = (h1.wrapping_add((i as u64).wrapping_mul(h2))) % m;
            self.bits[idx as usize / 64] |= 1 << (idx % 64);
        }
    }

    /// Checks if the filter might contain the value.
    pub fn contains(&self, item: &impl Hash) -> bool {
        let (h1, h2) = self.hash(item);
        let m = (N * 64) as u64;
        for i in 0..K {
            let idx = (h1.wrapping_add((i as u64).wrapping_mul(h2))) % m;
            if self.bits[idx as usize / 64] & (1 << (idx % 64)) == 0 {
                return false;
            }
        }
        true
    }

    /// Combined [`contains`] and [`insert`].
    ///
    /// Returns `true` if the value was already in the filter.
    /// Adds the value to the filter if it was not already present.
    pub fn check_insert(&mut self, item: &impl Hash) -> bool {
        let res = self.contains(item);
        if !res {
            self.insert(item);
        }
        res
    }

    fn hash(&self, item: &impl Hash) -> (u64, u64) {
        let mut hasher = FixedHasher.build_hasher();
        item.hash(&mut hasher);
        let hash = hasher.finish();
        (hash as u32 as u64, hash >> 32)
    }
}
