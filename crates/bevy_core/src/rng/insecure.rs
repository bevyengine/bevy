pub use rand::{
    rngs::{SmallRng, StdRng},
    seq::SliceRandom,
    CryptoRng, Rng, RngCore, SeedableRng,
};
use std::ops::Deref;

/// A seed for seeding an [`InsecureRng`].
pub struct InsecureSeed([u8; 32]);

impl From<[u8; 32]> for InsecureSeed {
    fn from(item: [u8; 32]) -> Self {
        InsecureSeed(item)
    }
}

impl Deref for InsecureSeed {
    type Target = [u8; 32];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A random number generator for use in non-cryptographic operations.
///
/// For cryptographic operations, use [`SecureRng`](super::secure::SecureRng).
#[derive(Clone)]
pub struct InsecureRng(SmallRng);

impl Deref for InsecureRng {
    type Target = SmallRng;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl InsecureRng {
    pub(crate) fn from_seed(seed: [u8; 32]) -> Self {
        InsecureRng(SmallRng::from_seed(seed))
    }
}
impl RngCore for InsecureRng {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }
    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest)
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.0.try_fill_bytes(dest)
    }
}
