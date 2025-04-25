#[cfg(any(unix, windows))]
use crate::type_registry::{ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_opaque;

// `Serialize` and `Deserialize` only for platforms supported by serde:
// https://github.com/serde-rs/serde/blob/3ffb86fc70efd3d329519e2dddfa306cc04f167c/serde/src/de/impls.rs#L1732
#[cfg(any(unix, windows))]
impl_reflect_opaque!(::std::ffi::OsString(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));

#[cfg(not(any(unix, windows)))]
impl_reflect_opaque!(::std::ffi::OsString(Clone, Debug, Hash, PartialEq));
