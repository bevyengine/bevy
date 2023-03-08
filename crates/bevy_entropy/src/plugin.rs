use std::marker::PhantomData;

use crate::{resource::GlobalEntropy, traits::SeedableEntropySource};
use bevy_app::{App, Plugin};
use rand_core::SeedableRng;

#[cfg(feature = "bevy_reflect")]
use crate::component::EntropyComponent;

/// Plugin for integrating a PRNG that implements `RngCore` into
/// the bevy engine, registering types for a global resource and
/// entropy components.
///
/// ```
/// use bevy_ecs::prelude::ResMut;
/// use bevy_app::App;
/// use bevy_entropy::prelude::*;
/// use rand_core::RngCore;
/// use rand_chacha::ChaCha8Rng;
///
/// fn main() {
///  App::new()
///    .add_plugin(EntropyPlugin::<ChaCha8Rng>::default())
///    .add_system(print_random_value)
///    .run();
/// }
///
/// fn print_random_value(mut rng: ResMut<GlobalEntropy<ChaCha8Rng>>) {
///   println!("Random value: {}", rng.next_u32());
/// }
/// ```
pub struct EntropyPlugin<R: SeedableEntropySource + 'static> {
    seed: Option<R::Seed>,
    _marker: PhantomData<&'static mut R>,
}

impl<R: SeedableEntropySource + 'static> EntropyPlugin<R>
where
    R::Seed: Send + Sync + Copy,
{
    /// Creates a new plugin instance configured for randomised,
    /// non-deterministic seeding of the global entropy resource.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            seed: None,
            _marker: PhantomData,
        }
    }

    /// Configures the plugin instance to have a set seed for the
    /// global entropy resource.
    #[inline]
    pub fn with_seed(mut self, seed: R::Seed) -> Self {
        self.seed = Some(seed);
        self
    }
}

impl<R: SeedableEntropySource + 'static> Default for EntropyPlugin<R>
where
    R::Seed: Send + Sync + Copy,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<R: SeedableEntropySource + 'static> Plugin for EntropyPlugin<R>
where
    R::Seed: Send + Sync + Copy,
{
    fn build(&self, app: &mut App) {
        #[cfg(feature = "bevy_reflect")]
        app.register_type::<GlobalEntropy<R>>()
            .register_type::<EntropyComponent<R>>();

        if let Some(seed) = self.seed {
            app.insert_resource(GlobalEntropy::<R>::from_seed(seed));
        } else {
            app.init_resource::<GlobalEntropy<R>>();
        }
    }

    fn is_unique(&self) -> bool {
        false
    }
}
