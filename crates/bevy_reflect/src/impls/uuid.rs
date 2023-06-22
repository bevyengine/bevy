use crate as bevy_reflect;

use crate::{std_traits::ReflectDefault, ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::{impl_from_reflect_value, impl_reflect_value};
use bevy_utils::Uuid;

impl_reflect_value!(::bevy_utils::Uuid(
    Serialize,
    Deserialize,
    Default,
    Debug,
    PartialEq,
    Hash
));
impl_from_reflect_value!(Uuid);
