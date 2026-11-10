use crate::impls::macros::annotated;
use crate::type_registry::{ReflectDeserialize, ReflectSerialize};
#[cfg(feature = "annotations")]
use bevy_annotations::SerializedAs;
use bevy_reflect_derive::impl_reflect_opaque;

annotated!(@SerializedAs::STRING; impl_reflect_opaque!(::core::net::SocketAddr(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
)));
