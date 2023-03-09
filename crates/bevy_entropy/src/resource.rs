use std::fmt::Debug;

use crate::traits::{EntropySource, SeedableEntropySource};
use bevy_ecs::prelude::Resource;
use rand_core::{RngCore, SeedableRng};

#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{FromReflect, Reflect, ReflectDeserialize, ReflectSerialize};

/// A Global [`RngCore`] instance, meant for use as a Resource. Gets
/// created automatically with [`crate::plugin::EntropyPlugin`], or
/// can be created and added manually.
///
/// # Example
///
/// ```
/// use bevy_ecs::prelude::ResMut;
/// use bevy_entropy::prelude::*;
/// use rand_core::RngCore;
/// use rand_chacha::ChaCha8Rng;
///
/// fn print_random_value(mut rng: ResMut<GlobalEntropy<ChaCha8Rng>>) {
///   println!("Random value: {}", rng.next_u32());
/// }
/// ```
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
    /// Create a new resource from a `RngCore` instance.
    #[inline]
    #[must_use]
    pub fn new(rng: R) -> Self {
        Self(rng)
    }
}

impl<R: SeedableEntropySource + 'static> GlobalEntropy<R> {
    /// Create a new resource with an `RngCore` instance seeded
    /// from a local entropy source. Generates a randomised,
    /// non-deterministic seed for the resource.
    #[inline]
    #[must_use]
    pub fn from_entropy() -> Self {
        // Source entropy from system as there's only one Resource instance
        // globally, so the overhead of a single operation is neglible.
        Self(R::from_entropy())
    }

    /// Reseeds the internal `RngCore` instance with a new seed.
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

#[cfg(test)]
mod tests {
    use rand_chacha::ChaCha8Rng;

    use super::*;

    #[test]
    fn rng_reflection() {
        use bevy_reflect::{
            serde::{ReflectSerializer, UntypedReflectDeserializer},
            TypeRegistry,
        };
        use ron::ser::to_string;
        use serde::de::DeserializeSeed;

        let mut registry = TypeRegistry::default();
        registry.register::<GlobalEntropy<ChaCha8Rng>>();

        let mut val = GlobalEntropy::<ChaCha8Rng>::from_seed([7; 32]);

        // Modify the state of the RNG instance
        val.next_u32();

        let ser = ReflectSerializer::new(&val, &registry);

        let serialized = to_string(&ser).unwrap();

        assert_eq!(
            &serialized,
            "{\"bevy_entropy::resource::GlobalEntropy<rand_chacha::chacha::ChaCha8Rng>\":((seed:(7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7),stream:0,word_pos:1))}"
        );

        let mut deserializer = ron::Deserializer::from_str(&serialized).unwrap();

        let de = UntypedReflectDeserializer::new(&registry);

        let value = de.deserialize(&mut deserializer).unwrap();

        let mut dynamic = value.take::<GlobalEntropy<ChaCha8Rng>>().unwrap();

        // The two instances should be the same
        assert_eq!(val, dynamic, "The deserialized GlobalEntropy should equal the original");
        // They should output the same numbers, as no state is lost between serialization and deserialization.
        assert_eq!(val.next_u32(), dynamic.next_u32(), "The deserialized GlobalEntropy should have the same output as original");
    }
}
