use crate::{self as bevy_reflect, impl_reflect_opaque, ReflectDeserialize, ReflectSerialize};

impl_reflect_opaque!(::wgpu_types::TextureFormat(
    Debug,
    Hash,
    PartialEq,
    Deserialize,
    Serialize,
));
