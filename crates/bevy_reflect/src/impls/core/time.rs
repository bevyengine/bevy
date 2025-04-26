use crate::{
    std_traits::ReflectDefault,
    type_registry::{ReflectDeserialize, ReflectSerialize},
};
use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::core::time::Duration(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));

#[cfg(test)]
mod tests {
    use bevy_reflect::{ReflectSerialize, TypeRegistry};
    use core::time::Duration;

    #[test]
    fn can_serialize_duration() {
        let mut type_registry = TypeRegistry::default();
        type_registry.register::<Duration>();

        let reflect_serialize = type_registry
            .get_type_data::<ReflectSerialize>(core::any::TypeId::of::<Duration>())
            .unwrap();
        let _serializable = reflect_serialize.get_serializable(&Duration::ZERO);
    }
}
