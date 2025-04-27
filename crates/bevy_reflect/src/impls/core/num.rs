use crate::type_registry::{ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::core::num::NonZeroI128(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroU128(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroIsize(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroUsize(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroI64(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroU64(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroU32(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroI32(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroI16(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroU16(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroU8(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroI8(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::Wrapping<T: Clone + Send + Sync>(Clone));
impl_reflect_opaque!(::core::num::Saturating<T: Clone + Send + Sync>(Clone));

#[cfg(test)]
mod tests {
    use bevy_reflect::{FromReflect, PartialReflect};

    #[test]
    fn nonzero_usize_impl_reflect_from_reflect() {
        let a: &dyn PartialReflect = &core::num::NonZero::<usize>::new(42).unwrap();
        let b: &dyn PartialReflect = &core::num::NonZero::<usize>::new(42).unwrap();
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        let forty_two: core::num::NonZero<usize> = FromReflect::from_reflect(a).unwrap();
        assert_eq!(forty_two, core::num::NonZero::<usize>::new(42).unwrap());
    }
}
