use crate::type_registry::{ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::core::net::SocketAddr(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
