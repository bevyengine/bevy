use crate::{self as bevy_reflect};
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_value;

impl_reflect_value!(::rand_chacha::ChaCha8Rng(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_chacha::ChaCha12Rng(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_chacha::ChaCha20Rng(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));
