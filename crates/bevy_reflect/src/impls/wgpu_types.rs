use crate::{self as bevy_reflect, impl_reflect_value, ReflectDeserialize, ReflectSerialize};
impl_reflect_value!(::wgpu_types::TextureFormat(
    Debug,
    Hash,
    PartialEq,
    Deserialize,
    Serialize,
));
