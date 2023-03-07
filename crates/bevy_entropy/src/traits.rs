use std::fmt::Debug;

use rand_core::{RngCore, SeedableRng};

#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serialize")]
pub trait EntropySource:
    RngCore
    + Clone
    + Debug
    + PartialEq
    + Sync
    + Send
    + Serialize
    + for<'a> Deserialize<'a>
    + private::SealedEntropy
{
}

#[cfg(feature = "serialize")]
impl<T> EntropySource for T where
    T: RngCore + Clone + Debug + PartialEq + Sync + Send + Serialize + for<'a> Deserialize<'a>
{
}

#[cfg(not(feature = "serialize"))]
pub trait EntropySource:
    RngCore + Clone + Debug + PartialEq + Sync + Send + private::SealedEntropy
{
}

#[cfg(not(feature = "serialize"))]
impl<T> EntropySource for T where T: RngCore + Clone + Debug + PartialEq + Sync + Send {}

#[cfg(feature = "serialize")]
pub trait SeedableEntropySource:
    RngCore
    + SeedableRng
    + Clone
    + Debug
    + PartialEq
    + Sync
    + Send
    + Serialize
    + for<'a> Deserialize<'a>
    + private::SealedSeedable
{
}

#[cfg(feature = "serialize")]
impl<T> SeedableEntropySource for T where
    T: RngCore
        + SeedableRng
        + Clone
        + Debug
        + PartialEq
        + Sync
        + Send
        + Serialize
        + for<'a> Deserialize<'a>
{
}

#[cfg(not(feature = "serialize"))]
pub trait SeedableEntropySource:
    RngCore + SeedableRng + Clone + Debug + PartialEq + Sync + Send + private::SealedSeedable
{
}

#[cfg(not(feature = "serialize"))]
impl<T> SeedableEntropySource for T where
    T: RngCore + SeedableRng + Clone + Debug + PartialEq + Sync + Send
{
}

mod private {
    pub trait SealedEntropy {}
    pub trait SealedSeedable {}

    impl<T: super::EntropySource> SealedEntropy for T {}
    impl<T: super::SeedableEntropySource> SealedSeedable for T {}
}
