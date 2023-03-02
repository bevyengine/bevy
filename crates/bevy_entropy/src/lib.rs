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
