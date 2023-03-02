#![warn(clippy::undocumented_unsafe_blocks)]
#![doc = include_str!("../README.md")]

pub mod component;
mod entropy_source;
pub mod prelude;
pub mod resource;

use std::{fmt::Debug, marker::PhantomData};

use bevy_app::{App, Plugin};
use component::EntropyComponent;
use rand_core::{RngCore, SeedableRng};
use resource::GlobalEntropy;
use serde::{Deserialize, Serialize};

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
pub struct EntropyPlugin<
    R: RngCore
        + SeedableRng
        + Clone
        + Debug
        + PartialEq
        + Sync
        + Send
        + Serialize
        + for<'a> Deserialize<'a>
        + 'static,
> {
    seed: Option<R::Seed>,
    _marker: PhantomData<&'static mut R>,
}

impl<
        R: RngCore
            + SeedableRng
            + Clone
            + Debug
            + PartialEq
            + Sync
            + Send
            + Serialize
            + for<'a> Deserialize<'a>
            + 'static,
    > EntropyPlugin<R>
where
    R::Seed: Send + Sync + Copy,
{
    pub fn new() -> Self {
        Self {
            seed: None,
            _marker: PhantomData,
        }
    }

    pub fn with_seed(mut self, seed: R::Seed) -> Self {
        self.seed = Some(seed);
        self
    }
}

impl<
        R: RngCore
            + SeedableRng
            + Clone
            + Debug
            + PartialEq
            + Sync
            + Send
            + Serialize
            + for<'a> Deserialize<'a>
            + 'static,
    > Default for EntropyPlugin<R>
where
    R::Seed: Send + Sync + Copy,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<
        R: RngCore
            + SeedableRng
            + Clone
            + Debug
            + PartialEq
            + Sync
            + Send
            + Serialize
            + for<'a> Deserialize<'a>
            + 'static,
    > Plugin for EntropyPlugin<R>
where
    R::Seed: Send + Sync + Copy,
{
    fn build(&self, app: &mut App) {
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
