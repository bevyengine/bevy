use crate::{self as bevy_reflect};
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_value;

impl_reflect_value!(::rand_xoshiro::Xoshiro512StarStar(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_xoshiro::Xoshiro512PlusPlus(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_xoshiro::Xoshiro512Plus(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_xoshiro::Xoshiro256StarStar(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_xoshiro::Xoshiro256PlusPlus(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_xoshiro::Xoshiro256Plus(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_xoshiro::Xoshiro128StarStar(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_xoshiro::Xoshiro128PlusPlus(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_xoshiro::Xoshiro128Plus(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_xoshiro::Xoroshiro128StarStar(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_xoshiro::Xoroshiro128PlusPlus(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_xoshiro::Xoroshiro128Plus(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect_value!(::rand_xoshiro::SplitMix64(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));
