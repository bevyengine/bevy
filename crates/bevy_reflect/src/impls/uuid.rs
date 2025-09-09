use crate::{std_traits::ReflectDefault, ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::uuid::Uuid(
    Serialize,
    Deserialize,
    Default,
    Clone,
    Debug,
    PartialEq,
    Hash
));

impl_reflect_opaque!(::uuid::NonNilUuid(
    Serialize,
    Deserialize,
    Clone,
    Debug,
    PartialEq,
    Hash
));
