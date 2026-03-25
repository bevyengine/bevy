use bevy_reflect_derive::impl_type_path;

use crate::impls::macros::impl_reflect_for_hashset;
#[cfg(feature = "functions")]
use crate::{from_reflect::FromReflect, type_path::TypePath, type_registry::GetTypeRegistration};
#[cfg(feature = "functions")]
use core::hash::{BuildHasher, Hash};

impl_reflect_for_hashset!(::std::collections::HashSet<V,S>);
impl_type_path!(::std::collections::HashSet<V, S>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(::std::collections::HashSet<V, S>;
    <
        V: Hash + Eq + FromReflect + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);
