use std::fmt::Debug;

use crate::{Deserialize, RngCore, SeedableRng, Serialize};

pub trait EntropySource:
    RngCore + Clone + Debug + PartialEq + Sync + Send + Serialize + for<'a> Deserialize<'a>
{
}

impl<T> EntropySource for T where
    T: RngCore + Clone + Debug + PartialEq + Sync + Send + Serialize + for<'a> Deserialize<'a>
{
}

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
{
}

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
