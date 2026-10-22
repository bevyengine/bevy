use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::core::any::TypeId(Clone, Debug, Hash, PartialEq,));

#[cfg(test)]
mod tests {
    use bevy_reflect::FromDynamic;

    use crate::dynamic::DynamicValue;

    #[test]
    fn type_id_should_from_dynamic() {
        let type_id = core::any::TypeId::of::<usize>();
        let output = <core::any::TypeId as FromDynamic>::from_dynamic(
            DynamicValue::try_from_reflect(&type_id).unwrap(),
        )
        .unwrap();
        assert_eq!(type_id, output);
    }
}
