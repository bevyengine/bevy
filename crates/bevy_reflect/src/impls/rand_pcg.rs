use crate::{self as bevy_reflect};
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_value;

impl_reflect_value!(::rand_pcg::Pcg32(Debug, PartialEq, Serialize, Deserialize));

impl_reflect_value!(::rand_pcg::Pcg64(Debug, PartialEq, Serialize, Deserialize));

impl_reflect_value!(::rand_pcg::Pcg64Mcg(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));
