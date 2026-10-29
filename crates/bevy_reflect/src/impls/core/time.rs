use crate::{
    std_traits::{ReflectAdd, ReflectAddAssign, ReflectDefault, ReflectSub, ReflectSubAssign},
    type_registry::{ReflectDeserialize, ReflectSerialize},
};
use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::core::time::Duration(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
));

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use bevy_reflect::{ReflectSerialize, TypeRegistry};
    use core::{any::TypeId, time::Duration};

    use crate::prelude::{ReflectAdd, ReflectAddAssign, ReflectSub, ReflectSubAssign};

    #[test]
    fn can_serialize_duration() {
        let mut type_registry = TypeRegistry::default();
        type_registry.register::<Duration>();

        let reflect_serialize = type_registry
            .get_type_data::<ReflectSerialize>(TypeId::of::<Duration>())
            .unwrap();
        let _serializable = reflect_serialize.get_serializable(&Duration::ZERO);
    }

    #[test]
    fn should_math_ops_duration() {
        let mut registry = TypeRegistry::new();
        registry.register::<Duration>();

        let reflect_add = registry
            .get_type_data::<ReflectAdd>(TypeId::of::<Duration>())
            .unwrap();
        let reflect_add_assign = registry
            .get_type_data::<ReflectAddAssign>(TypeId::of::<Duration>())
            .unwrap();
        let reflect_sub = registry
            .get_type_data::<ReflectSub>(TypeId::of::<Duration>())
            .unwrap();
        let reflect_sub_assign = registry
            .get_type_data::<ReflectSubAssign>(TypeId::of::<Duration>())
            .unwrap();

        let mut a = Duration::from_secs(10);
        let b = Duration::from_secs(4);

        assert_eq!(
            reflect_add
                .add(Box::new(a), Box::new(b))
                .unwrap()
                .reflect_partial_eq(&Duration::from_secs(14)),
            Some(true)
        );
        assert_eq!(
            reflect_sub
                .sub(Box::new(a), Box::new(b))
                .unwrap()
                .reflect_partial_eq(&Duration::from_secs(6)),
            Some(true)
        );

        reflect_add_assign.add_assign(&mut a, Box::new(b)).unwrap();
        assert_eq!(a, Duration::from_secs(14));

        reflect_sub_assign.sub_assign(&mut a, Box::new(b)).unwrap();
        assert_eq!(a, Duration::from_secs(10));
    }
}
