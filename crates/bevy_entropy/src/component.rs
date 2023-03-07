use std::fmt::Debug;

use crate::{
    resource::GlobalEntropy,
    thread_local_entropy::ThreadLocalEntropy,
    traits::{EntropySource, SeedableEntropySource},
};
use bevy_ecs::{prelude::Component, system::ResMut, world::Mut};
use rand_core::{RngCore, SeedableRng};

#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{FromReflect, Reflect, ReflectDeserialize, ReflectSerialize};

#[derive(Debug, Clone, PartialEq, Eq, Component)]
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
pub struct EntropyComponent<R: EntropySource + 'static>(R);

impl<R: EntropySource + 'static> EntropyComponent<R> {
    #[inline]
    #[must_use]
    pub fn new(rng: R) -> Self {
        Self(rng)
    }
}

impl<R: SeedableEntropySource + 'static> EntropyComponent<R> {
    #[inline]
    #[must_use]
    pub fn from_entropy() -> Self {
        // Source entropy from thread local user-space RNG instead of
        // system entropy source to reduce overhead when creating many
        // rng instances for many entities at once.
        Self(R::from_rng(ThreadLocalEntropy).unwrap())
    }

    #[inline]
    pub fn reseed(&mut self, seed: R::Seed) {
        self.0 = R::from_seed(seed);
    }
}

impl<R: SeedableEntropySource + 'static> Default for EntropyComponent<R> {
    fn default() -> Self {
        Self::from_entropy()
    }
}

impl<R: EntropySource + 'static> RngCore for EntropyComponent<R> {
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

impl<R: SeedableEntropySource + 'static> SeedableRng for EntropyComponent<R> {
    type Seed = R::Seed;

    fn from_seed(seed: Self::Seed) -> Self {
        Self::new(R::from_seed(seed))
    }
}

impl<R: EntropySource + 'static> From<R> for EntropyComponent<R> {
    fn from(value: R) -> Self {
        Self::new(value)
    }
}

impl<R: SeedableEntropySource + 'static> From<&mut EntropyComponent<R>> for EntropyComponent<R> {
    fn from(rng: &mut EntropyComponent<R>) -> Self {
        Self::from_rng(rng).unwrap()
    }
}

impl<R: SeedableEntropySource + 'static> From<&mut Mut<'_, EntropyComponent<R>>>
    for EntropyComponent<R>
{
    fn from(rng: &mut Mut<'_, EntropyComponent<R>>) -> Self {
        Self::from(rng.as_mut())
    }
}

impl<R: SeedableEntropySource + 'static> From<&mut ResMut<'_, GlobalEntropy<R>>>
    for EntropyComponent<R>
{
    fn from(rng: &mut ResMut<'_, GlobalEntropy<R>>) -> Self {
        Self::from_rng(rng.as_mut()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use rand_chacha::ChaCha8Rng;

    use super::*;

    #[test]
    fn forking() {
        let mut rng1 = EntropyComponent::<ChaCha8Rng>::default();

        let rng2 = EntropyComponent::from(&mut rng1);

        assert_ne!(rng1, rng2, "forked EntropyComponents should not match each other");
    }
}
