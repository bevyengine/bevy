mod insecure;
mod secure;

pub use self::insecure::{InsecureRng, InsecureSeed};
pub use self::secure::{SecureRng, SecureSeed};
use bevy_ecs::world::World;
use bevy_utils::tracing::trace;
use getrandom::getrandom;
#[doc(hidden)]
pub use rand::{
    rngs::{SmallRng, StdRng},
    seq::SliceRandom,
    CryptoRng, Rng, RngCore, SeedableRng,
};

/// Helper for configuring and creating the default random number generators.
/// For end-users who want full control, insert the default random number generators into the resource map manually.
/// If the random number generators are already inserted, this helper will do nothing.
#[derive(Clone, Debug)]
pub struct DefaultRngOptions {
    /// The seed to use for secure / cryptographic operations.
    secure_seed: [u8; 32],
    /// The seed to use for insecure / non-cryptographic operations. If set to `None`,
    /// the seed from `secure_seed` will be reused, saving an allocation.
    insecure_seed: Option<[u8; 32]>,
}

impl Default for DefaultRngOptions {
    fn default() -> Self {
        let mut seed: [u8; 32] = [0; 32];
        getrandom(&mut seed).expect("failed to get cryptographic seed for rng");

        // The default is to use the same secure / cryptographic seed for both rngs, so 
        // we do not specify an insecure seed.
        DefaultRngOptions {
            secure_seed: seed,
            insecure_seed: None,
        }
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
            insecure_seed: Some(seed),
            ..Default::default()
        }
    }

    /// Create a configuration that forces using particular seeds.
    pub fn with_seeds(secure_seed: SecureSeed, insecure_seed: InsecureSeed) -> Self {
        DefaultRngOptions {
            secure_seed: *secure_seed,
            insecure_seed: Some(*insecure_seed),
        }
    }

    /// Inserts the default random number generators into the given resource map based on the configured values.
    pub fn create_default_rngs(&self, world: &mut World) {
        if !world.contains_resource::<SecureRng>() {
            trace!("Creating secure RNG with seed: {:x?}", self.secure_seed);
            world.insert_resource(SecureRng::from_seed(self.secure_seed));
        }
        if !world.contains_resource::<InsecureRng>() {
            // Only use the insecure seed if set.
            let seed = self.insecure_seed.unwrap_or(self.secure_seed);
            trace!("Creating insecure RNG with seed: {:x?}", seed);
            world.insert_resource(InsecureRng::from_seed(seed));
        }
    }
}
