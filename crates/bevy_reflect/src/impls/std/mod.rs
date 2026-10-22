mod collections;
mod ffi;
mod path;

#[cfg(test)]
mod tests {
    use crate::dynamic::DynamicValue;
    use crate::{FromDynamic, PartialReflect};
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn should_partial_eq_hash_map() {
        let mut a = <HashMap<_, _>>::default();
        a.insert(0usize, 1.23_f64);
        let b = a.clone();
        let mut c = <HashMap<_, _>>::default();
        c.insert(0usize, 3.21_f64);

        let a: &dyn PartialReflect = &a;
        let b: &dyn PartialReflect = &b;
        let c: &dyn PartialReflect = &c;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn path_should_from_dynamic() {
        let path = Path::new("hello_world.rs");
        let output = <&'static Path as FromDynamic>::from_dynamic(
            DynamicValue::try_from_reflect(&path).unwrap(),
        )
        .unwrap();
        assert_eq!(path, output);
    }

    #[test]
    fn type_id_should_from_dynamic() {
        let type_id = core::any::TypeId::of::<usize>();
        let output = <core::any::TypeId as FromDynamic>::from_dynamic(
            DynamicValue::try_from_reflect(&type_id).unwrap(),
        )
        .unwrap();
        assert_eq!(type_id, output);
    }

    #[test]
    fn static_str_should_from_dynamic() {
        let expected = "Hello, World!";
        let output = <&'static str as FromDynamic>::from_dynamic(
            DynamicValue::try_from_reflect(&expected).unwrap(),
        )
        .unwrap();
        assert_eq!(expected, output);
    }
}
