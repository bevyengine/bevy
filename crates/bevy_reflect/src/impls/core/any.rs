use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::core::any::TypeId(Clone, Debug, Hash, PartialEq,));

#[cfg(test)]
mod tests {
    use bevy_reflect::FromReflect;

    #[test]
    fn type_id_should_from_reflect() {
        let type_id = core::any::TypeId::of::<usize>();
        let output = <core::any::TypeId as FromReflect>::from_reflect(&type_id).unwrap();
        assert_eq!(type_id, output);
    }
}
