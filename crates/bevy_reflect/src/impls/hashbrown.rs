use crate::impls::macros::{impl_reflect_for_hashmap, impl_reflect_for_hashset};
#[cfg(feature = "functions")]
use crate::{
    from_reflect::FromReflect, type_info::MaybeTyped, type_path::TypePath,
    type_registry::GetTypeRegistration,
};
use bevy_reflect_derive::impl_type_path;
#[cfg(feature = "functions")]
use core::hash::{BuildHasher, Hash};

impl_reflect_for_hashmap!(hashbrown::hash_map::HashMap<K, V, S>);
impl_type_path!(::hashbrown::hash_map::HashMap<K, V, S>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(::hashbrown::hash_map::HashMap<K, V, S>;
    <
        K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
        V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);

impl_reflect_for_hashset!(::hashbrown::hash_set::HashSet<V,S>);
impl_type_path!(::hashbrown::hash_set::HashSet<V, S>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(::hashbrown::hash_set::HashSet<V, S>;
    <
        V: Hash + Eq + FromReflect + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);

#[cfg(test)]
mod tests {
    use crate::Reflect;
    use static_assertions::assert_impl_all;

    #[test]
    fn should_reflect_hashmaps() {
        // We specify `foldhash::fast::RandomState` directly here since without the `default-hasher`
        // feature, hashbrown uses an empty enum to force users to specify their own
        assert_impl_all!(hashbrown::HashMap<u32, f32, foldhash::fast::RandomState>: Reflect);
    }
}
