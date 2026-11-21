use crate::{
    impl_reflect_opaque, std_traits::ReflectDefault, ReflectDeserialize, ReflectSerialize,
};

impl_reflect_opaque!(::wgpu_types::TextureFormat(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Deserialize,
    Serialize,
));
impl_reflect_opaque!(::wgpu_types::BlendState(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Deserialize,
    Serialize,
));
impl_reflect_opaque!(::wgpu_types::Face(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Deserialize,
    Serialize,
));
impl_reflect_opaque!(::wgpu_types::DepthBiasState(
    Clone,
    Debug,
    Default,
    PartialEq,
    Deserialize,
    Serialize,
));
