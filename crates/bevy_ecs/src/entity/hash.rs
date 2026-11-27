use core::hash::{BuildHasher, Hasher};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// A [`BuildHasher`] that results in a [`EntityHasher`].
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Default, Clone))]
pub struct EntityHash;

impl BuildHasher for EntityHash {
    type Hasher = EntityHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        Self::Hasher::default()
    }
}

/// A very fast hash that is only designed to work on generational indices
/// like [`Entity`](super::Entity). It will panic if attempting to hash a type containing
/// non-u64 fields.
///
/// This is heavily optimized for typical cases, where you have mostly live
/// entities, and works particularly well for contiguous indices.
///
/// If you have an unusual case -- say all your indices are multiples of 256
/// or most of the entities are dead generations -- then you might want also to
/// try [`DefaultHasher`](bevy_platform::hash::DefaultHasher) for a slower hash
/// computation but fewer lookup conflicts.
#[derive(Debug, Default)]
pub struct EntityHasher {
    hash: u64,
}

impl EntityHasher {
    /// Hashes the bits of an [`Entity`](super::Entity) for a 64-bit target.
    #[inline]
    pub(crate) const fn hash_for_64_bit(bits: u64) -> u64 {
        // SwissTable (and thus `hashbrown`) cares about two things from the hash:
        // - H1: low bits (masked by `2ⁿ-1`) to pick the slot in which to store the item
        // - H2: high 7 bits are used to SIMD optimize hash collision probing
        // For more see <https://abseil.io/about/design/swisstables#metadata-layout>

        // This hash function assumes that the entity ids are still well-distributed,
        // so for H1 leaves the entity id alone in the low bits so that id locality
        // will also give memory locality for things spawned together.
        // For H2, take advantage of the fact that while multiplication doesn't
        // spread entropy to the low bits, it's incredibly good at spreading it
        // upward, which is exactly where we need it the most.

        // While this does include the generation in the output, it doesn't do so
        // *usefully*.  H1 won't care until you have over 3 billion entities in
        // the table, and H2 won't care until something hits generation 33 million.
        // Thus the comment suggesting that this is best for live entities,
        // where there won't be generation conflicts where it would matter.

        // The high 32 bits of this are ⅟φ for Fibonacci hashing.  That works
        // particularly well for hashing for the same reason as described in
        // <https://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences/>
        // It loses no information because it has a modular inverse.
        // (Specifically, `0x144c_bc89_u32 * 0x9e37_79b9_u32 == 1`.)
        //
        // The low 32 bits make that part of the just product a pass-through.
        const UPPER_PHI: u64 = 0x9e37_79b9_0000_0001;

        // This is `(MAGIC * index + generation) << 32 + index`, in a single instruction.
        bits.wrapping_mul(UPPER_PHI)
    }

    /// Hashes the bits of an [`Entity`](super::Entity) for a 32-bit target,
    /// where `hashbrown` truncates the hash to the low 32 bits.
    #[inline]
    pub(crate) const fn hash_for_32_bit(bits: u64) -> u64 {
        let hash = Self::hash_for_64_bit(bits);

        // Here H2 is bits 25..32: the top of the inverted index, which is all ones
        // for every index below 2²⁵. Without mixing, every entity shares one H2 and
        // probing compares every key it passes. Mix in bits 18..25 of the upper
        // half, which (unlike its top bits) can't cancel out index bits 25..32.
        // The upper half is unchanged, so this remains a bijection.
        const H2_32_BIT: u64 = 0xFE00_0000;
        hash ^ ((hash >> 25) & H2_32_BIT)
    }
}

impl Hasher for EntityHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    fn write(&mut self, _bytes: &[u8]) {
        panic!("EntityHasher can only hash u64 fields.");
    }

    #[inline]
    fn write_u64(&mut self, bits: u64) {
        // 64-bit targets take H2 from the upper half, so skip the extra mixing.
        self.hash = if cfg!(target_pointer_width = "64") {
            Self::hash_for_64_bit(bits)
        } else {
            Self::hash_for_32_bit(bits)
        };
    }
}
