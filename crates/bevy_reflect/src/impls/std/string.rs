use crate::{
    std_traits::ReflectDefault,
    type_registry::{ReflectDeserialize, ReflectSerialize},
};
use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::alloc::string::String(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
