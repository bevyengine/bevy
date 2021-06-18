use bevy_ecs::world::World;
use bevy_utils::tracing::trace;
use getrandom::getrandom;
pub use rand::{
    rngs::{SmallRng, StdRng},
    CryptoRng, Rng, RngCore, SeedableRng, seq::SliceRandom,
};

#[derive(Clone, Debug)]
pub struct DefaultRngOptions {
    /// The seed to use for secure / cryptographic operations.
    pub secure_seed: [u8; 32],
    /// The seed to use for insecure / non-cryptographic operations.
    pub insecure_seed: [u8; 32],
}

impl Default for DefaultRngOptions {
    fn default() -> Self {
        let mut seed: [u8; 32] = [0; 32];
        getrandom(&mut seed).expect("failed to get seed for crypto rng");

        // The default is to use the same secure / cryptographic seed for both Rngs.
        DefaultRngOptions {
            secure_seed: seed,
            insecure_seed: seed,
        }
    }
}

#[derive(Clone)]
pub struct SecureRng(StdRng);

impl SecureRng {
    fn from_seed(seed: [u8; 32]) -> Self {
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

#[derive(Clone)]
pub struct InsecureRng(SmallRng);

impl InsecureRng {
    fn from_seed(seed: [u8; 32]) -> Self {
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

impl DefaultRngOptions {
    /// Create a configuration that forces using a particular secure seed.
    pub fn with_secure_seed(seed: [u8; 32]) -> Self {
        DefaultRngOptions {
            secure_seed: seed,
            ..Default::default()
        }
    }

    /// Create a configuration that forces using a particular insecure seed.
    pub fn with_insecure_seed(seed: [u8; 32]) -> Self {
        DefaultRngOptions {
            insecure_seed: seed,
            ..Default::default()
        }
    }

    /// Inserts the default Rngs into the given resource map based on the configured values.
    pub fn create_default_rngs(&self, world: &mut World) {
        if !world.contains_resource::<SecureRng>() {
            trace!("Creating secure RNG with seed: {:x?}", self.secure_seed);
            world.insert_resource(SecureRng::from_seed(self.secure_seed));
        }
        if !world.contains_resource::<InsecureRng>() {
            trace!("Creating insecure RNG with seed: {:x?}", self.insecure_seed);
            world.insert_resource(InsecureRng::from_seed(self.insecure_seed));
        }
    }
}
