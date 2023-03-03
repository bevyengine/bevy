use std::fmt::Debug;

use crate::{Deserialize, RngCore, SeedableRng, Serialize};
use bevy_ecs::prelude::Resource;
use bevy_reflect::{FromReflect, Reflect, ReflectDeserialize, ReflectSerialize};

#[derive(Debug, Clone, PartialEq, Eq, Resource, Reflect, FromReflect, Serialize, Deserialize)]
#[serde(bound(deserialize = "R: for<'a> Deserialize<'a>"))]
#[reflect_value(Debug, PartialEq, Serialize, Deserialize)]
pub struct GlobalEntropy<
    R: RngCore
        + Clone
        + Debug
        + PartialEq
        + Sync
        + Send
        + Serialize
        + for<'a> Deserialize<'a>
        + 'static,
>(R);

impl<
        R: RngCore
            + Clone
            + Debug
            + PartialEq
            + Sync
            + Send
            + Serialize
            + for<'a> Deserialize<'a>
            + 'static,
    > GlobalEntropy<R>
{
    #[inline]
    #[must_use]
    pub fn new(rng: R) -> Self {
        Self(rng)
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
    > GlobalEntropy<R>
{
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
    > Default for GlobalEntropy<R>
{
    fn default() -> Self {
        Self::from_entropy()
    }
}

impl<
        R: RngCore
            + Clone
            + Debug
            + PartialEq
            + Sync
            + Send
            + Serialize
            + for<'a> Deserialize<'a>
            + 'static,
    > From<R> for GlobalEntropy<R>
{
    fn from(value: R) -> Self {
        Self::new(value)
    }
}

impl<
        R: RngCore
            + Clone
            + Debug
            + PartialEq
            + Sync
            + Send
            + Serialize
            + for<'a> Deserialize<'a>
            + 'static,
    > RngCore for GlobalEntropy<R>
{
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
    > SeedableRng for GlobalEntropy<R>
{
    type Seed = R::Seed;

    fn from_seed(seed: Self::Seed) -> Self {
        Self::new(R::from_seed(seed))
    }
}
