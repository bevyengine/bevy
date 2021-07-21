use bevy_app::{AppBuilder, Plugin};
use bevy_utils::tracing::{debug, trace};
use rand::{rngs::StdRng, RngCore, SeedableRng};

pub mod prelude {
    #[doc(hidden)]
    pub use crate::Entropy;
}

/// Provides a source of entropy.
/// This enables deterministic execution.
#[derive(Default)]
pub struct EntropyPlugin;

impl Plugin for EntropyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let world = app.world();
        if !world.contains_resource::<Entropy>() {
            trace!("Creating entropy");
            app.init_resource::<Entropy>();
        }
    }
}

/// A resource that provides entropy.
pub struct Entropy(StdRng);

impl Default for Entropy {
    /// The default entropy source is non-deterministic and seeded from the operating system.
    /// For a deterministic source, use [`Entropy::from`].
    fn default() -> Self {
        debug!("Entropy created via the operating system");
        let rng = StdRng::from_entropy();
        Entropy(rng)
    }
}

impl Entropy {
    /// Create a deterministic source of entropy. All random number generators
    /// later seeded from an [`Entropy`] created this way will be deterministic.
    /// If determinism is not required, use [`Entropy::default`].
    pub fn from(seed: [u8; 32]) -> Self {
        debug!("Entropy created via seed: {:?} ", seed);
        let rng = StdRng::from_seed(seed);
        Entropy(rng)
    }

    /// Fill `dest` with entropy data. For an allocating alternative, see [`Entropy::get`].
    pub fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest)
    }

    /// Allocate and return entropy data. For a non-allocating alternative, see [`Entropy::fill_bytes`].
    pub fn get(&mut self) -> [u8; 32] {
        let mut dest = [0; 32];
        self.0.fill_bytes(&mut dest);
        dest
    }
}
