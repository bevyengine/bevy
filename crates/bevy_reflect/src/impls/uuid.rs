use crate as bevy_reflect;

use crate::{std_traits::ReflectDefault, ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_value;

impl_reflect_value!(::uuid::Uuid(
    Serialize,
    Deserialize,
    Default,
    Debug,
    PartialEq,
    Hash
));
