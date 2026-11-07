use crate::{std_traits::ReflectDefault, ReflectDeserialize, ReflectSerialize};
use bevy_annotations::SerializedAs;
use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::uuid::Uuid(
    Serialize,
    Deserialize,
    Default,
    Clone,
    Debug,
    PartialEq,
    PartialOrd,
    Hash,
    @SerializedAs::String { format: Some("uuid"), pattern: None }
));

impl_reflect_opaque!(::uuid::NonNilUuid(
    Serialize,
    Deserialize,
    Clone,
    Debug,
    PartialEq,
    Hash,
    @SerializedAs::String { format: Some("uuid"), pattern: None }
));
