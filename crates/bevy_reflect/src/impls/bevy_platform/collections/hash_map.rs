use bevy_reflect_derive::impl_type_path;

use crate::impls::macros::impl_reflect_for_hashmap;
#[cfg(feature = "functions")]
use crate::{
    from_reflect::FromReflect, type_info::MaybeTyped, type_path::TypePath,
    type_registry::GetTypeRegistration,
};
#[cfg(feature = "functions")]
use core::hash::{BuildHasher, Hash};

impl_reflect_for_hashmap!(bevy_platform::collections::HashMap<K, V, S>);
impl_type_path!(::bevy_platform::collections::HashMap<K, V, S>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(::bevy_platform::collections::HashMap<K, V, S>;
    <
        K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
        V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);

#[cfg(test)]
mod tests {
    use crate::{PartialReflect, Reflect};
    use static_assertions::assert_impl_all;

    #[test]
    fn should_partial_eq_hash_map() {
        let mut a = <bevy_platform::collections::HashMap<_, _>>::default();
        a.insert(0usize, 1.23_f64);
        let b = a.clone();
        let mut c = <bevy_platform::collections::HashMap<_, _>>::default();
        c.insert(0usize, 3.21_f64);

        let a: &dyn PartialReflect = &a;
        let b: &dyn PartialReflect = &b;
        let c: &dyn PartialReflect = &c;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_reflect_hashmaps() {
        assert_impl_all!(bevy_platform::collections::HashMap<u32, f32>: Reflect);
    }
}
