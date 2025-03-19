use crate::{impl_reflect_opaque, ReflectDeserialize, ReflectSerialize};

impl_reflect_opaque!(::wgpu_types::TextureFormat(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Deserialize,
    Serialize,
));
