use std::fmt::Debug;

use crate::traits::{EntropySource, SeedableEntropySource};
use bevy_ecs::prelude::Resource;
use rand_core::{RngCore, SeedableRng};

#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{FromReflect, Reflect, ReflectDeserialize, ReflectSerialize};

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect, FromReflect))]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serialize",
    serde(bound(deserialize = "R: for<'a> Deserialize<'a>"))
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect_value(Debug, PartialEq, Serialize, Deserialize)
)]
pub struct GlobalEntropy<R: EntropySource + 'static>(R);

impl<R: EntropySource + 'static> GlobalEntropy<R> {
    #[inline]
    #[must_use]
    pub fn new(rng: R) -> Self {
        Self(rng)
    }
}

impl<R: SeedableEntropySource + 'static> GlobalEntropy<R> {
    #[inline]
    #[must_use]
    pub fn from_entropy() -> Self {
        // Source entropy from system as there's only one Resource instance
        // globally, so the overhead of a single operation is neglible.
        Self(R::from_entropy())
    }

    #[inline]
    pub fn reseed(&mut self, seed: R::Seed) {
        self.0 = R::from_seed(seed);
    }
}

impl<R: SeedableEntropySource + 'static> Default for GlobalEntropy<R> {
    fn default() -> Self {
        Self::from_entropy()
    }
}

impl<R: EntropySource + 'static> RngCore for GlobalEntropy<R> {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    #[inline]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest);
    }

    #[inline]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.0.try_fill_bytes(dest)
    }
}

impl<R: SeedableEntropySource + 'static> SeedableRng for GlobalEntropy<R> {
    type Seed = R::Seed;

    fn from_seed(seed: Self::Seed) -> Self {
        Self::new(R::from_seed(seed))
    }
}

impl<R: EntropySource + 'static> From<R> for GlobalEntropy<R> {
    fn from(value: R) -> Self {
        Self::new(value)
    }
}

impl<R: SeedableEntropySource + 'static> From<&mut R> for GlobalEntropy<R> {
    fn from(value: &mut R) -> Self {
        Self::from_rng(value).unwrap()
    }
}
