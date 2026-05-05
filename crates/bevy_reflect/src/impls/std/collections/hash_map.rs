use bevy_reflect_derive::impl_type_path;

use crate::impls::macros::impl_reflect_for_hashmap;
#[cfg(feature = "functions")]
use crate::{
    from_reflect::FromReflect, type_info::MaybeTyped, type_path::TypePath,
    type_registry::GetTypeRegistration,
};
#[cfg(feature = "functions")]
use core::hash::{BuildHasher, Hash};

impl_reflect_for_hashmap!(::std::collections::HashMap<K, V, S>);
impl_type_path!(::std::collections::hash_map::RandomState);
impl_type_path!(::std::collections::HashMap<K, V, S>);

#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(::std::collections::HashMap<K, V, S>;
    <
        K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
        V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);

#[cfg(test)]
mod tests {
    use crate::Reflect;
    use static_assertions::assert_impl_all;

    #[test]
    fn should_reflect_hashmaps() {
        assert_impl_all!(std::collections::HashMap<u32, f32>: Reflect);
    }
}
