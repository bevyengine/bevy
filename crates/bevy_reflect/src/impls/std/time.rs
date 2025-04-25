use crate::{
    std_traits::ReflectDefault,
    type_registry::{ReflectDeserialize, ReflectSerialize},
};
use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::core::time::Duration(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(::bevy_platform::time::Instant(
    Clone, Debug, Hash, PartialEq
));
