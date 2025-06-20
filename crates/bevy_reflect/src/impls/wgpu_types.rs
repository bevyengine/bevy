use crate::{std_traits::ReflectDefault, ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::wgpu_types::PrimitiveTopology(
    Clone,
    Debug,
    Default,
    Hash,
    PartialEq,
    Deserialize,
    Serialize,
));

impl_reflect_opaque!(::wgpu_types::TextureFormat(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Deserialize,
    Serialize,
));

impl_reflect_opaque!(::wgpu_types::VertexFormat(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Deserialize,
    Serialize,
));
