pub use rand::{
    rngs::{SmallRng, StdRng},
    seq::SliceRandom,
    CryptoRng, Rng, RngCore, SeedableRng,
};
use std::ops::Deref;

/// A seed for seeding a [`SecureRng`].
pub struct SecureSeed([u8; 32]);
impl From<[u8; 32]> for SecureSeed {
    fn from(item: [u8; 32]) -> Self {
        SecureSeed(item)
    }
}
impl Deref for SecureSeed {
    type Target = [u8; 32];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A random number generator suitable for use in cryptographic operations.
///
/// For non-cryptographic operations, use [`InsecureRng`](super::insecure::InsecureRng).
#[derive(Clone)]
pub struct SecureRng(StdRng);

impl Deref for SecureRng {
    type Target = StdRng;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SecureRng {
    pub(crate) fn from_seed(seed: [u8; 32]) -> Self {
        SecureRng(StdRng::from_seed(seed))
    }
}
impl RngCore for SecureRng {
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
