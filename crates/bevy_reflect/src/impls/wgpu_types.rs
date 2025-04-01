use crate::{ReflectDeserialize, ReflectSerialize, impl_reflect_opaque};

impl_reflect_opaque!(::wgpu_types::TextureFormat(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Deserialize,
    Serialize,
));
