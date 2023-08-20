use crate::{self as bevy_reflect};
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_value;

impl_reflect_value!(::wyrand::WyRand(Debug, PartialEq, Serialize, Deserialize));
